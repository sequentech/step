// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::Result;
use deadpool_postgres::Transaction;
use std::collections::HashMap;
use tokio_postgres::row::Row;
use tracing::instrument;
use uuid::Uuid;

/**
 * Returns a hash map with the list of elections (Vec<String> value) associated
 * with each area (String key).
 */
#[instrument(skip(transaction), err)]
pub async fn get_elections_by_area(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<HashMap<String, Vec<String>>> {
    let total_areas_statement = transaction
        .prepare(
            r#"
            SELECT
                a.id AS area_id,
                c.election_id AS election_id
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
                c.election_event_id = $2;
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

    let mut areas_to_elections = HashMap::new();

    for row in rows {
        let area_id: String = row.try_get("area_id")?;
        let election_id: String = row.try_get("election_id")?;

        areas_to_elections
            .entry(area_id)
            .or_insert_with(Vec::new)
            .push(election_id);
    }

    Ok(areas_to_elections)
}
