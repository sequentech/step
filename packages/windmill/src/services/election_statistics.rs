// SPDX-FileCopyrightText: 2023, 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::cast_votes::CastVoteStatus;
use anyhow::Result;
use deadpool_postgres::Transaction;
use tokio_postgres::row::Row;
use tracing::instrument;
use uuid::Uuid;

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
                        statistics, 
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
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(election_event_id)?,
                &Uuid::parse_str(election_id)?,
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
    election_id: &str,
) -> Result<i64> {
    let status = CastVoteStatus::Valid.to_string();
    let total_distinct_voters_statement = transaction
        .prepare(
            r#"
            SELECT
                COUNT(DISTINCT voter_id_string) AS total_distinct_voters
            FROM
                sequent_backend.election el
            LEFT JOIN 
                sequent_backend.cast_vote cv ON el.id = cv.election_id
            WHERE
                el.tenant_id = $1 AND
                el.election_event_id = $2 AND
                el.id = $3 AND
                cv.status = $4;
            "#,
        )
        .await?;

    let rows: Vec<Row> = transaction
        .query(
            &total_distinct_voters_statement,
            &[
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(election_event_id)?,
                &Uuid::parse_str(election_id)?,
                &status,
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
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(election_event_id)?,
                &Uuid::parse_str(election_id)?,
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
