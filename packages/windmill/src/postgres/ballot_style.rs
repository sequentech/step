// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Local};
use deadpool_postgres::Transaction;
use sequent_core::services::date::ISO8601;
use sequent_core::types::hasura::core::BallotStyle;
use tokio_postgres::row::Row;
use tracing::instrument;
use uuid::Uuid;

pub struct BallotStyleWrapper(pub BallotStyle);

impl TryFrom<Row> for BallotStyleWrapper {
    type Error = anyhow::Error;
    fn try_from(item: Row) -> Result<Self> {
        Ok(BallotStyleWrapper(BallotStyle {
            id: item.try_get::<_, Uuid>("id")?.to_string(),
            tenant_id: item.try_get::<_, Uuid>("tenant_id")?.to_string(),
            election_id: item.try_get::<_, Uuid>("election_id")?.to_string(),
            area_id: item
                .try_get::<_, Option<Uuid>>("area_id")?
                .map(|val| val.to_string()),
            created_at: item.get("created_at"),
            last_updated_at: item.get("last_updated_at"),
            labels: item.try_get("labels")?,
            annotations: item.try_get("annotations")?,
            ballot_eml: item.try_get("ballot_eml")?,
            ballot_signature: item.try_get("ballot_signature")?,
            status: item.get("status"),
            election_event_id: item.try_get::<_, Uuid>("election_event_id")?.to_string(),
            deleted_at: item.get("deleted_at"),
            ballot_publication_id: item
                .try_get::<_, Uuid>("ballot_publication_id")?
                .to_string(),
        }))
    }
}

#[instrument(err, skip(hasura_transaction, ballot_eml))]
pub async fn insert_ballot_style(
    hasura_transaction: &Transaction<'_>,
    ballot_style_id: &str,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
    area_id: &str,
    ballot_eml: Option<String>,
    status: Option<String>,
    ballot_publication_id: &str,
) -> Result<BallotStyle> {
    let statement = hasura_transaction
        .prepare(
            r#"
                INSERT INTO
                    sequent_backend.ballot_style
                (id, tenant_id, election_event_id, election_id, area_id, ballot_eml, status, ballot_publication_id, created_at, last_updated_at)
                VALUES(
                    $1,
                    $2,
                    $3,
                    $4,
                    $5,
                    $6,
                    $7,
                    $8,
                    NOW(),
                    NOW()
                )
                RETURNING
                    *;
            "#,
        )
        .await
        .map_err(|err| anyhow!("Error preparing insert statement: {}", err))?;
    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &Uuid::parse_str(ballot_style_id)?,
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(election_event_id)?,
                &Uuid::parse_str(election_id)?,
                &Uuid::parse_str(area_id)?,
                &ballot_eml,
                &status,
                &Uuid::parse_str(ballot_publication_id)?,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error inserting row: {}", err))?;

    let elements: Vec<BallotStyle> = rows
        .into_iter()
        .map(|row| -> Result<BallotStyle> {
            row.try_into()
                .map(|res: BallotStyleWrapper| -> BallotStyle { res.0 })
        })
        .collect::<Result<Vec<BallotStyle>>>()
        .with_context(|| "Error converting rows into documents")?;

    elements
        .get(0)
        .map(|val| val.clone())
        .ok_or(anyhow!("Row not inserted"))
}

#[instrument(skip(hasura_transaction), err)]
pub async fn get_all_ballot_styles(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    area_id: &str,
    authorized_election_ids: &Vec<String>,
) -> Result<Vec<BallotStyle>> {
    let query: tokio_postgres::Statement = hasura_transaction
        .prepare(
            r#"
            SELECT
                *
            FROM
                sequent_backend.ballot_style
            WHERE
                tenant_id = $1 AND
                area_id = $2 AND
                election_id = ANY($3) AND
                deleted_at IS NULL;
            "#,
        )
        .await
        .map_err(|err| anyhow!("Error preparing statement: {}", err))?;

    let rows: Vec<Row> = hasura_transaction
        .query(&query, &[&tenant_id, &area_id, authorized_election_ids])
        .await
        .map_err(|err| anyhow!("Error executing query: {}", err))?;

    let results: Vec<BallotStyle> = rows
        .into_iter()
        .map(|row| -> Result<BallotStyle> {
            row.try_into()
                .map(|res: BallotStyleWrapper| -> BallotStyle { res.0 })
        })
        .collect::<Result<Vec<BallotStyle>>>()
        .map_err(|err| anyhow!("Error collecting ballot styles: {}", err))?;

    Ok(results)
}

#[instrument(skip(hasura_transaction), err)]
pub async fn get_ballot_styles_by_ballot_publication_by_id(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    ballot_publication_id: &str,
) -> Result<Vec<BallotStyle>> {
    let query: tokio_postgres::Statement = hasura_transaction
        .prepare(
            r#"
            SELECT
                *
            FROM
                sequent_backend.ballot_style
            WHERE
                tenant_id = $1 AND
                election_event_id = $2 AND
                ballot_publication_id = $3 AND
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

    let results: Vec<BallotStyle> = rows
        .into_iter()
        .map(|row| -> Result<BallotStyle> {
            row.try_into()
                .map(|res: BallotStyleWrapper| -> BallotStyle { res.0 })
        })
        .collect::<Result<Vec<BallotStyle>>>()?;

    Ok(results)
}

#[instrument(skip(hasura_transaction), err)]
pub async fn export_event_ballot_styles(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<Vec<BallotStyle>> {
    let query: tokio_postgres::Statement = hasura_transaction
        .prepare(
            r#"
            SELECT
                *
            FROM
                sequent_backend.ballot_style
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

    let results: Vec<BallotStyle> = rows
        .into_iter()
        .map(|row| -> Result<BallotStyle> {
            row.try_into()
                .map(|res: BallotStyleWrapper| -> BallotStyle { res.0 })
        })
        .collect::<Result<Vec<BallotStyle>>>()?;

    Ok(results)
}
