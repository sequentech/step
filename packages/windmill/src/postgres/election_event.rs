// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::import_election_event::ImportElectionEventSchema;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::types::hasura::core::ElectionEvent as ElectionEventData;
use tokio_postgres::row::Row;
use tracing::{event, instrument, Level};
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
            audit_election_event_id: item.get("audit_election_event_id"),
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
    data.election_event_data.validate()?;

    let statement = hasura_transaction
        .prepare(
            r#"
                INSERT INTO sequent_backend.election_event
                (id, created_at, updated_at, labels, annotations, tenant_id, name, description, presentation, bulletin_board_reference, is_archived, voting_channels, dates, status, user_boards, encryption_protocol, is_audit, audit_election_event_id, public_key, alias, statistics)
                VALUES
                ($1, NOW(), NOW(), $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19);
            "#,
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &Uuid::parse_str(&data.election_event_data.id)?,
                &data.election_event_data.labels,
                &data.election_event_data.annotations,
                &Uuid::parse_str(&data.election_event_data.tenant_id)?,
                &data.election_event_data.name,
                &data.election_event_data.description,
                &data.election_event_data.presentation,
                &data.election_event_data.bulletin_board_reference,
                &data.election_event_data.is_archived,
                &data.election_event_data.voting_channels,
                &data.election_event_data.dates,
                &data.election_event_data.status,
                &data.election_event_data.user_boards,
                &data.election_event_data.encryption_protocol,
                &data.election_event_data.is_audit,
                &data
                    .election_event_data
                    .audit_election_event_id
                    .as_ref()
                    .and_then(|s| Uuid::parse_str(&s).ok()),
                &data.election_event_data.public_key,
                &data.election_event_data.alias,
                &data.election_event_data.statistics,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error running the document query: {err}"))?;

    Ok(())
}

#[instrument(err, skip_all)]
pub async fn export_election_event(
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
