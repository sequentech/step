// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use chrono::TimeZone;
use sequent_core::monitoring::compute::Column;
use sequent_core::monitoring::payload::Notice;
use serde_json::json;

fn column(name: &str, kind: ColumnKind) -> Column {
    Column {
        name: name.to_string(),
        kind,
    }
}

fn at(hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 5, 11, hour, 0, 0).unwrap()
}

/// A by-group query with awkward labels and a zero denominator, and a
/// series; and a widget whose producer is not connected.
fn data() -> ExportData {
    let groups = QueryResult {
        columns: vec![
            column("group", ColumnKind::Text),
            column("voted", ColumnKind::Integer),
            column("pct", ColumnKind::Number),
            column("pct_label", ColumnKind::Text),
        ],
        rows: vec![
            vec![
                json!("Madrid, \"Centro\""),
                json!(3),
                json!(0.75),
                json!("75.0%"),
            ],
            vec![json!("Line\nbreak"), json!(0), json!(null), json!("—")],
            vec![
                json!("Ñandú 東京 O'Brien"),
                json!(1),
                json!(1),
                json!("100.0%"),
            ],
            vec![json!("=SUM(A1)"), json!(2), json!(0.5), json!("50.0%")],
        ],
        notices: vec![Notice::UnregisteredAttemptsExcluded],
    };
    let series = QueryResult {
        columns: vec![
            column("bucket_start", ColumnKind::Text),
            column("bucket_utc", ColumnKind::Text),
            column("voted", ColumnKind::Integer),
        ],
        rows: vec![
            vec![
                json!("2026-05-11T10:00:00"),
                json!("2026-05-11T08:00:00Z"),
                json!(4),
            ],
            vec![
                json!("2026-05-11T11:00:00"),
                json!("2026-05-11T09:00:00Z"),
                json!(5),
            ],
        ],
        notices: vec![],
    };
    ExportData {
        election_event_id: "00000000-0000-4000-8000-000000000001".to_string(),
        dashboard_id: "overview".to_string(),
        widget_id: None,
        snapshot_revision: 7,
        as_of: at(9),
        scope: ScopeSelection {
            region: Some("Europe\n-- DROP".to_string()),
            post: None,
            country: None,
        },
        pinned_post: None,
        from: Some(at(8)),
        to: Some(at(10)),
        widgets: vec![
            (
                "turnout".to_string(),
                WidgetData::Evaluated {
                    queries: IndexMap::from([
                        ("data".to_string(), groups),
                        ("activity".to_string(), series),
                    ]),
                },
            ),
            (
                "helpdesk".to_string(),
                WidgetData::NotConnected {
                    reason: PendingProducer::HelpdeskIntegration,
                },
            ),
        ],
    }
}

#[test]
fn csv_is_one_long_table_with_every_value_quoted_as_needed() {
    let csv = String::from_utf8(build_file(&data(), MonitoringExportFormat::Csv).unwrap()).unwrap();
    let expected = concat!(
        "widget_id,query,row,group,voted,pct,pct_label,bucket_start,bucket_utc,notice\r\n",
        "turnout,data,1,\"Madrid, \"\"Centro\"\"\",3,0.75,75.0%,,,UNREGISTERED_ATTEMPTS_EXCLUDED\r\n",
        "turnout,data,2,\"Line\nbreak\",0,,—,,,UNREGISTERED_ATTEMPTS_EXCLUDED\r\n",
        "turnout,data,3,Ñandú 東京 O'Brien,1,1,100.0%,,,UNREGISTERED_ATTEMPTS_EXCLUDED\r\n",
        "turnout,data,4,'=SUM(A1),2,0.5,50.0%,,,UNREGISTERED_ATTEMPTS_EXCLUDED\r\n",
        "turnout,activity,1,,4,,,2026-05-11T10:00:00,2026-05-11T08:00:00Z,\r\n",
        "turnout,activity,2,,5,,,2026-05-11T11:00:00,2026-05-11T09:00:00Z,\r\n",
        "helpdesk,,,,,,,,,NOT_CONNECTED: HELPDESK_INTEGRATION\r\n",
    );
    assert_eq!(csv, expected);
}

#[test]
fn sql_creates_the_table_and_inserts_every_row_as_literals() {
    let sql = String::from_utf8(build_file(&data(), MonitoringExportFormat::Sql).unwrap()).unwrap();
    let expected = concat!(
        "-- Monitoring export (PostgreSQL)\n",
        "-- Election event: 00000000-0000-4000-8000-000000000001\n",
        "-- Dashboard: overview; widget: all\n",
        "-- Snapshot revision 7 as of 2026-05-11T09:00:00Z\n",
        "-- Scope: region Europe -- DROP; post all; country all\n",
        "-- Series buckets from 2026-05-11T08:00:00Z up to, not including, 2026-05-11T10:00:00Z\n",
        "BEGIN;\n",
        "CREATE TABLE \"monitoring_export\" (\n",
        "  \"widget_id\" TEXT,\n",
        "  \"query\" TEXT,\n",
        "  \"row\" BIGINT,\n",
        "  \"group\" TEXT,\n",
        "  \"voted\" BIGINT,\n",
        "  \"pct\" DOUBLE PRECISION,\n",
        "  \"pct_label\" TEXT,\n",
        "  \"bucket_start\" TEXT,\n",
        "  \"bucket_utc\" TEXT,\n",
        "  \"notice\" TEXT\n",
        ");\n",
        "INSERT INTO \"monitoring_export\" (\"widget_id\", \"query\", \"row\", \"group\", \"voted\", \"pct\", \"pct_label\", \"bucket_start\", \"bucket_utc\", \"notice\") VALUES\n",
        "  ('turnout', 'data', 1, 'Madrid, \"Centro\"', 3, 0.75, '75.0%', NULL, NULL, 'UNREGISTERED_ATTEMPTS_EXCLUDED'),\n",
        "  ('turnout', 'data', 2, 'Line\nbreak', 0, NULL, '—', NULL, NULL, 'UNREGISTERED_ATTEMPTS_EXCLUDED'),\n",
        "  ('turnout', 'data', 3, 'Ñandú 東京 O''Brien', 1, 1, '100.0%', NULL, NULL, 'UNREGISTERED_ATTEMPTS_EXCLUDED'),\n",
        "  ('turnout', 'data', 4, '=SUM(A1)', 2, 0.5, '50.0%', NULL, NULL, 'UNREGISTERED_ATTEMPTS_EXCLUDED'),\n",
        "  ('turnout', 'activity', 1, NULL, 4, NULL, NULL, '2026-05-11T10:00:00', '2026-05-11T08:00:00Z', NULL),\n",
        "  ('turnout', 'activity', 2, NULL, 5, NULL, NULL, '2026-05-11T11:00:00', '2026-05-11T09:00:00Z', NULL),\n",
        "  ('helpdesk', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, 'NOT_CONNECTED: HELPDESK_INTEGRATION');\n",
        "COMMIT;\n",
    );
    assert_eq!(sql, expected);
}

#[test]
fn sql_inserts_are_batched() {
    let mut data = data();
    let rows: Vec<Vec<Value>> = (0..(INSERT_BATCH + 1))
        .map(|n| vec![json!(format!("g{n}")), json!(n)])
        .collect();
    data.widgets = vec![(
        "w".to_string(),
        WidgetData::Evaluated {
            queries: IndexMap::from([(
                "data".to_string(),
                QueryResult {
                    columns: vec![
                        column("group", ColumnKind::Text),
                        column("voted", ColumnKind::Integer),
                    ],
                    rows,
                    notices: vec![],
                },
            )]),
        },
    )];
    let sql = String::from_utf8(build_file(&data, MonitoringExportFormat::Sql).unwrap()).unwrap();
    assert_eq!(sql.matches("INSERT INTO").count(), 2);
    assert_eq!(sql.matches("  ('w', 'data', ").count(), INSERT_BATCH + 1);
}

#[test]
fn sql_literal_quotes_text_and_writes_the_rest_as_sql() {
    assert_eq!(sql_literal(&json!(null)).unwrap(), "NULL");
    assert_eq!(sql_literal(&json!(true)).unwrap(), "TRUE");
    assert_eq!(sql_literal(&json!(false)).unwrap(), "FALSE");
    assert_eq!(sql_literal(&json!(42)).unwrap(), "42");
    assert_eq!(sql_literal(&json!(-3)).unwrap(), "-3");
    assert_eq!(sql_literal(&json!(0.125)).unwrap(), "0.125");
    assert_eq!(sql_literal(&json!("it's")).unwrap(), "'it''s'");
    assert_eq!(sql_literal(&json!("''")).unwrap(), "''''''");
    assert_eq!(sql_literal(&json!("a\\b")).unwrap(), "'a\\b'");
    assert_eq!(
        sql_literal(&json!("');DROP TABLE x;--")).unwrap(),
        "''');DROP TABLE x;--'"
    );
    assert_eq!(sql_literal(&json!("—東京")).unwrap(), "'—東京'");
    assert_eq!(sql_literal(&json!(["a'"])).unwrap(), "'[\"a''\"]'");
    assert!(matches!(
        sql_literal(&json!("a\0b")),
        Err(MonitoringExportError::Invalid(_))
    ));
}

#[test]
fn a_column_two_queries_fill_differently_holds_both() {
    let mut data = data();
    data.widgets = vec![(
        "w".to_string(),
        WidgetData::Evaluated {
            queries: IndexMap::from([
                (
                    "a".to_string(),
                    QueryResult {
                        columns: vec![
                            column("value", ColumnKind::Integer),
                            column("row", ColumnKind::Integer),
                        ],
                        rows: vec![vec![json!(1), json!(9)]],
                        notices: vec![],
                    },
                ),
                (
                    "b".to_string(),
                    QueryResult {
                        columns: vec![column("value", ColumnKind::Text)],
                        rows: vec![vec![json!("x")]],
                        notices: vec![],
                    },
                ),
            ]),
        },
    )];
    let table = export_table(&data);
    assert_eq!(
        table.columns[3..5],
        [
            ("value".to_string(), ExportColumnType::Text),
            ("row_value".to_string(), ExportColumnType::Integer)
        ]
    );
    let sql = String::from_utf8(to_sql(&data, &table).unwrap()).unwrap();
    assert!(sql.contains("  ('w', 'a', 1, '1', 9, NULL),\n  ('w', 'b', 1, 'x', NULL, NULL);"));
}

fn series() -> QueryResult {
    QueryResult {
        columns: vec![
            column("bucket_utc", ColumnKind::Text),
            column("voted", ColumnKind::Integer),
        ],
        rows: (7..11)
            .map(|hour| vec![json!(format!("2026-05-11T{hour:02}:00:00Z")), json!(hour)])
            .collect(),
        notices: vec![],
    }
}

fn hours(result: &QueryResult) -> Vec<u64> {
    result
        .rows
        .iter()
        .map(|row| row[1].as_u64().unwrap())
        .collect()
}

#[test]
fn buckets_are_kept_from_the_start_up_to_not_including_the_end() {
    let mut result = series();
    keep_buckets(&mut result, Some(at(8)), Some(at(10)));
    assert_eq!(hours(&result), vec![8, 9]);

    let mut result = series();
    keep_buckets(&mut result, Some(at(9)), None);
    assert_eq!(hours(&result), vec![9, 10]);

    let mut result = series();
    keep_buckets(&mut result, None, Some(at(8)));
    assert_eq!(hours(&result), vec![7]);

    let mut result = series();
    keep_buckets(&mut result, None, None);
    assert_eq!(hours(&result), vec![7, 8, 9, 10]);
}

#[test]
fn a_result_without_buckets_is_kept_whole() {
    let mut result = QueryResult {
        columns: vec![column("voted", ColumnKind::Integer)],
        rows: vec![vec![json!(3)]],
        notices: vec![],
    };
    keep_buckets(&mut result, Some(at(8)), Some(at(9)));
    assert_eq!(result.rows.len(), 1);
}

#[test]
fn the_request_reads_as_harvest_sends_it() {
    let request: MonitoringExportRequest = serde_json::from_value(json!({
        "tenant_id": "00000000-0000-4000-8000-000000000001",
        "election_event_id": "00000000-0000-4000-8000-000000000002",
        "dashboard_id": "overview",
        "election_ids": ["00000000-0000-4000-8000-000000000003"],
        "snapshot_revision": 3,
        "format": "SQL",
        "from": "2026-05-11T08:00:00Z",
        "document_id": "d"
    }))
    .unwrap();
    assert_eq!(request.format, MonitoringExportFormat::Sql);
    assert_eq!(request.from, Some(at(8)));
    assert_eq!(request.to, None);
    assert_eq!(request.widget_id, None);
    assert!(request.selector_values.is_empty());
    assert_eq!(request.scope, ScopeSelection::default());
}

#[test]
fn the_file_is_named_by_dashboard_widget_and_revision() {
    let mut data = data();
    assert_eq!(
        file_name(&data, MonitoringExportFormat::Csv),
        "monitoring-overview-r7.csv"
    );
    data.widget_id = Some("turnout/../x".to_string());
    assert_eq!(
        file_name(&data, MonitoringExportFormat::Sql),
        "monitoring-overview-turnout____x-r7.sql"
    );
}

#[test]
fn each_error_names_its_code() {
    let pruned = MonitoringExportError::SnapshotPruned { revision: 4 };
    assert!(pruned
        .to_string()
        .starts_with("SNAPSHOT_PRUNED: snapshot revision 4"));
    let pending = MonitoringExportError::ScopePending {
        widget_id: "turnout".to_string(),
    };
    assert!(pending.to_string().starts_with("SCOPE_PENDING: "));
}
