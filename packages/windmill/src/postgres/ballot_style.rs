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

/// Inserts ballot styles of one publication with a single statement.
#[instrument(err, skip(hasura_transaction, styles), fields(styles = styles.len()))]
pub async fn insert_ballot_styles(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    ballot_publication_id: Uuid,
    styles: &[NewBallotStyle],
) -> Result<()> {
    super::trusted_write::trusted_write(hasura_transaction).await?;
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
pub async fn get_all_ballot_styles(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    area_id: &str,
    authorized_election_ids: &Vec<String>,
) -> Result<Vec<BallotStyle>> {
    let tenant_uuid = parse_uuid_v4(tenant_id)?;
    let area_uuid = parse_uuid_v4(area_id)?;
    let authorized_election_uuids: Vec<Uuid> = authorized_election_ids
        .iter()
        .map(|id| parse_uuid_v4(id))
        .collect::<Result<_>>()?;

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
        .query(
            &query,
            &[&tenant_uuid, &area_uuid, &authorized_election_uuids],
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

/// The published ballot styles of the elections. A style counts as published
/// when its publication is generated, published and not deleted: generating a
/// publication inserts its styles before it is published, so a style that is
/// only not deleted can belong to a draft.
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

/// `None` unless the ballot style is the one published for the voter's area
/// and election. Its `ballot_eml` is read only when `with_ballot_eml` asks
/// for it, since it is large.
#[instrument(skip(hasura_transaction), err)]
pub async fn get_published_ballot_eml(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &Uuid,
    election_event_id: &Uuid,
    election_id: &Uuid,
    area_id: &Uuid,
    ballot_style_id: &Uuid,
    with_ballot_eml: bool,
) -> Result<Option<Option<String>>> {
    let statement = hasura_transaction
        .prepare(
            r#"
                SELECT CASE WHEN $6 THEN style.ballot_eml END AS ballot_eml
                FROM sequent_backend.ballot_style style
                JOIN sequent_backend.ballot_publication publication ON
                    publication.id = style.ballot_publication_id AND
                    publication.tenant_id = style.tenant_id AND
                    publication.election_event_id = style.election_event_id
                WHERE
                    style.id = $5 AND
                    style.tenant_id = $1 AND
                    style.election_event_id = $2 AND
                    style.election_id = $3 AND
                    style.area_id = $4 AND
                    style.deleted_at IS NULL AND
                    publication.deleted_at IS NULL AND
                    publication.published_at IS NOT NULL;
            "#,
        )
        .await?;
    let row = hasura_transaction
        .query_opt(
            &statement,
            &[
                &tenant_id,
                &election_event_id,
                &election_id,
                &area_id,
                &ballot_style_id,
                &with_ballot_eml,
            ],
        )
        .await?;

    Ok(row
        .map(|row| row.try_get::<_, Option<String>>("ballot_eml"))
        .transpose()?)
}

/// The content of the ballot styles of the current, published ballot
/// publication for `area_id` and `election_id`.
#[instrument(skip(hasura_transaction), err)]
pub async fn get_published_ballot_emls(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    area_id: &str,
    election_id: &str,
) -> Result<Vec<String>> {
    let rows = hasura_transaction
        .query(
            r#"
            SELECT
                s.ballot_eml
            FROM
                sequent_backend.ballot_style s
            JOIN sequent_backend.ballot_publication p ON p.id = s.ballot_publication_id
                AND p.tenant_id = s.tenant_id
                AND p.election_event_id = s.election_event_id
            WHERE
                s.tenant_id = $1 AND
                s.election_event_id = $2 AND
                s.area_id = $3 AND
                s.election_id = $4 AND
                s.deleted_at IS NULL AND
                s.ballot_eml IS NOT NULL AND
                p.is_generated IS TRUE AND
                p.published_at IS NOT NULL AND
                p.deleted_at IS NULL;
            "#,
            &[
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
                &parse_uuid_v4(area_id)?,
                &parse_uuid_v4(election_id)?,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error executing query: {}", err))?;

    rows.into_iter()
        .map(|row| {
            row.try_get::<_, String>("ballot_eml")
                .map_err(|err| anyhow!("Error reading ballot style: {}", err))
        })
        .collect()
}
