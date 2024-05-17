// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::import_election_event::ImportElectionEventSchema;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::{Client as DbClient, Transaction};
use tokio_postgres::row::Row;
use tracing::{event, instrument, Level};
use uuid::Uuid;

#[instrument(err, skip_all)]
pub async fn insert_candidate(
    hasura_transaction: &Transaction<'_>,
    data: &ImportElectionEventSchema,
) -> Result<()> {
    for candidate in &data.candidates {
        candidate.data.validate()?;

        let statement = hasura_transaction
        .prepare(
            r#"
                INSERT INTO sequent_backend.candidate
                (id, tenant_id, election_event_id, contest_id, created_at, last_updated_at, labels, annotations, name, description, type, presentation, is_public, alias, image_document_id)
                VALUES
                ($1, $2, $3, $4, NOW(), NOW(), $5, $6, $7, $8, $9, $10, $11, $12, $13);
            "#,
        )
        .await?;

        let rows: Vec<Row> = hasura_transaction
            .query(
                &statement,
                &[
                    &candidate.id,
                    &Uuid::parse_str(&candidate.data.tenant_id)?,
                    &Uuid::parse_str(&candidate.data.election_event_id)?,
                    &candidate
                        .data
                        .contest_id
                        .as_ref()
                        .and_then(|id| Uuid::parse_str(&id).ok()),
                    &candidate.data.labels,
                    &candidate.data.annotations,
                    &candidate.data.name,
                    &candidate.data.description,
                    &candidate.data.r#type,
                    &candidate.data.presentation,
                    &candidate.data.is_public,
                    &candidate.data.alias,
                    &candidate.data.image_document_id,
                ],
            )
            .await
            .map_err(|err| anyhow!("Error running the document query: {err}"))?;
    }

    Ok(())
}
