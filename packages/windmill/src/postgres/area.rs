// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::import::import_election_event::ImportElectionEventSchema;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::services::area_tree::{TreeNode, TreeNodeArea};
use sequent_core::types::{hasura::core::Area, keycloak::UserArea};
use serde::{Deserialize, Serialize};
use sha2::digest::const_oid::db::rfc5911::ID_AES_192_CBC;
use std::collections::HashMap;
use tokio_postgres::row::Row;
use tracing::instrument;
use uuid::Uuid;

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
        }))
    }
}
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
 * Returns a map of area-names of an election event addressable by area-id
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

/**
 * Returns a vector of areas per election event, with the posibility of
 * filtering by area_id
 */
#[instrument(skip(hasura_transaction), err)]
pub async fn get_area_by_id(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    area_id: &str,
) -> Result<Option<Area>> {
    let total_areas_statement = hasura_transaction
        .prepare(
            r#"
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
                parent_id
            FROM
                sequent_backend.area
            WHERE
                tenant_id = $1 AND
                id = $2;
            "#,
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &total_areas_statement,
            &[&Uuid::parse_str(tenant_id)?, &Uuid::parse_str(area_id)?],
        )
        .await?;

    let areas: Vec<Area> = rows
        .into_iter()
        .map(|row| -> Result<Area> { row.try_into().map(|res: AreaWrapper| -> Area { res.0 }) })
        .collect::<Result<Vec<Area>>>()?;

    Ok(areas.get(0).map(|area| area.clone()))
}

#[instrument(err, skip_all)]
pub async fn upsert_area_parents(
    hasura_transaction: &Transaction<'_>,
    areas: &Vec<Area>,
) -> Result<()> {
    for area in areas {
        let statement = hasura_transaction
            .prepare(
                r#"
                UPDATE
                    sequent_backend.area
                SET
                    parent_id = $1
                WHERE
                    id = $2 AND
                    tenant_id = $3 AND
                    election_event_id = $4;
            "#,
            )
            .await?;

        let parent_id: Option<Uuid> = area
            .parent_id
            .clone()
            .map(|parent_id| Uuid::parse_str(&parent_id).ok())
            .flatten();

        let rows: Vec<Row> = hasura_transaction
            .query(
                &statement,
                &[
                    &parent_id,
                    &Uuid::parse_str(&area.id)?,
                    &Uuid::parse_str(&area.tenant_id)?,
                    &Uuid::parse_str(&area.election_event_id)?,
                ],
            )
            .await
            .map_err(|err| anyhow!("Error running query: {err}"))?;
    }

    Ok(())
}

#[instrument(err, skip_all)]
pub async fn insert_areas(hasura_transaction: &Transaction<'_>, areas: &Vec<Area>) -> Result<()> {
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
            r#"
                INSERT INTO sequent_backend.area
                (id, tenant_id, election_event_id, created_at, last_updated_at, labels, annotations, name, description, type, parent_id)
                VALUES
                ($1, $2, $3, NOW(), NOW(), $4, $5, $6, $7, $8, $9);
            "#,
        )
        .await?;

        let parent_id: Option<Uuid> = area
            .parent_id
            .clone()
            .map(|parent_id| Uuid::parse_str(&parent_id).ok())
            .flatten();

        let _rows: Vec<Row> = hasura_transaction
            .query(
                &statement,
                &[
                    &Uuid::parse_str(&area.id)?,
                    &Uuid::parse_str(&area.tenant_id)?,
                    &Uuid::parse_str(&area.election_event_id)?,
                    &area.labels,
                    &area.annotations,
                    &area.name,
                    &area.description,
                    &area.r#type,
                    &parent_id,
                ],
            )
            .await
            .map_err(|err| anyhow!("Error running the document query: {err}"))?;
    }

    Ok(())
}

#[instrument(err, skip_all)]
pub async fn update_areas(hasura_transaction: &Transaction<'_>, areas: &Vec<Area>) -> Result<()> {
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
            .ok_or(anyhow!("Can't find area"))?;

        let statement = hasura_transaction
            .prepare(
                r#"
                UPDATE sequent_backend.area
                SET
                    last_updated_at = NOW(),
                    labels = $1,
                    annotations = $2,
                    name = $3,
                    description = $4,
                    type = $5,
                    parent_id = $6
                WHERE id = $7 AND tenant_id = $8 AND election_event_id = $9;
                "#,
            )
            .await?;

        let parent_id: Option<Uuid> = area
            .parent_id
            .clone()
            .map(|parent_id| Uuid::parse_str(&parent_id).ok())
            .flatten();

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
                    &Uuid::parse_str(&area.id)?,
                    &Uuid::parse_str(&area.tenant_id)?,
                    &Uuid::parse_str(&area.election_event_id)?,
                ],
            )
            .await
            .map_err(|err| anyhow!("Error executing update query: {err}"))?;
    }

    Ok(())
}

#[instrument(err, skip_all)]
pub async fn get_event_areas(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<Vec<Area>> {
    let statement = hasura_transaction
        .prepare(
            r#"
                SELECT
                    *
                FROM
                    sequent_backend.area
                WHERE
                    tenant_id = $1 AND
                    election_event_id = $2;
            "#,
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(election_event_id)?,
            ],
        )
        .await?;

    let election_events: Vec<Area> = rows
        .into_iter()
        .map(|row| -> Result<Area> { row.try_into().map(|res: AreaWrapper| -> Area { res.0 }) })
        .collect::<Result<Vec<Area>>>()?;

    Ok(election_events)
}

#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct AreaElection {
    pub id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub annotations: Option<String>,
}

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

/**
 * Returns a vec of the areas related to giving election.
 */
#[instrument(skip(hasura_transaction), err)]
pub async fn get_areas_by_election_id(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
) -> Result<Vec<Area>> {
    let statement: tokio_postgres::Statement = hasura_transaction
        .prepare(
            r#"
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
            "#,
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(election_event_id)?,
                &Uuid::parse_str(election_id)?,
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

#[instrument(skip(hasura_transaction), err)]
pub async fn insert_area_contests(
    hasura_transaction: &Transaction<'_>,
    area_id: &Uuid,
    contest_ids: &[Uuid],
    election_event_id: &Uuid,
    tenant_id: &Uuid,
) -> Result<()> {
    // Insert new area_contests
    for contest_id in contest_ids {
        let id = Uuid::new_v4();

        let statement = hasura_transaction
            .prepare(
                r#"
                INSERT INTO sequent_backend.area_contest
                (id, tenant_id, election_event_id, contest_id, area_id, created_at, last_updated_at)
                VALUES
                ($1, $2, $3, $4, $5, NOW(), NOW());
            "#,
            )
            .await?;

        let _rows: Vec<Row> = hasura_transaction
            .query(
                &statement,
                &[&id, tenant_id, election_event_id, contest_id, area_id],
            )
            .await
            .map_err(|err| anyhow!("Error running the document query: {err}"))?;
    }

    Ok(())
}

#[instrument(skip(hasura_transaction), err)]
pub async fn delete_area_contests(
    hasura_transaction: &Transaction<'_>,
    area_id: &Uuid,
    election_event_id: &Uuid,
    tenant_id: &Uuid,
) -> Result<()> {
    // Delete existing area_contest rows for this area
    let query: String = format!(
        r#"
            DELETE FROM sequent_backend.area_contest 
            WHERE area_id = $1 
            AND tenant_id = $2 
            AND election_event_id = $3;
            "#
    );

    // Now prepare the statement with the dynamically generated query
    let statement = hasura_transaction.prepare(&query).await?;

    hasura_transaction
        .execute(
            &statement,
                &[area_id, tenant_id, election_event_id],
        )
        .await
        .map_err(|err| anyhow!("Error executing the delete query: {err}"))?;

    Ok(())
}
