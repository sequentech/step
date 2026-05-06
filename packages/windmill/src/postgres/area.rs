// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::import::import_election_event::ImportElectionEventSchema;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::services::area_tree::{TreeNode, TreeNodeArea};
use sequent_core::services::uuid_validation::parse_uuid_v4;
use sequent_core::types::{hasura::core::Area, keycloak::UserArea};
use serde::{Deserialize, Serialize};
use sha2::digest::const_oid::db::rfc5911::ID_AES_192_CBC;
use std::collections::HashMap;
use tokio_postgres::row::Row;
use tracing::instrument;
use uuid::Uuid;

/// Newtype for converting a `tokio_postgres::Row` from `sequent_backend.area` into [`Area`].
pub struct AreaWrapper(pub Area);

impl TryFrom<Row> for AreaWrapper {
    type Error = anyhow::Error;
    fn try_from(item: Row) -> Result<Self> {
        Ok(AreaWrapper(Area {
            id: item.try_get::<_, Uuid>("id")?.to_string(),
            tenant_id: item.try_get::<_, Uuid>("tenant_id")?.to_string(),
            election_event_id: item.try_get::<_, Uuid>("election_event_id")?.to_string(),
            created_at: item.get("created_at"),
            last_updated_at: item.get("last_updated_at"),
            labels: item.try_get("labels")?,
            annotations: item.try_get("annotations")?,
            name: item.try_get("name")?,
            description: item.try_get("description")?,
            r#type: item.try_get("type")?,
            parent_id: item
                .try_get::<_, Option<Uuid>>("parent_id")?
                .map(|val| val.to_string()),
            presentation: item.try_get("presentation")?,
        }))
    }
}
/// Returns a vector of areas per election event, with the posibility of
/// filtering by area_id
///
/// # Errors
///
/// Fails if any `area_ids` entry is not a valid UUID, if `tenant_id` / `election_event_id`
/// cannot be parsed, or if preparing or executing the query fails.
#[instrument(skip(hasura_transaction, area_ids), err)]
pub async fn get_areas(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    area_ids: &[String],
) -> Result<Vec<UserArea>> {
    let area_uuids: Vec<Uuid> = area_ids
        .iter()
        .map(|id| parse_uuid_v4(id))
        .collect::<anyhow::Result<Vec<Uuid>>>()
        .with_context(|| "Error parsing as uuids the area_ids")?;
    let total_areas_statement = hasura_transaction
        .prepare(
            r"
            SELECT
                id, name
            FROM
                sequent_backend.area a
            WHERE
                a.tenant_id = $1 AND
                a.election_event_id = $2 AND
                a.id = ANY($3);
            ",
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &total_areas_statement,
            &[
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
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

/// Maps area display name → area id for every area in the election event.
///
/// # Errors
///
/// Fails on invalid UUID parameters or database errors while preparing or running the query.
#[instrument(skip(hasura_transaction), err)]
pub async fn get_areas_by_name(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<HashMap<String, String>> {
    let total_areas_statement = hasura_transaction
        .prepare(
            r"
            SELECT
                id, name
            FROM
                sequent_backend.area a
            WHERE
                a.tenant_id = $1 AND
                a.election_event_id = $2;
            ",
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &total_areas_statement,
            &[
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
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

/// Maps area id → display name for every area in the election event.
///
/// # Errors
///
/// Fails on invalid UUID parameters or database errors while preparing or running the query.
#[instrument(skip(hasura_transaction), err)]
pub async fn get_areas_by_id(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<HashMap<String, String>> {
    let total_areas_statement = hasura_transaction
        .prepare(
            r"
            SELECT
                id, name
            FROM
                sequent_backend.area a
            WHERE
                a.tenant_id = $1 AND
                a.election_event_id = $2;
            ",
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &total_areas_statement,
            &[
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
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

/// Builds, for each area id, the list of election ids linked through `area_contest` → `contest`.
///
/// # Errors
///
/// Fails on invalid UUID parameters or database errors while preparing or running the query.
#[instrument(skip(hasura_transaction), err)]
pub async fn get_elections_by_area(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<HashMap<String, Vec<String>>> {
    let total_areas_statement = hasura_transaction
        .prepare(
            r"
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
            ",
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &total_areas_statement,
            &[
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
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

/// Returns the full [`Area`] row for `area_id` in `tenant_id`, or `None` when no row exists.
///
/// # Errors
///
/// Fails on invalid UUID parameters, if row decoding into [`Area`] fails, or on database errors.
#[instrument(skip(hasura_transaction), err)]
pub async fn get_area_by_id(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    area_id: &str,
) -> Result<Option<Area>> {
    let total_areas_statement = hasura_transaction
        .prepare(
            r"
            SELECT
                id,
                tenant_id,
                election_event_id,
                created_at,
                last_updated_at,
                labels,
                annotations,
                name,
                description,
                type,
                parent_id,
                presentation
            FROM
                sequent_backend.area
            WHERE
                tenant_id = $1 AND
                id = $2;
            ",
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &total_areas_statement,
            &[&parse_uuid_v4(tenant_id)?, &parse_uuid_v4(area_id)?],
        )
        .await?;

    let areas: Vec<Area> = rows
        .into_iter()
        .map(|row| -> Result<Area> { row.try_into().map(|res: AreaWrapper| -> Area { res.0 }) })
        .collect::<Result<Vec<Area>>>()?;

    Ok(areas.first().cloned())
}
/// Updates `parent_id` on each [`Area`] row to match the in-memory tree ordering from import.
///
/// # Errors
///
/// Fails when a UUID on an area cannot be parsed, when preparing or executing an `UPDATE` fails,
/// or when Postgres returns an error for any row in the batch.
#[instrument(err, skip_all)]
pub async fn upsert_area_parents(
    hasura_transaction: &Transaction<'_>,
    areas: &Vec<Area>,
) -> Result<()> {
    for area in areas {
        let statement = hasura_transaction
            .prepare(
                r"
                UPDATE
                    sequent_backend.area
                SET
                    parent_id = $1
                WHERE
                    id = $2 AND
                    tenant_id = $3 AND
                    election_event_id = $4;
            ",
            )
            .await?;

        let parent_id: Option<Uuid> = area
            .parent_id
            .clone()
            .and_then(|parent_id| parse_uuid_v4(&parent_id).ok());

        let rows: Vec<Row> = hasura_transaction
            .query(
                &statement,
                &[
                    &parent_id,
                    &parse_uuid_v4(&area.id)?,
                    &parse_uuid_v4(&area.tenant_id)?,
                    &parse_uuid_v4(&area.election_event_id)?,
                ],
            )
            .await
            .map_err(|err| anyhow!("Error running query: {err}"))?;
    }

    Ok(())
}
/// Insert areas into the database.
///
/// # Errors
///
/// Fails if the area tree cannot be built, if an area id is missing from the map, if any UUID
/// field is invalid, or if an `INSERT` fails at the database layer.
#[instrument(err, skip_all)]
pub async fn insert_areas(hasura_transaction: &Transaction<'_>, areas: &[Area]) -> Result<()> {
    let tree_node_areas: Vec<TreeNodeArea> = areas.iter().map(|area| area.into()).collect();
    let areas_tree = TreeNode::<()>::from_areas(tree_node_areas)?;
    let areas_map: HashMap<String, Area> = areas
        .iter()
        .map(|area| (area.id.clone(), area.clone()))
        .collect();
    for area_node in areas_tree.iter() {
        let Some(area_tree_node) = area_node.area.clone() else {
            continue;
        };
        let area = areas_map
            .get(&area_tree_node.id)
            .ok_or(anyhow!("Can'd find area"))?;

        let statement = hasura_transaction
        .prepare(
            r"
                INSERT INTO sequent_backend.area
                (id, tenant_id, election_event_id, created_at, last_updated_at, labels, annotations, name, description, type, parent_id, presentation)
                VALUES
                ($1, $2, $3, NOW(), NOW(), $4, $5, $6, $7, $8, $9, $10);
            ",
        )
        .await?;

        let parent_id: Option<Uuid> = area
            .parent_id
            .clone()
            .and_then(|parent_id| parse_uuid_v4(&parent_id).ok());

        let _rows: Vec<Row> = hasura_transaction
            .query(
                &statement,
                &[
                    &parse_uuid_v4(&area.id)?,
                    &parse_uuid_v4(&area.tenant_id)?,
                    &parse_uuid_v4(&area.election_event_id)?,
                    &area.labels,
                    &area.annotations,
                    &area.name,
                    &area.description,
                    &area.r#type,
                    &parent_id,
                    &area.presentation,
                ],
            )
            .await
            .map_err(|err| anyhow!("Error running the document query: {err}"))?;
    }

    Ok(())
}
/// Returns every area row for the tenant and election event.
///
/// # Errors
///
/// Fails on invalid UUID parameters, when decoding a row into [`Area`] fails, or on database errors.
#[instrument(err, skip_all)]
pub async fn get_event_areas(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<Vec<Area>> {
    let statement = hasura_transaction
        .prepare(
            r"
                SELECT
                    *
                FROM
                    sequent_backend.area
                WHERE
                    tenant_id = $1 AND
                    election_event_id = $2;
            ",
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
            ],
        )
        .await?;

    let election_events: Vec<Area> = rows
        .into_iter()
        .map(|row| -> Result<Area> { row.try_into().map(|res: AreaWrapper| -> Area { res.0 }) })
        .collect::<Result<Vec<Area>>>()?;

    Ok(election_events)
}

/// Minimal area fields returned when listing areas tied to a specific election.
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct AreaElection {
    /// Area primary key as a string UUID.
    pub id: String,
    /// Area name.
    pub name: Option<String>,
    /// Area description.
    pub description: Option<String>,
    /// Optional free-form operator notes serialized as a string.
    pub annotations: Option<String>,
}

/// Newtype mapping `sequent_backend.area` projection rows into [`AreaElection`].
pub struct AreaElectionWrapper(pub AreaElection);

impl TryFrom<Row> for AreaElectionWrapper {
    type Error = anyhow::Error;
    fn try_from(item: Row) -> Result<Self> {
        Ok(AreaElectionWrapper(AreaElection {
            id: item.try_get::<_, Uuid>("id")?.to_string(),
            name: item.try_get("name")?,
            description: item.try_get("description")?,
            annotations: item.try_get("annotations")?,
        }))
    }
}

/// Returns distinct [`Area`] rows linked to `election_id` through `area_contest` → `contest`.
///
/// # Errors
///
/// Fails on invalid UUID parameters, when decoding rows fails, or when the `SELECT DISTINCT` query fails.
#[instrument(skip(hasura_transaction), err)]
pub async fn get_areas_by_election_id(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
) -> Result<Vec<Area>> {
    let statement: tokio_postgres::Statement = hasura_transaction
        .prepare(
            r"
           SELECT DISTINCT ON (a.id)
                *
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
            ",
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
                &parse_uuid_v4(election_id)?,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error running get_areas_by_election_id query: {err}"))?;

    let areas: Vec<Area> = rows
        .into_iter()
        .map(|row| -> Result<Area> { row.try_into().map(|res: AreaWrapper| -> Area { res.0 }) })
        .collect::<Result<Vec<Area>>>()?;

    Ok(areas)
}

/// Returns full [`Area`] rows for the subset of ids in `area_ids` within the tenant and event.
///
/// # Errors
///
/// Fails if any id string is not a valid UUID or if the `id = ANY($3)` query fails.
#[instrument(skip(hasura_transaction), err)]
pub async fn get_areas_by_ids(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    area_ids: &Vec<String>,
) -> Result<Vec<Area>> {
    let uuid_tenant_id = parse_uuid_v4(tenant_id)?;
    let uuid_election_event_id = parse_uuid_v4(election_event_id)?;
    let uuid_area_ids: Vec<Uuid> = area_ids
        .iter()
        .map(|id| parse_uuid_v4(id))
        .collect::<Result<_, _>>()?;

    let statement = hasura_transaction
        .prepare(
            r"
            SELECT
                *
            FROM
                sequent_backend.area
            WHERE
                tenant_id = $1 AND
                election_event_id = $2 AND
                id = ANY($3);
            ",
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[&uuid_tenant_id, &uuid_election_event_id, &uuid_area_ids],
        )
        .await?;

    let areas: Vec<Area> = rows
        .into_iter()
        .map(|row| row.try_into().map(|res: AreaWrapper| res.0))
        .collect::<Result<Vec<Area>>>()?;

    Ok(areas)
}
/// Removes all `sequent_backend.area_contest` rows for one area before re-linking contests.
///
/// # Errors
///
/// Fails on invalid UUID strings for `tenant_id` / `area_id`, or when the `DELETE` cannot be executed.
#[instrument(skip(hasura_transaction), err)]
pub async fn delete_area_contests(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &Uuid,
    area_id: &str,
) -> Result<()> {
    // Delete existing area_contest rows for this area
    let query = r"
            DELETE FROM sequent_backend.area_contest 
            WHERE tenant_id = $1 
            AND election_event_id = $2 
            AND area_id = $3;
            ";

    // Now prepare the statement with the dynamically generated query
    let statement = hasura_transaction.prepare(query).await?;

    hasura_transaction
        .execute(
            &statement,
            &[
                &parse_uuid_v4(tenant_id)?,
                &election_event_id,
                &parse_uuid_v4(area_id)?,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error executing the delete query: {err}"))?;

    Ok(())
}
/// Persists label, presentation, hierarchy, and metadata changes for an existing [`Area`] row.
///
/// # Errors
///
/// Fails when UUID fields on `area` cannot be parsed, when preparing or executing the `UPDATE` fails,
/// or when Postgres rejects the statement (constraint violation, connection error, …).
#[instrument(err, skip_all)]
pub async fn update_area(hasura_transaction: &Transaction<'_>, area: Area) -> Result<()> {
    let statement = hasura_transaction
        .prepare(
            r"
                UPDATE sequent_backend.area
                SET
                    last_updated_at = NOW(),
                    labels = $1,
                    annotations = $2,
                    name = $3,
                    description = $4,
                    type = $5,
                    parent_id = $6,
                    presentation = $10
                WHERE id = $7 AND tenant_id = $8 AND election_event_id = $9;
                ",
        )
        .await?;

    let parent_id: Option<Uuid> = area
        .parent_id
        .clone()
        .and_then(|parent_id| parse_uuid_v4(&parent_id).ok());

    let _rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &area.labels,
                &area.annotations,
                &area.name,
                &area.description,
                &area.r#type,
                &parent_id,
                &parse_uuid_v4(&area.id)?,
                &parse_uuid_v4(&area.tenant_id)?,
                &parse_uuid_v4(&area.election_event_id)?,
                &area.presentation,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error executing update query: {err}"))?;

    Ok(())
}
/// Inserts a single [`Area`] row.
///
/// # Errors
///
/// Fails on invalid UUIDs in `area`, when the `INSERT` cannot be prepared or executed, or on unique violations.
#[instrument(err, skip_all)]
pub async fn insert_area(hasura_transaction: &Transaction<'_>, area: Area) -> Result<()> {
    let statement = hasura_transaction
        .prepare(
            r"
                INSERT INTO sequent_backend.area
                (id, tenant_id, election_event_id, created_at, last_updated_at, labels, annotations, name, description, type, parent_id, presentation)
                VALUES
                ($1, $2, $3, NOW(), NOW(), $4, $5, $6, $7, $8, $9, $10);
            ",
        )
        .await?;

    let parent_id: Option<Uuid> = area
        .parent_id
        .clone()
        .and_then(|parent_id| parse_uuid_v4(&parent_id).ok());

    let _rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &parse_uuid_v4(&area.id)?,
                &parse_uuid_v4(&area.tenant_id)?,
                &parse_uuid_v4(&area.election_event_id)?,
                &area.labels,
                &area.annotations,
                &area.name,
                &area.description,
                &area.r#type,
                &parent_id,
                &area.presentation,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error running the document query: {err}"))?;

    Ok(())
}
