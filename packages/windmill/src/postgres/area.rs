// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::types::keycloak::UserArea;
use std::collections::HashMap;
use tokio_postgres::row::Row;
use tracing::instrument;
use uuid::Uuid;

/**
 * Returns a vector of areas per election event, with the posibility of
 * filtering by area_id
 */
#[instrument(skip(hasura_transaction), err)]
pub async fn get_areas(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    area_ids: &Vec<String>,
) -> Result<Vec<UserArea>> {
    let area_uuids: Vec<Uuid> = area_ids
        .iter()
        .map(|id| Uuid::parse_str(id))
        .collect::<Result<Vec<Uuid>, uuid::Error>>()
        .with_context(|| "Error parsing as uuids the area_ids")?;
    let total_areas_statement = hasura_transaction
        .prepare(
            r#"
            SELECT
                id, name
            FROM
                sequent_backend.area a
            WHERE
                a.tenant_id = $1 AND
                a.election_event_id = $2 AND
                a.id = ANY($3);
            "#,
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &total_areas_statement,
            &[
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(election_event_id)?,
                &area_uuids.as_slice(),
            ],
        )
        .await?;

    let areas: Vec<UserArea> = rows
        .iter()
        .map(|row| {
            let area_id: Uuid = row
                .try_get("id")
                .with_context(|| "Error getting id from row")?;

            let area_name: String = row
                .try_get("name")
                .with_context(|| "Error getting name from row")?;

            Ok(UserArea {
                id: Some(area_id.to_string()),
                name: Some(area_name),
            })
        })
        .collect::<Result<Vec<UserArea>>>()?;

    Ok(areas)
}

/**
 * Returns a map of areas per election event by name
 */
#[instrument(skip(hasura_transaction), err)]
pub async fn get_areas_by_name(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<HashMap<String, String>> {
    let total_areas_statement = hasura_transaction
        .prepare(
            r#"
            SELECT
                id, name
            FROM
                sequent_backend.area a
            WHERE
                a.tenant_id = $1 AND
                a.election_event_id = $2;
            "#,
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &total_areas_statement,
            &[
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(election_event_id)?,
            ],
        )
        .await?;

    let areas_map: HashMap<String, String> = rows
        .iter()
        .map(|row| {
            let area_id: Uuid = row
                .try_get("id")
                .with_context(|| "Error getting id from row")?;

            let area_name: String = row
                .try_get("name")
                .with_context(|| "Error getting name from row")?;

            Ok((area_name, area_id.to_string()))
        })
        .collect::<Result<HashMap<String, String>>>()?;
    Ok(areas_map)
}

/**
 * Returns a map of areas per election event by id
 */
#[instrument(skip(hasura_transaction), err)]
pub async fn get_areas_by_id(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<HashMap<String, String>> {
    let total_areas_statement = hasura_transaction
        .prepare(
            r#"
            SELECT
                id, name
            FROM
                sequent_backend.area a
            WHERE
                a.tenant_id = $1 AND
                a.election_event_id = $2;
            "#,
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &total_areas_statement,
            &[
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(election_event_id)?,
            ],
        )
        .await?;

    let areas_map: HashMap<String, String> = rows
        .iter()
        .map(|row| {
            let area_id: Uuid = row
                .try_get("id")
                .with_context(|| "Error getting id from row")?;

            let area_name: String = row
                .try_get("name")
                .with_context(|| "Error getting name from row")?;

            Ok((area_id.to_string(), area_name))
        })
        .collect::<Result<HashMap<String, String>>>()?;
    Ok(areas_map)
}

/**
 * Returns a hash map with the list of elections (Vec<String> value) associated
 * with each area (String key).
 */
#[instrument(skip(hasura_transaction), err)]
pub async fn get_elections_by_area(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<HashMap<String, Vec<String>>> {
    let total_areas_statement = hasura_transaction
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

    let rows: Vec<Row> = hasura_transaction
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
        let area_id: Uuid = row.try_get("area_id")?;
        let election_id: Uuid = row.try_get("election_id")?;

        areas_to_elections
            .entry(area_id.to_string())
            .or_insert_with(Vec::new)
            .push(election_id.to_string());
    }

    Ok(areas_to_elections)
}
