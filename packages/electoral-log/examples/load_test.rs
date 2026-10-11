// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Load test for the PostgreSQL electoral log.
//!
//! Appends synthetic records shaped like production ones through the store API that
//! Windmill uses, and times appends, the queries issued by the portals and reports,
//! proofs and audits as one board grows. Each voter produces four Keycloak events and
//! one cast vote, spread over time like concurrent voters. Connection settings come
//! from the `ELECTORAL_LOG_PG_*` variables; run it against a disposable database:
//!
//! ```text
//! cargo run --release --example load_test -- fill --target 1000000
//! cargo run --release --example load_test -- probe
//! cargo run --release --example load_test -- appends --count 2000 --concurrency 8
//! cargo run --release --example load_test -- audit
//! ```

use anyhow::{ensure, Context, Result};
use clap::{Parser, Subcommand};
use electoral_log::messages::statement::StatementType;
use electoral_log::{
    adapters::postgres::{LogScope, PostgresStore},
    proofs::{Checkpoint, JournalError, Uuid},
    BoardClient, ElectoralLogMessage, Filter, LogEntry, LogQuery, LogVisibility, NumberColumn,
    NumberComparison, OrderColumn, SortDirection, SqlCompOperators, TextColumn,
};
use openssl::ssl::{SslConnector, SslMethod, SslVerifyMode};
use postgres_openssl::MakeTlsConnector;
use std::{
    env,
    fs::{File, OpenOptions},
    future::Future,
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::task::JoinSet;
use tokio_postgres::{config::SslMode, NoTls};

const EVENTS_PER_VOTER: u64 = 5;
/// Voters whose events interleave; a voter's events are this many records apart.
const VOTERS_PER_WINDOW: u64 = 20_000;
const EVENTS_PER_SECOND: u64 = 1_000;
const ELECTIONS: u64 = 5;
const AREAS: u64 = 1_000;
const FIRST_TIMESTAMP: i64 = 1_791_158_816;
/// Message sizes measured on signed development records.
const CAST_VOTE_BYTES: u64 = 702;
const KEYCLOAK_EVENT_BYTES: u64 = 330;
const KEYCLOAK_EVENT_SPREAD: u64 = 120;
const SENDER_PK: &str = "MCowBQYDK2VwAyEAm4Zq0pW7cTn3sV9yKx2bLf6RjH1dGu8oEa5iNw0Pz4=";
/// Rows per admin portal page.
const ADMIN_PAGE: i64 = 50;
/// Rows per ballot-locator page.
const BALLOT_PAGE: i64 = 10;
/// Rows per report batch (`ELECTORAL_LOG_ROWS_LIMIT`).
const REPORT_BATCH: i64 = 2_500;
/// Rows per cast-vote export batch.
const CAST_VOTE_EXPORT_BATCH: i64 = 1_000;
/// Areas visible to an election-scoped administrator in the visibility probe.
const VISIBLE_AREAS: u64 = 10;
/// Time after which a probe stops repeating a slow query.
const PROBE_BUDGET: Duration = Duration::from_secs(30);

/// Seeds that keep the synthetic identifiers of each kind distinct.
#[derive(Clone, Copy)]
enum Namespace {
    Delivery = 1,
    User,
    Election,
    Area,
    Ballot,
    Payload,
    ExtraDelivery,
    Sample,
}

impl Namespace {
    fn seed(self) -> u64 {
        (self as u64).wrapping_mul(0x5851_f42d_4c95_7f2d)
    }
}

/// SplitMix64 finalizer: a bijection, so distinct inputs give distinct outputs.
fn mix(value: u64) -> u64 {
    let mut z = value.wrapping_add(0x9e37_79b9_7f4a_7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

fn uuid(namespace: Namespace, n: u64) -> String {
    let high = mix(n.wrapping_add(namespace.seed()));
    let low = mix(high ^ namespace.seed());
    format!(
        "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
        high >> 32,
        (high >> 16) & 0xffff,
        high & 0xffff,
        low >> 48,
        low & 0xffff_ffff_ffff
    )
}

fn user_id(voter: u64) -> String {
    uuid(Namespace::User, voter)
}

fn username(voter: u64) -> String {
    format!("voter{voter:09}")
}

fn election_id(voter: u64) -> String {
    uuid(Namespace::Election, voter % ELECTIONS)
}

fn area_id(voter: u64) -> String {
    uuid(Namespace::Area, voter % AREAS)
}

fn ballot_id(voter: u64) -> String {
    (0..4)
        .map(|part| {
            format!(
                "{:016x}",
                mix(Namespace::Ballot.seed() ^ (voter * 4 + part))
            )
        })
        .collect()
}

fn payload(position: u64, length: u64) -> Vec<u8> {
    (0..length.div_ceil(8))
        .flat_map(|word| mix(Namespace::Payload.seed() ^ (position << 7) ^ word).to_le_bytes())
        .take(usize::try_from(length).unwrap_or(usize::MAX))
        .collect()
}

/// The voter and event number of the record at a position of the board.
fn slot(position: u64) -> (u64, u64) {
    let window = VOTERS_PER_WINDOW * EVENTS_PER_VOTER;
    let offset = position % window;
    (
        position / window * VOTERS_PER_WINDOW + offset % VOTERS_PER_WINDOW,
        offset / VOTERS_PER_WINDOW,
    )
}

/// Position of a voter's cast vote.
fn cast_vote_position(voter: u64) -> u64 {
    let window = voter / VOTERS_PER_WINDOW;
    window * VOTERS_PER_WINDOW * EVENTS_PER_VOTER
        + (EVENTS_PER_VOTER - 1) * VOTERS_PER_WINDOW
        + voter % VOTERS_PER_WINDOW
}

fn timestamp(position: u64) -> i64 {
    FIRST_TIMESTAMP.saturating_add(i64::try_from(position / EVENTS_PER_SECOND).unwrap_or(i64::MAX))
}

fn record(position: u64, delivery_id: String) -> LogEntry {
    let (voter, event) = slot(position);
    let cast_vote = event == EVENTS_PER_VOTER - 1;
    let created = timestamp(position);
    let length = if cast_vote {
        CAST_VOTE_BYTES
    } else {
        KEYCLOAK_EVENT_BYTES + mix(position) % KEYCLOAK_EVENT_SPREAD
    };
    LogEntry {
        delivery_id,
        message: ElectoralLogMessage {
            id: 0,
            created,
            sender_pk: SENDER_PK.into(),
            statement_timestamp: created,
            statement_kind: if cast_vote {
                StatementType::CastVote
            } else {
                StatementType::KeycloakUserEvent
            }
            .to_string(),
            message: payload(position, length),
            version: if cast_vote { "2" } else { "1" }.into(),
            user_id: Some(user_id(voter)),
            username: Some(username(voter)),
            election_id: cast_vote.then(|| election_id(voter)),
            area_id: Some(area_id(voter)),
            ballot_id: cast_vote.then(|| ballot_id(voter)),
        },
    }
}

fn fill_record(position: u64) -> LogEntry {
    record(
        position,
        format!("{}:event", uuid(Namespace::Delivery, position)),
    )
}

#[derive(Parser)]
#[command(about = "Load test for the PostgreSQL electoral log")]
struct Cli {
    /// Board to load and measure.
    #[arg(
        long,
        default_value = "loadtenant0123456789abcdeevent0123456789abcdef0123456789abcdef"
    )]
    board: String,
    /// File that keeps the checkpoints recorded while filling, for proofs and audits.
    #[arg(long, default_value = "load-checkpoints.jsonl")]
    checkpoints: PathBuf,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Append records until the board holds `target` of them.
    Fill {
        #[arg(long)]
        target: u64,
        /// Records per append.
        #[arg(long, default_value_t = 5_000)]
        batch: u64,
        /// Records between progress reports and saved checkpoints.
        #[arg(long, default_value_t = 250_000)]
        report: u64,
    },
    /// Time concurrent appends of new records.
    Appends {
        /// Appends to run.
        #[arg(long, default_value_t = 1_000)]
        count: u64,
        /// Records per append.
        #[arg(long, default_value_t = 1)]
        batch: u64,
        /// Appends in flight.
        #[arg(long, default_value_t = 1)]
        concurrency: u64,
    },
    /// Time the queries and proofs that the portals, reports and APIs issue.
    Probe {
        /// Runs of each query.
        #[arg(long, default_value_t = 5)]
        repeat: usize,
        /// Records proven per proof probe, at random positions.
        #[arg(long, default_value_t = 200)]
        proofs: u64,
    },
    /// Audit the board against the saved checkpoints.
    Audit,
    /// Print the probe's sample values as psql variables, for EXPLAIN scripts.
    Params,
}

struct Load {
    board: String,
    store: PostgresStore,
    client: BoardClient,
    sql: tokio_postgres::Client,
    checkpoints: PathBuf,
}

async fn connect() -> Result<tokio_postgres::Client> {
    let required = |name| env::var(name).with_context(|| format!("{name} must be set"));
    let mut config = tokio_postgres::Config::new();
    config
        .host(&required("ELECTORAL_LOG_PG_HOST")?)
        .port(required("ELECTORAL_LOG_PG_PORT")?.parse()?)
        .user(&required("ELECTORAL_LOG_PG_USER")?)
        .password(required("ELECTORAL_LOG_PG_PASSWORD")?)
        .dbname(&required("ELECTORAL_LOG_PG_DATABASE")?);
    let mode = env::var("ELECTORAL_LOG_PG_SSLMODE").unwrap_or_else(|_| "require".into());
    if mode == "disable" {
        let (client, connection) = config.connect(NoTls).await?;
        tokio::spawn(report_failure(connection));
        return Ok(client);
    }
    // TLS as the store configures it: `require` encrypts without verifying the server.
    let mut tls = SslConnector::builder(SslMethod::tls())?;
    if mode == "verify-full" {
        if let Some(ca) = env::var("ELECTORAL_LOG_PG_SSLROOTCERT")
            .ok()
            .filter(|path| !path.is_empty())
        {
            tls.set_ca_file(ca)?;
        }
    } else {
        tls.set_verify(SslVerifyMode::NONE);
    }
    config.ssl_mode(SslMode::Require);
    let (client, connection) = config.connect(MakeTlsConnector::new(tls.build())).await?;
    tokio::spawn(report_failure(connection));
    Ok(client)
}

async fn report_failure(connection: impl Future<Output = Result<(), tokio_postgres::Error>>) {
    if let Err(error) = connection.await {
        eprintln!("Connection failed: {error}");
    }
}

fn millis(duration: Duration) -> String {
    format!("{:.1}ms", duration.as_secs_f64() * 1e3)
}

fn percentile(sorted: &[Duration], percent: usize) -> Duration {
    sorted
        .get((sorted.len().saturating_sub(1)) * percent / 100)
        .copied()
        .unwrap_or_default()
}

/// Run an operation up to `repeat` times, or until the runs take `PROBE_BUDGET`, and
/// print the rows it returned and its latencies.
async fn time(
    name: &str,
    repeat: usize,
    mut operation: impl AsyncFnMut(u64) -> Result<u64>,
) -> Result<()> {
    let mut times = Vec::with_capacity(repeat);
    let mut rows = 0;
    let begun = Instant::now();
    for run in 0..repeat {
        let started = Instant::now();
        rows = operation(u64::try_from(run)?).await?;
        times.push(started.elapsed());
        if begun.elapsed() > PROBE_BUDGET {
            break;
        }
    }
    let first = times.first().copied().unwrap_or_default();
    let runs = times.len();
    times.sort();
    println!(
        "{name:<34}\trows={rows}\truns={runs}\tfirst={}\tp50={}\tp95={}\tmax={}",
        millis(first),
        millis(percentile(&times, 50)),
        millis(percentile(&times, 95)),
        millis(percentile(&times, 100)),
    );
    Ok(())
}

impl Load {
    async fn database_bytes(&self) -> Result<i64> {
        Ok(self
            .sql
            .query_one("SELECT pg_database_size(current_database())", &[])
            .await?
            .try_get(0)?)
    }

    async fn size(&self) -> Result<u64> {
        match self.store.journal().checkpoint(&self.board).await {
            Ok(checkpoint) => Ok(checkpoint.tree_size),
            Err(JournalError::NotFound(_)) => Ok(0),
            Err(error) => Err(error.into()),
        }
    }

    fn save_checkpoint(&self, checkpoint: &Checkpoint) -> Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.checkpoints)?;
        writeln!(file, "{}", serde_json::to_string(checkpoint)?)?;
        Ok(())
    }

    fn saved_checkpoints(&self, log_uid: Uuid) -> Result<Vec<Checkpoint>> {
        if !Path::new(&self.checkpoints).exists() {
            return Ok(Vec::new());
        }
        BufReader::new(File::open(&self.checkpoints)?)
            .lines()
            .map(|line| Ok(serde_json::from_str::<Checkpoint>(&line?)?))
            .filter(|checkpoint| {
                checkpoint
                    .as_ref()
                    .map_or(true, |c| c.log_name == self.board && c.log_uid == log_uid)
            })
            .collect()
    }

    async fn fill(&self, target: u64, batch: u64, report: u64) -> Result<()> {
        ensure!(
            batch > 0 && report > 0,
            "Batch and report sizes must be positive"
        );
        self.client.create_board(&self.board).await?;
        let mut size = self.size().await?;
        let mut window = (size, Instant::now(), Duration::ZERO);
        while size < target {
            let end = target.min(size.saturating_add(batch));
            let started = Instant::now();
            self.client
                .append_iter(
                    &self.board,
                    &mut (size..end).map(|position| Ok(fill_record(position))),
                )
                .await?;
            window.2 = window.2.max(started.elapsed());
            size = end;
            if size - window.0 >= report || size == target {
                let checkpoint = self.store.journal().checkpoint(&self.board).await?;
                let elapsed = window.1.elapsed().as_secs_f64();
                let rate = (size - window.0) as f64 / elapsed;
                let gigabytes = self.database_bytes().await? as f64 / 1e9;
                println!(
                    "fill\tsize={}\trate={rate:.0}/s\tslowest_append={}\tdb={gigabytes:.2}GB",
                    checkpoint.tree_size,
                    millis(window.2),
                );
                self.save_checkpoint(&checkpoint)?;
                window = (size, Instant::now(), Duration::ZERO);
            }
        }
        Ok(())
    }

    async fn appends(&self, count: u64, batch: u64, concurrency: u64) -> Result<()> {
        ensure!(
            batch > 0 && concurrency > 0,
            "Batch and concurrency must be positive"
        );
        let first_position = self.size().await?;
        let run = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let base = mix(u64::try_from(run % u128::from(u64::MAX))?);
        let next = Arc::new(AtomicU64::new(0));
        let started = Instant::now();
        let mut workers = JoinSet::new();
        for _ in 0..concurrency {
            let (client, board, next) = (self.client.clone(), self.board.clone(), next.clone());
            workers.spawn(async move {
                let mut latencies = Vec::new();
                loop {
                    let append = next.fetch_add(1, Ordering::Relaxed);
                    if append >= count {
                        return Ok::<_, anyhow::Error>(latencies);
                    }
                    let mut entries = (append * batch..(append + 1) * batch).map(|n| {
                        Ok(record(
                            first_position + n,
                            uuid(Namespace::ExtraDelivery, base.wrapping_add(n)),
                        ))
                    });
                    let begun = Instant::now();
                    client.append_iter(&board, &mut entries).await?;
                    latencies.push(begun.elapsed());
                }
            });
        }
        let mut latencies = Vec::new();
        while let Some(worker) = workers.join_next().await {
            latencies.extend(worker??);
        }
        let elapsed = started.elapsed().as_secs_f64();
        latencies.sort();
        let rate = (count * batch) as f64 / elapsed;
        println!(
            "appends\tsize={first_position}\tbatch={batch}\tconcurrency={concurrency}\tcount={count}\trate={rate:.0} records/s\tp50={}\tp95={}\tp99={}\tmax={}",
            millis(percentile(&latencies, 50)),
            millis(percentile(&latencies, 95)),
            millis(percentile(&latencies, 99)),
            millis(percentile(&latencies, 100)),
        );
        Ok(())
    }

    async fn record_ids(&self) -> Result<(i64, i64)> {
        let row = self
            .sql
            .query_one(
                "SELECT min(id), max(id) FROM electoral_log_messages WHERE board_name = $1",
                &[&self.board],
            )
            .await?;
        Ok((
            row.try_get::<_, Option<i64>>(0)?.context("Empty board")?,
            row.try_get::<_, Option<i64>>(1)?.context("Empty board")?,
        ))
    }

    async fn probe(&self, repeat: usize, proofs: u64) -> Result<()> {
        let board = self.board.as_str();
        let client = &self.client;
        let current = self.store.journal().checkpoint(board).await?;
        let size = current.tree_size;
        let (first_id, last_id) = self.record_ids().await?;
        let middle_id = first_id + (last_id - first_id) / 2;
        let voter = size / EVENTS_PER_VOTER / 2;
        ensure!(
            cast_vote_position(voter) < size,
            "Fill the board with at least two windows of voters first"
        );
        let election = election_id(voter);
        let minute = timestamp(size / 2);
        let gigabytes = self.database_bytes().await? as f64 / 1e9;
        println!("probe\tsize={size}\tdb={gigabytes:.2}GB\tvoter={voter}");

        let page = |order: Vec<(OrderColumn, SortDirection)>, filters: Vec<Filter>| LogQuery {
            filters,
            order,
            limit: ADMIN_PAGE,
            ..LogQuery::default()
        };
        let newest = || vec![(OrderColumn::Id, SortDirection::Desc)];
        let like = |column, value: String| Filter::Text(column, SqlCompOperators::Like, value);
        let equal = |column, value: String| Filter::Text(column, SqlCompOperators::Equal, value);
        let rows = |n: usize| u64::try_from(n).unwrap_or(u64::MAX);
        let list = async |query: LogQuery| -> Result<u64> {
            Ok(rows(client.query(board, &query).await?.len()))
        };
        let count = async |query: LogQuery| -> Result<u64> {
            Ok(u64::try_from(client.count(board, &query).await?)?)
        };

        // Admin portal list: each page load runs the list query and a count of it.
        time("admin.list.id_desc", repeat, async |_| {
            list(page(newest(), vec![])).await
        })
        .await?;
        time("admin.count.all", repeat, async |_| {
            count(page(newest(), vec![])).await
        })
        .await?;
        for (name, column) in [
            ("admin.list.created_desc", OrderColumn::Created),
            (
                "admin.list.statement_timestamp_desc",
                OrderColumn::StatementTimestamp,
            ),
            ("admin.list.statement_kind_desc", OrderColumn::StatementKind),
            ("admin.list.user_id_desc", OrderColumn::UserId),
        ] {
            time(name, repeat, async |_| {
                list(page(vec![(column, SortDirection::Desc)], vec![])).await
            })
            .await?;
        }
        time("admin.list.offset_middle", repeat, async |_| {
            list(LogQuery {
                offset: i64::try_from(size / 2)?,
                ..page(newest(), vec![])
            })
            .await
        })
        .await?;
        let filters = [
            (
                "admin.filter.user_id",
                like(TextColumn::UserId, user_id(voter)),
            ),
            (
                "admin.filter.username",
                like(TextColumn::Username, username(voter)),
            ),
            (
                "admin.filter.statement_kind",
                like(
                    TextColumn::StatementKind,
                    StatementType::CastVote.to_string(),
                ),
            ),
            (
                "admin.filter.ballot_id",
                like(TextColumn::BallotId, ballot_id(voter)),
            ),
        ];
        for (name, filter) in filters {
            time(&format!("{name}.list"), repeat, async |_| {
                list(page(newest(), vec![filter.clone()])).await
            })
            .await?;
            time(&format!("{name}.count"), repeat, async |_| {
                count(page(newest(), vec![filter.clone()])).await
            })
            .await?;
        }
        // The admin portal filters both timestamps to the minute.
        let minute_filters = |column| {
            vec![
                Filter::Number(column, NumberComparison::GreaterThanOrEqual, minute),
                Filter::Number(column, NumberComparison::LessThan, minute + 60),
            ]
        };
        for (name, column) in [
            ("admin.filter.created_minute", NumberColumn::Created),
            (
                "admin.filter.statement_timestamp_minute",
                NumberColumn::StatementTimestamp,
            ),
        ] {
            time(&format!("{name}.list"), repeat, async |_| {
                list(page(newest(), minute_filters(column))).await
            })
            .await?;
            time(&format!("{name}.count"), repeat, async |_| {
                count(page(newest(), minute_filters(column))).await
            })
            .await?;
        }
        time("admin.filter.id.list", repeat, async |_| {
            list(page(
                newest(),
                vec![Filter::Number(
                    NumberColumn::Id,
                    NumberComparison::Equal,
                    middle_id,
                )],
            ))
            .await
        })
        .await?;
        let scoped = || LogQuery {
            visibility: Some(LogVisibility {
                election_id: Some(election.clone()),
                area_ids: (0..VISIBLE_AREAS)
                    .map(|area| area_id(voter + area))
                    .collect(),
            }),
            ..page(newest(), vec![])
        };
        time("admin.scoped.list", repeat, async |_| list(scoped()).await).await?;
        time("admin.scoped.count", repeat, async |_| {
            count(scoped()).await
        })
        .await?;
        let with_user = || LogQuery {
            only_with_user: true,
            ..page(newest(), vec![])
        };
        time("admin.only_with_user.list", repeat, async |_| {
            list(with_user()).await
        })
        .await?;
        time("admin.only_with_user.count", repeat, async |_| {
            count(with_user()).await
        })
        .await?;

        // Ballot locator: a count of the election's cast votes and one page of them, or
        // the voter's own cast vote when a ballot ID is given.
        let ballots = || {
            vec![
                equal(
                    TextColumn::StatementKind,
                    StatementType::CastVote.to_string(),
                ),
                equal(TextColumn::ElectionId, election.clone()),
            ]
        };
        time("ballots.count", repeat, async |_| {
            count(page(newest(), ballots())).await
        })
        .await?;
        time("ballots.page", repeat, async |_| {
            list(LogQuery {
                limit: BALLOT_PAGE,
                ..page(newest(), ballots())
            })
            .await
        })
        .await?;
        time("ballots.locate", repeat, async |_| {
            let mut filters = ballots();
            filters.push(equal(TextColumn::UserId, user_id(voter)));
            filters.push(like(TextColumn::BallotId, ballot_id(voter)));
            list(LogQuery {
                limit: REPORT_BATCH,
                ..page(newest(), filters)
            })
            .await
        })
        .await?;

        // Reports and exports.
        let oldest = || vec![(OrderColumn::Id, SortDirection::Asc)];
        time("report.batch_after_id", repeat, async |_| {
            Ok(rows(
                client
                    .get_electoral_log_messages_batch(board, REPORT_BATCH, middle_id)
                    .await?
                    .len(),
            ))
        })
        .await?;
        time("report.batch_at_offset_middle", repeat, async |_| {
            Ok(rows(
                client
                    .get_electoral_log_messages_at_offset(
                        board,
                        REPORT_BATCH,
                        i64::try_from(size / 2)?,
                    )
                    .await?
                    .len(),
            ))
        })
        .await?;
        time("export.cast_votes_after_id", repeat, async |_| {
            list(LogQuery {
                filters: vec![
                    equal(
                        TextColumn::StatementKind,
                        StatementType::CastVote.to_string(),
                    ),
                    Filter::Number(NumberColumn::Id, NumberComparison::GreaterThan, middle_id),
                ],
                order: oldest(),
                limit: CAST_VOTE_EXPORT_BATCH,
                ..LogQuery::default()
            })
            .await
        })
        .await?;

        // Proofs.
        let journal = self.store.journal();
        time("proof.checkpoint", repeat, async |_| {
            journal.checkpoint(board).await?;
            Ok(1)
        })
        .await?;
        // Prove existing records at random positions, so gaps in the IDs are not sampled.
        let proofs = usize::try_from(proofs)?;
        let span = u64::try_from(last_id - first_id + 1)?;
        let mut sample = Vec::with_capacity(proofs);
        for n in 0..u64::try_from(proofs)? {
            let point = first_id + i64::try_from(mix(Namespace::Sample.seed() ^ n) % span)?;
            sample.push(
                self.sql
                    .query_one(
                        "SELECT min(id) FROM electoral_log_messages WHERE board_name = $1 AND id >= $2",
                        &[&board, &point],
                    )
                    .await?
                    .try_get::<_, Option<i64>>(0)?
                    .context("No record at or after a sampled ID")?,
            );
        }
        let sampled = |n: u64| sample[usize::try_from(n).unwrap_or_default() % sample.len()];
        time("proof.record_current", proofs, async |n| {
            let proof = self
                .store
                .record_proof(&journal, LogScope::Board(board), sampled(n), None)
                .await?;
            proof.verify(&proof.inclusion.checkpoint.clone())?;
            Ok(1)
        })
        .await?;
        let trusted = self
            .saved_checkpoints(current.log_uid)?
            .into_iter()
            .filter(|checkpoint| checkpoint.tree_size <= size / 2)
            .max_by_key(|checkpoint| checkpoint.tree_size);
        if let Some(trusted) = trusted {
            println!("trusted\tsize={}", trusted.tree_size);
            time("proof.consistency", repeat, async |_| {
                journal.consistency(&trusted).await?.verify(&trusted)?;
                Ok(1)
            })
            .await?;
            time("proof.record_trusted", proofs, async |n| {
                let proof = self
                    .store
                    .record_proof(&journal, LogScope::Board(board), sampled(n), Some(&trusted))
                    .await?;
                proof.verify(&trusted)?;
                Ok(1)
            })
            .await?;
        }
        Ok(())
    }

    async fn params(&self) -> Result<()> {
        let current = self.store.journal().checkpoint(&self.board).await?;
        let size = current.tree_size;
        let (first_id, last_id) = self.record_ids().await?;
        let voter = size / EVENTS_PER_VOTER / 2;
        let areas: Vec<_> = (0..VISIBLE_AREAS)
            .map(|area| area_id(voter + area))
            .collect();
        for (name, value) in [
            ("board", self.board.clone()),
            ("log_uid", current.log_uid.to_string()),
            ("size", size.to_string()),
            ("half", (size / 2).to_string()),
            (
                "middle_id",
                (first_id + (last_id - first_id) / 2).to_string(),
            ),
            ("user_id", user_id(voter)),
            ("username", username(voter)),
            ("election_id", election_id(voter)),
            ("ballot_id", ballot_id(voter)),
            ("minute", timestamp(size / 2).to_string()),
            ("areas", format!("{{{}}}", areas.join(","))),
        ] {
            println!("\\set {name} '{value}'");
        }
        Ok(())
    }

    async fn audit(&self) -> Result<()> {
        let current = self.store.journal().checkpoint(&self.board).await?;
        let published = self.saved_checkpoints(current.log_uid)?;
        let started = Instant::now();
        let report = self.store.audit(&self.board, &published).await?;
        println!(
            "audit\tsize={}\tclean={}\tduration={:.1}s\tchecked={}\tpublished={}",
            report.leaves,
            report.is_clean(),
            started.elapsed().as_secs_f64(),
            report.checked_hashes,
            report.published_checked,
        );
        for finding in report.findings() {
            println!("finding\t{finding}");
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let store = PostgresStore::from_env()?;
    let load = Load {
        board: cli.board,
        client: BoardClient::new(Arc::new(store.clone())),
        store,
        sql: connect().await?,
        checkpoints: cli.checkpoints,
    };
    match cli.command {
        Command::Fill {
            target,
            batch,
            report,
        } => load.fill(target, batch, report).await,
        Command::Appends {
            count,
            batch,
            concurrency,
        } => load.appends(count, batch, concurrency).await,
        Command::Probe { repeat, proofs } => load.probe(repeat, proofs).await,
        Command::Audit => load.audit().await,
        Command::Params => load.params().await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn identifiers_are_unique_and_shaped_like_production() {
        let deliveries: HashSet<_> = (0..100_000).map(|n| uuid(Namespace::Delivery, n)).collect();
        assert_eq!(deliveries.len(), 100_000);
        assert_eq!(uuid(Namespace::User, 7).len(), 36);
        assert_eq!(ballot_id(7).len(), 64);
        assert_eq!(fill_record(0).delivery_id.len(), 42);
    }

    #[test]
    fn each_voter_casts_one_vote_after_four_events() {
        let mut events = std::collections::HashMap::<u64, Vec<u64>>::new();
        let window = VOTERS_PER_WINDOW * EVENTS_PER_VOTER;
        for position in 0..2 * window {
            let (voter, event) = slot(position);
            events.entry(voter).or_default().push(event);
            if event == EVENTS_PER_VOTER - 1 {
                assert_eq!(cast_vote_position(voter), position);
                let entry = fill_record(position);
                assert_eq!(
                    entry.message.statement_kind,
                    StatementType::CastVote.to_string()
                );
                assert_eq!(entry.message.ballot_id, Some(ballot_id(voter)));
                assert_eq!(entry.message.message.len(), 702);
            }
        }
        assert_eq!(
            events.len(),
            usize::try_from(2 * VOTERS_PER_WINDOW).unwrap()
        );
        assert!(events.values().all(|e| *e == vec![0, 1, 2, 3, 4]));
    }
}
