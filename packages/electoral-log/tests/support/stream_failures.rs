// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use super::{collect_messages, order_clause};
use immudb_rs::{sql_value::Value, Row, SqlQueryResult, SqlValue};
use std::collections::HashMap;

fn batch() -> SqlQueryResult {
    let fields = [
        ("id", Value::N(1)),
        ("created", Value::Ts(0)),
        ("statement_timestamp", Value::Ts(0)),
        ("sender_pk", Value::S("test".into())),
        ("statement_kind", Value::S("CastVote".into())),
        ("message", Value::Bs(vec![1, 2, 3])),
        ("version", Value::S("1".into())),
    ];
    SqlQueryResult {
        rows: vec![Row {
            columns: fields
                .iter()
                .map(|(name, _)| format!("(board.{name})"))
                .collect(),
            values: fields
                .into_iter()
                .map(|(_, value)| SqlValue { value: Some(value) })
                .collect(),
        }],
        ..Default::default()
    }
}

#[tokio::test]
async fn a_stream_failure_after_valid_rows_never_returns_a_partial_page() {
    let stream = tokio_stream::iter([
        Ok(batch()),
        Err(tonic::Status::unavailable(
            "synthetic disconnected database",
        )),
    ]);
    let error = collect_messages(stream).await.unwrap_err();
    assert!(format!("{error:#}").contains("synthetic disconnected database"));
}

#[tokio::test]
async fn a_malformed_row_after_valid_rows_never_returns_a_partial_page() {
    let mut malformed = batch();
    malformed.rows[0].values.clear();
    let error = collect_messages(tokio_stream::iter([Ok(batch()), Ok(malformed)]))
        .await
        .unwrap_err();
    assert!(format!("{error:#}").contains("unequal column and value counts"));
}

#[tokio::test]
async fn empty_batches_are_valid_and_do_not_end_the_stream() {
    let rows = collect_messages(tokio_stream::iter([
        Ok(SqlQueryResult::default()),
        Ok(batch()),
        Ok(SqlQueryResult::default()),
    ]))
    .await
    .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].message, vec![1, 2, 3]);
}

#[test]
fn ordering_is_deterministic_and_rejects_sql_fragments() {
    assert_eq!(
        order_clause::<String, String>(None).unwrap(),
        "ORDER BY id DESC"
    );
    assert_eq!(
        order_clause::<String, String>(Some(HashMap::new())).unwrap(),
        "ORDER BY id DESC"
    );
    assert_eq!(
        order_clause(Some(HashMap::from([("id", "desc"), ("created", "asc")]))).unwrap(),
        "ORDER BY created ASC, id DESC"
    );

    for (field, direction) in [
        ("id; SELECT 1", "ASC"),
        ("unknown", "ASC"),
        ("id", "ASC; SELECT 1"),
        ("id", ""),
    ] {
        assert!(order_clause(Some(HashMap::from([(field, direction)]))).is_err());
    }
}
