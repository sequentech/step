// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Result};
use deadpool_postgres::Transaction;
use sequent_core::services::uuid_validation::parse_uuid_v4;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio_postgres::row::Row;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TallyResultsPublication {
    pub id: String,
    pub tenant_id: String,
    pub election_event_id: String,
    pub tally_session_id: String,
    pub tally_session_execution_id: String,
    pub results_event_id: String,
    pub task_execution_id: Option<String>,
    pub route_scope: String,
    pub route_election_id: Option<String>,
    pub election_ids: Vec<String>,
    pub access: String,
    pub visibility_scope: String,
    pub published_contest_ids: Value,
    pub contest_publication_state: Value,
    pub documents: Value,
    pub manifest: Option<Value>,
    pub publication_status: String,
    pub version: i32,
    pub error_message: Option<String>,
    pub published_by_user_id: Option<String>,
}

impl TryFrom<Row> for TallyResultsPublication {
    type Error = anyhow::Error;

    fn try_from(row: Row) -> Result<Self> {
        Ok(TallyResultsPublication {
            id: row.try_get::<_, Uuid>("id")?.to_string(),
            tenant_id: row.try_get::<_, Uuid>("tenant_id")?.to_string(),
            election_event_id: row.try_get::<_, Uuid>("election_event_id")?.to_string(),
            tally_session_id: row.try_get::<_, Uuid>("tally_session_id")?.to_string(),
            tally_session_execution_id: row
                .try_get::<_, Uuid>("tally_session_execution_id")?
                .to_string(),
            results_event_id: row.try_get::<_, Uuid>("results_event_id")?.to_string(),
            task_execution_id: row
                .try_get::<_, Option<Uuid>>("task_execution_id")?
                .map(|id| id.to_string()),
            route_scope: row.try_get("route_scope")?,
            route_election_id: row
                .try_get::<_, Option<Uuid>>("route_election_id")?
                .map(|id| id.to_string()),
            election_ids: row
                .try_get::<_, Vec<Uuid>>("election_ids")?
                .iter()
                .map(|id| id.to_string())
                .collect(),
            access: row.try_get("access")?,
            visibility_scope: row.try_get("visibility_scope")?,
            published_contest_ids: row.try_get("published_contest_ids")?,
            contest_publication_state: row.try_get("contest_publication_state")?,
            documents: row.try_get("documents")?,
            manifest: row.try_get("manifest")?,
            publication_status: row.try_get("publication_status")?,
            version: row.try_get("version")?,
            error_message: row.try_get("error_message")?,
            published_by_user_id: row
                .try_get::<_, Option<Uuid>>("published_by_user_id")?
                .map(|id| id.to_string()),
        })
    }
}

pub async fn next_publication_version(
    tx: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    route_scope: &str,
    route_election_id: Option<&str>,
) -> Result<i32> {
    let route_election_uuid = route_election_id.map(parse_uuid_v4).transpose()?;
    let statement = tx
        .prepare(
            r#"
                SELECT COALESCE(MAX(version), 0) + 1 AS version
                FROM sequent_backend.tally_results_publication
                WHERE tenant_id = $1
                  AND election_event_id = $2
                  AND route_scope = $3
                  AND (
                    ($4::uuid IS NULL AND route_election_id IS NULL)
                    OR route_election_id = $4
                  );
            "#,
        )
        .await?;
    let row = tx
        .query_one(
            &statement,
            &[
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
                &route_scope,
                &route_election_uuid,
            ],
        )
        .await?;

    Ok(row.try_get("version")?)
}

#[allow(clippy::too_many_arguments)]
pub async fn insert_publishing_publication(
    tx: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    tally_session_id: &str,
    tally_session_execution_id: &str,
    results_event_id: &str,
    task_execution_id: &str,
    route_scope: &str,
    route_election_id: Option<&str>,
    election_ids: &[String],
    access: &str,
    visibility_scope: &str,
    contest_ids: &[String],
    published_by_user_id: Option<&str>,
) -> Result<TallyResultsPublication> {
    let version = next_publication_version(
        tx,
        tenant_id,
        election_event_id,
        route_scope,
        route_election_id,
    )
    .await?;
    let election_uuids = election_ids
        .iter()
        .map(|id| parse_uuid_v4(id))
        .collect::<std::result::Result<Vec<Uuid>, _>>()?;
    let contest_state = contest_ids
        .iter()
        .map(|id| (id.clone(), Value::String("published".to_string())))
        .collect::<serde_json::Map<String, Value>>();
    let route_election_uuid = route_election_id.map(parse_uuid_v4).transpose()?;
    let published_by_uuid = published_by_user_id.map(parse_uuid_v4).transpose()?;
    let contest_ids_value = serde_json::to_value(contest_ids)?;
    let contest_state_value = Value::Object(contest_state);

    let statement = tx
        .prepare(
            r#"
                INSERT INTO sequent_backend.tally_results_publication (
                    tenant_id,
                    election_event_id,
                    tally_session_id,
                    tally_session_execution_id,
                    results_event_id,
                    task_execution_id,
                    route_scope,
                    route_election_id,
                    election_ids,
                    access,
                    visibility_scope,
                    published_contest_ids,
                    contest_publication_state,
                    documents,
                    publication_status,
                    version,
                    published_by_user_id
                )
                VALUES (
                    $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
                    $11, $12, $13, '{}'::jsonb, 'Publishing', $14, $15
                )
                RETURNING *;
            "#,
        )
        .await?;
    let rows = tx
        .query(
            &statement,
            &[
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
                &parse_uuid_v4(tally_session_id)?,
                &parse_uuid_v4(tally_session_execution_id)?,
                &parse_uuid_v4(results_event_id)?,
                &parse_uuid_v4(task_execution_id)?,
                &route_scope,
                &route_election_uuid,
                &election_uuids,
                &access,
                &visibility_scope,
                &contest_ids_value,
                &contest_state_value,
                &version,
                &published_by_uuid,
            ],
        )
        .await?;

    rows.into_iter()
        .next()
        .ok_or_else(|| anyhow!("No publication row returned"))?
        .try_into()
}

pub async fn get_publication_by_id(
    tx: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    publication_id: &str,
) -> Result<TallyResultsPublication> {
    let statement = tx
        .prepare(
            r#"
                SELECT *
                FROM sequent_backend.tally_results_publication
                WHERE tenant_id = $1
                  AND election_event_id = $2
                  AND id = $3;
            "#,
        )
        .await?;
    let rows = tx
        .query(
            &statement,
            &[
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
                &parse_uuid_v4(publication_id)?,
            ],
        )
        .await?;

    rows.into_iter()
        .next()
        .ok_or_else(|| anyhow!("Publication not found"))?
        .try_into()
}

pub async fn get_active_publication_for_route(
    tx: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    route_scope: &str,
    route_election_id: Option<&str>,
) -> Result<Option<TallyResultsPublication>> {
    let route_election_uuid = route_election_id.map(parse_uuid_v4).transpose()?;
    let statement = tx
        .prepare(
            r#"
                SELECT *
                FROM sequent_backend.tally_results_publication
                WHERE tenant_id = $1
                  AND election_event_id = $2
                  AND route_scope = $3
                  AND publication_status = 'Published'
                  AND revoked_at IS NULL
                  AND (
                    ($4::uuid IS NULL AND route_election_id IS NULL)
                    OR route_election_id = $4
                  )
                ORDER BY version DESC
                LIMIT 1;
            "#,
        )
        .await?;
    let rows = tx
        .query(
            &statement,
            &[
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
                &route_scope,
                &route_election_uuid,
            ],
        )
        .await?;

    rows.into_iter().next().map(TryInto::try_into).transpose()
}

pub async fn list_active_public_publications(
    tx: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<Vec<TallyResultsPublication>> {
    let statement = tx
        .prepare(
            r#"
                SELECT *
                FROM sequent_backend.tally_results_publication
                WHERE tenant_id = $1
                  AND election_event_id = $2
                  AND publication_status = 'Published'
                  AND revoked_at IS NULL
                ORDER BY route_scope, route_election_id NULLS FIRST, version DESC;
            "#,
        )
        .await?;
    let rows = tx
        .query(
            &statement,
            &[
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
            ],
        )
        .await?;

    rows.into_iter().map(TryInto::try_into).collect()
}

pub async fn mark_publication_published(
    tx: &Transaction<'_>,
    publication: &TallyResultsPublication,
    documents: Value,
    manifest: Value,
) -> Result<()> {
    let route_election_uuid = publication
        .route_election_id
        .as_deref()
        .map(parse_uuid_v4)
        .transpose()?;

    let supersede_statement = tx
        .prepare(
            r#"
                UPDATE sequent_backend.tally_results_publication
                SET publication_status = 'Superseded',
                    updated_at = now()
                WHERE tenant_id = $1
                  AND election_event_id = $2
                  AND id <> $3
                  AND route_scope = $4
                  AND publication_status = 'Published'
                  AND revoked_at IS NULL
                  AND (
                    ($5::uuid IS NULL AND route_election_id IS NULL)
                    OR route_election_id = $5
                  );
            "#,
        )
        .await?;
    tx.execute(
        &supersede_statement,
        &[
            &parse_uuid_v4(&publication.tenant_id)?,
            &parse_uuid_v4(&publication.election_event_id)?,
            &parse_uuid_v4(&publication.id)?,
            &publication.route_scope,
            &route_election_uuid,
        ],
    )
    .await?;

    let publish_statement = tx
        .prepare(
            r#"
                UPDATE sequent_backend.tally_results_publication
                SET publication_status = 'Published',
                    documents = $4,
                    manifest = $5,
                    published_at = now(),
                    updated_at = now()
                WHERE tenant_id = $1
                  AND election_event_id = $2
                  AND id = $3;
            "#,
        )
        .await?;
    tx.execute(
        &publish_statement,
        &[
            &parse_uuid_v4(&publication.tenant_id)?,
            &parse_uuid_v4(&publication.election_event_id)?,
            &parse_uuid_v4(&publication.id)?,
            &documents,
            &manifest,
        ],
    )
    .await?;

    Ok(())
}

pub async fn mark_publication_failed(
    tx: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    publication_id: &str,
    error_message: &str,
) -> Result<()> {
    let statement = tx
        .prepare(
            r#"
                UPDATE sequent_backend.tally_results_publication
                SET publication_status = 'Failed',
                    error_message = $4,
                    updated_at = now()
                WHERE tenant_id = $1
                  AND election_event_id = $2
                  AND id = $3;
            "#,
        )
        .await?;
    tx.execute(
        &statement,
        &[
            &parse_uuid_v4(tenant_id)?,
            &parse_uuid_v4(election_event_id)?,
            &parse_uuid_v4(publication_id)?,
            &error_message,
        ],
    )
    .await?;

    Ok(())
}

pub async fn mark_publication_superseded(
    tx: &Transaction<'_>,
    publication: &TallyResultsPublication,
) -> Result<()> {
    let statement = tx
        .prepare(
            r#"
                UPDATE sequent_backend.tally_results_publication
                SET publication_status = 'Superseded',
                    updated_at = now()
                WHERE tenant_id = $1
                  AND election_event_id = $2
                  AND id = $3
                  AND publication_status = 'Published';
            "#,
        )
        .await?;
    tx.execute(
        &statement,
        &[
            &parse_uuid_v4(&publication.tenant_id)?,
            &parse_uuid_v4(&publication.election_event_id)?,
            &parse_uuid_v4(&publication.id)?,
        ],
    )
    .await?;

    Ok(())
}

pub async fn revoke_publication(
    tx: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    publication_id: &str,
) -> Result<()> {
    let statement = tx
        .prepare(
            r#"
                UPDATE sequent_backend.tally_results_publication
                SET publication_status = 'Revoked',
                    revoked_at = now(),
                    updated_at = now()
                WHERE tenant_id = $1
                  AND election_event_id = $2
                  AND id = $3
                  AND publication_status = 'Published';
            "#,
        )
        .await?;
    let affected_rows = tx
        .execute(
            &statement,
            &[
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
                &parse_uuid_v4(publication_id)?,
            ],
        )
        .await?;

    if affected_rows == 0 {
        return Err(anyhow!("Publication not found or not currently published"));
    }

    Ok(())
}
