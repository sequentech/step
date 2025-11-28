// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::Result;
use deadpool_postgres::Transaction;
use tokio_postgres::row::Row;
use tracing::instrument;
use uuid::Uuid;

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
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(election_event_id)?,
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
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(election_event_id)?,
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
    inc_emails_sent: i64,
    inc_sms_sent: i64,
) -> Result<()> {
    let update_stats_statement = transaction
        .prepare(
            r#"
            UPDATE
                sequent_backend.election_event
            SET
                statistics = jsonb_set(
                    jsonb_set(
                        COALESCE(statistics, '{}'),
                        '{num_emails_sent}', 
                        (COALESCE(statistics->>'num_emails_sent', '0')::int8 + $3)::text::jsonb
                    ),
                    '{num_sms_sent}', 
                    (COALESCE(statistics->>'num_sms_sent', '0')::int8 + $4)::text::jsonb
                )
            WHERE
                tenant_id = $1 AND
                id = $2;
            "#,
        )
        .await?;

    transaction
        .query(
            &update_stats_statement,
            &[
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(election_event_id)?,
                &inc_emails_sent,
                &inc_sms_sent,
            ],
        )
        .await?;

    Ok(())
}

#[instrument(skip(transaction), err)]
pub async fn get_count_distinct_voters(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<i64> {
    let total_distinct_voters_statement = transaction
        .prepare(
            r#"
            SELECT
                COUNT(DISTINCT voter_id_string) AS total_distinct_voters
            FROM
                sequent_backend.cast_vote
            WHERE
                tenant_id = $1 AND
                election_event_id = $2;
            "#,
        )
        .await?;

    let rows: Vec<Row> = transaction
        .query(
            &total_distinct_voters_statement,
            &[
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(election_event_id)?,
            ],
        )
        .await?;

    // all rows contain the count and if there's no rows well, count is clearly
    // zero
    let total_distinct_voters: i64 = if rows.len() == 0 {
        0
    } else {
        rows[0].try_get::<&str, i64>("total_distinct_voters")?
    };

    Ok(total_distinct_voters)
}
