// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! VOTE-FREEZE sealing cost, measured once (not a benchmark with an SLA):
//! one election with 130 ballot boxes, the largest with 50,000 ballots and
//! the other 129 small, on a private migrated Hasura database and the
//! event's own electoral-log database. Votes go in through `accept_ballot`
//! and onto the log through the sequencer's steps. It reports the votes and
//! their records, the close, every `seal_box`, the steps of the largest seal
//! (read, selection with the Ballot ID check and hashing, manifest build,
//! signing), the publication of the largest record, and the overhead of the
//! `ballot_box_seal_guard` trigger on votes (the same votes with the trigger
//! enabled and disabled).
//!
//! Ignored: run it by hand with `HASURA_DB__*` and `ELECTORAL_LOG_PG_*` set
//! (the stack's `.devcontainer/.env` exported, `HASURA_DB__HOST=postgres`);
//! it sets its own `MASTER_SECRET`. Optionally `PERF_BALLOT_TEMPLATE=<a file with a stored ballot's content>`
//! to hash real-sized ballots, and
//! `cargo test -p windmill --release --test perf_ballot_box_seal -- --ignored --nocapture`.

#![recursion_limit = "256"]

#[path = "support/event_database.rs"]
mod event_database;
#[path = "support/schema.rs"]
mod schema;

use anyhow::Result;
use async_trait::async_trait;
use b4::messages::message::Signer;
use deadpool_postgres::Transaction;
use electoral_log::adapters::ballot_box_seal::{box_ballots, BallotBoxId};
use electoral_log::adapters::postgres::PostgresStore;
use electoral_log::messages::message::{Message, SigningData};
use electoral_log::seal::{ballot_hash, build, BallotBoxSealManifest, SEAL_FORMAT_V1};
use event_database::{EventDatabase, Vote};
use sequent_core::ballot::{
    EGracePeriodPolicy, ElectionEventStatus, ElectionPresentation, ElectionStatus,
    SignedHashableBallot, VotingPeriodEnd, VotingStatus, VotingStatusChannel, TYPES_VERSION,
};
use serde_json::{json, Value};
use std::sync::{Mutex, Once};
use std::time::Instant;
use strand::backend::ristretto::RistrettoCtx;
use strand::serialization::StrandSerialize;
use strand::signature::StrandSignatureSk;
use uuid::Uuid;
use windmill::postgres::ballot_box_seal::list_for_elections;
use windmill::postgres::trusted_write;
use windmill::services::ballot_box_seal::publish::publish_box;
use windmill::services::ballot_box_seal::seal::{computed_ballot_id, entries, seal_box};
use windmill::services::ballot_box_seal::{Census, ProductionSealEnvironment, SealEnvironment};
use windmill::services::protocol_manager::{
    create_protocol_manager_keys, get_event_store, get_protocol_manager,
};
use windmill::services::voting_status::update_election_status;

const BOXES: usize = 130;
const LARGEST_BALLOTS: usize = 50_000;
/// Of the largest box: voters who vote twice (the second ballot counts).
const LARGEST_REVOTERS: usize = 5_000;
/// Of the largest box: voters not in the census (not eligible).
const LARGEST_NOT_IN_CENSUS: usize = 500;
const SMALL_BALLOTS: usize = 50;
const GUARD_VOTES: usize = 1_000;
const ADMIN: &str = "admin";

/// The secrets of an event are encrypted with the deployment's master secret.
fn master_secret() {
    static SET: Once = Once::new();
    SET.call_once(|| std::env::set_var("MASTER_SECRET", "5a".repeat(32)));
}

struct PerfEnvironment {
    census: Census,
    uploads: Mutex<Vec<(String, usize)>>,
}

#[async_trait]
impl SealEnvironment for PerfEnvironment {
    async fn event_store(&self, election_event_id: &str) -> Result<PostgresStore> {
        get_event_store(election_event_id).await
    }

    async fn request_sequencer(&self, _: &str) -> Result<()> {
        Ok(())
    }

    fn sequencer_wait(&self) -> std::time::Duration {
        std::time::Duration::ZERO
    }

    async fn signing_key(
        &self,
        hasura_transaction: &Transaction<'_>,
        tenant_id: &str,
        election_event_id: &str,
        board: &str,
    ) -> Result<StrandSignatureSk> {
        ProductionSealEnvironment
            .signing_key(hasura_transaction, tenant_id, election_event_id, board)
            .await
    }

    async fn census(
        &self,
        _: &sequent_core::types::hasura::core::ElectionEvent,
        _: &str,
        _: &str,
    ) -> Result<Census> {
        Ok(self.census.clone())
    }

    async fn upload_record(
        &self,
        _: &Transaction<'_>,
        _: &str,
        _: &str,
        name: &str,
        json: &[u8],
        document_id: Uuid,
        _: sequent_core::ballot::BallotBoxSealRecordPolicy,
    ) -> Result<Uuid> {
        self.uploads
            .lock()
            .unwrap()
            .push((name.to_string(), json.len()));
        Ok(document_id)
    }
}

/// A ballot's content and Ballot ID: the template (a real stored ballot) with
/// a unique issue date, or a contest-less synthetic ballot.
fn ballot(template: &Option<Value>, seed: &str) -> (String, String) {
    let content = match template {
        Some(template) => {
            let mut value = template.clone();
            value["issue_date"] = Value::String(seed.to_string());
            value.to_string()
        }
        None => serde_json::to_string(&SignedHashableBallot {
            version: TYPES_VERSION,
            issue_date: seed.to_string(),
            contests: vec![],
            config: seed.to_string(),
            ballot_style_hash: seed.to_string(),
            voter_signing_pk: None,
            voter_ballot_signature: None,
        })
        .unwrap(),
    };
    let ballot_id = computed_ballot_id(&content, event_database::FORMAT).unwrap();
    (content, ballot_id)
}

/// The votes of one box, in the order they are cast.
struct Rows {
    voters: Vec<String>,
    contents: Vec<String>,
    ballot_ids: Vec<String>,
}

fn box_rows(template: &Option<Value>, prefix: &str, voters: usize, revoters: usize) -> Rows {
    let mut rows = Rows {
        voters: vec![],
        contents: vec![],
        ballot_ids: vec![],
    };
    let mut push = |voter: String, seed: String| {
        let (content, ballot_id) = ballot(template, &seed);
        rows.voters.push(voter);
        rows.contents.push(content);
        rows.ballot_ids.push(ballot_id);
    };
    for voter in 0..voters {
        push(format!("{prefix}-v{voter}"), format!("{prefix}-{voter}-1"));
    }
    for voter in 0..revoters {
        push(format!("{prefix}-v{voter}"), format!("{prefix}-{voter}-2"));
    }
    rows
}

/// Casts every vote of `rows` into the box (election, area), one
/// `accept_ballot` statement each, as votes arrive.
async fn cast_rows(
    database: &EventDatabase,
    election: Uuid,
    area: Uuid,
    rows: &Rows,
) -> std::time::Duration {
    let started = Instant::now();
    for index in 0..rows.voters.len() {
        database
            .cast(&Vote {
                election,
                area,
                voter: &rows.voters[index],
                content: &rows.contents[index],
                ballot_id: &rows.ballot_ids[index],
                channel: "ONLINE",
            })
            .await;
    }
    started.elapsed()
}

fn secs(duration: std::time::Duration) -> String {
    format!("{:.3} s", duration.as_secs_f64())
}

#[test]
#[ignore = "perf measurement: run by hand against a PostgreSQL server"]
fn seal_130_boxes_the_largest_with_50k_ballots() {
    event_database::run(async {
        let template: Option<Value> = std::env::var("PERF_BALLOT_TEMPLATE").ok().map(|path| {
            serde_json::from_str(std::fs::read_to_string(path).unwrap().trim()).unwrap()
        });
        println!(
            "ballots: {}",
            match &template {
                Some(value) => format!("real template, {} bytes each", value.to_string().len()),
                None => "synthetic, contest-less".to_string(),
            }
        );

        // The event, one election to seal and one for the guard's votes, 130
        // areas, the event's PM key and electoral-log database.
        let pool = schema::pool().await;
        let (tenant, event, election, scratch_election) = (
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
        );
        let areas: Vec<Uuid> = (0..BOXES).map(|_| Uuid::new_v4()).collect();
        let database = EventDatabase::create(tenant, event).await;
        let board = database.board.clone();
        let key = {
            let mut client = pool.get().await.unwrap();
            let tx = client.transaction().await.unwrap();
            trusted_write(&tx).await.unwrap();
            tx.execute(
                "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
                &[&tenant, &format!("tenant-{tenant}")],
            )
            .await
            .unwrap();
            let mut event_status = ElectionEventStatus::default();
            event_status.set_status_by_channel(VotingStatusChannel::ONLINE, VotingStatus::OPEN);
            tx.execute(
                "INSERT INTO sequent_backend.election_event
                     (id, tenant_id, encryption_protocol, status, presentation,
                      bulletin_board_reference, voting_channels)
                 VALUES ($1, $2, 'RSA256', $3, $4, $5, $6)",
                &[
                    &event,
                    &tenant,
                    &serde_json::to_value(&event_status).unwrap(),
                    &json!({ "ballot_box_seal_policy": "seal-at-close" }),
                    &json!({"id": 1, "database_name": board, "is_archived": false}),
                    &json!({"online": true, "kiosk": false}),
                ],
            )
            .await
            .unwrap();
            let presentation = ElectionPresentation {
                grace_period_policy: Some(EGracePeriodPolicy::NO_GRACE_PERIOD),
                grace_period_secs: None,
                voting_period_end: Some(VotingPeriodEnd::ALLOWED),
                ..Default::default()
            };
            let mut status = ElectionStatus::default();
            status.set_status_by_channel(VotingStatusChannel::ONLINE, VotingStatus::OPEN);
            for id in [election, scratch_election] {
                tx.execute(
                    "INSERT INTO sequent_backend.election
                         (id, tenant_id, election_event_id, status, voting_channels,
                          presentation, num_allowed_revotes)
                     VALUES ($1, $2, $3, $4, $5, $6, 0)",
                    &[
                        &id,
                        &tenant,
                        &event,
                        &serde_json::to_value(&status).unwrap(),
                        &json!({"online": true, "kiosk": false}),
                        &serde_json::to_value(&presentation).unwrap(),
                    ],
                )
                .await
                .unwrap();
            }
            for (index, area) in areas.iter().enumerate() {
                tx.execute(
                    "INSERT INTO sequent_backend.area (id, tenant_id, election_event_id, name)
                     VALUES ($1, $2, $3, $4)",
                    &[area, &tenant, &event, &format!("Area {index}")],
                )
                .await
                .unwrap();
            }
            master_secret();
            create_protocol_manager_keys(&tx, &tenant.to_string(), &event.to_string(), &board)
                .await
                .unwrap();
            let key = get_protocol_manager::<RistrettoCtx>(
                &tx,
                &tenant.to_string(),
                Some(&event.to_string()),
                &board,
            )
            .await
            .unwrap()
            .get_signing_key()
            .clone();
            tx.commit().await.unwrap();
            key
        };

        // Ballots: the largest box, then 129 small ones.
        let generated = Instant::now();
        let largest_voters = LARGEST_BALLOTS - LARGEST_REVOTERS;
        let largest = box_rows(&template, "big", largest_voters, LARGEST_REVOTERS);
        let small: Vec<Rows> = (1..BOXES)
            .map(|index| box_rows(&template, &format!("s{index}"), SMALL_BALLOTS, 0))
            .collect();
        println!(
            "generate ballots (content + Ballot ID, client side): {}",
            secs(generated.elapsed())
        );
        let mut census: Census = Census::new();
        for voter in LARGEST_NOT_IN_CENSUS..largest_voters {
            census.insert(format!("big-v{voter}"), 1);
        }
        for index in 1..BOXES {
            for voter in 0..SMALL_BALLOTS {
                census.insert(format!("s{index}-v{voter}"), 1);
            }
        }

        let elapsed = cast_rows(&database, election, areas[0], &largest).await;
        println!(
            "cast the largest box ({} accept_ballot statements, guard on): {}",
            largest.voters.len(),
            secs(elapsed)
        );
        let started = Instant::now();
        for (index, rows) in small.iter().enumerate() {
            cast_rows(&database, election, areas[index + 1], rows).await;
        }
        println!(
            "cast the 129 small boxes ({} ballots each): {}",
            SMALL_BALLOTS,
            secs(started.elapsed())
        );
        let started = Instant::now();
        let appended = database.sequence(&key).await;
        println!(
            "sequence: sign and append {appended} cast-vote records: {}",
            secs(started.elapsed())
        );

        // Guard overhead: the same kind of votes, into a box that is never
        // sealed, with the trigger enabled and disabled.
        let event_client = database.store.client().await.unwrap();
        let mut report = vec![];
        for (round, enabled) in [true, false, true, false].into_iter().enumerate() {
            let toggle = if enabled { "ENABLE" } else { "DISABLE" };
            event_client
                .batch_execute(&format!(
                    "ALTER TABLE ballot_box_ballot {toggle} TRIGGER ballot_box_seal_guard"
                ))
                .await
                .unwrap();
            let rows = box_rows(&template, &format!("guard{round}"), GUARD_VOTES, 0);
            let elapsed = cast_rows(&database, scratch_election, areas[0], &rows).await;
            report.push((enabled, elapsed));
        }
        event_client
            .batch_execute("ALTER TABLE ballot_box_ballot ENABLE TRIGGER ballot_box_seal_guard")
            .await
            .unwrap();
        for (enabled, elapsed) in &report {
            println!(
                "guard {}: {GUARD_VOTES} votes {} ({:.0} µs/vote)",
                if *enabled { "on " } else { "off" },
                secs(*elapsed),
                elapsed.as_secs_f64() * 1e6 / GUARD_VOTES as f64,
            );
        }

        // Stop Voting: the close hook inserts one pending seal per box.
        let started = Instant::now();
        {
            let mut client = pool.get().await.unwrap();
            let tx = client.transaction().await.unwrap();
            update_election_status(
                tenant.to_string(),
                Some(ADMIN),
                Some(ADMIN),
                &tx,
                &event.to_string(),
                &election.to_string(),
                &VotingStatus::CLOSED,
                &Some(vec![VotingStatusChannel::ONLINE]),
            )
            .await
            .unwrap();
            tx.commit().await.unwrap();
        }
        println!(
            "close (Stop Voting with the close hook): {}",
            secs(started.elapsed())
        );
        let seals = {
            let mut client = pool.get().await.unwrap();
            let tx = client.transaction().await.unwrap();
            list_for_elections(&tx, &tenant, &event, &[election])
                .await
                .unwrap()
        };
        assert_eq!(seals.len(), BOXES, "one pending seal per box");
        let largest_seal = seals
            .iter()
            .find(|seal| seal.area_id == areas[0])
            .unwrap()
            .clone();

        // The steps of the largest seal, outside seal_box (same inputs).
        {
            let (event_id, election_id, area_id) = (
                event.to_string(),
                election.to_string(),
                areas[0].to_string(),
            );
            let mut client = database.store.client().await.unwrap();
            let tx = client.transaction().await.unwrap();
            let started = Instant::now();
            let mut ballots = vec![];
            box_ballots(
                &tx,
                BallotBoxId {
                    election_event_id: &event_id,
                    election_id: &election_id,
                    area_id: &area_id,
                },
                |ballot| {
                    ballots.push(ballot);
                    Ok(())
                },
            )
            .await
            .unwrap();
            tx.rollback().await.unwrap();
            println!(
                "largest: read {} ballots: {}",
                ballots.len(),
                secs(started.elapsed())
            );
            let started = Instant::now();
            for ballot in &ballots {
                computed_ballot_id(&ballot.content, &ballot.format).unwrap();
            }
            println!(
                "largest: Ballot ID check alone (parse + hash_ballot): {}",
                secs(started.elapsed())
            );
            let started = Instant::now();
            for ballot in &ballots {
                ballot_hash(&ballot.content).unwrap();
            }
            println!(
                "largest: SHA-512 of the contents alone: {}",
                secs(started.elapsed())
            );
            let started = Instant::now();
            let sealed_entries = entries(&ballots, &census).unwrap();
            println!(
                "largest: entries() = selection + Ballot ID check + hashing: {}",
                secs(started.elapsed())
            );
            let started = Instant::now();
            let built = build(BallotBoxSealManifest {
                format: SEAL_FORMAT_V1.to_string(),
                tenant_id: tenant.to_string(),
                election_event_id: event_id.clone(),
                election_id: election_id.clone(),
                area_id: area_id.clone(),
                closed_at: 1,
                grace_deadline: 1,
                sealed_at: 2,
                close_request_id: None,
                eligible_voters: census.len() as u64,
                entries: sealed_entries,
            })
            .unwrap();
            println!(
                "largest: build (sort, Borsh, SHA-512): {} ({} manifest bytes; {} counted of {})",
                secs(started.elapsed()),
                built.bytes.len(),
                built.manifest.ballots_counted(),
                built.manifest.ballots_in_box()
            );
            let started = Instant::now();
            let message = Message::ballot_box_sealed_message(
                &built.manifest,
                built.hash,
                "Election",
                "Area 0",
                &SigningData::new(key.clone(), "", key.clone()),
            )
            .unwrap();
            let bytes = message.strand_serialize().unwrap();
            println!(
                "largest: sign the BallotBoxSealed message: {} ({} bytes)",
                secs(started.elapsed()),
                bytes.len()
            );
        }

        // seal_box for every box, one after the other (production fans them out).
        let environment = PerfEnvironment {
            census,
            uploads: Mutex::new(vec![]),
        };
        let mut client = pool.get().await.unwrap();
        let started = Instant::now();
        let largest_seal_time = {
            let started = Instant::now();
            let outcome = seal_box(&mut client, &environment, &largest_seal.id)
                .await
                .unwrap();
            println!(
                "seal_box largest ({LARGEST_BALLOTS} ballots): {} -> {outcome:?}",
                secs(started.elapsed())
            );
            started.elapsed()
        };
        let mut small_times = vec![];
        for seal in seals.iter().filter(|seal| seal.id != largest_seal.id) {
            let started = Instant::now();
            seal_box(&mut client, &environment, &seal.id).await.unwrap();
            small_times.push(started.elapsed());
        }
        small_times.sort();
        println!(
            "seal_box 129 small ({SMALL_BALLOTS} ballots each): total {}, median {}, max {}",
            secs(small_times.iter().sum()),
            secs(small_times[small_times.len() / 2]),
            secs(*small_times.last().unwrap())
        );
        println!(
            "seal_box all 130 sequentially: {} (largest {})",
            secs(started.elapsed()),
            secs(largest_seal_time)
        );

        // Publish the largest and a small one: the record.
        let started = Instant::now();
        let outcome = publish_box(&mut client, &environment, &largest_seal.id)
            .await
            .unwrap();
        let record_bytes = environment
            .uploads
            .lock()
            .unwrap()
            .last()
            .map(|(_, size)| *size);
        println!(
            "publish_box largest: {} -> {outcome:?}; record {:?} bytes",
            secs(started.elapsed()),
            record_bytes
        );
        let small_seal = seals
            .iter()
            .find(|seal| seal.id != largest_seal.id)
            .unwrap();
        let started = Instant::now();
        publish_box(&mut client, &environment, &small_seal.id)
            .await
            .unwrap();
        let record_bytes = environment
            .uploads
            .lock()
            .unwrap()
            .last()
            .map(|(_, size)| *size);
        println!(
            "publish_box small: {}; record {:?} bytes",
            secs(started.elapsed()),
            record_bytes
        );

        database.drop().await;
    })
}
