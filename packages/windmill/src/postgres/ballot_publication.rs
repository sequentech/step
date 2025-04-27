// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use chrono::{DateTime, Local};
use deadpool_postgres::Transaction;
use sequent_core::services::date::ISO8601;
use sequent_core::types::hasura::core::BallotPublication;
use serde::Serialize;
use serde_json::Value;
use tokio_postgres::row::Row;
use tracing::instrument;
use uuid::Uuid;

pub struct BallotPublicationWrapper(pub BallotPublication);

impl TryFrom<Row> for BallotPublicationWrapper {
    type Error = anyhow::Error;
    fn try_from(item: Row) -> Result<Self> {
        Ok(BallotPublicationWrapper(BallotPublication {
            id: item.try_get::<_, Uuid>("id")?.to_string(),
            tenant_id: item.try_get::<_, Uuid>("tenant_id")?.to_string(),
            election_event_id: item.try_get::<_, Uuid>("election_event_id")?.to_string(),
            labels: item.try_get("labels")?,
            annotations: item.try_get("annotations")?,
            created_at: item.get("created_at"),
            deleted_at: item.get("deleted_at"),
            created_by_user_id: item.try_get("created_by_user_id")?,
            is_generated: item.try_get("is_generated")?,
            election_ids: item
                .try_get::<_, Option<Vec<Uuid>>>("election_ids")?
                .map(|uuids| {
                    uuids
                        .clone()
                        .into_iter()
                        .map(|uuid| uuid.to_string())
                        .collect()
                }),
            published_at: item.get("published_at"),
            election_id: item
                .try_get::<_, Option<Uuid>>("election_id")?
                .map(|val| val.to_string()),
        }))
    }
}

#[instrument(skip(hasura_transaction), err)]
pub async fn get_ballot_publication_by_id(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    ballot_publication_id: &str,
) -> Result<Option<BallotPublication>> {
    let query = hasura_transaction
        .prepare(
            r#"
            SELECT
                *
            FROM
                sequent_backend.ballot_publication
            WHERE
                tenant_id = $1 AND
                election_event_id = $2 AND
                id = $3 AND
                deleted_at IS NULL;
            "#,
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &query,
            &[
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(election_event_id)?,
                &Uuid::parse_str(ballot_publication_id)?,
            ],
        )
        .await?;

    let results: Vec<BallotPublication> = rows
        .into_iter()
        .map(|row| -> Result<BallotPublication> {
            row.try_into()
                .map(|res: BallotPublicationWrapper| -> BallotPublication { res.0 })
        })
        .collect::<Result<Vec<BallotPublication>>>()?;

    Ok(results
        .get(0)
        .map(|element: &BallotPublication| element.clone()))
}

pub async fn update_ballot_publication_status(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    ballot_publication_id: &str,
    is_generated: bool,
    published_at: Option<DateTime<Local>>,
) -> Result<Option<BallotPublication>> {
    //let published_at_str = published_at.clone().map(|naive| ISO8601::to_string(&naive));
    let query = hasura_transaction
        .prepare(
            r#"
            UPDATE
                sequent_backend.ballot_publication
            SET
                is_generated = $4,
                published_at = $5
            WHERE
                tenant_id = $1 AND
                election_event_id = $2 AND
                id = $3 AND
                deleted_at IS NULL
            RETURNING
                *;
            "#,
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &query,
            &[
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(election_event_id)?,
                &Uuid::parse_str(ballot_publication_id)?,
                &is_generated,
                &published_at,
            ],
        )
        .await?;

    let results: Vec<BallotPublication> = rows
        .into_iter()
        .map(|row| -> Result<BallotPublication> {
            row.try_into()
                .map(|res: BallotPublicationWrapper| -> BallotPublication { res.0 })
        })
        .collect::<Result<Vec<BallotPublication>>>()?;

    Ok(results
        .get(0)
        .map(|element: &BallotPublication| element.clone()))
}

#[instrument(skip(hasura_transaction), err)]
pub async fn get_latest_ballot_publication(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<Option<BallotPublication>> {
    let query = hasura_transaction
        .prepare(
            r#"
            SELECT
                *
            FROM
                sequent_backend.ballot_publication
            WHERE
                tenant_id = $1 AND
                election_event_id = $2 AND
                deleted_at IS NULL
            ORDER BY
                published_at DESC
            LIMIT 1;
            "#,
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &query,
            &[
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(election_event_id)?,
            ],
        )
        .await?;

    let results: Vec<BallotPublication> = rows
        .into_iter()
        .map(|row| -> Result<BallotPublication> {
            row.try_into()
                .map(|res: BallotPublicationWrapper| -> BallotPublication { res.0 })
        })
        .collect::<Result<Vec<BallotPublication>>>()?;

    Ok(results.first().cloned())
}

#[instrument(skip(hasura_transaction), err)]
pub async fn get_ballot_publication(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<Vec<BallotPublication>> {
    let query = hasura_transaction
        .prepare(
            r#"
            SELECT
                *
            FROM
                sequent_backend.ballot_publication
            WHERE
                tenant_id = $1 AND
                election_event_id = $2 AND
                deleted_at IS NULL;
            "#,
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &query,
            &[
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(election_event_id)?,
            ],
        )
        .await?;

    let results: Vec<BallotPublication> = rows
        .into_iter()
        .map(|row| -> Result<BallotPublication> {
            row.try_into()
                .map(|res: BallotPublicationWrapper| -> BallotPublication { res.0 })
        })
        .collect::<Result<Vec<BallotPublication>>>()?;

    Ok(results)
}

#[derive(Debug, Serialize)]
struct InsertableBallotPublication {
    id: Uuid,
    tenant_id: Uuid,
    election_event_id: Uuid,
    labels: Option<Value>,
    annotations: Option<Value>,
    created_at: Option<DateTime<Local>>,
    deleted_at: Option<DateTime<Local>>,
    created_by_user_id: Option<String>,
    is_generated: Option<bool>,
    election_ids: Option<Vec<Uuid>>,
    published_at: Option<DateTime<Local>>,
    election_id: Option<Uuid>,
}

#[instrument(err, skip_all)]
pub async fn insert_many_ballot_publications(
    hasura_transaction: &Transaction<'_>,
    publications: Vec<BallotPublication>,
) -> Result<Vec<BallotPublication>> {
    if publications.is_empty() {
        return Ok(vec![]);
    }

    let insertable: Vec<InsertableBallotPublication> = publications
        .into_iter()
        .map(|p| {
            let election_id = p.election_id.map(|id| Uuid::parse_str(&id)).transpose()?;
            let election_ids = p.election_ids.map(|ids| {
                ids.into_iter()
                    .filter_map(|id| Uuid::parse_str(&id).ok())
                    .collect()
            });

            Ok(InsertableBallotPublication {
                id: Uuid::parse_str(&p.id)?,
                tenant_id: Uuid::parse_str(&p.tenant_id)?,
                election_event_id: Uuid::parse_str(&p.election_event_id)?,
                labels: p.labels.clone(),
                annotations: p.annotations.clone(),
                created_at: p.created_at,
                deleted_at: p.deleted_at,
                created_by_user_id: p.created_by_user_id.clone(),
                is_generated: p.is_generated,
                election_ids: election_ids,
                published_at: p.published_at,
                election_id: election_id,
            })
        })
        .collect::<Result<_>>()?;

    let json_data = serde_json::to_value(&insertable)?;

    let sql = r#"
        WITH data AS (
            SELECT * FROM jsonb_to_recordset($1::jsonb) AS t(
                id UUID,
                tenant_id UUID,
                election_event_id UUID,
                labels JSONB,
                annotations JSONB,
                created_at TIMESTAMPTZ,
                deleted_at TIMESTAMPTZ,
                created_by_user_id TEXT,
                is_generated BOOL,
                election_ids UUID[],
                published_at TIMESTAMPTZ,
                election_id UUID
            )
        )
        INSERT INTO sequent_backend.ballot_publication (
            id, tenant_id, election_event_id, labels, annotations,
            created_at, deleted_at, created_by_user_id, is_generated,
            election_ids, published_at, election_id
        )
        SELECT
            id, tenant_id, election_event_id, labels, annotations,
            created_at, deleted_at, created_by_user_id, is_generated,
            election_ids, published_at, election_id
        FROM data
        RETURNING *;
    "#;

    let statement = hasura_transaction.prepare(sql).await?;
    let rows = hasura_transaction.query(&statement, &[&json_data]).await?;

    let inserted = rows
        .into_iter()
        .map(|row| {
            let wrapper: BallotPublicationWrapper = row.try_into()?;
            Ok(wrapper.0)
        })
        .collect::<Result<Vec<BallotPublication>>>()?;

    Ok(inserted)
}
