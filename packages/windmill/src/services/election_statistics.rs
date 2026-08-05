// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::Result;
use deadpool_postgres::Transaction;
use sequent_core::services::uuid_validation::parse_uuid_v4;
use tokio_postgres::row::Row;
use tracing::instrument;

#[instrument(skip(transaction), err)]
pub async fn update_election_statistics(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
    inc_emails_sent: i64,
    inc_sms_sent: i64,
) -> Result<()> {
    let update_stats_statement = transaction
        .prepare(
            r#"
            UPDATE
                sequent_backend.election
            SET
                statistics = jsonb_set(
                    jsonb_set(
                        COALESCE(statistics, '{}'),
                        '{num_emails_sent}', 
                        (COALESCE(statistics->>'num_emails_sent', '0')::int8 + $4)::text::jsonb
                    ),
                    '{num_sms_sent}', 
                    (COALESCE(statistics->>'num_sms_sent', '0')::int8 + $5)::text::jsonb
                )
            WHERE
                tenant_id = $1 AND
                election_event_id = $2 AND
                id = $3;
            "#,
        )
        .await?;

    transaction
        .query(
            &update_stats_statement,
            &[
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
                &parse_uuid_v4(election_id)?,
                &inc_emails_sent,
                &inc_sms_sent,
            ],
        )
        .await?;

    Ok(())
}

#[instrument(skip(transaction), err)]
pub async fn get_count_areas(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
) -> Result<i64> {
    let total_areas_statement = transaction
        .prepare(
            r#"
            SELECT
                count(DISTINCT a.id) as total_areas
            FROM
                sequent_backend.area a
            JOIN
                sequent_backend.area_contest ac ON
                    a.id = ac.area_id AND
                    a.election_event_id = ac.election_event_id AND
                    a.tenant_id = ac.tenant_id
            JOIN
                sequent_backend.contest c ON
                    ac.contest_id = c.id AND
                    ac.election_event_id = c.election_event_id AND
                    ac.tenant_id = c.tenant_id
            WHERE
                c.tenant_id = $1 AND
                c.election_event_id = $2 AND
                c.election_id = $3;
            "#,
        )
        .await?;

    let rows: Vec<Row> = transaction
        .query(
            &total_areas_statement,
            &[
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
                &parse_uuid_v4(election_id)?,
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
