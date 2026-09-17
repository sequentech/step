// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
use immu_board::{util::get_event_board, ElectoralLogMessage};
use immudb_rs::{sql_value::Value, Row, SqlValue};

fn row() -> Row {
    Row {
        columns: [
            "id",
            "created",
            "sender_pk",
            "statement_timestamp",
            "statement_kind",
            "message",
            "version",
        ]
        .map(|column| format!("(electoral_log_messages.{column})"))
        .to_vec(),
        values: [
            Value::N(17),
            Value::Ts(100),
            Value::S("key".into()),
            Value::Ts(99),
            Value::S("Ballot".into()),
            Value::Bs(vec![0, 255, 16]),
            Value::S("1".into()),
        ]
        .map(|value| SqlValue { value: Some(value) })
        .to_vec(),
    }
}
#[test]
fn complete_rows_map_distinct_fields_and_allow_optional_voter_identity() {
    let mut source = row();
    let mapped = ElectoralLogMessage::try_from(&source).unwrap();
    assert_eq!(
        mapped,
        ElectoralLogMessage {
            id: 17,
            created: 100,
            sender_pk: "key".into(),
            statement_timestamp: 99,
            statement_kind: "Ballot".into(),
            message: vec![0, 255, 16],
            version: "1".into(),
            user_id: None,
            username: None
        }
    );
    source.columns.extend([
        "(electoral_log_messages.user_id)".into(),
        "(electoral_log_messages.username)".into(),
    ]);
    source.values.extend([
        SqlValue {
            value: Some(Value::S("voter-17".into())),
        },
        SqlValue {
            value: Some(Value::S("Synthetic Voter".into())),
        },
    ]);
    let mapped = ElectoralLogMessage::try_from(&source).unwrap();
    assert_eq!(mapped.user_id.as_deref(), Some("voter-17"));
    assert_eq!(mapped.username.as_deref(), Some("Synthetic Voter"));
    source.values[7].value = None;
    source.values[8].value = None;
    let mapped = ElectoralLogMessage::try_from(&source).unwrap();
    assert!(mapped.user_id.is_none());
    assert!(mapped.username.is_none());
    source.columns.reverse();
    source.values.reverse();
    assert_eq!(ElectoralLogMessage::try_from(&source).unwrap(), mapped);
}
#[test]
fn wrong_value_types_are_rejected_with_a_valid_control() {
    assert!(ElectoralLogMessage::try_from(&row()).is_ok());
    for index in 0..7 {
        for replacement in [None, Some(Value::B(true))] {
            let mut source = row();
            source.values[index].value = replacement;
            assert!(ElectoralLogMessage::try_from(&source)
                .unwrap_err()
                .to_string()
                .contains("invalid column value"));
        }
    }
    for optional in ["user_id", "username"] {
        let mut source = row();
        source
            .columns
            .push(format!("(electoral_log_messages.{optional})"));
        source.values.push(SqlValue {
            value: Some(Value::N(1)),
        });
        assert!(ElectoralLogMessage::try_from(&source)
            .unwrap_err()
            .to_string()
            .contains("invalid column value"));
    }
}
#[test]
fn malformed_column_names_return_errors_without_panicking() {
    assert!(ElectoralLogMessage::try_from(&row()).is_ok());
    for column in [
        "",
        ".",
        "(",
        "id",
        "(t.)",
        "(t.unknown)",
        "t.id",
        "(t.id",
        "t.id)",
    ] {
        let mut source = row();
        source.columns[0] = column.into();
        assert!(
            ElectoralLogMessage::try_from(&source).is_err(),
            "{column:?}"
        );
    }
}
#[test]
fn truncated_missing_and_duplicate_columns_cannot_become_default_fields() {
    assert!(ElectoralLogMessage::try_from(&row()).is_ok());
    let mut missing_value = row();
    missing_value.values.pop();
    assert!(ElectoralLogMessage::try_from(&missing_value).is_err());
    let mut missing_column = row();
    missing_column.columns.pop();
    assert!(ElectoralLogMessage::try_from(&missing_column).is_err());
    for index in 0..7 {
        let mut missing = row();
        missing.columns.remove(index);
        missing.values.remove(index);
        assert!(
            ElectoralLogMessage::try_from(&missing).is_err(),
            "missing index {index}"
        );
        let mut duplicate = row();
        duplicate.columns.push(duplicate.columns[index].clone());
        duplicate.values.push(duplicate.values[index].clone());
        assert!(
            ElectoralLogMessage::try_from(&duplicate).is_err(),
            "duplicate index {index}"
        );
    }
}
#[test]
fn board_names_namespace_tenant_and_event_after_removing_uuid_hyphens() {
    assert_eq!(
        get_event_board(
            "01234567-89ab-cdef-0123-456789abcdef",
            "fedcba98-7654-3210-fedc-ba9876543210"
        ),
        "tenant0123456789abcdef0123456789abcdefeventfedcba9876543210fedcba9876543210"
    );
    assert_ne!(
        get_event_board("tenant-a", "event-b"),
        get_event_board("event-b", "tenant-a")
    );
    assert_eq!(get_event_board("A", "B"), "tenantAeventB");
}
