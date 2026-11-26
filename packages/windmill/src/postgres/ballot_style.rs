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

/// A generated ballot style, before it is stored.
pub struct NewBallotStyle {
    pub id: Uuid,
    pub election_id: Uuid,
    pub area_id: Uuid,
    pub ballot_eml: String,
}

/// Inserts the ballot styles of one publication with a single statement.
#[instrument(err, skip(hasura_transaction, styles), fields(styles = styles.len()))]
pub async fn insert_ballot_styles(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    ballot_publication_id: Uuid,
    styles: &[NewBallotStyle],
) -> Result<()> {
    let ids: Vec<Uuid> = styles.iter().map(|style| style.id).collect();
    let election_ids: Vec<Uuid> = styles.iter().map(|style| style.election_id).collect();
    let area_ids: Vec<Uuid> = styles.iter().map(|style| style.area_id).collect();
    let emls: Vec<&str> = styles
        .iter()
        .map(|style| style.ballot_eml.as_str())
        .collect();
    let inserted = hasura_transaction
        .execute(
            r#"
                INSERT INTO
                    sequent_backend.ballot_style
                (id, tenant_id, election_event_id, election_id, area_id, ballot_eml, ballot_publication_id, created_at, last_updated_at)
                SELECT
                    style.id, $1, $2, style.election_id, style.area_id, style.ballot_eml, $3, NOW(), NOW()
                FROM
                    UNNEST($4::uuid[], $5::uuid[], $6::uuid[], $7::text[])
                    AS style(id, election_id, area_id, ballot_eml);
            "#,
            &[
                &tenant_id,
                &election_event_id,
                &ballot_publication_id,
                &ids,
                &election_ids,
                &area_ids,
                &emls,
            ],
        )
        .await
        .context("Error inserting ballot styles")?;
    if inserted != styles.len() as u64 {
        return Err(anyhow!(
            "Inserted {inserted} of {} ballot styles",
            styles.len()
        ));
    }
    Ok(())
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

/// The published ballot styles of the elections, only those of `area_id` when
/// given. A style counts as published when its publication is generated,
/// published and not deleted: generating a publication inserts its styles
/// before it is published, so a style that is only not deleted can belong to a
/// draft.
#[instrument(skip(hasura_transaction), err)]
pub async fn get_ballot_styles_by_elections(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    authorized_election_ids: &Vec<String>,
    area_id: Option<&str>,
) -> Result<Vec<BallotStyle>> {
    let authorized_election_ids_uuids: Vec<Uuid> = authorized_election_ids
        .iter()
        .map(|id| parse_uuid_v4(id))
        .collect::<Result<_, _>>()?;
    let area_uuid = area_id.map(parse_uuid_v4).transpose()?;

    let query: tokio_postgres::Statement = hasura_transaction
        .prepare(
            r#"
            SELECT
                style.*
            FROM
                sequent_backend.ballot_style AS style
            JOIN sequent_backend.ballot_publication AS publication ON
                publication.id = style.ballot_publication_id AND
                publication.tenant_id = style.tenant_id AND
                publication.election_event_id = style.election_event_id
            WHERE
                style.tenant_id = $1 AND
                style.election_event_id = $2 AND
                style.election_id = ANY($3) AND
                ($4::uuid IS NULL OR style.area_id = $4::uuid) AND
                style.deleted_at IS NULL AND
                publication.is_generated IS TRUE AND
                publication.published_at IS NOT NULL AND
                publication.deleted_at IS NULL;
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
                &area_uuid,
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
    area_id: Option<&str>,
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
            AND ($4::uuid IS NULL OR area_id = $4)
        ORDER BY election_id ASC, area_id ASC
        {limit_clause}"
    );

    let query = hasura_transaction.prepare(query_str.as_str()).await?;
    let area_uuid = area_id.map(parse_uuid_v4).transpose()?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &query,
            &[
                &parse_uuid_v4(election_event_id)?,
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(ballot_publication_id)?,
                &area_uuid,
            ],
        )
        .await?;

    let styles: Vec<BallotStyle> = rows
        .into_iter()
        .map(|row| row.try_into().map(|wrapper: BallotStyleWrapper| wrapper.0))
        .collect::<anyhow::Result<Vec<BallotStyle>>>()?;

    Ok(styles)
}
