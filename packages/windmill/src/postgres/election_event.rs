// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::{
    services::import_election_event::ImportElectionEventSchema,
    types::scheduled_event::EventProcessors,
};
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::ballot::VotingStatus;
use sequent_core::types::hasura::core::ElectionEvent as ElectionEventData;
use serde_json::Value;
use tokio_postgres::row::Row;
use tracing::{event, info, instrument, Level};
use uuid::Uuid;

pub struct ElectionEventWrapper(pub ElectionEventData);

impl TryFrom<Row> for ElectionEventWrapper {
    type Error = anyhow::Error;
    fn try_from(item: Row) -> Result<Self> {
        Ok(ElectionEventWrapper(ElectionEventData {
            id: item.try_get::<_, Uuid>("id")?.to_string(),
            created_at: item.get("created_at"),
            updated_at: item.get("updated_at"),
            labels: item.try_get("labels")?,
            annotations: item.try_get("annotations")?,
            tenant_id: item.try_get::<_, Uuid>("tenant_id")?.to_string(),
            name: item.get("name"),
            description: item.get("description"),
            presentation: item.try_get("presentation")?,
            bulletin_board_reference: item.try_get("bulletin_board_reference")?,
            is_archived: item.get("is_archived"),
            voting_channels: item.try_get("voting_channels")?,
            dates: item.try_get("dates")?,
            status: item.try_get("status")?,
            user_boards: item.get("user_boards"),
            encryption_protocol: item.get("encryption_protocol"),
            is_audit: item.get("is_audit"),
            audit_election_event_id: item
                .try_get::<_, Option<Uuid>>("audit_election_event_id")?
                .map(|val| val.to_string()),
            public_key: item.get("public_key"),
            alias: item.get("alias"),
            statistics: item.try_get("statistics")?,
        }))
    }
}

#[instrument(err, skip_all)]
pub async fn insert_election_event(
    hasura_transaction: &Transaction<'_>,
    data: &ImportElectionEventSchema,
) -> Result<()> {
    data.election_event.validate()?;

    // Prepare the new SQL statement
    let statement = hasura_transaction
        .prepare(
            r#"
                INSERT INTO sequent_backend.event_list
                (
                    id,
                    election,
                    created_at,
                    updated_at,
                    tenant_id,
                    schedule,
                    event_type,
                    receivers,
                    template,
                    election_event_id
                )
                VALUES
                (
                    $1,
                    $2,
                    NOW(),
                    NOW(),
                    $3,
                    $4,
                    $5,
                    $6,
                    $7,
                    $8
                );
            "#,
        )
        .await?;

    // Bind data to the SQL statement
    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                // Generate a new UUID for the id field
                &Uuid::new_v4(),
                
                // The election field should use the name from data.election_event
                &data.election_event.name,
                
                // The tenant_id field should use the tenant_id from data.election_event
                &Uuid::parse_str(&data.election_event.tenant_id)?,
                
                // The schedule field should use the dates from data.election_event
                &data.election_event.dates,
                
                // The event_type field should be a fixed string "event"
                &"event".to_string(),
                
                // The receivers field should be an empty array (assuming your DB accepts this format)
                &"[]".to_string(),
                
                // The template field should be an empty string
                &"".to_string(),
                
                // Generate a new UUID for the election_event_id field
                &Uuid::parse_str(&data.election_event.id)?,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error running the document query: {err}"))?;

    Ok(())
}
// #[instrument(err, skip_all)]
// pub async fn insert_election_event(
//     hasura_transaction: &Transaction<'_>,
//     data: &ImportElectionEventSchema,
// ) -> Result<()> {
//     data.election_event.validate()?;

//     let statement = hasura_transaction
//         .prepare(
//             r#"
//                 INSERT INTO sequent_backend.election_event
//                 (id, created_at, updated_at, labels, annotations, tenant_id, name, description, presentation, bulletin_board_reference, is_archived, voting_channels, dates, status, user_boards, encryption_protocol, is_audit, audit_election_event_id, public_key, alias, statistics)
//                 VALUES
//                 ($1, NOW(), NOW(), $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19);
//             "#,
//         )
//         .await?;

//     let rows: Vec<Row> = hasura_transaction
//         .query(
//             &statement,
//             &[
//                 &Uuid::parse_str(&data.election_event.id)?,
//                 &data.election_event.labels,
//                 &data.election_event.annotations,
//                 &Uuid::parse_str(&data.election_event.tenant_id)?,
//                 &data.election_event.name,
//                 &data.election_event.description,
//                 &data.election_event.presentation,
//                 &data.election_event.bulletin_board_reference,
//                 &data.election_event.is_archived,
//                 &data.election_event.voting_channels,
//                 &data.election_event.dates,
//                 &data.election_event.status,
//                 &data.election_event.user_boards,
//                 &data.election_event.encryption_protocol,
//                 &data.election_event.is_audit,
//                 &data
//                     .election_event
//                     .audit_election_event_id
//                     .as_ref()
//                     .and_then(|s| Uuid::parse_str(&s).ok()),
//                 &data.election_event.public_key,
//                 &data.election_event.alias,
//                 &data.election_event.statistics,
//             ],
//         )
//         .await
//         .map_err(|err| anyhow!("Error running the document query: {err}"))?;

//     Ok(())
// }

#[instrument(err, skip_all)]
pub async fn get_election_event_by_id(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<ElectionEventData> {
    let statement = hasura_transaction
        .prepare(
            r#"
                SELECT
                    id, created_at, updated_at, labels, annotations, tenant_id, name, description, presentation, bulletin_board_reference, is_archived, voting_channels, dates, status, user_boards, encryption_protocol, is_audit, audit_election_event_id, public_key, alias, statistics
                FROM
                    sequent_backend.election_event
                WHERE
                    tenant_id = $1 AND
                    id = $2;
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

    let election_events: Vec<ElectionEventData> = rows
        .into_iter()
        .map(|row| -> Result<ElectionEventData> {
            row.try_into()
                .map(|res: ElectionEventWrapper| -> ElectionEventData { res.0 })
        })
        .collect::<Result<Vec<ElectionEventData>>>()?;

    election_events
        .get(0)
        .map(|election_event| election_event.clone())
        .ok_or(anyhow!("Election event {election_event_id} not found"))
}

#[instrument(skip(hasura_transaction), err)]
pub async fn update_election_event_dates(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    dates: Value,
) -> Result<()> {
    let tenant_uuid: uuid::Uuid =
        Uuid::parse_str(tenant_id).with_context(|| "Error parsing tenant_id as UUID")?;
    let election_event_uuid: uuid::Uuid = Uuid::parse_str(election_event_id)
        .with_context(|| "Error parsing election_event_id as UUID")?;
    let statement = hasura_transaction
        .prepare(
            r#"
                UPDATE sequent_backend.election_event
                SET dates = $1
                WHERE tenant_id = $2 AND id = $3;
            "#,
        )
        .await?;
    let _row: Vec<Row> = hasura_transaction
        .query(&statement, &[&dates, &tenant_uuid, &election_event_uuid])
        .await
        .map_err(|err| anyhow!("Error running the update_election_dates query: {err}"))?;

    Ok(())
}

#[instrument(skip(hasura_transaction), err)]
pub async fn update_elections_status_by_election_event(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    status: Value,
) -> Result<Vec<String>> {
    let statement = hasura_transaction
        .prepare(
            r#"
                UPDATE sequent_backend.election
                SET
                status = $1
                WHERE tenant_id = $2 AND election_event_id = $3
                RETURNING id;
            "#,
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &status,
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(election_event_id)?,
            ],
        )
        .await
        .map_err(|err| {
            anyhow!("Error running the update_elections_status_by_election_event query: {err}")
        })?;

    //retrieve all the election ids to log them when a status is changed in the election event level.
    let ids: Vec<String> = rows
        .iter()
        .map(|row| {
            let election_id: Uuid = row
                .try_get("id")
                .with_context(|| "Error getting id from row")?;
            Ok(election_id.to_string())
        })
        .collect::<Result<Vec<String>>>()?;

    Ok(ids)
}

#[instrument(skip(hasura_transaction), err)]
pub async fn update_election_event_status(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    status: Value,
) -> Result<()> {
    let statement = hasura_transaction
        .prepare(
            r#"
                UPDATE sequent_backend.election_event
                SET status = $1
                WHERE tenant_id = $2 AND id = $3;
            "#,
        )
        .await?;

    let _rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &status,
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(election_event_id)?,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error running the update_election_event_staut query: {err}"))?;

    Ok(())
}

#[instrument(err, skip_all)]
pub async fn get_election_event_by_election_area(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_id: &str,
    area_id: &str,
) -> Result<ElectionEventData> {
    let statement = hasura_transaction
        .prepare(
            r#"
                SELECT
                    election_event.*
                FROM
                    sequent_backend.area_contest AS area_contest
                INNER JOIN
                    sequent_backend.contest AS contest
                ON
                    contest.id = area_contest.contest_id
                INNER JOIN
                    sequent_backend.election_event AS election_event
                ON
                    election_event.id = area_contest.election_event_id
                WHERE
                    area_contest.tenant_id = $1 AND
                    area_contest.area_id =$3 AND
                    contest.tenant_id = $1 AND
                    contest.election_id = $2 AND
                    election_event.tenant_id = $1;
            "#,
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(election_id)?,
                &Uuid::parse_str(area_id)?,
            ],
        )
        .await?;

    let election_events: Vec<ElectionEventData> = rows
        .into_iter()
        .map(|row| -> Result<ElectionEventData> {
            row.try_into()
                .map(|res: ElectionEventWrapper| -> ElectionEventData { res.0 })
        })
        .collect::<Result<Vec<ElectionEventData>>>()?;

    election_events
        .get(0)
        .map(|election_event| election_event.clone())
        .ok_or(anyhow!("Election event not found"))
}
