// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Local};
use deadpool_postgres::Transaction;
use sequent_core::services::date::ISO8601;
use sequent_core::services::uuid_validation::parse_uuid_v4;
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
                &parse_uuid_v4(ballot_style_id)?,
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
                &parse_uuid_v4(election_id)?,
                &parse_uuid_v4(area_id)?,
                &ballot_eml,
                &status,
                &parse_uuid_v4(ballot_publication_id)?,
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
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
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
pub async fn get_ballot_styles_by_elections(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    authorized_election_ids: &Vec<String>,
) -> Result<Vec<BallotStyle>> {
    let authorized_election_ids_uuids: Vec<Uuid> = authorized_election_ids
        .iter()
        .map(|id| parse_uuid_v4(id))
        .collect::<Result<_, _>>()?;

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
                election_id = ANY($3) AND
                deleted_at IS NULL;
            "#,
        )
        .await
        .map_err(|err| anyhow!("Error preparing statement: {}", err))?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &query,
            &[
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
                &authorized_election_ids_uuids,
            ],
        )
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
pub async fn get_publication_ballot_styles(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    ballot_publication_id: &str,
    limit: Option<usize>,
) -> Result<Vec<BallotStyle>> {
    let limit_clause = if let Some(limit) = limit {
        format!("LIMIT {}", limit)
    } else {
        String::new()
    };

    let query_str = format!(
        "
        SELECT
            *
        FROM
            sequent_backend.ballot_style
        WHERE
            election_event_id = $1
            AND tenant_id = $2
            AND ballot_publication_id = $3
        ORDER BY election_id ASC, area_id ASC
        {limit_clause}"
    );

    let query = hasura_transaction.prepare(query_str.as_str()).await?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &query,
            &[
                &parse_uuid_v4(election_event_id)?,
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(ballot_publication_id)?,
            ],
        )
        .await?;

    let styles: Vec<BallotStyle> = rows
        .into_iter()
        .map(|row| row.try_into().map(|wrapper: BallotStyleWrapper| wrapper.0))
        .collect::<anyhow::Result<Vec<BallotStyle>>>()?;

    Ok(styles)
}

/// A ballot style a voter can cast with, and the public key of its election's
/// keys ceremony.
pub struct CastVoteBallotStyle {
    pub ballot_eml: Option<String>,
    pub keys_ceremony_public_key: Option<String>,
}

/// Finds the ballot style only when it is not deleted, belongs to the voter's
/// tenant, event, election and area, and comes from a generated publication
/// that is not deleted. The EML is read only when `with_ballot_eml` is set.
#[instrument(skip(hasura_transaction), err)]
pub async fn get_cast_vote_ballot_style(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &Uuid,
    election_event_id: &Uuid,
    election_id: &Uuid,
    area_id: &Uuid,
    ballot_style_id: &Uuid,
    with_ballot_eml: bool,
) -> Result<Option<CastVoteBallotStyle>> {
    let statement = hasura_transaction
        .prepare(
            r#"
                SELECT
                    CASE WHEN $6 THEN style.ballot_eml END AS ballot_eml,
                    keys_ceremony.status ->> 'public_key' AS keys_ceremony_public_key
                FROM sequent_backend.ballot_style style
                JOIN sequent_backend.ballot_publication publication ON
                    publication.id = style.ballot_publication_id AND
                    publication.tenant_id = style.tenant_id AND
                    publication.election_event_id = style.election_event_id
                JOIN sequent_backend.election election ON
                    election.id = style.election_id AND
                    election.tenant_id = style.tenant_id AND
                    election.election_event_id = style.election_event_id
                LEFT JOIN sequent_backend.keys_ceremony keys_ceremony ON
                    keys_ceremony.id = election.keys_ceremony_id AND
                    keys_ceremony.tenant_id = election.tenant_id AND
                    keys_ceremony.election_event_id = election.election_event_id
                WHERE
                    style.id = $5 AND
                    style.tenant_id = $1 AND
                    style.election_event_id = $2 AND
                    style.election_id = $3 AND
                    style.area_id = $4 AND
                    style.deleted_at IS NULL AND
                    publication.is_generated IS TRUE AND
                    publication.deleted_at IS NULL;
            "#,
        )
        .await?;
    let row = hasura_transaction
        .query_opt(
            &statement,
            &[
                tenant_id,
                election_event_id,
                election_id,
                area_id,
                ballot_style_id,
                &with_ballot_eml,
            ],
        )
        .await?;

    row.map(|row| -> Result<CastVoteBallotStyle> {
        Ok(CastVoteBallotStyle {
            ballot_eml: row.try_get("ballot_eml")?,
            keys_ceremony_public_key: row.try_get("keys_ceremony_public_key")?,
        })
    })
    .transpose()
}
