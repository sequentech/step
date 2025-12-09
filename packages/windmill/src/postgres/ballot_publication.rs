// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use chrono::{DateTime, Local};
use deadpool_postgres::Transaction;
use sequent_core::services::date::ISO8601;
use sequent_core::types::hasura::core::BallotPublication;
use tokio::try_join;
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
                id = $3;
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

pub async fn update_ballot_publication(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    ballot_publication_id: &str,
    is_generated: bool,
    published_at: Option<DateTime<Local>>,
) -> Result<Option<BallotPublication>> {
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
                id = $3
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

#[instrument(skip(hasura_transaction), err)]
pub async fn insert_ballot_publication(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_ids: Vec<String>,
    user_id: String,
    election_id: Option<String>,
) -> Result<Option<BallotPublication>> {
    let election_id_uuid = election_id
        .map(|id_str| Uuid::parse_str(&id_str))
        .transpose()?;

    let election_ids_uuid: Vec<Uuid> = election_ids
        .iter()
        .map(|s| Uuid::parse_str(s))
        .collect::<Result<Vec<Uuid>, _>>()?;

    let query = hasura_transaction
        .prepare(
            r#"
            INSERT INTO sequent_backend.ballot_publication
                (election_ids, election_event_id, tenant_id, created_by_user_id, election_id)
            VALUES
                ($1, $2, $3, $4, $5)
            RETURNING
                *;
            "#,
        )
        .await?;

    let rows = hasura_transaction
        .query(
            &query,
            &[
                &election_ids_uuid,
                &Uuid::parse_str(election_event_id)?,
                &Uuid::parse_str(tenant_id)?,
                &user_id,
                &election_id_uuid,
            ],
        )
        .await?;
    println!("Rows: {:?}", rows);

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
pub async fn get_previous_publication_election(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    published_at: Option<DateTime<Local>>,
    election_id: &str,
) -> Result<Option<BallotPublication>> {
    let query = hasura_transaction
        .prepare(
            r#"
            SELECT
                id,
                tenant_id,
                election_event_id,
                labels,
                annotations,
                created_at,
                deleted_at,
                created_by_user_id,
                is_generated,
                election_ids,
                published_at,
                election_id
            FROM sequent_backend.ballot_publication
            WHERE election_event_id = $1
              AND tenant_id = $2
              AND election_ids @> ARRAY[$3]::uuid[]
              AND published_at IS NOT NULL
              AND published_at < $4
            ORDER BY published_at DESC
            LIMIT 1;
            "#,
        )
        .await?;

    let rows = hasura_transaction
        .query(
            &query,
            &[
                &Uuid::parse_str(election_event_id)?,
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(election_id)?,
                &published_at,
            ],
        )
        .await?;

    let results = rows
        .into_iter()
        .map(|row| -> Result<BallotPublication> {
            row.try_into()
                .map(|res: BallotPublicationWrapper| -> BallotPublication { res.0 })
        })
        .collect::<Result<Vec<BallotPublication>>>()?;

    Ok(results.get(0).cloned())
}

#[instrument(skip(hasura_transaction), err)]
pub async fn get_previous_publication(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    published_at: Option<DateTime<Local>>,
) -> Result<Option<BallotPublication>> {
    let query = hasura_transaction
        .prepare(
            r#"
            SELECT
                id,
                tenant_id,
                election_event_id,
                labels,
                annotations,
                created_at,
                deleted_at,
                created_by_user_id,
                is_generated,
                election_ids,
                published_at,
                election_id
            FROM sequent_backend.ballot_publication
            WHERE election_event_id = $1
              AND tenant_id = $2
              AND election_id IS NULL
              AND published_at IS NOT NULL
              AND published_at < $3
            ORDER BY published_at DESC
            LIMIT 1;
            "#,
        )
        .await?;

    let rows = hasura_transaction
        .query(
            &query,
            &[
                &Uuid::parse_str(election_event_id)?,
                &Uuid::parse_str(tenant_id)?,
                &published_at,
            ],
        )
        .await?;

    let results = rows
        .into_iter()
        .map(|row| -> Result<BallotPublication> {
            row.try_into()
                .map(|res: BallotPublicationWrapper| -> BallotPublication { res.0 })
        })
        .collect::<Result<Vec<BallotPublication>>>()?;

    Ok(results.get(0).cloned())
}

#[instrument(skip(hasura_transaction), err)]
pub async fn soft_delete_other_ballot_publications(
    hasura_transaction: &Transaction<'_>,
    ballot_publication_id: &str,
    election_event_id: &str,
    tenant_id: &str,
    election_id: Option<String>,
) -> Result<(Vec<String>, Vec<String>)> {
    let ballot_pub_uuid = Uuid::parse_str(ballot_publication_id)?;
    let election_event_uuid = Uuid::parse_str(election_event_id)?;
    let tenant_uuid = Uuid::parse_str(tenant_id)?;

    let election_uuid = election_id
        .map(|id_str| Uuid::parse_str(&id_str))
        .transpose()?;

    let election_id_str = match election_uuid {
        Some(_) => "AND election_id = $4".to_string(),
        None => "".to_string(),
    };

    // Publication update query
    let pub_query_str = format!(
        r#"
        UPDATE sequent_backend.ballot_publication
        SET deleted_at = NOW()
        WHERE id <> $1
          AND election_event_id = $2
          AND tenant_id = $3
          AND deleted_at IS NULL
          {}
        RETURNING id;
        "#,
        election_id_str
    );

    let pub_query = hasura_transaction.prepare(pub_query_str.as_str()).await?;

    let mut params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> =
        vec![&ballot_pub_uuid, &election_event_uuid, &tenant_uuid];
    if let Some(ref election_uuid) = election_uuid {
        params.push(election_uuid);
    }

    // Ballot style update query string.
    let style_query_str = format!(
        r#"
        UPDATE sequent_backend.ballot_style
        SET deleted_at = NOW()
        WHERE ballot_publication_id <> $1
          AND election_event_id = $2
          AND tenant_id = $3
          AND deleted_at IS NULL
          {}
        RETURNING id;
        "#,
        election_id_str
    );

    let style_query = hasura_transaction.prepare(style_query_str.as_str()).await?;

    // Execute both queries in parallel
    let (pub_rows, style_rows) = futures::future::try_join(
        hasura_transaction.query(&pub_query, &params),
        hasura_transaction.query(&style_query, &params),
    )
    .await?;

    let publication_ids: Vec<String> = pub_rows
        .into_iter()
        .map(|row| {
            let id: Uuid = row.try_get("id")?;
            Ok(id.to_string())
        })
        .collect::<Result<Vec<String>>>()?;

    let style_ids: Vec<String> = style_rows
        .into_iter()
        .map(|row| {
            let id: Uuid = row.try_get("id")?;
            Ok(id.to_string())
        })
        .collect::<Result<Vec<String>>>()?;

    Ok((publication_ids, style_ids))
}
