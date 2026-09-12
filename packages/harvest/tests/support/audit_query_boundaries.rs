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

#[test]
fn audit_rows_preserve_each_typed_field_and_reject_unknown_columns() {
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
    let mut row = Row {
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
    };
    let converted = PgAuditRow::try_from(&row).unwrap();
    assert_eq!(
        serde_json::to_value(converted).unwrap(),
        json!({
            "id":42, "audit_type":"SESSION", "class":"READ", "command":"SELECT",
            "dbname":"fixture-db", "server_timestamp":123456, "session_id":"fixture-session",
            "statement":"SELECT @value", "user":"fixture-user"
        })
    );
    // An unexpected schema must fail explicitly, rather than quietly relabeling
    // the value as another audit field.
    row.columns[0] = "(fixture.unexpected)".into();
    assert!(PgAuditRow::try_from(&row).is_err());
}
