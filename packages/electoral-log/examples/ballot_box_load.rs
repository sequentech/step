// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Load test for the ballot box.
//!
//! Accepts votes through `PostgresStore::accept_ballot`, the statement every vote of
//! a ballot-box event runs, from many clients at once, and times the reads that
//! voters, dashboards, the voter list, the console and the tally make of an event's
//! ballot box. Connection settings come from the `ELECTORAL_LOG_PG_*` variables, and
//! `--database` picks the database. Run it against a disposable database:
//!
//! ```text
//! cargo run --release --example ballot_box_load -- accept --seconds 300 --clients 32
//! cargo run --release --example ballot_box_load -- sequence --event <id>
//! cargo run --release --example ballot_box_load -- settle --event <id>
//! cargo run --release --example ballot_box_load -- reads --event <id>
//! ```
//!
//! Voters are named `voter-<n>`, and each votes in one area, chosen from its number.

use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use electoral_log::adapters::ballot_box::{
    AcceptBallot, AcceptOutcome, BallotStatus, PendingBallot,
};
use electoral_log::adapters::ballot_box_reads::{
    tally_ballots_query, BallotIdMatch, BucketRange, IpBallotsFilter,
};
use electoral_log::adapters::console::{ConsoleFilters, ConsoleTable, PageOrder, PageRequest};
use electoral_log::adapters::postgres::{PostgresConnection, PostgresStore};
use electoral_log::messages::message::{Message, SigningData};
use electoral_log::messages::newtypes::{
    CastVoteHash, ElectionIdString, EventIdString, PseudonymHash, VoterCountryString,
    VoterIpString, VotingChannelString,
};
use electoral_log::ports::ElectoralLogStore;
use electoral_log::LogEntry;
use sha2::{Digest, Sha512};
use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use strand::signature::StrandSignatureSk;
use tokio::task::JoinSet;
use uuid::Uuid;

/// Different contents each client cycles through.
const CONTENTS_PER_CLIENT: usize = 16;
/// Errors printed in full; later ones are only counted.
const PRINTED_ERRORS: u64 = 5;

#[derive(Parser)]
struct Cli {
    /// Database to use; the one of `ELECTORAL_LOG_PG_DATABASE` when absent.
    #[arg(long, global = true)]
    database: Option<String>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Accept votes from concurrent clients and report throughput and latency.
    Accept(AcceptArgs),
    /// Append the records of an event's queued ballots to a board as Windmill's
    /// sequencer does, and report records per second.
    Sequence {
        #[arg(long)]
        event: String,
        /// Board to append to; one named after the event when absent.
        #[arg(long)]
        board: Option<String>,
        #[arg(long, default_value_t = 5_000)]
        batch: i64,
        /// Stop after this many seconds, or once the queue is empty.
        #[arg(long, default_value_t = 600)]
        seconds: u64,
        /// Wait this long for more ballots when the queue is empty, before stopping.
        #[arg(long, default_value_t = 0)]
        linger: u64,
    },
    /// Empty an event's sequencer queue, as if the sequencer had appended every ballot.
    Settle {
        #[arg(long)]
        event: String,
    },
    /// Time the reads of an event's ballot box.
    Reads(ReadsArgs),
}

#[derive(Args)]
struct AcceptArgs {
    /// Election event; a new one, with a new ballot box, when absent.
    #[arg(long)]
    event: Option<String>,
    /// Elections of the event; generated ones when absent.
    #[arg(long, value_delimiter = ',')]
    elections: Vec<String>,
    /// Areas of the event; generated ones when absent.
    #[arg(long, value_delimiter = ',')]
    areas: Vec<String>,
    #[arg(long, default_value_t = 5)]
    election_count: u64,
    #[arg(long, default_value_t = 1000)]
    area_count: u64,
    #[arg(long, default_value_t = 60)]
    seconds: u64,
    #[arg(long, default_value_t = 32)]
    clients: usize,
    /// Size of each ballot's content.
    #[arg(long, default_value_t = 2048)]
    bytes: usize,
    #[arg(long, default_value_t = 1_000_000)]
    voters: u64,
    /// Votes a voter may cast in an election; 0 means unlimited.
    #[arg(long, default_value_t = 3)]
    allowed_votes: i32,
    /// Votes per second to aim at, over all clients; as many as possible when absent.
    #[arg(long)]
    rate: Option<f64>,
    #[arg(long, default_value_t = 10)]
    report_every: u64,
    /// Create and drop the ballot box of another event every this many seconds.
    #[arg(long)]
    churn_every: Option<u64>,
}

#[derive(Args)]
struct ReadsArgs {
    #[arg(long)]
    event: String,
    /// Voters the accept runs used, to pick existing ones.
    #[arg(long, default_value_t = 1_000_000)]
    voters: u64,
    /// Times each read runs.
    #[arg(long, default_value_t = 20)]
    repeat: usize,
    /// Areas whose tally input is read.
    #[arg(long, default_value_t = 3)]
    tally_areas: usize,
}

/// A small, fast generator; the load test needs spread, not unpredictability.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed ^ 0x9e37_79b9_7f4a_7c15)
    }

    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    fn below(&mut self, bound: u64) -> u64 {
        self.next() % bound.max(1)
    }
}

/// A version-4 UUID derived from a seed and a number, so that runs agree on IDs.
fn derived_uuid(seed: &str, n: u64) -> String {
    let digest = Sha512::digest(format!("{seed}:{n}").as_bytes());
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Uuid::from_bytes(bytes).to_string()
}

fn voter_id(n: u64) -> String {
    format!("voter-{n:08}")
}

fn percentile(sorted: &[u32], fraction: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let index = ((sorted.len() - 1) as f64 * fraction).round() as usize;
    sorted[index] as f64 / 1000.0
}

#[derive(Default)]
struct Counters {
    accepted: AtomicU64,
    refused: AtomicU64,
    errors: AtomicU64,
}

/// What one client did.
#[derive(Default)]
struct ClientReport {
    /// Microseconds per accepted vote.
    latencies: Vec<u32>,
    too_many: u64,
    other_area: u64,
    duplicates: u64,
}

struct Plan {
    event: String,
    elections: Vec<String>,
    areas: Vec<String>,
    voters: u64,
    allowed_votes: i32,
    bytes: usize,
    deadline: Instant,
    /// Time between two votes of one client, when the rate is limited.
    interval: Option<Duration>,
}

async fn client_loop(
    store: PostgresStore,
    plan: Arc<Plan>,
    counters: Arc<Counters>,
    number: u64,
) -> ClientReport {
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_nanos() as u64)
        .unwrap_or_default();
    let mut rng = Rng::new(number.wrapping_mul(7919) ^ seed);
    let alphabet = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let contents: Vec<String> = (0..CONTENTS_PER_CLIENT)
        .map(|_| {
            (0..plan.bytes)
                .map(|_| alphabet[rng.below(alphabet.len() as u64) as usize] as char)
                .collect()
        })
        .collect();
    let mut report = ClientReport::default();
    let mut next = Instant::now();
    while Instant::now() < plan.deadline {
        if let Some(interval) = plan.interval {
            let now = Instant::now();
            if next > now {
                tokio::time::sleep(next - now).await;
            }
            next += interval;
        }
        let n = rng.below(plan.voters);
        let voter = voter_id(n);
        let election = &plan.elections[rng.below(plan.elections.len() as u64) as usize];
        let area = &plan.areas[(n % plan.areas.len() as u64) as usize];
        let ballot_id = Uuid::new_v4().simple().to_string();
        let content = &contents[rng.below(CONTENTS_PER_CLIENT as u64) as usize];
        let pseudonym_hash = Sha512::digest(voter.as_bytes());
        let ballot_hash = Sha512::digest(ballot_id.as_bytes());
        let ip = format!("192.0.2.{}", n % 250);
        let ballot = AcceptBallot {
            election_event_id: &plan.event,
            election_id: election,
            area_id: area,
            voter_id: &voter,
            ballot_id: &ballot_id,
            format: "load-test",
            content,
            voter_signature: None,
            pseudonym_hash: &pseudonym_hash,
            ballot_hash: &ballot_hash,
            voting_channel: "ONLINE",
            status: BallotStatus::Valid,
            voter_ip: Some(ip.as_str()),
            voter_country: Some("ES"),
            username: Some(voter.as_str()),
            allowed_votes: plan.allowed_votes,
        };
        let started = Instant::now();
        match store.accept_ballot(&ballot).await {
            Ok(AcceptOutcome::Accepted { .. }) => {
                report
                    .latencies
                    .push(started.elapsed().as_micros().min(u32::MAX as u128) as u32);
                counters.accepted.fetch_add(1, Ordering::Relaxed);
            }
            Ok(outcome) => {
                match outcome {
                    AcceptOutcome::TooManyVotes => report.too_many += 1,
                    AcceptOutcome::VotedInOtherArea => report.other_area += 1,
                    _ => report.duplicates += 1,
                }
                counters.refused.fetch_add(1, Ordering::Relaxed);
            }
            Err(error) => {
                if counters.errors.fetch_add(1, Ordering::Relaxed) < PRINTED_ERRORS {
                    eprintln!("accept failed: {error:#}");
                }
            }
        }
    }
    report
}

async fn pending(store: &PostgresStore, event: &str) -> Result<i64> {
    store.pending_count(event).await
}

async fn accept(store: PostgresStore, args: AcceptArgs) -> Result<()> {
    let event = match args.event {
        Some(event) => event,
        None => {
            let event = Uuid::new_v4().to_string();
            store.create_ballot_box(&event).await?;
            event
        }
    };
    let elections = if args.elections.is_empty() {
        (0..args.election_count)
            .map(|n| derived_uuid(&format!("{event}:election"), n))
            .collect()
    } else {
        args.elections
    };
    let areas = if args.areas.is_empty() {
        (0..args.area_count)
            .map(|n| derived_uuid(&format!("{event}:area"), n))
            .collect()
    } else {
        args.areas
    };
    println!(
        "event {event}: {} elections, {} areas, {} voters, {} clients, {} bytes, {} s",
        elections.len(),
        areas.len(),
        args.voters,
        args.clients,
        args.bytes,
        args.seconds
    );
    let started = Instant::now();
    let plan = Arc::new(Plan {
        event: event.clone(),
        elections,
        areas,
        voters: args.voters,
        allowed_votes: args.allowed_votes,
        bytes: args.bytes,
        deadline: started + Duration::from_secs(args.seconds),
        interval: args
            .rate
            .map(|rate| Duration::from_secs_f64(args.clients as f64 / rate)),
    });
    let counters = Arc::new(Counters::default());
    let mut clients = JoinSet::new();
    for number in 0..args.clients {
        clients.spawn(client_loop(
            store.clone(),
            plan.clone(),
            counters.clone(),
            number as u64,
        ));
    }

    let churn = args.churn_every.map(|every| {
        let store = store.clone();
        let deadline = plan.deadline;
        tokio::spawn(async move {
            let mut timings = Vec::new();
            while Instant::now() + Duration::from_secs(every) < deadline {
                tokio::time::sleep(Duration::from_secs(every)).await;
                let other = Uuid::new_v4().to_string();
                let created = Instant::now();
                store.create_ballot_box(&other).await?;
                let create = created.elapsed();
                let dropped = Instant::now();
                store.drop_ballot_box(&other).await?;
                let drop = dropped.elapsed();
                println!(
                    "churn: created a ballot box in {} ms, dropped it in {} ms",
                    create.as_millis(),
                    drop.as_millis()
                );
                timings.push((create, drop));
            }
            anyhow::Ok(timings)
        })
    });

    let mut last = (started, 0u64);
    while Instant::now() < plan.deadline {
        let wake = (last.0 + Duration::from_secs(args.report_every)).min(plan.deadline);
        tokio::time::sleep_until(wake.into()).await;
        let accepted = counters.accepted.load(Ordering::Relaxed);
        let now = Instant::now();
        let rate = (accepted - last.1) as f64 / (now - last.0).as_secs_f64();
        let queued = pending(&store, &event).await.unwrap_or(-1);
        println!(
            "{:>5.0} s: {:>8.0} votes/s, {:>9} accepted, {:>7} refused, {:>4} errors, {:>9} queued",
            (now - started).as_secs_f64(),
            rate,
            accepted,
            counters.refused.load(Ordering::Relaxed),
            counters.errors.load(Ordering::Relaxed),
            queued
        );
        last = (now, accepted);
    }

    let mut latencies = Vec::new();
    let (mut too_many, mut other_area, mut duplicates) = (0, 0, 0);
    while let Some(report) = clients.join_next().await {
        let report = report?;
        latencies.extend(report.latencies);
        too_many += report.too_many;
        other_area += report.other_area;
        duplicates += report.duplicates;
    }
    let elapsed = started.elapsed().as_secs_f64();
    latencies.sort_unstable();
    println!(
        "summary: {} accepted in {:.1} s = {:.0} votes/s; latency ms p50 {:.2} p95 {:.2} p99 {:.2} max {:.2}; \
         refused: {} over the limit, {} other area, {} duplicate; {} errors; {} queued",
        latencies.len(),
        elapsed,
        latencies.len() as f64 / elapsed,
        percentile(&latencies, 0.50),
        percentile(&latencies, 0.95),
        percentile(&latencies, 0.99),
        percentile(&latencies, 1.0),
        too_many,
        other_area,
        duplicates,
        counters.errors.load(Ordering::Relaxed),
        pending(&store, &event).await?
    );
    if let Some(churn) = churn {
        let timings = churn.await??;
        let worst = |pick: fn(&(Duration, Duration)) -> Duration| {
            timings
                .iter()
                .map(pick)
                .max()
                .unwrap_or_default()
                .as_millis()
        };
        println!(
            "churn: {} ballot boxes created and dropped; slowest create {} ms, slowest drop {} ms",
            timings.len(),
            worst(|timing| timing.0),
            worst(|timing| timing.1)
        );
    }
    Ok(())
}

/// The cast-vote record of an accepted ballot, built as Windmill's sequencer builds
/// it (`windmill::services::ballot_box::ballot_record`), signed by the sender and the
/// system.
fn ballot_record(
    event: &str,
    ballot: &PendingBallot,
    signing_key: &StrandSignatureSk,
) -> Result<LogEntry> {
    let signing_data = SigningData::new(signing_key.clone(), &ballot.voter_id, signing_key.clone());
    let pseudonym: [u8; 64] = ballot.pseudonym_hash.as_slice().try_into()?;
    let hash: [u8; 64] = ballot.ballot_hash.as_slice().try_into()?;
    let message = Message::cast_vote_with_channel_message(
        EventIdString(event.to_string()),
        ElectionIdString(Some(ballot.election_id.clone())),
        PseudonymHash::new(pseudonym),
        CastVoteHash::new(hash),
        &signing_data,
        VoterIpString(format!("ip: {}", ballot.voter_ip.as_deref().unwrap_or(""))),
        VoterCountryString(format!(
            "country: {}",
            ballot.voter_country.as_deref().unwrap_or("")
        )),
        VotingChannelString(ballot.voting_channel.clone()),
        Some(ballot.voter_id.clone()),
        ballot.username.clone(),
        ballot.area_id.clone(),
    )?;
    Ok(LogEntry {
        delivery_id: format!("ballot-box:{event}:{}", ballot.seq),
        message: (&message).try_into()?,
    })
}

async fn sequence(
    store: PostgresStore,
    event: String,
    board: Option<String>,
    batch: i64,
    seconds: u64,
    linger: u64,
) -> Result<()> {
    let board = board.unwrap_or_else(|| format!("loadtest{}", event.replace('-', "")));
    store.create_board(&board).await?;
    let signing_key = StrandSignatureSk::generate()?;
    let started = Instant::now();
    let deadline = started + Duration::from_secs(seconds);
    let mut idle_since: Option<Instant> = None;
    let (mut records, mut batches) = (0usize, 0usize);
    let (mut read, mut build, mut append, mut remove) = (
        Duration::ZERO,
        Duration::ZERO,
        Duration::ZERO,
        Duration::ZERO,
    );
    let mut last_report = (Instant::now(), 0usize);
    while Instant::now() < deadline {
        let phase = Instant::now();
        let pending = store.pending_ballots(&event, batch).await?;
        read += phase.elapsed();
        if pending.is_empty() {
            let since = *idle_since.get_or_insert_with(Instant::now);
            if since.elapsed() >= Duration::from_secs(linger) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
            continue;
        }
        idle_since = None;
        let phase = Instant::now();
        let entries = pending
            .iter()
            .map(|ballot| ballot_record(&event, ballot, &signing_key))
            .collect::<Result<Vec<_>>>()?;
        build += phase.elapsed();
        let phase = Instant::now();
        store
            .append(&board, &mut entries.into_iter().map(anyhow::Ok))
            .await?;
        append += phase.elapsed();
        let seqs: Vec<i64> = pending.iter().map(|ballot| ballot.seq).collect();
        let phase = Instant::now();
        store.remove_pending(&event, &seqs).await?;
        remove += phase.elapsed();
        records += seqs.len();
        batches += 1;
        if last_report.0.elapsed() >= Duration::from_secs(10) {
            println!(
                "{:>5.0} s: {:>7.0} records/s, {:>9} appended, {:>9} queued",
                started.elapsed().as_secs_f64(),
                (records - last_report.1) as f64 / last_report.0.elapsed().as_secs_f64(),
                records,
                store.pending_count(&event).await?
            );
            last_report = (Instant::now(), records);
        }
    }
    let busy = read + build + append + remove;
    println!(
        "sequenced {records} records in {batches} batches, {:.1} s busy = {:.0} records/s; \
         read {:.0} %, build and sign {:.0} %, append {:.0} %, remove {:.0} %",
        busy.as_secs_f64(),
        records as f64 / busy.as_secs_f64().max(1e-9),
        100.0 * read.as_secs_f64() / busy.as_secs_f64().max(1e-9),
        100.0 * build.as_secs_f64() / busy.as_secs_f64().max(1e-9),
        100.0 * append.as_secs_f64() / busy.as_secs_f64().max(1e-9),
        100.0 * remove.as_secs_f64() / busy.as_secs_f64().max(1e-9),
    );
    Ok(())
}

/// Run a read `repeat` times and print its median and slowest time.
async fn timed<T, F, Fut>(name: &str, repeat: usize, mut read: F) -> Result<T>
where
    F: FnMut(usize) -> Fut,
    Fut: Future<Output = Result<T>>,
{
    let mut times = Vec::new();
    let mut last = None;
    for attempt in 0..repeat.max(1) {
        let started = Instant::now();
        last = Some(
            read(attempt)
                .await
                .with_context(|| format!("{name} failed"))?,
        );
        times.push(started.elapsed().as_micros() as u32);
    }
    times.sort_unstable();
    println!(
        "{name:<44} median {:>9.2} ms  max {:>9.2} ms  ({} runs)",
        percentile(&times, 0.5),
        percentile(&times, 1.0),
        times.len()
    );
    Ok(last.expect("at least one run"))
}

async fn reads(store: PostgresStore, args: ReadsArgs) -> Result<()> {
    let event = args.event.as_str();
    let client = store.client().await?;
    let suffix = event.replace('-', "");
    let row = client
        .query_one(
            &format!(
                "SELECT pg_total_relation_size('ballot_box_ballot_{suffix}'), \
                        pg_total_relation_size('ballot_box_voter_{suffix}'), \
                        (SELECT count(*) FROM ballot_box_pending WHERE election_event_id = $1::text::uuid)"
            ),
            &[&event],
        )
        .await?;
    let (ballot_bytes, voter_bytes, queued): (i64, i64, i64) = (row.get(0), row.get(1), row.get(2));
    let counted = Instant::now();
    let ballots: i64 = client
        .query_one(
            "SELECT count(*) FROM ballot_box_ballot WHERE election_event_id = $1::text::uuid",
            &[&event],
        )
        .await?
        .get(0);
    println!(
        "event {event}: {ballots} ballots ({} MB with indexes, counted in {} ms), voter rows {} MB, {queued} queued",
        ballot_bytes / 1_000_000,
        counted.elapsed().as_millis(),
        voter_bytes / 1_000_000
    );
    let elections: Vec<String> = client
        .query(
            "SELECT DISTINCT election_id::text FROM ballot_box_voter \
             WHERE election_event_id = $1::text::uuid ORDER BY 1",
            &[&event],
        )
        .await?
        .iter()
        .map(|row| row.get(0))
        .collect();
    let election = elections.first().context("The event has no votes")?.clone();
    let areas: Vec<String> = client
        .query(
            "SELECT DISTINCT area_id::text FROM (SELECT area_id FROM ballot_box_voter \
             WHERE election_event_id = $1::text::uuid AND election_id = $2::text::uuid LIMIT 10000) a \
             ORDER BY 1 LIMIT $3",
            &[&event, &election, &(args.tally_areas as i64)],
        )
        .await?
        .iter()
        .map(|row| row.get(0))
        .collect();
    drop(client);

    let mut rng = Rng::new(42);
    let picks: Vec<String> = (0..args.repeat.max(50))
        .map(|_| voter_id(rng.below(args.voters)))
        .collect();
    let found = timed("voter status (voter_ballots)", args.repeat, |n| {
        let store = store.clone();
        let voter = picks[n].clone();
        async move { store.voter_ballots(event, &voter).await }
    })
    .await?;
    if let Some(ballot) = found.first() {
        timed(
            "ballot locator (voter_ballot_contents)",
            args.repeat,
            |_| {
                store.voter_ballot_contents(
                    event,
                    &ballot.voter_id,
                    &ballot.election_id,
                    BallotIdMatch::Exact(&ballot.ballot_id),
                )
            },
        )
        .await?;
    }
    let page_of_voters: Vec<String> = picks.iter().take(50).cloned().collect();
    timed(
        "voter list page, 50 voters (votes_of_voters)",
        args.repeat,
        |_| store.votes_of_voters(event, &page_of_voters, None),
    )
    .await?;
    let repeat_scans = args.repeat.min(5);
    timed("participation of the event", repeat_scans, |_| {
        store.participation(event, None)
    })
    .await?;
    timed("participation of an election", repeat_scans, |_| {
        store.participation(event, Some(&election))
    })
    .await?;
    timed("voters by channel", repeat_scans, |_| {
        store.voters_by_channel(event, None)
    })
    .await?;
    let range = BucketRange {
        resolution: "hour",
        time_zone: "UTC",
        start: "",
        end: "",
        last_buckets: Some(24),
    };
    timed("ballots per hour, last 24", repeat_scans, |_| {
        store.ballots_per_bucket(event, None, &range, "ONLINE")
    })
    .await?;
    let first_50 = IpBallotsFilter {
        limit: 50,
        ..Default::default()
    };
    timed("ballots by IP address, first 50", repeat_scans, |_| {
        store.ballots_by_ip(event, &first_50)
    })
    .await?;

    let store = &store;
    let filters = ConsoleFilters::default();
    let request = |table, after| PageRequest {
        table,
        board: "",
        election_event_id: event,
        filters: &filters,
        order: PageOrder::NewestFirst,
        after,
        limit: 25,
    };
    let first = timed("console: first page of ballots", args.repeat, |_| {
        let page = request(ConsoleTable::Ballots, None);
        async move { store.console_page(&page).await }
    })
    .await?;
    if let Some(next) = first.next.as_deref() {
        timed("console: next page of ballots", args.repeat, |_| {
            let page = request(ConsoleTable::Ballots, Some(next));
            async move { store.console_page(&page).await }
        })
        .await?;
    }
    timed("console: first page of voters", args.repeat, |_| {
        let page = request(ConsoleTable::Voters, None);
        async move { store.console_page(&page).await }
    })
    .await?;
    let status = ConsoleFilters {
        status: Some("rejected".into()),
        ..Default::default()
    };
    timed(
        "console: rejected ballots (none match)",
        repeat_scans,
        |_| {
            let page = PageRequest {
                filters: &status,
                ..request(ConsoleTable::Ballots, None)
            };
            async move { store.console_page(&page).await }
        },
    )
    .await?;

    for area in &areas {
        let query = tally_ballots_query(event, &election, area)?;
        let rows = timed(
            &format!("tally input of area {}", &area[..8]),
            repeat_scans,
            |_| {
                let store = store.clone();
                let query = query.clone();
                async move {
                    let rows = store.client().await?.query(query.as_str(), &[]).await?;
                    anyhow::Ok(rows.len())
                }
            },
        )
        .await?;
        println!("  {rows} voters in the area's tally input");
        timed(
            &format!("unsequenced ballots of area {}", &area[..8]),
            repeat_scans,
            |_| store.unsequenced_count(event, &election, area),
        )
        .await?;
    }
    if let Some(area) = areas.first() {
        let query = tally_ballots_query(event, &election, area)?;
        let plan = store
            .client()
            .await?
            .query(format!("EXPLAIN {query}").as_str(), &[])
            .await?;
        println!("tally input plan:");
        for row in plan {
            println!("  {}", row.get::<_, String>(0));
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let connection = PostgresConnection::from_env()?;
    let database = cli
        .database
        .clone()
        .unwrap_or_else(|| connection.database().to_string());
    let pool = match &cli.command {
        Command::Accept(args) => args.clients + 4,
        _ => 8,
    };
    let store = connection.store(&database, pool)?;
    store.initialize().await?;
    match cli.command {
        Command::Accept(args) => accept(store, args).await,
        Command::Sequence {
            event,
            board,
            batch,
            seconds,
            linger,
        } => sequence(store, event, board, batch, seconds, linger).await,
        Command::Settle { event } => {
            let removed = store
                .client()
                .await?
                .execute(
                    "DELETE FROM ballot_box_pending WHERE election_event_id = $1::text::uuid",
                    &[&event],
                )
                .await?;
            println!("{removed} queued ballots marked as appended");
            Ok(())
        }
        Command::Reads(args) => reads(store, args).await,
    }
}
