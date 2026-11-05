// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Audit searches accept operator input. Values must stay in bound parameters;
//! column names and directions must come from the request's closed enums.

use super::*;
use serde_json::json;

fn request(extra: serde_json::Value) -> GetPgauditBody {
    let mut body =
        json!({"tenant_id": "tenant-a", "election_event_id": "event-a"});
    body.as_object_mut()
        .unwrap()
        .extend(extra.as_object().unwrap().clone());
    serde_json::from_value(body).unwrap()
}

#[test]
fn sql_looking_search_text_is_a_parameter_and_never_part_of_the_query() {
    const SEARCH: &str = "%' OR 1=1; DROP TABLE pgaudit_hasura; --";
    let input = request(json!({"filter": {"statement": SEARCH}}));
    let (sql, params) = input.as_sql(true).unwrap();
    assert_eq!(sql, "WHERE statement LIKE @param_statement");
    assert!(!sql.contains(SEARCH));
    assert_eq!(params.len(), 1);
    assert_eq!(params[0].name, "param_statement");
    assert_eq!(
        params[0].value.as_ref().unwrap().value,
        Some(Value::S(SEARCH.into()))
    );
}

#[test]
fn numeric_identifiers_are_typed_and_invalid_identifiers_fail_before_execution()
{
    let (sql, params) = request(json!({"filter": {"id": "42"}}))
        .as_sql(true)
        .unwrap();
    assert_eq!(sql, "WHERE id = @param_id");
    assert_eq!(params[0].value.as_ref().unwrap().value, Some(Value::N(42)));
    for invalid in ["42 OR 1=1", "9223372036854775808", "", "1.5"] {
        assert!(request(json!({"filter": {"id": invalid}}))
            .as_sql(true)
            .is_err());
    }
}

#[test]
fn arbitrary_columns_tables_and_sort_directions_cannot_enter_sql() {
    for field in [
        json!({"filter": {"secret_column": "x"}}),
        json!({"order_by": {"id; DROP TABLE pgaudit_hasura": "asc"}}),
        json!({"order_by": {"id": "asc; SELECT 1"}}),
        json!({"audit_table": "pgaudit_hasura; SELECT 1"}),
    ] {
        let mut body =
            json!({"tenant_id":"tenant-a", "election_event_id":"event-a"});
        body.as_object_mut()
            .unwrap()
            .extend(field.as_object().unwrap().clone());
        assert!(serde_json::from_value::<GetPgauditBody>(body).is_err());
    }
}

#[test]
fn count_queries_keep_filters_but_ignore_pagination_and_ordering() {
    let input = request(json!({
        "filter": {"class": "READ"}, "order_by": {"id": "desc"},
        "limit": 7, "offset": 200
    }));
    let (sql, params) = input.as_sql(true).unwrap();
    assert_eq!(sql, "WHERE class LIKE @param_class");
    assert_eq!(params.len(), 1);
}

#[test]
fn negative_offsets_are_clamped_without_changing_the_requested_page_size() {
    let input = request(json!({"limit": 7, "offset": -8}));
    let (sql, params) = input.as_sql(false).unwrap();
    assert_eq!(sql, "LIMIT @limit OFFSET @offset");
    assert_eq!(params[0].value.as_ref().unwrap().value, Some(Value::N(7)));
    assert_eq!(params[1].value.as_ref().unwrap().value, Some(Value::N(0)));
}

fn complete_row() -> Row {
    let entries = [
        ("id", Value::N(42)),
        ("audit_type", Value::S("SESSION".into())),
        ("class", Value::S("READ".into())),
        ("command", Value::S("SELECT".into())),
        ("dbname", Value::S("fixture-db".into())),
        ("server_timestamp", Value::Ts(123456)),
        ("session_id", Value::S("fixture-session".into())),
        ("statement", Value::S("SELECT @value".into())),
        ("user", Value::S("fixture-user".into())),
    ];
    Row {
        columns: entries
            .iter()
            .map(|(name, _)| format!("(fixture.{name})"))
            .collect(),
        values: entries
            .iter()
            .map(|(_, value)| SqlValue {
                value: Some(value.clone()),
            })
            .collect(),
    }
}

#[test]
fn audit_rows_preserve_each_typed_field_and_reject_unknown_columns() {
    let mut row = complete_row();
    let converted = PgAuditRow::try_from(&row).unwrap();
    assert_eq!(
        serde_json::to_value(converted).unwrap(),
        json!({
            "id":42, "audit_type":"SESSION", "class":"READ", "command":"SELECT",
            "dbname":"fixture-db", "server_timestamp":123456, "session_id":"fixture-session",
            "statement":"SELECT @value", "user":"fixture-user"
        })
    );
    // zip() would silently discard either surplus. Require a one-to-one row
    // shape so corrupted input cannot look like a successfully decoded audit.
    let mut extra_column = row.clone();
    extra_column.columns.push("(fixture.unmatched)".into());
    assert!(PgAuditRow::try_from(&extra_column).is_err());
    let mut extra_value = row.clone();
    extra_value.values.push(SqlValue {
        value: Some(Value::N(99)),
    });
    assert!(PgAuditRow::try_from(&extra_value).is_err());

    // An unexpected schema must fail explicitly, rather than quietly relabeling
    // the value as another audit field.
    row.columns[0] = "(fixture.unexpected)".into();
    assert!(PgAuditRow::try_from(&row).is_err());
}

#[test]
fn audit_mapping_uses_column_names_for_both_tables_in_any_order() {
    let expected =
        serde_json::to_value(PgAuditRow::try_from(&complete_row()).unwrap())
            .unwrap();
    for table in ["pgaudit_hasura", "pgaudit_keycloak"] {
        let mut row = complete_row();
        row.columns = row
            .columns
            .iter()
            .map(|name| name.replace("fixture", table))
            .collect();
        row.columns.reverse();
        row.values.reverse();
        assert_eq!(
            serde_json::to_value(PgAuditRow::try_from(&row).unwrap()).unwrap(),
            expected
        );
    }
}

#[test]
fn each_missing_audit_field_is_rejected_instead_of_becoming_a_default() {
    PgAuditRow::try_from(&complete_row()).unwrap();
    for missing in 0..9 {
        let mut row = complete_row();
        let name = row.columns.remove(missing);
        row.values.remove(missing);
        let error = PgAuditRow::try_from(&row).expect_err(&name);
        assert!(error.to_string().contains("missing"), "{name}: {error}");
    }
    assert!(PgAuditRow::try_from(&Row {
        columns: vec![],
        values: vec![]
    })
    .is_err());
}

#[test]
fn duplicate_audit_fields_cannot_replace_a_missing_field() {
    PgAuditRow::try_from(&complete_row()).unwrap();
    for duplicate in 0..9 {
        let mut row = complete_row();
        let replaced = (duplicate + 1) % 9;
        row.columns[replaced] =
            row.columns[duplicate].replace("fixture", "other");
        row.values[replaced] = row.values[duplicate].clone();
        let error = PgAuditRow::try_from(&row).unwrap_err();
        assert!(error.to_string().contains("duplicate"), "{error}");
    }
}

#[test]
fn audit_fields_reject_null_and_wrong_types_without_returning_a_partial_row() {
    PgAuditRow::try_from(&complete_row()).unwrap();
    for field in 0..9 {
        for value in [None, Some(Value::B(true))] {
            let mut row = complete_row();
            row.values[field].value = value;
            assert!(
                PgAuditRow::try_from(&row).is_err(),
                "{}",
                row.columns[field]
            );
        }
    }
}

#[test]
fn empty_ordering_omits_order_by_and_retains_pagination() {
    let (ordered, _) = request(json!({"order_by": {"id": "desc"}, "limit": 7}))
        .as_sql(false)
        .unwrap();
    assert_eq!(ordered, "ORDER BY id desc LIMIT @limit");
    let (empty, params) = request(json!({"order_by": {}, "limit": 7}))
        .as_sql(false)
        .unwrap();
    assert_eq!(empty, "LIMIT @limit");
    assert_eq!(params[0].value.as_ref().unwrap().value, Some(Value::N(7)));
}

#[test]
fn count_rows_preserve_zero_and_nonzero_counts_and_reject_wrong_types() {
    for count in [0, 42, i64::MAX] {
        let row = Row {
            columns: vec!["count".into()],
            values: vec![SqlValue {
                value: Some(Value::N(count)),
            }],
        };
        assert_eq!(Aggregate::try_from(&row).unwrap().count, count);
    }
    for value in [None, Some(Value::S("42".into()))] {
        let row = Row {
            columns: vec!["count".into()],
            values: vec![SqlValue { value }],
        };
        assert!(Aggregate::try_from(&row).is_err());
    }
}

#[test]
fn malformed_count_rows_cannot_silently_become_zero_or_the_last_value() {
    let valid = Row {
        columns: vec!["count".into()],
        values: vec![SqlValue {
            value: Some(Value::N(42)),
        }],
    };
    assert_eq!(Aggregate::try_from(&valid).unwrap().count, 42);
    for (columns, values, diagnostic) in [
        (0, 0, "got 0 columns and 0 values"),
        (0, 1, "got 0 columns and 1 values"),
        (1, 0, "got 1 columns and 0 values"),
        (1, 2, "got 1 columns and 2 values"),
        (2, 1, "got 2 columns and 1 values"),
        (2, 2, "got 2 columns and 2 values"),
    ] {
        let row = Row {
            columns: vec!["count".into(); columns],
            values: vec![
                SqlValue {
                    value: Some(Value::N(42))
                };
                values
            ],
        };
        let error = Aggregate::try_from(&row).unwrap_err();
        assert!(error.to_string().contains("exactly one"), "{error}");
        assert!(error.to_string().contains(diagnostic), "{error}");
    }
}

#[test]
fn multiple_sort_fields_have_stable_precedence() {
    // The request map has no order, so an unsorted rendering matches the
    // alphabetical precedence of all nine fields only by a 1 in 9! chance.
    for order in [
        json!({"user": "desc", "statement": "asc", "session_id": "desc",
            "server_timestamp": "asc", "id": "desc", "dbname": "asc",
            "command": "desc", "class": "asc", "audit_type": "desc"}),
        json!({"audit_type": "desc", "class": "asc", "command": "desc",
            "dbname": "asc", "id": "desc", "server_timestamp": "asc",
            "session_id": "desc", "statement": "asc", "user": "desc"}),
    ] {
        let (sql, _) = request(json!({"order_by": order, "limit": 7}))
            .as_sql(false)
            .unwrap();
        assert_eq!(
            sql,
            "ORDER BY audit_type desc, class asc, command desc, dbname asc, \
             id desc, server_timestamp asc, session_id desc, statement asc, \
             user desc LIMIT @limit"
        );
    }
}
