// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! VOTE-FREEZE from close to count with the real pieces: Stop Voting
//! creates the seal, the sealer seals and the publisher posts and publishes
//! it, the public record verifies offline, and the tally counts the box from
//! that seal and records its hash in the contest annotations.
//!
//! Needs `HASURA_DB__*` and `IMMUDB_*` (the stack's `.devcontainer/.env`,
//! or the immudb service of the CI jobs) and sets its own `MASTER_SECRET`.
//! The census and the public bucket are the test's own; the signing key is
//! the event's protocol manager key.

#![recursion_limit = "256"]

#[path = "support/schema.rs"]
mod schema;

use anyhow::Result;
use async_trait::async_trait;
use chrono::{Duration, Utc};
use deadpool_postgres::Transaction;
use electoral_log::client::board_client::{
    ElectoralLogVarCharColumn, SqlCompOperators, WhereClauseBTreeMap,
};
use electoral_log::messages::statement::StatementType;
use electoral_log::seal::{verify_record, SealRecord};
use sequent_core::ballot::{
    ContestEncryptionPolicy, EGracePeriodPolicy, ElectionEventStatus, ElectionPresentation,
    ElectionStatus, SignedHashableBallot, VotingPeriodEnd, VotingStatus, VotingStatusChannel,
    WeightedVotingPolicy, TYPES_VERSION,
};
use sequent_core::types::hasura::core::{ElectionEvent, TallySessionContestAnnotations};
use sequent_core::types::participation::ParticipationChannel;
use serde_json::json;
use std::sync::{Mutex, Once};
use strand::signature::StrandSignatureSk;
use uuid::Uuid;
use windmill::postgres::ballot_box_seal::{list_for_elections, BallotBoxSealStatus};
use windmill::postgres::election_event::get_election_event_by_id;
use windmill::postgres::trusted_write;
use windmill::services::ballot_box_seal::publish::{publish_box, PublishOutcome};
use windmill::services::ballot_box_seal::seal::{computed_ballot_id, seal_box, SealOutcome};
use windmill::services::ballot_box_seal::{Census, ProductionSealEnvironment, SealEnvironment};
use windmill::services::ceremonies::insert_ballots::contest_annotations;
use windmill::services::ceremonies::sealed_box_ballots::{
    sealed_box_ballots, SealedBox, SealedBoxLog,
};
use windmill::services::protocol_manager::{create_protocol_manager_keys, get_board_client};
use windmill::services::voting_status::update_election_status;

const ADMIN: &str = "admin";
const SESSION: &str = "chain-tally-session";
const BOARD_PREFIX: &str = "sealchain";
const BOARD_ATTEMPTS: u32 = 5;

/// The secrets of an event are encrypted with the deployment's master secret.
fn master_secret() {
    static SET: Once = Once::new();
    SET.call_once(|| std::env::set_var("MASTER_SECRET", "5a".repeat(32)));
}

/// The test's census and public bucket, with the event's real key.
struct TestEnvironment {
    census: Census,
    uploads: Mutex<Vec<Vec<u8>>>,
}

#[async_trait]
impl SealEnvironment for TestEnvironment {
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

    async fn census(&self, _: &ElectionEvent, _: &str, _: &str) -> Result<Census> {
        Ok(self.census.clone())
    }

    async fn upload_record(
        &self,
        _: &Transaction<'_>,
        _: &str,
        _: &str,
        _: &str,
        json: &[u8],
        document_id: Uuid,
        _: sequent_core::ballot::BallotBoxSealRecordPolicy,
    ) -> Result<Uuid> {
        self.uploads.lock().unwrap().push(json.to_vec());
        Ok(document_id)
    }
}

/// A stored ballot's content and its Ballot ID.
fn ballot(seed: &str) -> (String, String) {
    let content = serde_json::to_string(&SignedHashableBallot {
        version: TYPES_VERSION,
        issue_date: seed.to_string(),
        contests: vec![],
        config: seed.to_string(),
        ballot_style_hash: seed.to_string(),
        voter_signing_pk: None,
        voter_ballot_signature: None,
    })
    .unwrap();
    let ballot_id = computed_ballot_id(&content, &ContestEncryptionPolicy::SINGLE_CONTEST).unwrap();
    (content, ballot_id)
}

#[tokio::test]
async fn a_closed_box_is_sealed_published_verified_and_counted_from_its_seal() {
    let pool = schema::pool().await;
    let (tenant, event, election, area) = (
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
    );
    let board = format!("{BOARD_PREFIX}{}", event.simple());

    // An open election of a seal-at-close event, with its board and key.
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
                &json!({"ballot_box_seal_policy": "seal-at-close"}),
                &json!({"id": 1, "database_name": board, "is_archived": false}),
                &json!({"online": true}),
            ],
        )
        .await
        .unwrap();
        let mut status = ElectionStatus::default();
        status.set_status_by_channel(VotingStatusChannel::ONLINE, VotingStatus::OPEN);
        let presentation = ElectionPresentation {
            grace_period_policy: Some(EGracePeriodPolicy::NO_GRACE_PERIOD),
            voting_period_end: Some(VotingPeriodEnd::ALLOWED),
            ..Default::default()
        };
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
                &json!({"online": true}),
                &serde_json::to_value(&presentation).unwrap(),
            ],
        )
        .await
        .unwrap();
        tx.execute(
            "INSERT INTO sequent_backend.area (id, tenant_id, election_event_id, name)
             VALUES ($1, $2, $3, 'North')",
            &[&area, &tenant, &event],
        )
        .await
        .unwrap();
        master_secret();
        create_protocol_manager_keys(&tx, &tenant.to_string(), &event.to_string(), &board)
            .await
            .unwrap();
        tx.commit().await.unwrap();
    }
    let mut attempts = 0;
    loop {
        attempts += 1;
        match get_board_client()
            .await
            .unwrap()
            .upsert_electoral_log_db(&board)
            .await
        {
            Ok(()) => break,
            Err(error) if attempts < BOARD_ATTEMPTS => {
                eprintln!("creating board {board}: {error}; retrying");
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            }
            Err(error) => panic!("creating board {board}: {error}"),
        }
    }

    // Alice votes twice, Bob once, Carol isn't in the census, Dave's
    // ballot was discarded.
    let mut contents = Vec::new();
    for (voter, status, seed, minutes_ago) in [
        ("voter-alice", "valid", "a1", 3),
        ("voter-alice", "valid", "a2", 2),
        ("voter-bob", "valid", "b1", 2),
        ("voter-carol", "valid", "c1", 2),
        ("voter-dave", "discarded", "d1", 2),
    ] {
        let (content, ballot_id) = ballot(seed);
        contents.push((seed, content.clone()));
        pool.get()
            .await
            .unwrap()
            .execute(
                "INSERT INTO sequent_backend.cast_vote
                     (tenant_id, election_event_id, election_id, area_id, voter_id_string,
                      status, content, ballot_id, created_at, annotations)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
                &[
                    &tenant,
                    &event,
                    &election,
                    &area,
                    &voter,
                    &status,
                    &content,
                    &ballot_id,
                    &(Utc::now() - Duration::minutes(minutes_ago)),
                    &json!({"voting_channel": "ONLINE"}),
                ],
            )
            .await
            .unwrap();
    }

    // Stop Voting creates the seal; the sealer and the publisher do the rest.
    {
        let mut client = pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        update_election_status(
            tenant.to_string(),
            Some("admin-id"),
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
    let environment = TestEnvironment {
        census: [("voter-alice", 1), ("voter-bob", 1), ("voter-zoe", 1)]
            .into_iter()
            .map(|(voter, weight)| (voter.to_string(), weight))
            .collect(),
        uploads: Mutex::new(vec![]),
    };
    let mut client = pool.get().await.unwrap();
    let seal_id = {
        let tx = client.transaction().await.unwrap();
        let seals = list_for_elections(&tx, &tenant, &event, &[election])
            .await
            .unwrap();
        assert_eq!(seals.len(), 1);
        seals[0].id
    };
    assert_eq!(
        seal_box(&mut client, &environment, &seal_id).await.unwrap(),
        SealOutcome::Sealed
    );
    assert_eq!(
        publish_box(&mut client, &environment, &seal_id)
            .await
            .unwrap(),
        PublishOutcome::Published
    );

    // The public record verifies offline.
    let uploads = environment.uploads.lock().unwrap().clone();
    assert_eq!(uploads.len(), 1);
    let record: SealRecord = serde_json::from_slice(&uploads[0]).unwrap();
    let check = verify_record(&record, None).unwrap();

    // The tally counts the box from that seal.
    let tx = client.transaction().await.unwrap();
    let seal = list_for_elections(&tx, &tenant, &event, &[election])
        .await
        .unwrap()
        .remove(0);
    assert_eq!(seal.status, BallotBoxSealStatus::Published);
    let election_event = get_election_event_by_id(&tx, &tenant.to_string(), &event.to_string())
        .await
        .unwrap();
    let log = SealedBoxLog::new(&tx, &tenant.to_string(), &election_event)
        .await
        .unwrap();
    let (tenant_id, event_id, election_id, area_id) = (
        tenant.to_string(),
        event.to_string(),
        election.to_string(),
        area.to_string(),
    );
    let counted = sealed_box_ballots(
        &tx,
        &log,
        &SealedBox {
            tenant_id: &tenant_id,
            election_event_id: &event_id,
            election_id: &election_id,
            area_id: &area_id,
            election_name: "Mayor",
            area_name: "North",
            tally_session_id: SESSION,
        },
        WeightedVotingPolicy::default(),
    )
    .await
    .unwrap();

    let content = |seed: &str| {
        contents
            .iter()
            .find(|(candidate, _)| *candidate == seed)
            .unwrap()
            .1
            .clone()
    };
    let mut ballot_contents = counted.merge_result.ballot_contents.clone();
    ballot_contents.sort();
    let mut expected = vec![(content("a2"), 1), (content("b1"), 1)];
    expected.sort();
    assert_eq!(ballot_contents, expected);
    assert_eq!(counted.merge_result.eligible_voters, 3);
    assert_eq!(counted.merge_result.ballots_without_voter, 1);
    assert_eq!(counted.merge_result.casted_ballots, 3);
    assert_eq!(
        counted.merge_result.casted_ballots_by_channel,
        [(ParticipationChannel::from("ONLINE"), 3)]
            .into_iter()
            .collect()
    );

    // One seal hash everywhere: the row, the verified record, the count and
    // the annotation the tally stores.
    let seal_hash = seal.seal_hash.clone().unwrap();
    assert_eq!(hex::encode(check.seal_hash), seal_hash);
    assert_eq!(counted.seal_hash, seal_hash);
    let annotations = serde_json::to_value(contest_annotations(
        &counted.merge_result,
        None,
        Some(counted.seal_hash.clone()),
    ))
    .unwrap();
    assert_eq!(annotations["ballot_box_seal_hash"], json!(seal_hash));
    let stored: TallySessionContestAnnotations = serde_json::from_value(annotations).unwrap();
    assert_eq!(
        stored.ballot_box_seal_hash.as_deref(),
        Some(seal_hash.as_str())
    );
    assert_eq!(stored.elegible_voters, 3);

    // The tally's verification is on the log once.
    let filter: WhereClauseBTreeMap = [
        (
            ElectoralLogVarCharColumn::StatementKind,
            (
                SqlCompOperators::Equal,
                StatementType::TallyBallotBoxVerified.to_string(),
            ),
        ),
        (
            ElectoralLogVarCharColumn::AreaId,
            (SqlCompOperators::Equal, area_id.clone()),
        ),
    ]
    .into_iter()
    .collect();
    let mut board_client = get_board_client().await.unwrap();
    let verified = board_client
        .get_electoral_log_messages_filtered::<String, String>(
            &board,
            Some(filter),
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .unwrap();
    assert_eq!(verified.len(), 1);
    drop(tx);
    let _ = board_client.delete_database(&board).await;
}
