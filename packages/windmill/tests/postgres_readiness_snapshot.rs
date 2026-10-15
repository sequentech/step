// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Actual readiness snapshots over migrated Hasura rows, an isolated Keycloak
//! census and an event's signed audit board. Missing approved configuration and
//! publication must refuse readiness without appending a success statement.

#[path = "support/event_database.rs"]
mod event_database;
#[path = "support/schema.rs"]
mod schema;

use chrono::{Duration, Utc};
use deadpool_postgres::Transaction;
use electoral_log::messages::message::{Message, SigningData};
use electoral_log::messages::newtypes::{AuditLogEntry, AuditStatementKind, EventIdString};
use electoral_log::messages::statement::{StatementEventType, StatementLogType};
use electoral_log::ports::ElectoralLogStore;
use electoral_log::{ElectoralLogMessage, LogEntry, LogQuery};
use event_database::{run, EventDatabase};
use sequent_core::election_config::design::{ballot_design_digests, BallotDesign, DesignKeys};
use sequent_core::election_config::manifest::{
    ConfigurationRevision, Content, Manifest, ManifestFormat, Produced,
};
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::types::keycloak::AREA_ID_ATTR_NAME;
use sequent_core::types::voter_import::VotersImportedEvidence;
use serde_json::json;
use uuid::Uuid;
use voting_crypto::signing::{SigningKey, VerificationKey};
use voting_crypto::{RecordDeserialize, RecordSerialize};
use windmill::domain::readiness::CheckKind;
use windmill::postgres::voter_import::{complete_import_task, record_voter_import};
use windmill::services::audit_log::AuditActor;
use windmill::services::electoral_log_board::{
    create_protocol_manager_keys, get_board_signing_key,
};
use windmill::services::readiness::{snapshot, verify};

fn signed_receipt(event: &str, receipt: &VotersImportedEvidence, key: &SigningKey) -> LogEntry {
    let mut details = serde_json::to_value(receipt).unwrap();
    details["actor"] = json!({"user_id":"admin-id", "username":"ada", "role":null});
    let message = Message::audit_message(
        EventIdString(event.into()),
        AuditLogEntry {
            kind: AuditStatementKind::VotersImported,
            event_type: StatementEventType::USER,
            log_type: StatementLogType::INFO,
            description: "Imported 2 voters".into(),
            details_json: details.to_string(),
            entry_id: receipt.import_id.clone(),
        },
        1_792_000_000,
        &SigningData::new(key.clone(), "ada", key.clone()),
        Some("admin-id".into()),
        Some("ada".into()),
        None,
        None,
    )
    .unwrap();
    LogEntry {
        delivery_id: format!("audit:{}", receipt.import_id),
        message: ElectoralLogMessage::try_from(&message).unwrap(),
    }
}

#[derive(Clone, Copy)]
struct FixtureIds {
    tenant: Uuid,
    event: Uuid,
    election: Uuid,
    area: Uuid,
}

/// Generate the actual published ballot through the same producer as Windmill.
/// Configuration admission/certificate validation has its own package tests;
/// this fixture persists the metadata that a successful admission supplies.
async fn published_content(tx: &Transaction<'_>, ids: FixtureIds) -> Content {
    let (tenant, event) = (ids.tenant.to_string(), ids.event.to_string());
    tx.execute(
        "UPDATE sequent_backend.election_event SET presentation = $2 WHERE id = $1",
        &[
            &ids.event,
            &json!({
                "ballot_box_seal_policy":"seal-at-close", "file_integrity_policy":"REQUIRED_SHA256"
            }),
        ],
    )
    .await
    .unwrap();
    tx.execute("UPDATE sequent_backend.election SET external_id = 'approved-election', presentation = $2, voting_channels = $3 WHERE id = $1", &[&ids.election, &json!({"i18n":{"en":{"name":"General election"}}}), &json!({"online":true})]).await.unwrap();
    let contest = Uuid::new_v4();
    tx.execute("INSERT INTO sequent_backend.contest (id, tenant_id, election_event_id, election_id, external_id, min_votes, max_votes, winning_candidates_num, is_encrypted, counting_algorithm, presentation) VALUES ($1, $2, $3, $4, 'president', 0, 1, 1, true, 'plurality-at-large', $5)", &[&contest, &ids.tenant, &ids.event, &ids.election, &json!({"i18n":{"en":{"name":"President"}}})]).await.unwrap();
    tx.execute("INSERT INTO sequent_backend.candidate (id, tenant_id, election_event_id, contest_id, external_id, presentation) VALUES ($1, $2, $3, $4, 'alice', $5)", &[&Uuid::new_v4(), &ids.tenant, &ids.event, &contest, &json!({"i18n":{"en":{"name":"Alice"}}})]).await.unwrap();
    tx.execute("INSERT INTO sequent_backend.area_contest (id, tenant_id, election_event_id, area_id, contest_id) VALUES ($1, $2, $3, $4, $5)", &[&Uuid::new_v4(), &ids.tenant, &ids.event, &ids.area, &contest]).await.unwrap();
    let areas = windmill::postgres::area::get_event_areas(tx, &tenant, &event)
        .await
        .unwrap();
    let elections = windmill::postgres::election::get_elections(tx, &tenant, &event)
        .await
        .unwrap();
    let event_row =
        windmill::postgres::election_event::get_election_event_by_id(tx, &tenant, &event)
            .await
            .unwrap();
    let contests = windmill::postgres::contest::export_contests(tx, &tenant, &event)
        .await
        .unwrap();
    let candidates = windmill::postgres::candidate::export_candidates(tx, &tenant, &event)
        .await
        .unwrap();
    let keys = DesignKeys::of_entities(&areas, &elections, &contests, &candidates).unwrap();
    let style = sequent_core::ballot_style::create_ballot_style(
        Uuid::new_v4().to_string(),
        areas[0].clone(),
        event_row,
        elections[0].clone(),
        contests.clone(),
        &contests,
        candidates,
        Default::default(),
        None,
        Some("ajR/I9RqyOwbpsVRucSNOgXVLCvLpfQxCgPoXGQ2RF4".into()),
    )
    .unwrap();
    let designs = ballot_design_digests(&[style.clone()], &keys)
        .unwrap()
        .into_iter()
        .map(|design| BallotDesign {
            area: design.area,
            election: design.election,
            sha256: design.sha256,
            version: 1,
        })
        .collect();
    let publication = Uuid::new_v4();
    tx.execute("INSERT INTO sequent_backend.ballot_publication (id, tenant_id, election_event_id) VALUES ($1, $2, $3)", &[&publication, &ids.tenant, &ids.event]).await.unwrap();
    tx.execute("INSERT INTO sequent_backend.ballot_style (id, tenant_id, election_event_id, election_id, area_id, ballot_publication_id, ballot_eml) VALUES ($1, $2, $3, $4, $5, $6, $7)", &[&Uuid::parse_str(&style.id).unwrap(), &ids.tenant, &ids.event, &ids.election, &ids.area, &publication, &serde_json::to_string(&style).unwrap()]).await.unwrap();
    // Complete through the production server helper: it supplies the trusted
    // transaction mark required by the publication authority guard.
    windmill::postgres::ballot_publication::update_ballot_publication(
        tx,
        &tenant,
        &event,
        &publication.to_string(),
        true,
        Some(chrono::Local::now()),
    )
    .await
    .unwrap()
    .expect("Publication completed");
    Content {
        files: vec![],
        ballot_designs: designs,
        reports: vec![],
    }
}

async fn imported_manifest(tx: &Transaction<'_>, ids: FixtureIds, revision: u64, content: Content) {
    let manifest = Manifest {
        format: ManifestFormat::V1,
        configuration: ConfigurationRevision {
            external_id: "approved-2028".into(),
            name: "Approved configuration".into(),
            revision,
            previous: None,
        },
        content_sha256: content.sha256().unwrap(),
        content,
        approvals: vec![],
        produced: Produced {
            at: "2026-10-14T10:00:00Z".into(),
            custodian: "Fixture custodian".into(),
            key_label: "fixture".into(),
            producer: Default::default(),
        },
        revocation_lists: vec![],
    };
    let approvers = json!([{"subject":"CN=Fixture approver", "issuer":"CN=Fixture CA", "serial":"01", "fingerprint_sha256":"11".repeat(32)}]);
    let hash = sequent_core::election_config::manifest::sha256_hex(&manifest.to_bytes().unwrap());
    tx.execute("INSERT INTO sequent_backend.configuration_package (tenant_id, election_event_id, external_id, revision, manifest_sha256, manifest, signer_subject, signer_serial, signer_fingerprint, approvers) VALUES ($1, $2, 'approved-2028', $3, $4, $5, 'CN=Fixture signer', '02', $6, $7)", &[&ids.tenant, &ids.event, &i64::try_from(revision).unwrap(), &hash, &serde_json::to_value(&manifest).unwrap(), &"22".repeat(32), &approvers]).await.unwrap();
}

/// Store a genuine native signed seal, its immutable event-store guard and the
/// Hasura work-list completion just as the successful sealer does.
async fn seal_tested_box(
    tx: &Transaction<'_>,
    ids: FixtureIds,
    db: &EventDatabase,
    key: &SigningKey,
) {
    use electoral_log::adapters::ballot_box_seal::{
        insert_seal, BallotBoxId, NewSeal, SealOutcome,
    };
    use windmill::postgres::ballot_box_seal::{
        insert_pending, list_for_elections, mark_sealed, ClosedBy, NewPendingSeal, SealLogRecord,
        SealedFields,
    };
    let closed = Utc::now() - Duration::hours(1);
    insert_pending(
        tx,
        &[NewPendingSeal {
            tenant_id: ids.tenant,
            election_event_id: ids.event,
            election_id: ids.election,
            area_id: ids.area,
            closed_at: closed,
            grace_deadline: closed,
            close_request_id: None,
            closed_by: ClosedBy::User {
                username: Some("ada".into()),
            },
        }],
    )
    .await
    .unwrap();
    let built = electoral_log::seal::build(electoral_log::seal::BallotBoxSealManifest {
        format: electoral_log::seal::SEAL_FORMAT_V1.into(),
        tenant_id: ids.tenant.to_string(),
        election_event_id: ids.event.to_string(),
        election_id: ids.election.to_string(),
        area_id: ids.area.to_string(),
        closed_at: closed.timestamp() as u64,
        grace_deadline: closed.timestamp() as u64,
        sealed_at: Utc::now().timestamp() as u64,
        close_request_id: None,
        eligible_voters: 2,
        entries: vec![],
    })
    .unwrap();
    let message = Message::ballot_box_sealed_message(
        &built.manifest,
        built.hash,
        "General election",
        "North",
        &SigningData::new(key.clone(), "ada", key.clone()),
    )
    .unwrap();
    let signed = message.record_serialize().unwrap();
    let entry = LogEntry {
        delivery_id: format!(
            "ballot-box-seal:{}:{}:{}",
            ids.event, ids.election, ids.area
        ),
        message: ElectoralLogMessage::try_from(&message).unwrap(),
    };
    let mut board_client = db.store.client().await.unwrap();
    let board_tx = board_client.transaction().await.unwrap();
    let stored = electoral_log::adapters::postgres::append_in(&board_tx, &db.board, &[entry])
        .await
        .unwrap();
    let outcome = SealOutcome::Sealed {
        seal_hash: built.hash.to_vec(),
        manifest: built.bytes.clone(),
        ballots_in_box: 0,
        ballots_counted: 0,
    };
    assert!(insert_seal(
        &board_tx,
        &NewSeal {
            ballot_box: BallotBoxId {
                election_event_id: &ids.event.to_string(),
                election_id: &ids.election.to_string(),
                area_id: &ids.area.to_string()
            },
            outcome: &outcome,
            signed_message: &signed,
            record: &stored[0]
        }
    )
    .await
    .unwrap());
    board_tx.commit().await.unwrap();
    let seals = list_for_elections(tx, &ids.tenant, &ids.event, &[ids.election])
        .await
        .unwrap();
    mark_sealed(
        tx,
        &seals[0].id,
        &SealedFields {
            sealed_at: Utc::now(),
            ballots_in_box: 0,
            ballots_counted: 0,
            seal_hash: hex::encode(built.hash),
            manifest: built.bytes,
            signed_message: signed,
            log_record: SealLogRecord::from(&stored[0]),
        },
    )
    .await
    .unwrap();
}

async fn assert_signed_report(
    db: &EventDatabase,
    key: &SigningKey,
    report: &windmill::domain::readiness::ReadinessReport,
) {
    let delivered = db
        .store
        .delivered(
            &db.board,
            &format!("readiness:{}", report.log_entry_id.as_ref().unwrap()),
        )
        .await
        .unwrap()
        .unwrap();
    let message = Message::record_deserialize(&delivered.message.message).unwrap();
    message
        .verify(&VerificationKey::from_sk(key).unwrap())
        .unwrap();
    let electoral_log::messages::statement::StatementBody::Audit(entry) = message.statement.body
    else {
        panic!("Expected readiness audit");
    };
    assert_eq!(entry.kind, AuditStatementKind::ReadinessVerified);
    let details: serde_json::Value = serde_json::from_str(&entry.details_json).unwrap();
    assert_eq!(details["report"], serde_json::to_value(report).unwrap());
    assert_eq!(details["actor"]["user_id"], "admin-id");
}

#[test]
fn real_snapshot_requires_delivered_valid_receipt_and_current_full_census_without_writing_a_pass() {
    run(async {
        std::env::set_var("MASTER_SECRET", "5a".repeat(32));
        let pool = schema::pool().await;
        let mut client = pool.get().await.unwrap();
        let database_name: String = client
            .query_one("SELECT current_database()", &[])
            .await
            .unwrap()
            .get(0);
        // This binary owns its entire migrated fixture database. The production
        // Keycloak pool reads committed census rows through another connection.
        std::env::set_var("KEYCLOAK_DB__DBNAME", database_name);
        client.batch_execute("CREATE TABLE realm (id text PRIMARY KEY, name text UNIQUE);
            CREATE TABLE user_entity (id text PRIMARY KEY, realm_id text, service_account_client_link text);
            CREATE TABLE user_attribute (user_id text, name text, value text);").await.unwrap();
        let tenant = Uuid::new_v4();
        let event = Uuid::new_v4();
        let election = Uuid::new_v4();
        let area = Uuid::new_v4();
        let task = Uuid::new_v4();
        let realm = get_event_realm(&tenant.to_string(), &event.to_string());
        client
            .execute("INSERT INTO realm VALUES ('realm', $1)", &[&realm])
            .await
            .unwrap();
        client
            .batch_execute(
                "INSERT INTO user_entity VALUES ('ada', 'realm', NULL), ('bob', 'realm', NULL)",
            )
            .await
            .unwrap();
        for voter in ["ada", "bob"] {
            client
                .execute(
                    "INSERT INTO user_attribute VALUES ($1, $2, $3)",
                    &[&voter, &AREA_ID_ATTR_NAME, &area.to_string()],
                )
                .await
                .unwrap();
        }
        let db = EventDatabase::create(tenant, event).await;
        let tx = client.transaction().await.unwrap();
        tx.execute(
            "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
            &[&tenant, &format!("snapshot-{tenant}")],
        )
        .await
        .unwrap();
        tx.execute("INSERT INTO sequent_backend.election_event (id, tenant_id, encryption_protocol, status, bulletin_board_reference) VALUES ($1, $2, 'RSA256', '{}'::jsonb, $3)", &[&event, &tenant, &json!({"id": 1, "database_name": db.board, "is_archived": false})]).await.unwrap();
        tx.execute("INSERT INTO sequent_backend.election (id, tenant_id, election_event_id, status) VALUES ($1, $2, $3, '{}'::jsonb)", &[&election, &tenant, &event]).await.unwrap();
        tx.execute("INSERT INTO sequent_backend.area (id, tenant_id, election_event_id, name) VALUES ($1, $2, $3, 'North')", &[&area, &tenant, &event]).await.unwrap();
        tx.execute("INSERT INTO sequent_backend.tasks_execution (id, tenant_id, election_event_id, name, type, execution_status, executed_by_user) VALUES ($1, $2, $3, 'Import voters', 'IMPORT_USERS', 'IN_PROGRESS', 'ada')", &[&task, &tenant, &event]).await.unwrap();
        let tenant_id = tenant.to_string();
        let event_id = event.to_string();
        create_protocol_manager_keys(&tx, &tenant_id, &event_id, &db.board)
            .await
            .unwrap();
        let key = get_board_signing_key(&tx, &tenant_id, Some(&event_id), &db.board)
            .await
            .unwrap();
        let actor = AuditActor {
            user_id: Some("admin-id".into()),
            username: Some("ada".into()),
            role: None,
        };
        let receipt = record_voter_import(
            &tx,
            &tenant_id,
            &event_id,
            &task.to_string(),
            Some(&actor),
            VotersImportedEvidence {
                import_id: String::new(),
                document_id: "document".into(),
                file_name: "certified.csv".into(),
                sha256: "ab".repeat(32),
                declared_sha256: Some("ab".repeat(32)),
                counts_by_post: [("North".into(), 2)].into(),
                imported_at: "2026-10-14T10:00:00Z".into(),
            },
        )
        .await
        .unwrap();
        complete_import_task(&tx, &tenant_id, &task.to_string())
            .await
            .unwrap();
        assert!(
            !snapshot(&tx, &tenant_id, &event_id)
                .await
                .unwrap()
                .voter_log_delivered
        );
        db.store
            .append(
                &db.board,
                &mut [signed_receipt(&event_id, &receipt, &key)]
                    .into_iter()
                    .map(Ok),
            )
            .await
            .unwrap();
        let state = snapshot(&tx, &tenant_id, &event_id).await.unwrap();
        assert!(state.voter_log_delivered);
        assert_eq!(state.voters, Some(receipt.clone()));
        assert_eq!(state.ballot_count, 0);
        assert_eq!(state.elections.len(), 1);
        assert!(state.configuration.is_none());
        assert!(state.publication.is_none());
        let before = db
            .store
            .count(&db.board, &LogQuery::default())
            .await
            .unwrap();
        let report = verify(&tx, &tenant_id, &event_id, &event_id, &actor)
            .await
            .unwrap();
        assert!(!report.passed);
        assert!(report
            .checks
            .iter()
            .any(|check| check.kind == CheckKind::Configuration && !check.passed));
        assert!(report
            .checks
            .iter()
            .any(|check| check.kind == CheckKind::Publication && !check.passed));
        assert_eq!(
            db.store
                .count(&db.board, &LogQuery::default())
                .await
                .unwrap(),
            before
        );
        // A later additive census change invalidates the formerly matching
        // signed file even though no new receipt has reached the log.
        let keycloak = windmill::services::database::get_keycloak_pool().await;
        let census = keycloak.get().await.unwrap();
        census
            .batch_execute("INSERT INTO user_entity VALUES ('delta', 'realm', NULL)")
            .await
            .unwrap();
        census
            .execute(
                "INSERT INTO user_attribute VALUES ('delta', $1, $2)",
                &[&AREA_ID_ATTR_NAME, &area.to_string()],
            )
            .await
            .unwrap();
        let changed = snapshot(&tx, &tenant_id, &event_id).await.unwrap();
        assert!(changed.voter_log_delivered);
        assert_eq!(changed.current_voter_counts, [("North".into(), 3)].into());
        let stale_report = verify(&tx, &tenant_id, &event_id, &event_id, &actor)
            .await
            .unwrap();
        assert!(stale_report
            .checks
            .iter()
            .any(|check| check.kind == CheckKind::CertifiedVoters && !check.passed));
        assert_eq!(
            db.store
                .count(&db.board, &LogQuery::default())
                .await
                .unwrap(),
            before
        );
        census.batch_execute("DELETE FROM user_attribute WHERE user_id = 'delta'; DELETE FROM user_entity WHERE id = 'delta'").await.unwrap();
        // Upgrade the same real official snapshot into a complete approved
        // pair, using distinct runtime IDs and different package revisions.
        let tested_event = Uuid::new_v4();
        let tested_election = Uuid::new_v4();
        let tested_area = Uuid::new_v4();
        let tested_task = Uuid::new_v4();
        let tested_db = EventDatabase::create(tenant, tested_event).await;
        let closed_status = json!({"voting_status":"CLOSED", "voting_period_dates":{
            "first_started_at":"2026-10-13T09:00:00Z", "last_stopped_at":"2026-10-13T10:00:00Z"
        }});
        tx.execute("INSERT INTO sequent_backend.election_event (id, tenant_id, encryption_protocol, status, bulletin_board_reference, presentation) VALUES ($1, $2, 'RSA256', $3, $4, $5)", &[&tested_event, &tenant, &json!({}), &json!({"id":1,"database_name":tested_db.board,"is_archived":false}), &json!({"ballot_box_seal_policy":"seal-at-close", "file_integrity_policy":"REQUIRED_SHA256"})]).await.unwrap();
        tx.execute("INSERT INTO sequent_backend.election (id, tenant_id, election_event_id, status) VALUES ($1, $2, $3, $4)", &[&tested_election, &tenant, &tested_event, &json!({})]).await.unwrap();
        tx.execute("INSERT INTO sequent_backend.area (id, tenant_id, election_event_id, name) VALUES ($1, $2, $3, 'North')", &[&tested_area, &tenant, &tested_event]).await.unwrap();
        tx.execute("INSERT INTO sequent_backend.tasks_execution (id, tenant_id, election_event_id, name, type, execution_status, executed_by_user) VALUES ($1, $2, $3, 'Import voters', 'IMPORT_USERS', 'IN_PROGRESS', 'ada')", &[&tested_task, &tenant, &tested_event]).await.unwrap();
        let tested_realm = get_event_realm(&tenant_id, &tested_event.to_string());
        census
            .execute(
                "INSERT INTO realm VALUES ($1, $2)",
                &[&tested_event.to_string(), &tested_realm],
            )
            .await
            .unwrap();
        for name in ["tested-ada", "tested-bob"] {
            census
                .execute(
                    "INSERT INTO user_entity VALUES ($1, $2, NULL)",
                    &[&name, &tested_event.to_string()],
                )
                .await
                .unwrap();
            census
                .execute(
                    "INSERT INTO user_attribute VALUES ($1, $2, $3)",
                    &[&name, &AREA_ID_ATTR_NAME, &tested_area.to_string()],
                )
                .await
                .unwrap();
        }
        create_protocol_manager_keys(&tx, &tenant_id, &tested_event.to_string(), &tested_db.board)
            .await
            .unwrap();
        let tested_key = get_board_signing_key(
            &tx,
            &tenant_id,
            Some(&tested_event.to_string()),
            &tested_db.board,
        )
        .await
        .unwrap();
        let tested_receipt = record_voter_import(
            &tx,
            &tenant_id,
            &tested_event.to_string(),
            &tested_task.to_string(),
            Some(&actor),
            receipt.clone(),
        )
        .await
        .unwrap();
        complete_import_task(&tx, &tenant_id, &tested_task.to_string())
            .await
            .unwrap();
        tested_db
            .store
            .append(
                &tested_db.board,
                &mut [signed_receipt(
                    &tested_event.to_string(),
                    &tested_receipt,
                    &tested_key,
                )]
                .into_iter()
                .map(Ok),
            )
            .await
            .unwrap();
        let official_ids = FixtureIds {
            tenant,
            event,
            election,
            area,
        };
        let tested_ids = FixtureIds {
            tenant,
            event: tested_event,
            election: tested_election,
            area: tested_area,
        };
        let official_content = published_content(&tx, official_ids).await;
        let tested_content = published_content(&tx, tested_ids).await;
        assert_eq!(
            official_content, tested_content,
            "Equivalent generated designs must survive ID renumbering"
        );
        imported_manifest(&tx, official_ids, 8, official_content).await;
        imported_manifest(&tx, tested_ids, 9, tested_content).await;
        tx.execute(
            "UPDATE sequent_backend.election_event SET status = $2 WHERE id = $1",
            &[&tested_event, &closed_status],
        )
        .await
        .unwrap();
        tx.execute(
            "UPDATE sequent_backend.election SET status = $2 WHERE id = $1",
            &[&tested_election, &closed_status],
        )
        .await
        .unwrap();
        seal_tested_box(&tx, tested_ids, &tested_db, &tested_key).await;
        windmill::services::readiness::lock_events(
            &tx,
            &tenant_id,
            &event_id,
            &tested_event.to_string(),
        )
        .await
        .unwrap();
        let passed = verify(
            &tx,
            &tenant_id,
            &event_id,
            &tested_event.to_string(),
            &actor,
        )
        .await
        .unwrap();
        assert!(passed.passed, "{:#?}", passed.checks);
        assert_eq!(
            passed.evidence.official.current_voter_counts,
            [("North".into(), 2)].into()
        );
        assert_eq!(passed.evidence.tested.expected_boxes.len(), 1);
        assert_eq!(
            passed.evidence.tested.expected_boxes,
            passed.evidence.tested.sealed_boxes
        );
        assert_signed_report(&db, &key, &passed).await;
        tx.batch_execute("SAVEPOINT pending_import").await.unwrap();
        tx.execute("INSERT INTO sequent_backend.tasks_execution (id, tenant_id, election_event_id, name, type, execution_status, executed_by_user) VALUES ($1, $2, $3, 'Pending import', 'IMPORT_USERS', 'IN_PROGRESS', 'ada')", &[&Uuid::new_v4(), &tenant, &event]).await.unwrap();
        let entries_before_pending = db
            .store
            .count(&db.board, &LogQuery::default())
            .await
            .unwrap();
        let pending = verify(
            &tx,
            &tenant_id,
            &event_id,
            &tested_event.to_string(),
            &actor,
        )
        .await
        .unwrap();
        assert!(!pending.passed);
        assert!(pending
            .checks
            .iter()
            .any(|check| check.kind == CheckKind::CertifiedVoters && !check.passed));
        assert_eq!(
            db.store
                .count(&db.board, &LogQuery::default())
                .await
                .unwrap(),
            entries_before_pending
        );
        tx.batch_execute("ROLLBACK TO SAVEPOINT pending_import")
            .await
            .unwrap();
        tx.execute("UPDATE sequent_backend.election_event SET presentation = jsonb_set(presentation, '{locked_down}', $2) WHERE id = $1", &[&event, &json!("locked-down")]).await.unwrap();
        let lockdown = verify(
            &tx,
            &tenant_id,
            &event_id,
            &tested_event.to_string(),
            &actor,
        )
        .await
        .unwrap();
        assert!(lockdown.passed, "{:#?}", lockdown.checks);
        assert!(lockdown.evidence.official.locked_down);
        assert_ne!(lockdown.log_entry_id, passed.log_entry_id);
        assert_signed_report(&db, &key, &lockdown).await;
        // Stored audit bytes alone are insufficient: an entry signed by a
        // different system identity must never become certified evidence.
        let newer = record_voter_import(
            &tx,
            &tenant_id,
            &event_id,
            &task.to_string(),
            Some(&actor),
            receipt,
        )
        .await
        .unwrap();
        db.store
            .append(
                &db.board,
                &mut [signed_receipt(
                    &event_id,
                    &newer,
                    &SigningKey::generate().unwrap(),
                )]
                .into_iter()
                .map(Ok),
            )
            .await
            .unwrap();
        let error = snapshot(&tx, &tenant_id, &event_id).await.unwrap_err();
        assert!(
            error
                .to_string()
                .contains("invalid electoral-log signature"),
            "{error:#}"
        );
        tx.rollback().await.unwrap();
        tested_db.drop().await;
        db.drop().await;
    });
}
