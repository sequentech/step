// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The audit log writer against real databases: the outbox in the schema of
//! the Hasura migrations, and the tenants' and the election events' logs on
//! the server `ELECTORAL_LOG_PG_*` names. Entries are queued in transactions
//! that commit, as the changes they record do, each test with its own tenant.

#[path = "support/event_database.rs"]
mod event_database;
#[path = "support/schema.rs"]
mod schema;

use chrono::{Duration, Utc};
use deadpool_postgres::Client;
use electoral_log::messages::message::Message;
use electoral_log::messages::statement::{StatementBody, StatementEventType, StatementType};
use electoral_log::{ElectoralLogMessage, LogQuery, OrderColumn, SortDirection};
use event_database::{run, EventDatabase};
use serde_json::{json, Value};
use std::sync::Once;
use uuid::Uuid;
use voting_crypto::signing::{Signature, VerificationKey};
use voting_crypto::RecordDeserialize;
use windmill::postgres::audit_log_outbox::{
    audit_log_outbox_backlog, fetch_unposted_audit_log_outbox, AuditLogTarget,
};
use windmill::services::audit_log::{
    diff_values, stage, AdminAction, AdminChange, AuditActor, AuditObject, AuditOrigin,
};
use windmill::services::audit_log_export::{
    record_export, sha256_hex, write_export, AuditExportFormat, AuditExportRequest, AuditExportRow,
    AuditExportScope,
};
use windmill::services::audit_log_writer::{
    delivery_id, monitoring_line, post_target, prepare_target_log, retry_after, PassOutcome,
    Usernames,
};
use windmill::services::audit_log_writer::{record_log_writer, LogWriterEvent};
use windmill::services::delete_election_event::{
    deleted_event_log_state, retire_election_event_electoral_log, stage_election_event_deletion,
};
use windmill::services::electoral_log_board::get_event_databases;
use windmill::services::electoral_log_board::{
    create_protocol_manager_keys, ensure_tenant_log, get_board_client, get_tenant_board,
};

const SLUG: &str = "audittest";

/// The writer's start and stop are queued for every tenant, so the tests, each
/// with its own tenant, take turns.
static SERIAL: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
const ADMIN_ID: &str = "8f2b7a52-5a0c-4c0e-9d57-3f1f4a8f6f10";

/// The boards are named after the environment, and their keys are encrypted
/// with the deployment's master secret.
fn environment() {
    static SET: Once = Once::new();
    SET.call_once(|| {
        std::env::set_var("MASTER_SECRET", "5a".repeat(32));
        std::env::set_var("ENV_SLUG", SLUG);
    });
}

/// Names the admin of the tests, and nobody else.
struct Directory;

impl Usernames for Directory {
    async fn username(&self, _tenant_id: Uuid, user_id: &str) -> Option<String> {
        (user_id == ADMIN_ID).then(|| "ada".to_string())
    }
}

async fn client() -> Client {
    environment();
    schema::pool().await.get().await.unwrap()
}

async fn new_tenant(client: &mut Client) -> Uuid {
    let tenant = Uuid::new_v4();
    client
        .execute(
            "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
            &[&tenant, &format!("tenant-{tenant}")],
        )
        .await
        .unwrap();
    tenant
}

fn settings_change(target: AuditLogTarget, from: &str, to: &str) -> AdminChange {
    AdminChange::new(
        target,
        AuditObject::new("tenant", target.tenant_id, Some("acme")),
        AdminAction::Update,
        AuditActor {
            user_id: Some(ADMIN_ID.to_string()),
            username: None,
            role: Some("admin-user".to_string()),
        },
        AuditOrigin::Harvest,
    )
    .with_changes(diff_values(
        "settings",
        Some(&json!({"logs_timezone": from, "gapi_key": "old"})),
        Some(&json!({"logs_timezone": to, "gapi_key": "new"})),
    ))
}

async fn queue(client: &mut Client, change: &AdminChange) -> Uuid {
    let transaction = client.transaction().await.unwrap();
    let id = stage(&transaction, change).await.unwrap();
    transaction.commit().await.unwrap();
    id
}

async fn records(board: &str) -> Vec<ElectoralLogMessage> {
    let query = LogQuery {
        order: vec![(OrderColumn::Id, SortDirection::Asc)],
        ..LogQuery::default()
    };
    get_board_client()
        .await
        .unwrap()
        .query(board, &query)
        .await
        .unwrap()
}

fn audit_details(record: &ElectoralLogMessage) -> (Message, Value) {
    let message = Message::record_deserialize(&record.message).unwrap();
    let StatementBody::Audit(entry) = &message.statement.body else {
        panic!("not an audit entry: {}", record.statement_kind);
    };
    let details = serde_json::from_str(&entry.details_json).unwrap();
    (message, details)
}

#[test]
fn a_tenant_change_is_signed_and_posted_once_to_the_tenant_log() {
    run(async {
        let _serial = SERIAL.lock().await;
        let mut client = client().await;
        let tenant = new_tenant(&mut client).await;
        let target = AuditLogTarget {
            tenant_id: tenant,
            election_event_id: None,
        };
        let first = queue(&mut client, &settings_change(target, "UTC", "Asia/Manila")).await;
        let second = queue(&mut client, &settings_change(target, "Asia/Manila", "UTC")).await;

        let outcome = post_target(&mut client, target, &Directory, Utc::now())
            .await
            .unwrap();
        assert_eq!(outcome, PassOutcome::Posted(2));

        let board = get_tenant_board(&tenant.to_string(), SLUG);
        let stored = records(&board).await;
        // The admin's public key is announced before the entries it signs.
        let kinds: Vec<&str> = stored
            .iter()
            .map(|record| record.statement_kind.as_str())
            .collect();
        assert_eq!(kinds, ["AdminPublicKey", "AdminChange", "AdminChange"]);
        let (message, details) = audit_details(&stored[1]);
        assert_eq!(message.statement.head.event.0, tenant.to_string());
        assert!(matches!(
            message.statement.head.kind,
            StatementType::AdminChange
        ));
        assert_eq!(message.statement.head.event_type, StatementEventType::USER);
        assert_eq!(
            message.statement.head.description,
            "Updated tenant \"acme\": settings.gapi_key, settings.logs_timezone"
        );
        assert_eq!(stored[1].user_id.as_deref(), Some(ADMIN_ID));
        // The username the trigger could not know is resolved when posting.
        assert_eq!(stored[1].username.as_deref(), Some("ada"));
        assert_eq!(details["actor"]["username"], "ada");
        assert_eq!(message.sender.name, "ada");
        assert_eq!(
            details["changes"],
            json!([
                {"path": "settings.gapi_key", "secret": true},
                {"path": "settings.logs_timezone", "old": "UTC", "new": "Asia/Manila"},
            ])
        );
        let (later, _) = audit_details(&stored[2]);
        let StatementBody::Audit(entry) = &later.statement.body else {
            unreachable!()
        };
        assert_eq!(entry.entry_id, second.to_string());

        // Both signatures verify: the admin's key and the log's system key.
        let transaction = client.transaction().await.unwrap();
        let log = ensure_tenant_log(&transaction, &tenant.to_string())
            .await
            .unwrap();
        transaction.commit().await.unwrap();
        let system = VerificationKey::from_sk(&log.system_key).unwrap();
        message.verify(&system).unwrap();
        assert_ne!(
            message.sender.pk.to_b64_string().unwrap(),
            system.to_b64_string().unwrap()
        );

        // Nothing waits, and another pass posts nothing more.
        let transaction = client.transaction().await.unwrap();
        assert!(fetch_unposted_audit_log_outbox(&transaction, target, 10)
            .await
            .unwrap()
            .is_empty());
        transaction.commit().await.unwrap();
        assert_eq!(
            post_target(&mut client, target, &Directory, Utc::now())
                .await
                .unwrap(),
            PassOutcome::Posted(0)
        );
        assert_eq!(records(&board).await.len(), 3);

        // An entry posted but not marked, as after a crash, is not posted again.
        client
            .execute(
                "UPDATE sequent_backend.audit_log_outbox SET posted_at = NULL WHERE id = $1",
                &[&first],
            )
            .await
            .unwrap();
        assert_eq!(
            post_target(&mut client, target, &Directory, Utc::now())
                .await
                .unwrap(),
            PassOutcome::Posted(1)
        );
        assert_eq!(records(&board).await.len(), 3);

        // What monitoring collects names the changed fields, never their values.
        let line = monitoring_line(&board, Some(stored[1].id), &stored[1]);
        assert_eq!(line["target"], "audit_log");
        assert_eq!(line["log"], board);
        assert_eq!(line["kind"], "AdminChange");
        assert_eq!(line["event_type"], "USER");
        assert_eq!(line["username"], "ada");
        assert_eq!(line["action"], "update");
        assert_eq!(line["object"]["type"], "tenant");
        assert_eq!(
            line["changed_fields"],
            json!(["settings.gapi_key", "settings.logs_timezone"])
        );
        assert_eq!(line["entry_sha256"].as_str().unwrap().len(), 64);
        let text = line.to_string();
        assert!(
            !text.contains("Asia/Manila") && !text.contains("\"old\""),
            "{text}"
        );
    });
}

#[test]
fn an_event_change_goes_to_the_events_log_and_a_deleted_events_to_the_tenants() {
    run(async {
        let _serial = SERIAL.lock().await;
        let mut client = client().await;
        let tenant = new_tenant(&mut client).await;
        let event = Uuid::new_v4();
        let database = EventDatabase::create(tenant, event).await;
        let transaction = client.transaction().await.unwrap();
        transaction
            .execute(
                "INSERT INTO sequent_backend.election_event
                     (id, tenant_id, encryption_protocol, bulletin_board_reference)
                 VALUES ($1, $2, 'RSA256', $3)",
                &[
                    &event,
                    &tenant,
                    &json!({"id": 1, "database_name": database.board, "is_archived": false}),
                ],
            )
            .await
            .unwrap();
        create_protocol_manager_keys(
            &transaction,
            &tenant.to_string(),
            &event.to_string(),
            &database.board,
        )
        .await
        .unwrap();
        transaction.commit().await.unwrap();

        let target = AuditLogTarget {
            tenant_id: tenant,
            election_event_id: Some(event),
        };
        let mut change = settings_change(target, "UTC", "Europe/Madrid");
        change.object = AuditObject::new("election", Uuid::nil(), Some("Senate"));
        queue(&mut client, &change).await;
        assert_eq!(
            post_target(&mut client, target, &Directory, Utc::now())
                .await
                .unwrap(),
            PassOutcome::Posted(1)
        );
        let stored = records(&database.board).await;
        assert_eq!(stored.len(), 1);
        let (message, _) = audit_details(&stored[0]);
        assert_eq!(message.statement.head.event.0, event.to_string());
        // The admin's public key is in the tenant's log, not the event's.
        let tenant_board = get_tenant_board(&tenant.to_string(), SLUG);
        let kinds: Vec<String> = records(&tenant_board)
            .await
            .into_iter()
            .map(|record| record.statement_kind)
            .collect();
        assert_eq!(kinds, ["AdminPublicKey"]);

        // Entries of an event that is gone are kept in the tenant's log.
        queue(&mut client, &change).await;
        for statement in [
            "DELETE FROM sequent_backend.secret WHERE election_event_id = $1",
            "DELETE FROM sequent_backend.election_event WHERE id = $1",
        ] {
            client.execute(statement, &[&event]).await.unwrap();
        }
        let log = prepare_target_log(&mut client, target, &Directory)
            .await
            .unwrap();
        assert_eq!(log.board, tenant_board);
        assert_eq!(log.rerouted_from, Some(event));
        assert_eq!(
            post_target(&mut client, target, &Directory, Utc::now())
                .await
                .unwrap(),
            PassOutcome::Posted(1)
        );
        let rerouted = records(&tenant_board).await;
        assert_eq!(rerouted.len(), 2);
        let (_, details) = audit_details(&rerouted[1]);
        assert_eq!(details["rerouted_from_election_event"], event.to_string());
        assert_eq!(records(&database.board).await.len(), 1);
        database.drop().await;
    });
}

#[test]
fn entries_wait_in_order_while_their_log_is_down() {
    run(async {
        let _serial = SERIAL.lock().await;
        let mut client = client().await;
        let tenant = new_tenant(&mut client).await;
        let event = Uuid::new_v4();
        // An event whose board has a key but no database: posting fails.
        let board = event_database::board_name(tenant, event);
        let transaction = client.transaction().await.unwrap();
        transaction
            .execute(
                "INSERT INTO sequent_backend.election_event
                     (id, tenant_id, encryption_protocol, bulletin_board_reference)
                 VALUES ($1, $2, 'RSA256', $3)",
                &[
                    &event,
                    &tenant,
                    &json!({"id": 1, "database_name": board, "is_archived": false}),
                ],
            )
            .await
            .unwrap();
        create_protocol_manager_keys(
            &transaction,
            &tenant.to_string(),
            &event.to_string(),
            &board,
        )
        .await
        .unwrap();
        transaction.commit().await.unwrap();
        let target = AuditLogTarget {
            tenant_id: tenant,
            election_event_id: Some(event),
        };
        let first = queue(&mut client, &settings_change(target, "UTC", "Asia/Manila")).await;
        let second = queue(&mut client, &settings_change(target, "Asia/Manila", "UTC")).await;

        let failed_at = Utc::now();
        let error = post_target(&mut client, target, &Directory, failed_at)
            .await
            .unwrap_err();
        assert!(
            format!("{error:#}").contains("no electoral-log database"),
            "{error:#}"
        );
        let transaction = client.transaction().await.unwrap();
        let waiting = fetch_unposted_audit_log_outbox(&transaction, target, 10)
            .await
            .unwrap();
        let ids: Vec<Uuid> = waiting.iter().map(|(id, _)| *id).collect();
        assert_eq!(ids, [first, second]);
        let row = waiting[0].1.as_ref().unwrap();
        assert_eq!(row.attempts, 1);
        let (count, oldest) = audit_log_outbox_backlog(&transaction).await.unwrap();
        assert!(count >= 2 && oldest >= 0.0);
        transaction.commit().await.unwrap();

        // Until the first entry may be tried again, the log is left alone.
        assert_eq!(
            post_target(&mut client, target, &Directory, failed_at)
                .await
                .unwrap(),
            PassOutcome::Backoff
        );
        assert_eq!(retry_after(1, failed_at), failed_at + Duration::seconds(2));
        assert_eq!(
            retry_after(40, failed_at),
            failed_at + Duration::seconds(600)
        );

        // Once the log is back, the entries are posted in the order they were queued.
        let database = EventDatabase::create(tenant, event).await;
        let outcome = post_target(
            &mut client,
            target,
            &Directory,
            Utc::now() + Duration::seconds(5),
        )
        .await
        .unwrap();
        assert_eq!(outcome, PassOutcome::Posted(2));
        let stored = records(&database.board).await;
        let deliveries: Vec<String> = stored
            .iter()
            .map(|record| {
                let (message, _) = audit_details(record);
                let StatementBody::Audit(entry) = message.statement.body else {
                    unreachable!()
                };
                entry.entry_id
            })
            .collect();
        assert_eq!(deliveries, [first.to_string(), second.to_string()]);
        assert_eq!(delivery_id(first), format!("audit:{first}"));
        database.drop().await;
    });
}

/// A second admin, whose entries an account export must leave out.
fn other_admins_change(target: AuditLogTarget) -> AdminChange {
    let mut change = settings_change(target, "UTC", "Europe/Madrid");
    change.actor = AuditActor {
        user_id: Some("3c0d9c0e-6a4b-4f0e-8d57-0a1b2c3d4e5f".to_string()),
        username: Some("bob".to_string()),
        role: None,
    };
    change
}

fn export_request(
    tenant: Uuid,
    scope: AuditExportScope,
    format: AuditExportFormat,
) -> AuditExportRequest {
    AuditExportRequest {
        tenant_id: tenant.to_string(),
        election_event_id: None,
        format,
        scope,
        username: None,
        election_id: None,
        area_id: None,
        created_from: None,
        created_to: None,
        time_zone: Some("Asia/Manila".to_string()),
        requested_by: AuditActor {
            user_id: Some(ADMIN_ID.to_string()),
            username: Some("ada".to_string()),
            role: None,
        },
    }
}

#[test]
fn exports_hold_what_the_list_shows_and_their_sql_verifies() {
    run(async {
        let _serial = SERIAL.lock().await;
        let mut client = client().await;
        let tenant = new_tenant(&mut client).await;
        let event = Uuid::new_v4();
        let database = EventDatabase::create(tenant, event).await;
        let transaction = client.transaction().await.unwrap();
        transaction
            .execute(
                "INSERT INTO sequent_backend.election_event
                     (id, tenant_id, encryption_protocol, bulletin_board_reference)
                 VALUES ($1, $2, 'RSA256', $3)",
                &[
                    &event,
                    &tenant,
                    &json!({"id": 1, "database_name": database.board, "is_archived": false}),
                ],
            )
            .await
            .unwrap();
        create_protocol_manager_keys(
            &transaction,
            &tenant.to_string(),
            &event.to_string(),
            &database.board,
        )
        .await
        .unwrap();
        transaction.commit().await.unwrap();
        let tenant_target = AuditLogTarget {
            tenant_id: tenant,
            election_event_id: None,
        };
        let event_target = AuditLogTarget {
            tenant_id: tenant,
            election_event_id: Some(event),
        };
        queue(
            &mut client,
            &settings_change(tenant_target, "UTC", "Asia/Manila"),
        )
        .await;
        queue(&mut client, &other_admins_change(tenant_target)).await;
        queue(
            &mut client,
            &settings_change(event_target, "UTC", "Asia/Manila"),
        )
        .await;
        for target in [tenant_target, event_target] {
            post_target(&mut client, target, &Directory, Utc::now())
                .await
                .unwrap();
        }
        let tenant_board = get_tenant_board(&tenant.to_string(), SLUG);

        // The whole tenant log, as SQL.
        let transaction = client.transaction().await.unwrap();
        let request = export_request(tenant, AuditExportScope::TenantLog, AuditExportFormat::Sql);
        let whole = write_export(&transaction, &request).await.unwrap();
        let listed = records(&tenant_board).await.len();
        assert_eq!(listed, 4);
        assert_eq!(whole.manifest.entries, 4);
        assert_eq!(whole.manifest.time_zone, "Asia/Manila");

        // One account, with its entries of every election event, merged by time.
        let mut account = export_request(
            tenant,
            AuditExportScope::AccountWithEvents,
            AuditExportFormat::Csv,
        );
        account.username = Some("ada".to_string());
        let merged = write_export(&transaction, &account).await.unwrap();
        assert_eq!(merged.manifest.entries, 3);
        let logs: Vec<(&str, u64)> = merged
            .manifest
            .logs
            .iter()
            .map(|log| (log.log.as_str(), log.entries))
            .collect();
        assert_eq!(
            logs,
            [("tenant", 2), (format!("event {event}").as_str(), 1)]
        );
        let csv = String::from_utf8(merged.contents).unwrap();
        let mut reader = csv::Reader::from_reader(csv.as_bytes());
        let rows: Vec<AuditExportRow> = reader.deserialize().map(Result::unwrap).collect();
        assert_eq!(rows.len(), 3);
        assert!(rows
            .iter()
            .all(|row| row.username.as_deref() == Some("ada")));
        assert!(rows
            .windows(2)
            .all(|pair| pair[0].created <= pair[1].created));
        let from_event = rows
            .iter()
            .filter(|row| row.log == format!("event {event}"));
        assert_eq!(from_event.count(), 1);
        assert!(rows[0].created_local.ends_with("+08:00"));

        // Without the events, and outside a range: fewer entries, as the list.
        account.scope = AuditExportScope::Account;
        assert_eq!(
            write_export(&transaction, &account)
                .await
                .unwrap()
                .manifest
                .entries,
            2
        );
        account.created_from = Some("2099-01-01T00:00:00Z".to_string());
        assert_eq!(
            write_export(&transaction, &account)
                .await
                .unwrap()
                .manifest
                .entries,
            0
        );
        let mut scoped = export_request(tenant, AuditExportScope::All, AuditExportFormat::Csv);
        scoped.election_event_id = Some(event.to_string());
        let of_event = write_export(&transaction, &scoped).await.unwrap();
        assert_eq!(of_event.manifest.entries, 1);
        assert_eq!(of_event.manifest.logs[0].log, format!("event {event}"));

        // The export is recorded in the log it exports, with the file's hash.
        let hash = sha256_hex(&whole.contents);
        record_export(&transaction, &request, &whole.manifest, &hash, "document")
            .await
            .unwrap();
        transaction.commit().await.unwrap();
        post_target(&mut client, tenant_target, &Directory, Utc::now())
            .await
            .unwrap();
        let stored = records(&tenant_board).await;
        let exported = stored.last().unwrap();
        assert_eq!(exported.statement_kind, "LogsExported");
        let (message, details) = audit_details(exported);
        assert_eq!(
            message.statement.head.description,
            "Exported the tenant log as SQL: 4 entries"
        );
        assert_eq!(details["sha256"], hash.as_str());
        assert_eq!(details["entries"], 4);
        assert_eq!(details["scope"], "TENANT_LOG");

        // The SQL export loads into PostgreSQL, and every entry's signatures verify
        // with the public keys it carries.
        let schema = format!("export_{}", Uuid::new_v4().simple());
        client
            .batch_execute(&format!(
                "CREATE SCHEMA {schema}; SET search_path TO {schema}"
            ))
            .await
            .unwrap();
        client
            .batch_execute(&String::from_utf8(whole.contents).unwrap())
            .await
            .unwrap();
        let manifest: Value = client
            .query_one("SELECT manifest FROM audit_log_export", &[])
            .await
            .unwrap()
            .get(0);
        assert_eq!(manifest["entries"], 4);
        assert_eq!(manifest["scope"], "TENANT_LOG");
        let system_key = VerificationKey::from_b64_string(
            manifest["logs"][0]["system_public_key"].as_str().unwrap(),
        )
        .unwrap();
        let loaded = client
            .query(
                "SELECT statement, sender_public_key, sender_signature, system_signature,
                        statement_kind, created_utc::text, username
                 FROM audit_log ORDER BY id",
                &[],
            )
            .await
            .unwrap();
        assert_eq!(loaded.len(), 4);
        for row in &loaded {
            let statement: Vec<u8> = row.get(0);
            let sender = VerificationKey::from_b64_string(row.get(1)).unwrap();
            let signature = |column: usize| {
                let bytes: Vec<u8> = row.get(column);
                Signature::from_bytes(bytes.try_into().unwrap()).unwrap()
            };
            sender.verify(&signature(2), &statement).unwrap();
            system_key.verify(&signature(3), &statement).unwrap();
        }
        // A changed statement no longer verifies.
        let mut tampered: Vec<u8> = loaded[1].get(0);
        tampered[0] ^= 1;
        let bytes: Vec<u8> = loaded[1].get(3);
        let signature = Signature::from_bytes(bytes.try_into().unwrap()).unwrap();
        assert!(system_key.verify(&signature, &tampered).is_err());
        client
            .batch_execute(&format!("RESET search_path; DROP SCHEMA {schema} CASCADE"))
            .await
            .unwrap();
        database.drop().await;
    });
}

#[test]
fn a_deleted_events_log_is_recorded_first_and_then_retired() {
    run(async {
        let _serial = SERIAL.lock().await;
        let mut client = client().await;
        let tenant = new_tenant(&mut client).await;
        let event = Uuid::new_v4();
        let database = EventDatabase::create(tenant, event).await;
        let transaction = client.transaction().await.unwrap();
        transaction
            .execute(
                "INSERT INTO sequent_backend.election_event
                     (id, tenant_id, encryption_protocol, bulletin_board_reference, presentation)
                 VALUES ($1, $2, 'RSA256', $3, $4)",
                &[
                    &event,
                    &tenant,
                    &json!({"id": 1, "database_name": database.board, "is_archived": false}),
                    &json!({"i18n": {"es": {"name": "Consulta"}, "en": {"name": "Referendum"}}}),
                ],
            )
            .await
            .unwrap();
        create_protocol_manager_keys(
            &transaction,
            &tenant.to_string(),
            &event.to_string(),
            &database.board,
        )
        .await
        .unwrap();
        transaction.commit().await.unwrap();
        let event_target = AuditLogTarget {
            tenant_id: tenant,
            election_event_id: Some(event),
        };
        queue(
            &mut client,
            &settings_change(event_target, "UTC", "Asia/Manila"),
        )
        .await;
        queue(
            &mut client,
            &settings_change(event_target, "Asia/Manila", "UTC"),
        )
        .await;
        post_target(&mut client, event_target, &Directory, Utc::now())
            .await
            .unwrap();
        let last = records(&database.board).await.last().unwrap().id;

        // The entry is queued in the deleting transaction, before anything is removed.
        let retired_at = 1_833_208_205;
        let deleted_by = AuditActor {
            user_id: Some(ADMIN_ID.to_string()),
            username: Some("ada".to_string()),
            role: None,
        };
        let transaction = client.transaction().await.unwrap();
        stage_election_event_deletion(
            &transaction,
            &tenant.to_string(),
            &event.to_string(),
            Some(&deleted_by),
            Some("task"),
            retired_at,
        )
        .await
        .unwrap();
        for statement in [
            "DELETE FROM sequent_backend.secret WHERE election_event_id = $1",
            "DELETE FROM sequent_backend.election_event WHERE id = $1",
        ] {
            transaction.execute(statement, &[&event]).await.unwrap();
        }
        transaction.commit().await.unwrap();
        retire_election_event_electoral_log(&tenant.to_string(), &event.to_string(), retired_at)
            .await
            .unwrap();

        let tenant_target = AuditLogTarget {
            tenant_id: tenant,
            election_event_id: None,
        };
        post_target(&mut client, tenant_target, &Directory, Utc::now())
            .await
            .unwrap();
        let tenant_board = get_tenant_board(&tenant.to_string(), SLUG);
        let stored = records(&tenant_board).await;
        let (message, details) = audit_details(stored.last().unwrap());
        assert_eq!(
            message.statement.head.description,
            "Deleted election event \"Referendum\""
        );
        assert_eq!(details["action"], "delete");
        assert_eq!(details["origin"], "windmill");
        assert_eq!(details["object"]["id"], event.to_string());
        let extra = &details["extra"];
        assert_eq!(extra["status"], "RETIRED");
        assert_eq!(extra["last_entry_id"], last);
        assert_eq!(extra["entries"], 2);
        assert_eq!(extra["state_hash"].as_str().unwrap().len(), 64);
        assert_eq!(extra["task_id"], "task");
        let retired = extra["log_database"].as_str().unwrap().to_string();
        assert!(retired.ends_with(&format!("_r{retired_at}_{}", event.simple())));

        // The log database is kept under that name, closed, and the event has none.
        let databases = get_event_databases().await.unwrap();
        assert!(!databases.has_event(&event.to_string()).await.unwrap());
        let kept = databases.retired_events(&tenant.to_string()).await.unwrap();
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].database_name, retired);

        // An event without a log is deleted all the same.
        let state = deleted_event_log_state(&tenant.to_string(), &event.to_string(), None, 1)
            .await
            .unwrap();
        assert_eq!(state, json!({"status": "ABSENT"}));
        let catalog = databases.catalog().client().await.unwrap();
        catalog
            .batch_execute(&format!("DROP DATABASE \"{retired}\" WITH (FORCE)"))
            .await
            .unwrap();
        catalog
            .execute(
                "DELETE FROM electoral_log_retired_events WHERE database_name = $1",
                &[&retired],
            )
            .await
            .unwrap();
    });
}

#[test]
fn the_writers_start_and_stop_are_entries_of_the_tenant_log() {
    run(async {
        let _serial = SERIAL.lock().await;
        let mut client = client().await;
        let tenant = new_tenant(&mut client).await;
        let target = AuditLogTarget {
            tenant_id: tenant,
            election_event_id: None,
        };
        let change = queue(&mut client, &settings_change(target, "UTC", "Asia/Manila")).await;
        post_target(&mut client, target, &Directory, Utc::now())
            .await
            .unwrap();
        for event in [LogWriterEvent::Stopped, LogWriterEvent::Started] {
            assert!(record_log_writer(&mut client, event).await.unwrap() >= 1);
        }
        post_target(&mut client, target, &Directory, Utc::now())
            .await
            .unwrap();
        let board = get_tenant_board(&tenant.to_string(), SLUG);
        let stored = records(&board).await;
        let kinds: Vec<&str> = stored
            .iter()
            .map(|record| record.statement_kind.as_str())
            .collect();
        assert_eq!(
            kinds,
            [
                "AdminPublicKey",
                "AdminChange",
                "LogWriterStopped",
                "LogWriterStarted"
            ]
        );
        let (message, details) = audit_details(&stored[2]);
        assert_eq!(
            message.statement.head.event_type,
            StatementEventType::SYSTEM
        );
        assert_eq!(
            message.statement.head.description,
            format!(
                "The log writer stopped (release {})",
                details["release"].as_str().unwrap()
            )
        );
        assert_eq!(details["last_entry_id"], change.to_string());
        assert_eq!(stored[2].user_id, None);
    });
}
