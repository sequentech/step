// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use electoral_log::client::{
    audit_types::ElectoralLogMessage, wbraid_store::WbraidAuditStore as BoardClient,
};

#[tokio::test]
async fn board_delivery_is_atomic_idempotent_and_rebuilds_its_query_index() {
    let url = std::env::var("WBRAID_AUDIT_TEST_DATABASE_URL")
        .expect("set WBRAID_AUDIT_TEST_DATABASE_URL to an isolated disposable PostgreSQL database");
    let mut client = BoardClient::new(&url).await.unwrap();
    let board = format!("audit-test-{}", std::process::id());
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
    assert_eq!(
        client
            .count_electoral_log_messages(&board, None)
            .await
            .unwrap(),
        2
    );
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
    assert_eq!(
        reopened
            .count_electoral_log_messages(&board, None)
            .await
            .unwrap(),
        2
    );
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
    assert_eq!(
        client
            .count_electoral_log_messages(&board, None)
            .await
            .unwrap(),
        4
    );
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
    assert_eq!(
        client
            .count_electoral_log_messages(&board, Some(&filter))
            .await
            .unwrap(),
        2
    );
    filter.area_ids.push("area-b".into());
    assert_eq!(
        client
            .count_electoral_log_messages(&board, Some(&filter))
            .await
            .unwrap(),
        3
    );
    filter.election_id = Some("foreign".into());
    filter.area_ids.clear();
    assert_eq!(
        client
            .count_electoral_log_messages(&board, Some(&filter))
            .await
            .unwrap(),
        1
    );
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
    assert_eq!(
        client
            .count_electoral_log_messages(&board, Some(&filter))
            .await
            .unwrap(),
        2
    );
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
    client.archive_board(&board).await.unwrap();
    let native = wbraid_b4::db::BoardDb::open_url(&url).await.unwrap();
    assert!(native
        .insert_message(&board, Some(b"bypass"), None, "1")
        .await
        .is_err());
    assert_eq!(
        client
            .count_electoral_log_messages(&board, None)
            .await
            .unwrap(),
        7
    );
    assert!(client
        .append_delivery(&board, &"f".repeat(64), &payload, &[row])
        .await
        .is_err());
}
