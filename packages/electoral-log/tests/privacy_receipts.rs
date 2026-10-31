// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Receipt audit/cast exclusion and atomic receive attribution on disposable PostgreSQL.
use anyhow::Result;
use chrono::Utc;
use electoral_log::adapters::ballot_box::{
    AcceptBallot, AcceptOutcome, AcceptReceived, BallotStatus,
};
use electoral_log::adapters::ballot_box_received::{
    AuditOutcome, ReceiveBallot, ReceiveOutcome, ReceivedNetwork, ReceivedSignature,
};
use electoral_log::adapters::postgres::PostgresStore;
use electoral_log::adapters::privacy::{BallotIdentity, ReceivedEvidenceKind};
use uuid::Uuid;

struct Fixture {
    store: PostgresStore,
    event: String,
    election: String,
    area: String,
    pseudonym: String,
}
impl Fixture {
    async fn new() -> Result<Self> {
        Self::with_identity(BallotIdentity::Pseudonym).await
    }
    async fn with_identity(identity: BallotIdentity) -> Result<Self> {
        let store = PostgresStore::new(std::env::var("ELECTORAL_LOG_TEST_DATABASE_URL")?.parse()?)?;
        store.initialize().await?;
        let event = Uuid::new_v4().to_string();
        store.create_ballot_box(&event).await?;
        let pseudonym = if identity == BallotIdentity::Pseudonym {
            store.set_ballot_identity(&event, identity).await?;
            store
                .allocate_voter_pseudonyms(&event, &["synthetic-voter".into()])
                .await?;
            store
                .voter_pseudonym(&event, "synthetic-voter")
                .await?
                .unwrap()
        } else {
            "synthetic-voter".into()
        };
        Ok(Self {
            store,
            event,
            election: Uuid::new_v4().to_string(),
            area: Uuid::new_v4().to_string(),
            pseudonym,
        })
    }
    fn ballot(&self) -> ReceiveBallot<'_> {
        ReceiveBallot {
            election_event_id: &self.event,
            election_id: &self.election,
            area_id: &self.area,
            voter_id: &self.pseudonym,
            ballot_hash: "synthetic-hash",
            content: "encrypted",
            voter_signing_pk: "synthetic-public-key",
            voter_ballot_signature: "synthetic-signature",
            vitrine: None,
        }
    }
    async fn cleanup(self) -> Result<()> {
        self.store.drop_ballot_box(&self.event).await?;
        Ok(())
    }
}
fn signature(_: chrono::DateTime<Utc>) -> Result<ReceivedSignature> {
    Ok(ReceivedSignature {
        ballot_id: "ABCD-1234".into(),
        key_id: "synthetic-key".into(),
        received_signature: "synthetic-receipt".into(),
    })
}

#[tokio::test]
#[ignore = "needs isolated ELECTORAL_LOG_TEST_DATABASE_URL"]
async fn authenticated_receipt_lookup_keeps_the_mapping_inside_its_query() -> Result<()> {
    for mode in [BallotIdentity::VoterId, BallotIdentity::Pseudonym] {
        let f = Fixture::with_identity(mode).await?;
        f.store
            .receive_ballot(&f.ballot(), Utc::now(), signature)
            .await?;
        // The legacy fixture deliberately never creates an identity-policy row.
        let owned = f
            .store
            .received_ballot_for_owner(
                &f.event,
                &f.election,
                &f.area,
                "synthetic-voter",
                "ABCD-1234",
                mode,
            )
            .await?
            .unwrap();
        assert_eq!(owned.voter_id, f.pseudonym);
        for (event, election, area, voter) in [
            (
                Uuid::new_v4().to_string(),
                f.election.clone(),
                f.area.clone(),
                "synthetic-voter",
            ),
            (
                f.event.clone(),
                Uuid::new_v4().to_string(),
                f.area.clone(),
                "synthetic-voter",
            ),
            (
                f.event.clone(),
                f.election.clone(),
                Uuid::new_v4().to_string(),
                "synthetic-voter",
            ),
            (
                f.event.clone(),
                f.election.clone(),
                f.area.clone(),
                "another-voter",
            ),
        ] {
            assert!(f
                .store
                .received_ballot_for_owner(&event, &election, &area, voter, "ABCD-1234", mode)
                .await?
                .is_none());
        }
        if mode == BallotIdentity::Pseudonym {
            assert!(f
                .store
                .received_ballot_for_owner(
                    &f.event,
                    &f.election,
                    &f.area,
                    &f.pseudonym,
                    "ABCD-1234",
                    mode
                )
                .await?
                .is_none());
            f.store
                .allocate_voter_pseudonyms(&f.event, &["000-first".into()])
                .await?;
            assert_eq!(f.store.purge_voter_pseudonyms_batch(&f.event, 1).await?, 1);
            assert!(f
                .store
                .voter_pseudonym(&f.event, "synthetic-voter")
                .await?
                .is_some());
            assert!(f
                .store
                .received_ballot_for_owner(
                    &f.event,
                    &f.election,
                    &f.area,
                    "synthetic-voter",
                    "ABCD-1234",
                    mode
                )
                .await?
                .is_none());
        }
        f.cleanup().await?;
    }
    Ok(())
}

#[tokio::test]
#[ignore = "needs isolated ELECTORAL_LOG_TEST_DATABASE_URL"]
async fn audit_is_owned_idempotent_and_excludes_concurrent_cast() -> Result<()> {
    let f = Fixture::new().await?;
    let received = f
        .store
        .receive_ballot(&f.ballot(), Utc::now(), signature)
        .await?;
    let ReceiveOutcome::Received(received) = received else {
        panic!("new receipt expected")
    };
    assert_eq!(
        f.store
            .audit_received_ballot(
                &f.event,
                &f.election,
                &f.area,
                "someone-else",
                &received.ballot_id
            )
            .await?,
        AuditOutcome::NotFound
    );
    let hash = [1; 64];
    let cast = AcceptBallot {
        election_event_id: &f.event,
        election_id: &f.election,
        area_id: &f.area,
        voter_id: &f.pseudonym,
        ballot_id: "synthetic-hash",
        format: "hashable-ballot",
        content: "encrypted",
        voter_signature: None,
        pseudonym_hash: &hash,
        ballot_hash: &hash,
        voting_channel: "ONLINE",
        status: BallotStatus::Valid,
        voter_ip: None,
        voter_country: None,
        username: Some(&f.pseudonym),
        allowed_votes: 1,
        vitrine: None,
    };
    let cast_receipt = AcceptReceived {
        ballot_id: &received.ballot_id,
        cast_at: Utc::now(),
        cast_signature: "cast",
        cast_receipt_signature: "receipt",
    };
    let (audited, casted) = tokio::join!(
        f.store.audit_received_ballot(
            &f.event,
            &f.election,
            &f.area,
            &f.pseudonym,
            &received.ballot_id
        ),
        f.store.accept_received_ballot(&cast, &cast_receipt)
    );
    match (audited?, casted?) {
        (AuditOutcome::Audited, AcceptOutcome::NotReceivable) => {
            assert_eq!(
                f.store
                    .audit_received_ballot(
                        &f.event,
                        &f.election,
                        &f.area,
                        &f.pseudonym,
                        &received.ballot_id
                    )
                    .await?,
                AuditOutcome::Audited
            );
            let evidence = f.store.pending_received_ballots(&f.event, 10).await?;
            assert_eq!(
                evidence
                    .iter()
                    .filter(|p| p.kind == ReceivedEvidenceKind::Audited)
                    .count(),
                1
            );
        }
        (AuditOutcome::AlreadyCast, AcceptOutcome::Accepted { .. }) => {
            assert_eq!(
                f.store
                    .audit_received_ballot(
                        &f.event,
                        &f.election,
                        &f.area,
                        &f.pseudonym,
                        &received.ballot_id
                    )
                    .await?,
                AuditOutcome::AlreadyCast
            );
        }
        outcomes => panic!("audit and cast must be exclusive: {outcomes:?}"),
    }
    f.cleanup().await?;
    // Exercise the audit winner regardless of the scheduler's race result.
    let f = Fixture::new().await?;
    f.store
        .receive_ballot(&f.ballot(), Utc::now(), signature)
        .await?;
    for _ in 0..2 {
        assert_eq!(
            f.store
                .audit_received_ballot(&f.event, &f.election, &f.area, &f.pseudonym, "ABCD-1234")
                .await?,
            AuditOutcome::Audited
        );
    }
    let evidence = f.store.pending_received_ballots(&f.event, 10).await?;
    assert_eq!(
        evidence
            .iter()
            .filter(|p| p.kind == ReceivedEvidenceKind::Audited)
            .count(),
        1
    );
    let ballot = f
        .store
        .received_ballot(&f.event, &f.election, &f.pseudonym, "synthetic-hash")
        .await?
        .unwrap();
    assert_eq!(
        ballot.status,
        electoral_log::adapters::ballot_box_received::ReceivedStatus::Audited
    );
    f.cleanup().await
}

#[tokio::test]
#[ignore = "needs isolated ELECTORAL_LOG_TEST_DATABASE_URL"]
async fn receive_attribution_rolls_back_together_and_retries_preserve_original_network(
) -> Result<()> {
    let f = Fixture::new().await?;
    let name = format!("privacy_receive_failure_{}", Uuid::new_v4().simple());
    let connection = f.store.client().await?;
    connection.batch_execute(&format!(
        "CREATE FUNCTION {name}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'synthetic_forensic_failure'; END $$;
         CREATE TRIGGER {name} BEFORE INSERT ON sequent_backend.forensic_network_record FOR EACH ROW
         WHEN (NEW.election_event_id = '{}'::uuid) EXECUTE FUNCTION {name}();", f.event)).await?;
    let original = ReceivedNetwork {
        ip: "192.0.2.10",
        country: Some("XX"),
        user_agent: Some("original-device"),
    };
    let failed = f
        .store
        .receive_ballot_with_network(&f.ballot(), Utc::now(), Some(&original), signature)
        .await;
    connection.batch_execute(&format!("DROP TRIGGER {name} ON sequent_backend.forensic_network_record; DROP FUNCTION {name}();")).await?;
    let failed = failed.expect_err("injected attribution failure must reach caller");
    assert_eq!(
        failed
            .downcast_ref::<tokio_postgres::Error>()
            .and_then(|error| error.as_db_error())
            .map(|error| error.message()),
        Some("synthetic_forensic_failure")
    );
    assert!(
        f.store
            .received_ballot(&f.event, &f.election, &f.pseudonym, "synthetic-hash")
            .await?
            .is_none(),
        "failed attribution must not leave a received ballot"
    );
    assert!(
        f.store
            .pending_received_ballots(&f.event, 10)
            .await?
            .is_empty(),
        "failed attribution must not queue evidence"
    );
    let first = f
        .store
        .receive_ballot_with_network(&f.ballot(), Utc::now(), Some(&original), signature)
        .await?;
    assert!(matches!(first, ReceiveOutcome::Received(_)));
    let retry = ReceivedNetwork {
        ip: "203.0.113.77",
        country: Some("YY"),
        user_agent: Some("retry-device"),
    };
    assert!(matches!(
        f.store
            .receive_ballot_with_network(&f.ballot(), Utc::now(), Some(&retry), signature)
            .await?,
        ReceiveOutcome::AlreadyReceived(_)
    ));
    let records = connection.query("SELECT ip,country,user_agent FROM sequent_backend.forensic_network_record WHERE election_event_id=$1::text::uuid", &[&f.event]).await?;
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].get::<_, String>(0), original.ip);
    assert_eq!(records[0].get::<_, String>(1), "XX");
    assert_eq!(records[0].get::<_, String>(2), "original-device");
    drop(connection);
    f.cleanup().await
}

#[tokio::test]
#[ignore = "needs isolated ELECTORAL_LOG_TEST_DATABASE_URL"]
async fn cast_attribution_failure_rolls_back_receipt_and_vote_side_effects() -> Result<()> {
    use electoral_log::adapters::privacy::ForensicNetworkRecord;
    let f = Fixture::new().await?;
    f.store
        .receive_ballot(&f.ballot(), Utc::now(), signature)
        .await?;
    let hash = [1; 64];
    let cast = AcceptBallot {
        election_event_id: &f.event,
        election_id: &f.election,
        area_id: &f.area,
        voter_id: &f.pseudonym,
        ballot_id: "synthetic-hash",
        format: "hashable-ballot",
        content: "encrypted",
        voter_signature: None,
        pseudonym_hash: &hash,
        ballot_hash: &hash,
        voting_channel: "ONLINE",
        status: BallotStatus::Valid,
        voter_ip: None,
        voter_country: None,
        username: Some(&f.pseudonym),
        allowed_votes: 1,
        vitrine: None,
    };
    let receipt = AcceptReceived {
        ballot_id: "ABCD-1234",
        cast_at: Utc::now(),
        cast_signature: "cast",
        cast_receipt_signature: "receipt",
    };
    let record = ForensicNetworkRecord {
        election_event_id: &f.event,
        election_id: Some(&f.election),
        area_id: Some(&f.area),
        event_kind: "cast",
        delivery_id: "cast-with-forensics",
        pseudonym: &f.pseudonym,
        ballot_id: Some(receipt.ballot_id),
        ip: "192.0.2.25",
        country: Some("XX"),
        user_agent: Some("synthetic-device"),
        recorded_at: receipt.cast_at,
    };
    let name = format!("privacy_cast_failure_{}", Uuid::new_v4().simple());
    let connection = f.store.client().await?;
    connection.batch_execute(&format!(
        "CREATE FUNCTION {name}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'synthetic_forensic_failure'; END $$;
         CREATE TRIGGER {name} BEFORE INSERT ON sequent_backend.forensic_network_record FOR EACH ROW
         WHEN (NEW.election_event_id = '{}'::uuid) EXECUTE FUNCTION {name}();", f.event)).await?;
    let failed = f
        .store
        .accept_received_ballot_with_forensics(&cast, &receipt, &record)
        .await;
    connection.batch_execute(&format!("DROP TRIGGER {name} ON sequent_backend.forensic_network_record; DROP FUNCTION {name}();")).await?;
    let failed = failed.expect_err("injected attribution failure must reach caller");
    assert_eq!(
        failed
            .downcast_ref::<tokio_postgres::Error>()
            .and_then(|error| error.as_db_error())
            .map(|error| error.message()),
        Some("synthetic_forensic_failure")
    );
    let stored = f
        .store
        .received_ballot(&f.event, &f.election, &f.pseudonym, "synthetic-hash")
        .await?
        .unwrap();
    assert_eq!(
        stored.status,
        electoral_log::adapters::ballot_box_received::ReceivedStatus::Received
    );
    assert!(f
        .store
        .voter_state(&f.event, &f.election, &f.pseudonym)
        .await?
        .is_none());
    assert!(f.store.pending_ballots(&f.event, 10).await?.is_empty());
    assert!(matches!(
        f.store
            .accept_received_ballot_with_forensics(&cast, &receipt, &record)
            .await?,
        AcceptOutcome::Accepted { .. }
    ));
    let records: i64 = connection.query_one("SELECT count(*) FROM sequent_backend.forensic_network_record WHERE election_event_id=$1::text::uuid", &[&f.event]).await?.get(0);
    assert_eq!(records, 1);
    drop(connection);
    f.cleanup().await
}

/// A bounded matched storage comparison. This deliberately excludes crypto,
/// HTTP, registration and summary refresh; it is not an end-to-end SLA test.
#[tokio::test]
#[ignore = "isolated PostgreSQL benchmark; run alone with --nocapture"]
async fn matched_receipt_and_cast_storage_cost() -> Result<()> {
    use electoral_log::adapters::privacy::ForensicNetworkRecord;
    use std::time::Instant;
    const WARMUPS: usize = 2;
    const SAMPLES: usize = 12;
    const BATCH: usize = 8;
    #[derive(Clone, Copy)]
    enum Mode {
        Legacy,
        Private,
        Forensic,
    }
    let modes = [Mode::Legacy, Mode::Private, Mode::Forensic];
    let fixtures = [
        Fixture::with_identity(BallotIdentity::VoterId).await?,
        Fixture::new().await?,
        Fixture::new().await?,
    ];
    let network = ReceivedNetwork {
        ip: "192.0.2.50",
        country: Some("XX"),
        user_agent: Some("matched-storage-benchmark"),
    };
    let mut samples: [Vec<f64>; 3] = std::array::from_fn(|_| Vec::with_capacity(SAMPLES));
    for sample in 0..(WARMUPS + SAMPLES) {
        // Rotate order so later modes do not consistently inherit a warmer DB.
        for offset in 0..modes.len() {
            let index = (sample + offset) % modes.len();
            let mode = modes[index];
            let f = &fixtures[index];
            let started = Instant::now();
            for item in 0..BATCH {
                let sequence = sample * BATCH + item;
                let ballot_hash = format!("matched-ballot-{sequence}");
                let code = format!("0000-{sequence:04X}");
                let mut ballot = f.ballot();
                ballot.ballot_hash = &ballot_hash;
                let received = f
                    .store
                    .receive_ballot_with_network(
                        &ballot,
                        Utc::now(),
                        match mode {
                            Mode::Forensic => Some(&network),
                            _ => None,
                        },
                        |_| {
                            Ok(ReceivedSignature {
                                ballot_id: code.clone(),
                                key_id: "synthetic-key".into(),
                                received_signature: "synthetic-receipt".into(),
                            })
                        },
                    )
                    .await?;
                let ReceiveOutcome::Received(received) = received else {
                    panic!("each benchmark ballot must be new")
                };
                let hash = [1; 64];
                let cast = AcceptBallot {
                    election_event_id: &f.event,
                    election_id: &f.election,
                    area_id: &f.area,
                    voter_id: &f.pseudonym,
                    ballot_id: &ballot_hash,
                    format: "hashable-ballot",
                    content: "encrypted",
                    voter_signature: None,
                    pseudonym_hash: &hash,
                    ballot_hash: &hash,
                    voting_channel: "ONLINE",
                    status: BallotStatus::Valid,
                    voter_ip: None,
                    voter_country: None,
                    username: Some(&f.pseudonym),
                    allowed_votes: 0,
                    vitrine: None,
                };
                let receipt = AcceptReceived {
                    ballot_id: &received.ballot_id,
                    cast_at: Utc::now(),
                    cast_signature: "cast",
                    cast_receipt_signature: "receipt",
                };
                let delivery = format!("cast:{sequence}");
                let outcome = match mode {
                    Mode::Forensic => {
                        f.store
                            .accept_received_ballot_with_forensics(
                                &cast,
                                &receipt,
                                &ForensicNetworkRecord {
                                    election_event_id: &f.event,
                                    election_id: Some(&f.election),
                                    area_id: Some(&f.area),
                                    event_kind: "cast",
                                    delivery_id: &delivery,
                                    pseudonym: &f.pseudonym,
                                    ballot_id: Some(receipt.ballot_id),
                                    ip: network.ip,
                                    country: network.country,
                                    user_agent: network.user_agent,
                                    recorded_at: receipt.cast_at,
                                },
                            )
                            .await?
                    }
                    _ => f.store.accept_received_ballot(&cast, &receipt).await?,
                };
                assert!(matches!(outcome, AcceptOutcome::Accepted { .. }));
            }
            if sample >= WARMUPS {
                samples[index].push(started.elapsed().as_secs_f64() * 1_000.0);
            }
        }
    }
    for (name, batches) in [
        "legacy-signed-receipt",
        "pseudonym-do-not-store",
        "pseudonym-forensic-store",
    ]
    .into_iter()
    .zip(samples)
    {
        let elapsed_ms: f64 = batches.iter().sum();
        let mut per_pair: Vec<_> = batches.iter().map(|ms| ms / BATCH as f64).collect();
        per_pair.sort_by(f64::total_cmp);
        println!("{name}: warmups={WARMUPS} samples={SAMPLES} batch={BATCH} elapsed_ms={elapsed_ms:.3} p50_batch_mean_ms_per_pair={:.3} p95_batch_mean_ms_per_pair={:.3} raw_batch_ms={batches:?}",
            per_pair[(SAMPLES * 50).div_ceil(100) - 1], per_pair[(SAMPLES * 95).div_ceil(100) - 1]);
    }
    for fixture in fixtures {
        fixture.cleanup().await?;
    }
    Ok(())
}
