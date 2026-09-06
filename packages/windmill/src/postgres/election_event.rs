// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::services::uuid_validation::parse_uuid_v4;
use sequent_core::types::hasura::core::ElectionEvent as ElectionEventData;
use serde_json::Value;
use tokio_postgres::row::Row;
use tracing::instrument;
use uuid::Uuid;

pub struct ElectionEventDatafix(pub ElectionEventData);
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
            description: item.get("description"),
            presentation: item.try_get("presentation")?,
            bulletin_board_reference: item.try_get("bulletin_board_reference")?,
            is_archived: item.get("is_archived"),
            voting_channels: item.try_get("voting_channels")?,
            status: item.try_get("status")?,
            user_boards: item.get("user_boards"),
            encryption_protocol: item.get("encryption_protocol"),
            is_audit: item.get("is_audit"),
            audit_election_event_id: item
                .try_get::<_, Option<Uuid>>("audit_election_event_id")?
                .map(|val| val.to_string()),
            public_key: item.get("public_key"),
            statistics: item.try_get("statistics")?,
            external_id: item.try_get("external_id")?,
        }))
    }
}

#[instrument(err, skip_all)]
pub async fn insert_election_event(
    hasura_transaction: &Transaction<'_>,
    election_event: &ElectionEventData,
) -> Result<()> {
    election_event.validate()?;

    let statement = hasura_transaction
        .prepare(
            r#"
                INSERT INTO sequent_backend.election_event
                (id, created_at, updated_at, labels, annotations, tenant_id, description, presentation, bulletin_board_reference, is_archived, voting_channels, status, user_boards, encryption_protocol, is_audit, audit_election_event_id, public_key, statistics, external_id)
                VALUES
                ($1, NOW(), NOW(), $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17);
            "#,
        )
        .await?;

    let _rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &parse_uuid_v4(&election_event.id)?,
                &election_event.labels,
                &election_event.annotations,
                &parse_uuid_v4(&election_event.tenant_id)?,
                &election_event.description,
                &election_event.presentation,
                &election_event.bulletin_board_reference,
                &election_event.is_archived,
                &election_event.voting_channels,
                &election_event.status,
                &election_event.user_boards,
                &election_event.encryption_protocol,
                &election_event.is_audit,
                &election_event
                    .audit_election_event_id
                    .as_ref()
                    .and_then(|s| parse_uuid_v4(s).ok()),
                &election_event.public_key,
                &election_event.statistics,
                &election_event.external_id,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error running the document query: {err}"))?;

    Ok(())
}

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
                    *
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
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
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
        .first()
        .cloned()
        .ok_or(anyhow!("Election event {election_event_id} not found"))
}

#[instrument(err, skip_all)]
pub async fn get_election_event_by_id_if_exist(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<Option<ElectionEventData>> {
    let statement = hasura_transaction
        .prepare(
            r#"
                SELECT
                    *
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
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
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

    let election_event = election_events.first().cloned();
    Ok(election_event)
}

/// Returns all the Election events as ElectionEventDatafix
#[instrument(err, skip_all)]
pub async fn get_all_tenant_election_events(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
) -> Result<Vec<ElectionEventDatafix>> {
    let statement = hasura_transaction
        .prepare(
            r#"
                SELECT
                    *
                FROM
                    sequent_backend.election_event
                WHERE
                    tenant_id = $1 AND
                    is_archived = false
            "#,
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(&statement, &[&parse_uuid_v4(tenant_id)?])
        .await?;

    let election_events: Vec<ElectionEventDatafix> = rows
        .into_iter()
        .map(|row| -> Result<ElectionEventDatafix> {
            row.try_into()
                .map(|res: ElectionEventWrapper| ElectionEventDatafix(res.0))
        })
        .collect::<Result<Vec<ElectionEventDatafix>>>()?;

    Ok(election_events)
}

pub async fn update_election_event_annotations(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    annotations: Value,
) -> Result<()> {
    let tenant_uuid: uuid::Uuid =
        parse_uuid_v4(tenant_id).with_context(|| "Error parsing tenant_id as UUID")?;
    let election_event_uuid: uuid::Uuid = parse_uuid_v4(election_event_id)
        .with_context(|| "Error parsing election_event_id as UUID")?;

    let statement = hasura_transaction
        .prepare(
            r#"
            UPDATE
                "sequent_backend".election_event
            SET
                annotations = $3
            WHERE
                tenant_id = $1
                AND id = $2;
            "#,
        )
        .await?;

    let _rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[&tenant_uuid, &election_event_uuid, &annotations],
        )
        .await
        .with_context(|| anyhow!("Error running the update_election_event_annotations query"))?;

    Ok(())
}

pub async fn update_election_event_presentation(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    presentation: Value,
) -> Result<()> {
    let tenant_uuid: uuid::Uuid =
        parse_uuid_v4(tenant_id).with_context(|| "Error parsing tenant_id as UUID")?;
    let election_event_uuid: uuid::Uuid = parse_uuid_v4(election_event_id)
        .with_context(|| "Error parsing election_event_id as UUID")?;

    let statement = hasura_transaction
        .prepare(
            r#"
            UPDATE
                "sequent_backend".election_event
            SET
                presentation = $3
            WHERE
                tenant_id = $1
                AND id = $2
            "#,
        )
        .await?;

    let _rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[&tenant_uuid, &election_event_uuid, &presentation],
        )
        .await
        .map_err(|err| {
            anyhow!("Error running the update_election_event_presentation query: {err}")
        })?;

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
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
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
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
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
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_id)?,
                &parse_uuid_v4(area_id)?,
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
        .first()
        .cloned()
        .ok_or(anyhow!("Election event not found"))
}

#[instrument(err, skip_all)]
pub async fn delete_election_event(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<()> {
    let related_tables = vec![
        "tally_results_publication",
        "secret",
        "area_contest",
        "results_election_area",
        "results_area_contest_candidate",
        "results_area_contest",
        "election_result",
        "results_contest_candidate",
        "results_contest",
        "results_election",
        "ballot_style",
        "ballot_publication",
        "candidate",
        "tally_session_contest",
        "tally_sheet",
        "tally_session_execution",
        "contest",
        "cast_vote",
        "election",
        "document",
        "event_execution",
        "tally_session",
        "keys_ceremony",
        "scheduled_event",
        "support_material",
        "results_event",
        "area",
        "tasks_execution",
        "report",
        "applications",
    ];

    for table in related_tables {
        let query: String = format!(
            r#"
            DELETE FROM sequent_backend.{}
            WHERE tenant_id = $1 AND election_event_id = $2;
            "#,
            table
        );

        // Now prepare the statement with the dynamically generated query
        let statement = hasura_transaction.prepare(&query).await?;
        hasura_transaction
            .execute(
                &statement,
                &[
                    &parse_uuid_v4(tenant_id)?,
                    &parse_uuid_v4(election_event_id)?,
                ],
            )
            .await
            .map_err(|err| anyhow!("Error executing the delete query: {err}"))?;
    }

    let statement = hasura_transaction
        .prepare(
            r#"
            DELETE FROM sequent_backend.election_event
            WHERE
                tenant_id = $1 AND
                id = $2;
        "#,
        )
        .await?;

    hasura_transaction
        .execute(
            &statement,
            &[
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error executing the delete query: {err}"))?;

    Ok(())
}

#[instrument(err, skip_all)]
pub async fn update_bulletin_board(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    board: &serde_json::Value,
) -> Result<()> {
    let update_bulletin_board = hasura_transaction
        .prepare(
            r#"
             UPDATE sequent_backend.election_event
             SET bulletin_board_reference = $1
             WHERE tenant_id = $2 AND id = $3;
             "#,
        )
        .await?;

    hasura_transaction
         .execute(
             &update_bulletin_board,
             &[
                 &board,
                 &parse_uuid_v4(tenant_id)?,
                 &parse_uuid_v4(election_event_id)?,
             ],
         )
         .await
         .with_context(|| format!("Error updating election event with board reference for tenant ID {} and election event ID {}", tenant_id, election_event_id))?;

    Ok(())
}

#[instrument(err, skip_all)]
pub async fn get_batch_election_events(
    hasura_transaction: &Transaction<'_>,
    limit: i64,
    offset: i64,
) -> Result<Vec<ElectionEventData>> {
    let statement = hasura_transaction
        .prepare(
            r#"
            SELECT 
                *
            FROM sequent_backend.election_event
            WHERE is_archived = false
            ORDER BY created_at ASC
            LIMIT $1
            OFFSET $2;
            "#,
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(&statement, &[&limit, &offset])
        .await?;

    let election_events: Vec<ElectionEventData> = rows
        .into_iter()
        .map(|row| -> Result<ElectionEventData> {
            row.try_into()
                .map(|res: ElectionEventWrapper| -> ElectionEventData { res.0 })
        })
        .collect::<Result<Vec<ElectionEventData>>>()?;

    Ok(election_events)
}
