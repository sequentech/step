// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use electoral_log::client::{
    audit_types::ElectoralLogMessage, wbraid_store::WbraidAuditStore as BoardClient,
};

#[tokio::test]
#[serial_test::serial]
async fn board_delivery_is_atomic_idempotent_and_rebuilds_its_query_index() {
    let url = std::env::var("WBRAID_AUDIT_TEST_DATABASE_URL")
        .expect("set WBRAID_AUDIT_TEST_DATABASE_URL to an isolated disposable PostgreSQL database");
    // The independent audit database must ignore protocol-server overrides.
    std::env::set_var(
        "WBRAID_B4_PG_DATABASE",
        "protocol-database-must-not-be-used",
    );
    let mut client = BoardClient::new(&url).await.unwrap();
    let board = format!("audit-test-{}", uuid::Uuid::new_v4().simple());
    client.upsert_electoral_log_db(&board).await.unwrap();
    client.upsert_electoral_log_db(&board).await.unwrap();
    let row = ElectoralLogMessage {
        id: 0,
        created: 1,
        sender_pk: "key".into(),
        statement_timestamp: 1,
        statement_kind: "fixture".into(),
        message: vec![1, 2, 3],
        version: "1".into(),
        user_id: None,
        username: None,
        election_id: None,
        area_id: None,
        ballot_id: None,
    };
    let delivery = "a".repeat(64);
    let payload = "b".repeat(64);
    assert!(client
        .append_delivery(&board, &delivery, &payload, &[row.clone(), row.clone()])
        .await
        .unwrap());
    assert!(!client
        .append_delivery(&board, &delivery, &payload, &[row.clone()])
        .await
        .unwrap());
    assert!(client
        .append_delivery(&board, &delivery, &"c".repeat(64), &[row.clone()])
        .await
        .is_err());
    assert_eq!(client.count_query(&board, None).await.unwrap(), 2);
    let rows = client
        .get_electoral_log_messages_batch(&board, 1, 0)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].message, row.message);
    assert_eq!(
        client
            .get_electoral_log_messages_batch(&board, 5, rows[0].id)
            .await
            .unwrap()
            .len(),
        1
    );
    // Reopen through the supplied board store to prove durable, independent reads.
    let mut reopened = BoardClient::new(&url).await.unwrap();
    assert_eq!(reopened.count_query(&board, None).await.unwrap(), 2);
    // An index failure occurs after the supplied board store committed the
    // whole batch. Recovery must index it, then report an existing delivery.
    let pool = sqlx::PgPool::connect(&url).await.unwrap();
    sqlx::raw_sql("CREATE FUNCTION sequent_audit.reject_index() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected index failure'; END $$; CREATE TRIGGER reject_index BEFORE INSERT ON sequent_audit.records FOR EACH ROW EXECUTE FUNCTION sequent_audit.reject_index();")
        .execute(&pool).await.unwrap();
    let second_id = "d".repeat(64);
    assert!(client
        .append_delivery(&board, &second_id, &payload, &[row.clone(), row.clone()])
        .await
        .is_err());
    sqlx::raw_sql("DROP TRIGGER reject_index ON sequent_audit.records; DROP FUNCTION sequent_audit.reject_index();")
        .execute(&pool).await.unwrap();
    assert!(!client
        .append_delivery(&board, &second_id, &payload, &[row.clone()])
        .await
        .unwrap());
    assert_eq!(client.count_query(&board, None).await.unwrap(), 4);
    use electoral_log::client::{
        audit_query::*,
        audit_types::{ElectoralLogVarCharColumn as Column, SqlCompOperators as Op},
    };
    let scoped: Vec<_> = [Some("election-a"), Some("election-b"), None]
        .into_iter()
        .enumerate()
        .map(|(index, election)| {
            let mut entry = row.clone();
            entry.election_id = election.map(str::to_owned);
            entry.area_id = (index == 1).then(|| "area-b".into());
            entry.user_id = Some(format!("user-{index}"));
            entry.statement_timestamp = index as i64 + 3;
            entry
        })
        .collect();
    client
        .append_delivery(&board, &"e".repeat(64), &payload, &scoped)
        .await
        .unwrap();
    let mut filter = AuditQuery {
        election_id: Some("election-a".into()),
        only_with_user: true,
        ..Default::default()
    };
    assert_eq!(client.count_query(&board, Some(&filter)).await.unwrap(), 2);
    filter.area_ids.push("area-b".into());
    assert_eq!(client.count_query(&board, Some(&filter)).await.unwrap(), 3);
    filter.election_id = Some("foreign".into());
    filter.area_ids.clear();
    assert_eq!(client.count_query(&board, Some(&filter)).await.unwrap(), 1);
    filter = AuditQuery {
        permitted_election_ids: Some(vec!["election-a".into()]),
        ..Default::default()
    };
    assert_eq!(client.count_query(&board, Some(&filter)).await.unwrap(), 6);
    assert!(client
        .query_messages(&board, &filter)
        .await
        .unwrap()
        .iter()
        .all(|row| row
            .election_id
            .as_deref()
            .is_none_or(|id| id == "election-a")));
    filter.permitted_election_ids = Some(vec![]);
    assert_eq!(client.count_query(&board, Some(&filter)).await.unwrap(), 5);
    filter.permitted_election_ids = Some(vec!["' OR TRUE --".into()]);
    assert_eq!(client.count_query(&board, Some(&filter)).await.unwrap(), 5);

    filter = AuditQuery {
        predicates: vec![AuditPredicate {
            column: AuditColumn::StatementTimestamp,
            operator: Op::GreaterThanOrEqual,
            value: AuditValue::Integer(4),
        }],
        order: vec![(AuditColumn::StatementTimestamp, true)],
        limit: Some(1),
        offset: Some(1),
        ..Default::default()
    };
    assert_eq!(client.count_query(&board, Some(&filter)).await.unwrap(), 2);
    assert_eq!(
        client.query_messages(&board, &filter).await.unwrap()[0].statement_timestamp,
        4
    );
    filter = AuditQuery {
        predicates: vec![AuditPredicate {
            column: AuditColumn::Text(Column::UserId),
            operator: Op::Equal,
            value: AuditValue::Text("' OR 1=1 --".into()),
        }],
        ..Default::default()
    };
    assert!(client
        .query_messages(&board, &filter)
        .await
        .unwrap()
        .is_empty());
    filter.limit = Some(-1);
    assert!(client.query_messages(&board, &filter).await.is_err());
    use electoral_log::client::database_audit::*;
    let tenant = uuid::Uuid::new_v4().to_string();
    let event = uuid::Uuid::new_v4().to_string();
    let audit_board =
        database_audit_board(&tenant, &event, DatabaseAuditTable::PgauditHasura).unwrap();
    client.upsert_electoral_log_db(&audit_board).await.unwrap();
    // A malformed database record must never poison the immutable board or
    // consume its delivery identity before the valid replay arrives.
    let mut invalid = row.clone();
    invalid.statement_kind = "pgaudit_hasura".into();
    let native = wbraid_b4::db::BoardDb::from_postgres_pool(pool.clone())
        .await
        .unwrap();
    assert!(client
        .append_delivery(&audit_board, &"1".repeat(64), &payload, &[invalid])
        .await
        .is_err());
    assert!(native.list_messages(&audit_board).await.unwrap().is_empty());
    let entry = DatabaseAuditRow {
        id: 0,
        audit_type: "SESSION".into(),
        class: "WRITE".into(),
        command: "INSERT".into(),
        dbname: "hasura".into(),
        server_timestamp: 123,
        session_id: "session".into(),
        statement: "INSERT INTO fixture VALUES ('quoted')".into(),
        user: "actor".into(),
    }
    .into_record(DatabaseAuditTable::PgauditHasura)
    .unwrap();
    client
        .append_delivery(&audit_board, &"1".repeat(64), &payload, &[entry])
        .await
        .unwrap();
    let database_query = AuditQuery {
        predicates: vec![AuditPredicate {
            column: AuditColumn::DatabaseText(DatabaseAuditColumn::Statement),
            operator: Op::Like,
            value: AuditValue::Text("%quoted%".into()),
        }],
        ..Default::default()
    };
    let found = client
        .query_messages(&audit_board, &database_query)
        .await
        .unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(
        serde_json::from_slice::<DatabaseAuditRow>(&found[0].message)
            .unwrap()
            .user,
        "actor"
    );
    assert_eq!(
        client
            .count_query(&audit_board, Some(&database_query))
            .await
            .unwrap(),
        1
    );
    assert_ne!(
        audit_board,
        database_audit_board(
            &uuid::Uuid::new_v4().to_string(),
            &event,
            DatabaseAuditTable::PgauditHasura
        )
        .unwrap()
    );
    // Readers recover an independently indexed database audit batch after restart.
    assert_eq!(
        BoardClient::new(&url)
            .await
            .unwrap()
            .query_messages(&audit_board, &database_query)
            .await
            .unwrap()
            .len(),
        1
    );
    client.archive_board(&board).await.unwrap();
    let native = wbraid_b4::db::BoardDb::from_postgres_pool(pool.clone())
        .await
        .unwrap();
    assert!(native
        .insert_message(&board, Some(b"bypass"), None, "1")
        .await
        .is_err());
    assert_eq!(client.count_query(&board, None).await.unwrap(), 7);
    assert!(client
        .append_delivery(&board, &"f".repeat(64), &payload, &[row])
        .await
        .is_err());
}

#[tokio::test]
#[serial_test::serial]
async fn database_audit_collector_acknowledges_replays_and_rejects_changed_payloads() {
    use electoral_log::client::database_audit::*;
    use std::{
        io::Write,
        process::{Command, Stdio},
    };
    let url = std::env::var("WBRAID_AUDIT_TEST_DATABASE_URL").expect("isolated audit database");
    let tenant = uuid::Uuid::new_v4().to_string();
    let event = uuid::Uuid::new_v4().to_string();
    let delivery = serde_json::json!({
        "tenant_id": tenant, "election_event_id": event,
        "table": "pgaudit_hasura", "delivery_id": "fixture.log:0..100",
        "rows": [{"audit_type":"SESSION", "class":"READ", "command":"SELECT",
                  "dbname":"backend", "server_timestamp":42, "session_id":"session-1",
                  "statement":"SELECT 1", "user":"actor"}]
    });
    let run = |lines: &str| {
        let mut child = Command::new(env!("CARGO_BIN_EXE_audit_ingest"))
            .env("WBRAID_AUDIT_DATABASE_URL", &url)
            .env_remove("WBRAID_AUDIT_PG_PASSWORD")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(lines.as_bytes())
            .unwrap();
        child.wait_with_output().unwrap()
    };
    let output = run(&format!("{delivery}\n{delivery}\n"));
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let acks = String::from_utf8(output.stdout).unwrap();
    let acks: Vec<_> = acks.lines().collect();
    assert_eq!(acks.len(), 2);
    assert_eq!(acks[0], acks[1]);
    assert_eq!(acks[0].len(), 64);
    let mut changed = delivery;
    changed["rows"][0]["statement"] = "SELECT 2".into();
    let conflict = run(&format!("{changed}\n"));
    assert!(!conflict.status.success());
    assert!(conflict.stdout.is_empty());
    let mut client = BoardClient::new(&url).await.unwrap();
    let board = database_audit_board(&tenant, &event, DatabaseAuditTable::PgauditHasura).unwrap();
    let records = client.get_electoral_log_messages(&board).await.unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(
        serde_json::from_slice::<DatabaseAuditRow>(&records[0].message)
            .unwrap()
            .statement,
        "SELECT 1"
    );
}
