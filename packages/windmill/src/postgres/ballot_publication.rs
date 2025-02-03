// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use chrono::{DateTime, Local};
use deadpool_postgres::Transaction;
use sequent_core::services::date::ISO8601;
use sequent_core::types::hasura::core::BallotPublication;
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
