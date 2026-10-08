// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! VOTE-FREEZE sealing cost, measured once (not a benchmark with an SLA):
//! one election with 130 ballot boxes, the largest with 50,000 ballots and
//! the other 129 small, on a private database of the stack's Postgres and a
//! private board of the stack's immudb. It reports the close, every
//! `seal_box`, the steps of the largest seal (read, selection with the
//! Ballot ID check and hashing, manifest build, signing), the publication
//! of the largest record, and the `cast_vote_seal_guard` overhead on
//! inserts (the same inserts with the trigger enabled and disabled).
//!
//! Ignored: run it by hand with the stack's `.devcontainer/.env` exported,
//! `HASURA_DB__HOST=postgres`, optionally `PERF_BALLOT_TEMPLATE=<a stored
//! cast_vote.content>` to hash real-sized ballots, and
//! `cargo test -p windmill --release --test perf_ballot_box_seal -- --ignored --nocapture`.

#![recursion_limit = "256"]

#[path = "support/schema.rs"]
mod schema;

use anyhow::Result;
use async_trait::async_trait;
use chrono::{Duration, Utc};
use deadpool_postgres::{Pool, Transaction};
use electoral_log::messages::message::{Message, SigningData};
use electoral_log::seal::{
    ballot_hash, build, BallotBoxSealManifest, DEFAULT_SEAL_CHANNEL, SEAL_FORMAT_V1,
};
use sequent_core::ballot::{
    ContestEncryptionPolicy, EGracePeriodPolicy, ElectionEventStatus, ElectionPresentation,
    ElectionStatus, SignedHashableBallot, VotingPeriodEnd, VotingStatus, VotingStatusChannel,
    TYPES_VERSION,
};
use serde_json::{json, Value};
use std::sync::Mutex;
use std::time::Instant;
use strand::serialization::StrandSerialize;
use strand::signature::StrandSignatureSk;
use uuid::Uuid;
use windmill::postgres::ballot_box_seal::list_for_elections;
use windmill::postgres::trusted_write;
use windmill::services::ballot_box_seal::publish::publish_box;
use windmill::services::ballot_box_seal::seal::{computed_ballot_id, entries, seal_box, BoxBallot};
use windmill::services::ballot_box_seal::{Census, ProductionSealEnvironment, SealEnvironment};
use windmill::services::protocol_manager::{create_protocol_manager_keys, get_board_client};
use windmill::services::voting_status::update_election_status;

const BOXES: usize = 130;
const LARGEST_BALLOTS: usize = 50_000;
/// Of the largest box: voters who vote twice (the second ballot counts).
const LARGEST_REVOTERS: usize = 5_000;
/// Of the largest box: voters not in the census (not eligible).
const LARGEST_NOT_IN_CENSUS: usize = 500;
const SMALL_BALLOTS: usize = 50;
const INSERT_CHUNK: usize = 5_000;
const GUARD_BULK_ROWS: usize = 10_000;
const GUARD_SINGLE_ROWS: usize = 1_000;
const BOARD_PREFIX: &str = "sealperf";
const ADMIN: &str = "admin";

struct PerfEnvironment {
    census: Census,
    uploads: Mutex<Vec<(String, usize)>>,
}

#[async_trait]
impl SealEnvironment for PerfEnvironment {
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
    let ballot_id = computed_ballot_id(&content, &ContestEncryptionPolicy::SINGLE_CONTEST).unwrap();
    (content, ballot_id)
}

struct Rows {
    voters: Vec<String>,
    contents: Vec<String>,
    ballot_ids: Vec<String>,
    created: Vec<chrono::DateTime<Utc>>,
}

fn box_rows(template: &Option<Value>, prefix: &str, voters: usize, revoters: usize) -> Rows {
    let mut rows = Rows {
        voters: vec![],
        contents: vec![],
        ballot_ids: vec![],
        created: vec![],
    };
    let now = Utc::now();
    let mut push = |voter: String, seed: String, minutes_ago: i64| {
        let (content, ballot_id) = ballot(template, &seed);
        rows.voters.push(voter);
        rows.contents.push(content);
        rows.ballot_ids.push(ballot_id);
        rows.created.push(now - Duration::minutes(minutes_ago));
    };
    for voter in 0..voters {
        push(
            format!("{prefix}-v{voter}"),
            format!("{prefix}-{voter}-1"),
            20,
        );
    }
    for voter in 0..revoters {
        push(
            format!("{prefix}-v{voter}"),
            format!("{prefix}-{voter}-2"),
            10,
        );
    }
    rows
}

async fn insert_rows(
    pool: &Pool,
    ids: (Uuid, Uuid, Uuid, Uuid),
    rows: &Rows,
    chunk: usize,
) -> std::time::Duration {
    let (tenant, event, election, area) = ids;
    let client = pool.get().await.unwrap();
    let started = Instant::now();
    for start in (0..rows.voters.len()).step_by(chunk) {
        let end = (start + chunk).min(rows.voters.len());
        client
            .execute(
                "INSERT INTO sequent_backend.cast_vote
                     (tenant_id, election_event_id, election_id, area_id, voter_id_string,
                      status, content, ballot_id, created_at, annotations)
                 SELECT $1, $2, $3, $4, voter, 'valid', content, ballot_id, created,
                        '{\"voting_channel\": \"ONLINE\"}'::jsonb
                 FROM unnest($5::text[], $6::text[], $7::text[], $8::timestamptz[])
                      AS t(voter, content, ballot_id, created)",
                &[
                    &tenant,
                    &event,
                    &election,
                    &area,
                    &rows.voters[start..end].to_vec(),
                    &rows.contents[start..end].to_vec(),
                    &rows.ballot_ids[start..end].to_vec(),
                    &rows.created[start..end].to_vec(),
                ],
            )
            .await
            .unwrap();
    }
    started.elapsed()
}

fn secs(duration: std::time::Duration) -> String {
    format!("{:.3} s", duration.as_secs_f64())
}

#[tokio::test]
#[ignore = "perf measurement: run by hand against the stack"]
async fn seal_130_boxes_the_largest_with_50k_ballots() {
    let template: Option<Value> = std::env::var("PERF_BALLOT_TEMPLATE")
        .ok()
        .map(|path| serde_json::from_str(std::fs::read_to_string(path).unwrap().trim()).unwrap());
    println!(
        "ballots: {}",
        match &template {
            Some(value) => format!("real template, {} bytes each", value.to_string().len()),
            None => "synthetic, contest-less".to_string(),
        }
    );

    // The event, one election, 130 areas, the event's PM key and board.
    let pool = schema::pool().await;
    let (tenant, event, election) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
    let areas: Vec<Uuid> = (0..BOXES).map(|_| Uuid::new_v4()).collect();
    let scratch_area = Uuid::new_v4();
    let board = format!("{BOARD_PREFIX}{}", event.simple());
    {
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
        tx.execute(
            "INSERT INTO sequent_backend.election
                 (id, tenant_id, election_event_id, status, voting_channels, presentation,
                  num_allowed_revotes)
             VALUES ($1, $2, $3, $4, $5, $6, 0)",
            &[
                &election,
                &tenant,
                &event,
                &serde_json::to_value(&status).unwrap(),
                &json!({"online": true, "kiosk": false}),
                &serde_json::to_value(&presentation).unwrap(),
            ],
        )
        .await
        .unwrap();
        for (index, area) in areas.iter().chain([&scratch_area]).enumerate() {
            tx.execute(
                "INSERT INTO sequent_backend.area (id, tenant_id, election_event_id, name)
                 VALUES ($1, $2, $3, $4)",
                &[area, &tenant, &event, &format!("Area {index}")],
            )
            .await
            .unwrap();
        }
        create_protocol_manager_keys(&tx, &tenant.to_string(), &event.to_string(), &board)
            .await
            .unwrap();
        tx.commit().await.unwrap();
    }
    get_board_client()
        .await
        .unwrap()
        .upsert_electoral_log_db(&board)
        .await
        .unwrap();

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

    let elapsed = insert_rows(
        &pool,
        (tenant, event, election, areas[0]),
        &largest,
        INSERT_CHUNK,
    )
    .await;
    println!(
        "insert the largest box ({} rows, {INSERT_CHUNK}-row statements, guard on): {}",
        largest.voters.len(),
        secs(elapsed)
    );
    let started = Instant::now();
    for (index, rows) in small.iter().enumerate() {
        insert_rows(
            &pool,
            (tenant, event, election, areas[index + 1]),
            rows,
            INSERT_CHUNK,
        )
        .await;
    }
    println!(
        "insert the 129 small boxes ({} rows each): {}",
        SMALL_BALLOTS,
        secs(started.elapsed())
    );

    // Guard overhead: the same inserts into an unsealed scratch box, with the
    // trigger enabled and disabled (bulk rolled back, single rows deleted).
    let guard_rows = box_rows(&template, "guard", GUARD_BULK_ROWS, 0);
    let ids = (tenant, event, election, scratch_area);
    let mut report = vec![];
    for enabled in [true, false, true, false] {
        let client = pool.get().await.unwrap();
        let toggle = if enabled { "ENABLE" } else { "DISABLE" };
        client
            .batch_execute(&format!(
                "ALTER TABLE sequent_backend.cast_vote {toggle} TRIGGER cast_vote_seal_guard"
            ))
            .await
            .unwrap();
        // Bulk: one 10,000-row statement, rolled back.
        let mut client = pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        let started = Instant::now();
        tx.execute(
            "INSERT INTO sequent_backend.cast_vote
                 (tenant_id, election_event_id, election_id, area_id, voter_id_string,
                  status, content, ballot_id, created_at, annotations)
             SELECT $1, $2, $3, $4, voter, 'valid', content, ballot_id, created,
                    '{\"voting_channel\": \"ONLINE\"}'::jsonb
             FROM unnest($5::text[], $6::text[], $7::text[], $8::timestamptz[])
                  AS t(voter, content, ballot_id, created)",
            &[
                &ids.0,
                &ids.1,
                &ids.2,
                &ids.3,
                &guard_rows.voters,
                &guard_rows.contents,
                &guard_rows.ballot_ids,
                &guard_rows.created,
            ],
        )
        .await
        .unwrap();
        let bulk = started.elapsed();
        tx.rollback().await.unwrap();
        // Single rows: 1,000 autocommitted inserts, as casts arrive.
        let client = pool.get().await.unwrap();
        let started = Instant::now();
        for index in 0..GUARD_SINGLE_ROWS {
            client
                .execute(
                    "INSERT INTO sequent_backend.cast_vote
                         (tenant_id, election_event_id, election_id, area_id, voter_id_string,
                          status, content, ballot_id, annotations)
                     VALUES ($1, $2, $3, $4, $5, 'valid', $6, $7,
                             '{\"voting_channel\": \"ONLINE\"}'::jsonb)",
                    &[
                        &ids.0,
                        &ids.1,
                        &ids.2,
                        &ids.3,
                        &guard_rows.voters[index],
                        &guard_rows.contents[index],
                        &guard_rows.ballot_ids[index],
                    ],
                )
                .await
                .unwrap();
        }
        let single = started.elapsed();
        client
            .execute(
                "DELETE FROM sequent_backend.cast_vote WHERE area_id = $1",
                &[&scratch_area],
            )
            .await
            .unwrap();
        report.push((enabled, bulk, single));
    }
    pool.get()
        .await
        .unwrap()
        .batch_execute("ALTER TABLE sequent_backend.cast_vote ENABLE TRIGGER cast_vote_seal_guard")
        .await
        .unwrap();
    for (enabled, bulk, single) in &report {
        println!(
            "guard {}: {GUARD_BULK_ROWS}-row INSERT {} ({:.1} µs/row); {GUARD_SINGLE_ROWS} single-row INSERTs {} ({:.0} µs/row)",
            if *enabled { "on " } else { "off" },
            secs(*bulk),
            bulk.as_secs_f64() * 1e6 / GUARD_BULK_ROWS as f64,
            secs(*single),
            single.as_secs_f64() * 1e6 / GUARD_SINGLE_ROWS as f64,
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
        let client = pool.get().await.unwrap();
        let started = Instant::now();
        let rows = client
            .query(
                "SELECT id, voter_id_string, status, content, ballot_id,
                        COALESCE(annotations->>'voting_channel', $5) AS channel, created_at
                 FROM sequent_backend.cast_vote
                 WHERE tenant_id = $1 AND election_event_id = $2 AND election_id = $3
                   AND area_id = $4",
                &[&tenant, &event, &election, &areas[0], &DEFAULT_SEAL_CHANNEL],
            )
            .await
            .unwrap();
        let ballots: Vec<BoxBallot> = rows
            .into_iter()
            .map(|row| BoxBallot {
                id: row.get("id"),
                voter_id: row.get("voter_id_string"),
                status: row.get("status"),
                content: row.get("content"),
                ballot_id: row.get("ballot_id"),
                channel: row.get("channel"),
                created_at: row.get("created_at"),
            })
            .collect();
        println!(
            "largest: read {} rows: {}",
            ballots.len(),
            secs(started.elapsed())
        );
        let started = Instant::now();
        for ballot in &ballots {
            computed_ballot_id(
                ballot.content.as_deref().unwrap(),
                &ContestEncryptionPolicy::SINGLE_CONTEST,
            )
            .unwrap();
        }
        println!(
            "largest: Ballot ID check alone (parse + hash_ballot): {}",
            secs(started.elapsed())
        );
        let started = Instant::now();
        for ballot in &ballots {
            ballot_hash(ballot.content.as_deref().unwrap()).unwrap();
        }
        println!(
            "largest: SHA-512 of the contents alone: {}",
            secs(started.elapsed())
        );
        let started = Instant::now();
        let sealed_entries =
            entries(&ballots, &census, ContestEncryptionPolicy::SINGLE_CONTEST).unwrap();
        println!(
            "largest: entries() = selection + Ballot ID check + hashing: {}",
            secs(started.elapsed())
        );
        let started = Instant::now();
        let built = build(BallotBoxSealManifest {
            format: SEAL_FORMAT_V1.to_string(),
            tenant_id: tenant.to_string(),
            election_event_id: event.to_string(),
            election_id: election.to_string(),
            area_id: areas[0].to_string(),
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
        let mut client = pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        let key = ProductionSealEnvironment
            .signing_key(&tx, &tenant.to_string(), &event.to_string(), &board)
            .await
            .unwrap();
        tx.rollback().await.unwrap();
        let started = Instant::now();
        let message = Message::ballot_box_sealed_message(
            &built.manifest,
            built.hash,
            "Election",
            "Area 0",
            &SigningData::new(key.clone(), "", key),
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

    // Publish the largest: the log delivery and the record.
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

    // Drop the private board.
    if let Ok(mut board_client) = get_board_client().await {
        let _ = board_client.delete_database(&board).await;
    }
}
