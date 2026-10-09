// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::Result;
use deadpool_postgres::Transaction;
use sequent_core::services::uuid_validation::parse_uuid_v4;
use std::collections::BTreeMap;
use tokio_postgres::row::Row;
use tracing::instrument;

/// SQL expression adding the counters of the jsonb parameter `$param`
/// (`{"num_emails_sent": 2, ...}`) to the row's `statistics`, keeping every
/// other key.
pub fn increment_statistics_sql(param: usize) -> String {
    format!(
        r#"COALESCE(statistics, '{{}}'::jsonb) || (
            SELECT COALESCE(
                jsonb_object_agg(
                    increment.key,
                    to_jsonb(
                        COALESCE((statistics->>increment.key)::int8, 0)
                            + increment.value::int8
                    )
                ),
                '{{}}'::jsonb
            )
            FROM jsonb_each_text(${param}::jsonb) AS increment
        )"#
    )
}

/**
 * Returns the count of areas per election event
 */
#[instrument(skip(transaction), err)]
pub async fn get_count_areas(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<i64> {
    let total_areas_statement = transaction
        .prepare(
            r#"
            SELECT
                COUNT(*) AS total_areas
            FROM
                sequent_backend.area a
            WHERE
                a.tenant_id = $1 AND
                a.election_event_id = $2;
            "#,
        )
        .await?;

    let rows: Vec<Row> = transaction
        .query(
            &total_areas_statement,
            &[
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
            ],
        )
        .await?;

    // all rows contain the count and if there's no rows well, count is clearly
    // zero
    let total_areas: i64 = if rows.len() == 0 {
        0
    } else {
        rows[0].try_get::<&str, i64>("total_areas")?
    };

    Ok(total_areas)
}

/**
 * Returns the count of elections in an election event
 */
#[instrument(skip(transaction), err)]
pub async fn get_count_elections(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<i64> {
    let total_elections_statement = transaction
        .prepare(
            r#"
            SELECT
                COUNT(*) AS total_elections
            FROM
                sequent_backend.election e
            WHERE
                e.tenant_id = $1 AND
                e.election_event_id = $2;
            "#,
        )
        .await?;

    let rows: Vec<Row> = transaction
        .query(
            &total_elections_statement,
            &[
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
            ],
        )
        .await?;

    // all rows contain the count and if there's no rows well, count is clearly
    // zero
    let total_elections: i64 = if rows.len() == 0 {
        0
    } else {
        rows[0].try_get::<&str, i64>("total_elections")?
    };

    Ok(total_elections)
}

#[instrument(skip(transaction), err)]
pub async fn update_election_event_statistics(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    increments: &BTreeMap<String, i64>,
) -> Result<()> {
    let increments_sql = increment_statistics_sql(3);
    let update_stats_statement = transaction
        .prepare(&format!(
            r#"
            UPDATE
                sequent_backend.election_event
            SET
                statistics = {increments_sql}
            WHERE
                tenant_id = $1 AND
                id = $2;
            "#,
        ))
        .await?;

    transaction
        .query(
            &update_stats_statement,
            &[
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
                &serde_json::to_value(increments)?,
            ],
        )
        .await?;

    Ok(())
}
