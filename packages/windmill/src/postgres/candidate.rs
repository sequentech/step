// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::sql_utils::escape_sql_literal;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use futures::pin_mut;
use sequent_core::services::uuid_validation::parse_uuid_v4;
use sequent_core::types::hasura::core::Candidate;
use tokio_postgres::binary_copy::BinaryCopyInWriter;
use tokio_postgres::row::Row;
use tokio_postgres::types::{ToSql, Type};
use tracing::instrument;
use uuid::Uuid;

/// Candidate column for insert operation
const CANDIDATE_COPY_COLUMNS: &str = "id, tenant_id, election_event_id, contest_id,
created_at, last_updated_at, labels, annotations, name, description, type,
presentation, is_public, alias, image_document_id";

/// Candidate columns types for insert operation (same order as the columns)
const CANDIDATE_COPY_TYPES: &[Type] = &[
    Type::UUID,
    Type::UUID,
    Type::UUID,
    Type::UUID,
    Type::TIMESTAMPTZ,
    Type::TIMESTAMPTZ,
    Type::JSONB,
    Type::JSONB,
    Type::VARCHAR,
    Type::TEXT,
    Type::VARCHAR,
    Type::JSONB,
    Type::BOOL,
    Type::TEXT,
    Type::TEXT,
];

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
    if candidates.is_empty() {
        return Ok(());
    }

    let tenant_uuid = parse_uuid_v4(tenant_id)?;
    let election_event_uuid = parse_uuid_v4(election_event_id)?;
    let now = chrono::Utc::now();

    let copy_sql =
        format!("COPY sequent_backend.candidate ({CANDIDATE_COPY_COLUMNS}) FROM STDIN BINARY");

    let sink = hasura_transaction
        .copy_in(&copy_sql)
        .await
        .with_context(|| format!("Error preparing candidate COPY IN: {copy_sql}"))?;
    let writer = BinaryCopyInWriter::new(sink, CANDIDATE_COPY_TYPES);
    pin_mut!(writer);

    for candidate in candidates {
        candidate.validate()?;

        let id = parse_uuid_v4(&candidate.id)?;
        let contest_id = candidate
            .contest_id
            .as_ref()
            .and_then(|contest_id| parse_uuid_v4(contest_id).ok());

        let row: [&(dyn ToSql + Sync); 15] = [
            &id,
            &tenant_uuid,
            &election_event_uuid,
            &contest_id,
            &now,
            &now,
            &candidate.labels,
            &candidate.annotations,
            &candidate.name,
            &candidate.description,
            &candidate.r#type,
            &candidate.presentation,
            &candidate.is_public,
            &candidate.alias,
            &candidate.image_document_id,
        ];

        writer
            .as_mut()
            .write(&row)
            .await
            .map_err(|err| anyhow!("Error writing candidate COPY row: {err}"))?;
    }

    writer
        .finish()
        .await
        .context("Error finishing candidate COPY IN transaction")?;

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
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
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

#[instrument(skip(hasura_transaction), err)]
pub async fn get_candidates_by_contest_id(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    contest_id: &str,
) -> Result<Vec<Candidate>> {
    let statement = hasura_transaction
        .prepare(
            r#"
            SELECT
                *
            FROM
                sequent_backend.candidate
            WHERE
                tenant_id = $1 AND
                election_event_id = $2 AND
                contest_id = $3;
            "#,
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
                &parse_uuid_v4(contest_id)?,
            ],
        )
        .await?;

    let candidate: Vec<Candidate> = rows
        .into_iter()
        .map(|row| -> Result<Candidate> {
            row.try_into()
                .map(|res: CandidateWrapper| -> Candidate { res.0 })
        })
        .collect::<Result<Vec<Candidate>>>()?;

    Ok(candidate)
}
