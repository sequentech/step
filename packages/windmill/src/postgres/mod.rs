// SPDX-FileCopyrightText: 2023-2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

pub mod application;
pub mod area;
pub mod area_contest;
pub mod ballot_publication;
pub mod ballot_style;
pub mod candidate;
pub mod cast_vote;
pub mod contest;
pub mod document;
pub mod election;
pub mod election_event;
pub mod keycloak_realm;
pub mod keys_ceremony;
pub mod lock;
pub mod maintenance;
pub mod render_report;
pub mod reports;
pub mod results_area_contest;
pub mod results_area_contest_candidate;
pub mod results_contest;
pub mod results_contest_candidate;
pub mod results_election;
pub mod results_election_area;
pub mod results_event;
pub mod scheduled_event;
pub mod secret;
pub mod tally_session;
pub mod tally_session_contest;
pub mod tally_session_execution;
pub mod tally_sheet;
pub mod tasks_execution;
pub mod template;
pub mod tenant;
pub mod trustee;

/// Keeps the rows that convert and logs the ones that do not, so that one
/// unreadable row does not hide the others in a query that spans every tenant.
pub fn skip_unparseable_rows<T>(rows: impl IntoIterator<Item = anyhow::Result<T>>) -> Vec<T> {
    rows.into_iter()
        .filter_map(|row| row.map_err(|err| tracing::error!("{err:#}")).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::anyhow;

    #[test]
    fn skip_unparseable_rows_keeps_the_rows_that_convert() {
        let rows = vec![
            Ok("first"),
            Err(anyhow!("Error mapping \"FOO\" into an EventProcessor")),
            Ok("third"),
            Err(anyhow!("error deserializing encryption_policy")),
            Ok("last"),
        ];
        assert_eq!(skip_unparseable_rows(rows), ["first", "third", "last"]);
    }

    #[test]
    fn skip_unparseable_rows_with_no_rows() {
        let rows: Vec<anyhow::Result<u8>> = vec![];
        assert!(skip_unparseable_rows(rows).is_empty());
    }
}

/// A value PostgreSQL stores but the row's Rust type cannot hold is an error
/// of that row, not a panic, so that `skip_unparseable_rows` skips just it.
/// Each test converts a row selected from literals, so the server needs no
/// schema.
#[cfg(test)]
mod row_conversion_tests {
    use super::reports::ReportWrapper;
    use super::scheduled_event::ScheduledEventWrapper;
    use crate::services::database::PgConfig;
    use deadpool_postgres::Runtime;
    use tokio_postgres::{NoTls, Row};

    async fn select(query: &str) -> Row {
        let pool = PgConfig::from_env()
            .expect("HASURA_DB__* names a PostgreSQL server")
            .hasura_db
            .create_pool(Some(Runtime::Tokio1), NoTls)
            .expect("a database pool");
        let client = pool.get().await.expect("a database connection");
        client.query_one(query, &[]).await.expect("the row")
    }

    fn schedule(created_at: &str) -> String {
        format!(
            "SELECT '00000000-0000-4000-8000-000000000001'::uuid AS id,
                    NULL::uuid AS tenant_id, NULL::uuid AS election_event_id,
                    '{created_at}'::timestamptz AS created_at,
                    NULL::timestamptz AS stopped_at, NULL::timestamptz AS archived_at,
                    NULL::jsonb AS labels, NULL::jsonb AS annotations,
                    'ALLOW_TALLY'::varchar AS event_processor, NULL::jsonb AS cron_config,
                    '{{}}'::jsonb AS event_payload, NULL::varchar AS task_id"
        )
    }

    fn report(permission_label: &str) -> String {
        format!(
            "SELECT '00000000-0000-4000-8000-000000000001'::uuid AS id,
                    '00000000-0000-4000-8000-000000000002'::uuid AS election_event_id,
                    '00000000-0000-4000-8000-000000000003'::uuid AS tenant_id,
                    NULL::uuid AS election_id, 'ELECTORAL_RESULTS'::text AS report_type,
                    NULL::text AS template_alias, '{{\"is_active\": true}}'::jsonb AS cron_config,
                    '2026-01-01T10:00:00Z'::timestamptz AS created_at,
                    'unencrypted'::text AS encryption_policy,
                    {permission_label}::text[] AS permission_label"
        )
    }

    #[tokio::test]
    #[ignore = "requires the PostgreSQL server named by HASURA_DB__*"]
    async fn a_schedule_row_converts() {
        let row = select(&schedule("2026-01-01T10:00:00Z")).await;
        assert!(ScheduledEventWrapper::try_from(row).is_ok());
    }

    #[tokio::test]
    #[ignore = "requires the PostgreSQL server named by HASURA_DB__*"]
    async fn a_schedule_created_at_infinity_is_an_error() {
        let row = select(&schedule("infinity")).await;
        let error = ScheduledEventWrapper::try_from(row)
            .err()
            .expect("infinity is not a DateTime<Utc>");
        assert!(format!("{error:#}").contains("created_at"), "{error:#}");
    }

    #[tokio::test]
    #[ignore = "requires the PostgreSQL server named by HASURA_DB__*"]
    async fn a_report_row_converts() {
        let row = select(&report("ARRAY['label']")).await;
        assert!(ReportWrapper::try_from(row).is_ok());
    }

    #[tokio::test]
    #[ignore = "requires the PostgreSQL server named by HASURA_DB__*"]
    async fn a_report_permission_label_with_a_null_label_is_an_error() {
        let row = select(&report("ARRAY[NULL]")).await;
        let error = ReportWrapper::try_from(row)
            .err()
            .expect("a NULL label is not a String");
        assert!(
            format!("{error:#}").contains("permission_label"),
            "{error:#}"
        );
    }
}
