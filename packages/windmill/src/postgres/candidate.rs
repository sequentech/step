// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::import::import_election_event::ImportElectionEventSchema;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::types::hasura::core::Candidate;
use tokio_postgres::row::Row;
use tracing::{event, instrument, Level};
use uuid::Uuid;

pub struct CandidateWrapper(pub Candidate);

impl TryFrom<Row> for CandidateWrapper {
    type Error = anyhow::Error;

    fn try_from(item: Row) -> Result<Self> {
        Ok(CandidateWrapper(Candidate {
            id: item.try_get::<_, Uuid>("id")?.to_string(),
            tenant_id: item.try_get::<_, Uuid>("tenant_id")?.to_string(),
            election_event_id: item.try_get::<_, Uuid>("election_event_id")?.to_string(),
            contest_id: item
                .try_get::<_, Option<Uuid>>("contest_id")?
                .map(|val| val.to_string()),
            created_at: item.get("created_at"),
            last_updated_at: item.get("last_updated_at"),
            labels: item.try_get("labels")?,
            annotations: item.try_get("annotations")?,
            name: item.try_get("name")?,
            alias: item.try_get("alias")?,
            description: item.try_get("description")?,
            r#type: item.try_get("type")?,
            presentation: item.try_get("presentation")?,
            is_public: item.try_get("is_public")?,
            image_document_id: item.try_get("image_document_id")?,
        }))
    }
}

#[instrument(err, skip_all)]
pub async fn insert_candidates(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    candidates: &Vec<Candidate>,
) -> Result<()> {
    for candidate in candidates {
        candidate.validate()?;

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
                    &Uuid::parse_str(&candidate.id)?,
                    &Uuid::parse_str(tenant_id)?,
                    &Uuid::parse_str(election_event_id)?,
                    &candidate
                        .contest_id
                        .as_ref()
                        .and_then(|id| Uuid::parse_str(&id).ok()),
                    &candidate.labels,
                    &candidate.annotations,
                    &candidate.name,
                    &candidate.description,
                    &candidate.r#type,
                    &candidate.presentation,
                    &candidate.is_public,
                    &candidate.alias,
                    &candidate.image_document_id,
                ],
            )
            .await
            .map_err(|err| anyhow!("Error running the document query: {err}"))?;
    }

    Ok(())
}

#[instrument(err, skip_all)]
pub async fn export_candidates(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<Vec<Candidate>> {
    let statement = hasura_transaction
        .prepare(
            r#"
                SELECT
                    id, tenant_id, election_event_id, contest_id, created_at, last_updated_at, labels, annotations, name, description, type, presentation, is_public, alias, image_document_id
                FROM
                    sequent_backend.candidate
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

    let election_events: Vec<Candidate> = rows
        .into_iter()
        .map(|row| -> Result<Candidate> {
            row.try_into()
                .map(|res: CandidateWrapper| -> Candidate { res.0 })
        })
        .collect::<Result<Vec<Candidate>>>()?;

    Ok(election_events)
}
