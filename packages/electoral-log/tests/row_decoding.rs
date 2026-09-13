// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! A malformed database response must fail, never turn into a plausible empty
//! audit record or panic while slicing a column label.

use electoral_log::{Aggregate, ElectoralLogMessage};
use immudb_rs::{sql_value::Value, Row, SqlValue};

const REQUIRED_COLUMNS: [&str; 7] = [
    "id",
    "created",
    "sender_pk",
    "statement_timestamp",
    "statement_kind",
    "message",
    "version",
];
const OPTIONAL_COLUMNS: [&str; 5] = ["user_id", "username", "election_id", "area_id", "ballot_id"];

fn row(entries: Vec<(&str, Option<Value>)>) -> Row {
    Row {
        columns: entries
            .iter()
            .map(|(name, _)| format!("(board.{name})"))
            .collect(),
        values: entries
            .into_iter()
            .map(|(_, value)| SqlValue { value })
            .collect(),
    }
}

fn complete_row() -> Row {
    row(vec![
        ("id", Some(Value::N(17))),
        ("created", Some(Value::Ts(1_700_000_001))),
        ("sender_pk", Some(Value::S("synthetic-public-key".into()))),
        ("statement_timestamp", Some(Value::Ts(1_700_000_000))),
        ("statement_kind", Some(Value::S("CastVote".into()))),
        ("message", Some(Value::Bs(vec![0, 127, 255]))),
        ("version", Some(Value::S("2".into()))),
    ])
}

#[test]
fn decodes_required_fields_without_inventing_optional_metadata() {
    let decoded = ElectoralLogMessage::try_from(&complete_row()).unwrap();
    assert_eq!(
        decoded,
        ElectoralLogMessage {
            id: 17,
            created: 1_700_000_001,
            sender_pk: "synthetic-public-key".into(),
            statement_timestamp: 1_700_000_000,
            statement_kind: "CastVote".into(),
            message: vec![0, 127, 255],
            version: "2".into(),
            user_id: None,
            username: None,
            election_id: None,
            area_id: None,
            ballot_id: None,
        }
    );
}

#[test]
fn optional_metadata_accepts_strings_and_both_database_null_representations() {
    for value in [
        Some(Value::S("reader-é".into())),
        Some(Value::Null(0)),
        None,
    ] {
        let mut input = complete_row();
        for name in OPTIONAL_COLUMNS {
            input.columns.push(format!("(board.{name})"));
            input.values.push(SqlValue {
                value: value.clone(),
            });
        }
        let decoded = ElectoralLogMessage::try_from(&input).unwrap();
        let expected = value.as_ref().and_then(|value| match value {
            Value::S(text) => Some(text.clone()),
            _ => None,
        });
        for actual in [
            decoded.user_id,
            decoded.username,
            decoded.election_id,
            decoded.area_id,
            decoded.ballot_id,
        ] {
            assert_eq!(actual, expected);
        }
    }
}

#[test]
fn every_required_column_must_be_present() {
    for (index, name) in REQUIRED_COLUMNS.iter().enumerate() {
        let mut input = complete_row();
        input.columns.remove(index);
        input.values.remove(index);
        assert!(
            ElectoralLogMessage::try_from(&input).is_err(),
            "accepted missing {name}"
        );
    }
}

#[test]
fn unequal_columns_and_values_cannot_be_silently_truncated_by_zip() {
    let mut fewer_values = complete_row();
    fewer_values.values.pop();
    assert!(ElectoralLogMessage::try_from(&fewer_values).is_err());

    let mut extra_values = complete_row();
    extra_values.values.push(SqlValue {
        value: Some(Value::N(99)),
    });
    assert!(ElectoralLogMessage::try_from(&extra_values).is_err());
}

#[test]
fn duplicate_columns_cannot_overwrite_an_earlier_audit_identity() {
    let mut input = complete_row();
    input.columns.push("(board.id)".into());
    input.values.push(SqlValue {
        value: Some(Value::N(999)),
    });
    assert!(ElectoralLogMessage::try_from(&input).is_err());
}

#[test]
fn malformed_column_labels_return_errors_including_short_and_unicode_inputs() {
    for label in [
        "",
        ".",
        "id",
        "(board.id",
        "board.id)",
        "(board.)",
        "(.id)",
        ".é",
        "(board.未知)",
    ] {
        let mut input = complete_row();
        input.columns[0] = label.into();
        // An unwind here would crash the audit reader, so call the parser
        // directly: the test should fail loudly on a panic.
        assert!(
            ElectoralLogMessage::try_from(&input).is_err(),
            "accepted {label:?}"
        );
    }
}

#[test]
fn required_values_reject_null_missing_and_incompatible_types() {
    for index in 0..REQUIRED_COLUMNS.len() {
        for value in [None, Some(Value::Null(0)), Some(Value::B(true))] {
            let mut input = complete_row();
            input.values[index] = SqlValue { value };
            assert!(ElectoralLogMessage::try_from(&input).is_err());
        }
    }
}

#[test]
fn optional_values_reject_incompatible_types() {
    for name in OPTIONAL_COLUMNS {
        let mut input = complete_row();
        input.columns.push(format!("(board.{name})"));
        input.values.push(SqlValue {
            value: Some(Value::N(12)),
        });
        assert!(
            ElectoralLogMessage::try_from(&input).is_err(),
            "accepted numeric {name}"
        );
    }
}

#[test]
fn column_order_does_not_change_the_decoded_record() {
    let input = complete_row();
    let expected = ElectoralLogMessage::try_from(&input).unwrap();
    let mut reversed = input;
    reversed.columns.reverse();
    reversed.values.reverse();
    assert_eq!(ElectoralLogMessage::try_from(&reversed).unwrap(), expected);
}

#[test]
fn aggregate_requires_exactly_one_nonnegative_integer() {
    assert_eq!(
        Aggregate::try_from(&row(vec![("count", Some(Value::N(0)))]))
            .unwrap()
            .count,
        0
    );
    assert_eq!(
        Aggregate::try_from(&row(vec![("count", Some(Value::N(i64::MAX)))]))
            .unwrap()
            .count,
        i64::MAX
    );

    for invalid in [
        row(vec![]),
        row(vec![("count", Some(Value::N(-1)))]),
        row(vec![("count", None)]),
        row(vec![("count", Some(Value::S("12".into())))]),
        row(vec![
            ("count", Some(Value::N(1))),
            ("extra", Some(Value::N(2))),
        ]),
        Row {
            columns: vec!["count".into()],
            values: vec![],
        },
    ] {
        assert!(
            Aggregate::try_from(&invalid).is_err(),
            "accepted {invalid:?}"
        );
    }
}
