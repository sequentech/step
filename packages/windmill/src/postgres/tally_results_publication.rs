// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::types::results_publication::{
    ContestPublicationState, ResultsPublicationStatus, ResultsRouteScope,
};
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::ballot::{ResultsWebsiteAccess, ResultsWebsiteVisibilityScope};
use sequent_core::services::uuid_validation::parse_uuid_v4;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
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
    pub route_scope: ResultsRouteScope,
    pub route_election_id: Option<String>,
    pub election_ids: Vec<String>,
    pub access: ResultsWebsiteAccess,
    pub visibility_scope: ResultsWebsiteVisibilityScope,
    pub published_contest_ids: Vec<String>,
    pub contest_publication_state: HashMap<String, ContestPublicationState>,
    pub documents: Value,
    pub manifest: Option<Value>,
    pub publication_status: ResultsPublicationStatus,
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
            route_scope: row
                .try_get::<_, String>("route_scope")?
                .parse()
                .context("Invalid tally results publication route_scope")?,
            route_election_id: row
                .try_get::<_, Option<Uuid>>("route_election_id")?
                .map(|id| id.to_string()),
            election_ids: row
                .try_get::<_, Vec<Uuid>>("election_ids")?
                .iter()
                .map(|id| id.to_string())
                .collect(),
            access: row
                .try_get::<_, String>("access")?
                .parse()
                .context("Invalid tally results publication access")?,
            visibility_scope: row
                .try_get::<_, String>("visibility_scope")?
                .parse()
                .context("Invalid tally results publication visibility_scope")?,
            published_contest_ids: serde_json::from_value(row.try_get("published_contest_ids")?)?,
            contest_publication_state: serde_json::from_value(
                row.try_get("contest_publication_state")?,
            )?,
            documents: row.try_get("documents")?,
            manifest: row.try_get("manifest")?,
            publication_status: row
                .try_get::<_, String>("publication_status")?
                .parse()
                .context("Invalid tally results publication status")?,
            version: row.try_get("version")?,
            error_message: row.try_get("error_message")?,
            published_by_user_id: row
                .try_get::<_, Option<Uuid>>("published_by_user_id")?
                .map(|id| id.to_string()),
        })
    }
}

pub async fn validate_new_publication_source(
    tx: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    tally_session_id: &str,
    tally_session_execution_id: &str,
    results_event_id: &str,
    election_ids: &[String],
    contest_ids: &[String],
    route_election_id: Option<&str>,
) -> Result<()> {
    if election_ids.is_empty() || contest_ids.is_empty() {
        return Err(anyhow!(
            "A publication requires at least one election and one contest"
        ));
    }

    let tenant_id = parse_uuid_v4(tenant_id)?;
    let election_event_id = parse_uuid_v4(election_event_id)?;
    let tally_session_id = parse_uuid_v4(tally_session_id)?;
    let tally_session_execution_id = parse_uuid_v4(tally_session_execution_id)?;
    let results_event_id = parse_uuid_v4(results_event_id)?;
    let election_ids = election_ids
        .iter()
        .map(|id| parse_uuid_v4(id))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let contest_ids = contest_ids
        .iter()
        .map(|id| parse_uuid_v4(id))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let route_election_id = route_election_id.map(parse_uuid_v4).transpose()?;

    if route_election_id.is_some_and(|route_id| !election_ids.contains(&route_id)) {
        return Err(anyhow!(
            "The route election must be included in the publication elections"
        ));
    }

    let statement = tx
        .prepare(
            r#"
                SELECT
                    EXISTS (
                        SELECT 1
                        FROM sequent_backend.tally_session_execution execution
                        JOIN sequent_backend.tally_session session
                          ON session.id = execution.tally_session_id
                         AND session.tenant_id = execution.tenant_id
                         AND session.election_event_id = execution.election_event_id
                        JOIN sequent_backend.results_event results_event
                          ON results_event.id = execution.results_event_id
                         AND results_event.tenant_id = execution.tenant_id
                         AND results_event.election_event_id = execution.election_event_id
                        WHERE execution.id = $4
                          AND execution.tenant_id = $1
                          AND execution.election_event_id = $2
                          AND execution.tally_session_id = $3
                          AND execution.results_event_id = $5
                          AND $6::uuid[] <@ session.election_ids
                    ) AS valid_execution,
                    (
                        SELECT COUNT(DISTINCT election.id)
                        FROM sequent_backend.election election
                        WHERE election.tenant_id = $1
                          AND election.election_event_id = $2
                          AND election.id = ANY($6::uuid[])
                    ) AS election_count,
                    (
                        SELECT COUNT(DISTINCT contest.id)
                        FROM sequent_backend.contest contest
                        WHERE contest.tenant_id = $1
                          AND contest.election_event_id = $2
                          AND contest.election_id = ANY($6::uuid[])
                          AND contest.id = ANY($7::uuid[])
                    ) AS contest_count,
                    (
                        SELECT COUNT(DISTINCT results.contest_id)
                        FROM sequent_backend.results_contest results
                        WHERE results.tenant_id = $1
                          AND results.election_event_id = $2
                          AND results.results_event_id = $5
                          AND results.election_id = ANY($6::uuid[])
                          AND results.contest_id = ANY($7::uuid[])
                    ) AS tallied_contest_count;
            "#,
        )
        .await?;
    let row = tx
        .query_one(
            &statement,
            &[
                &tenant_id,
                &election_event_id,
                &tally_session_id,
                &tally_session_execution_id,
                &results_event_id,
                &election_ids,
                &contest_ids,
            ],
        )
        .await?;

    let valid_execution: bool = row.try_get("valid_execution")?;
    let election_count: i64 = row.try_get("election_count")?;
    let contest_count: i64 = row.try_get("contest_count")?;
    let tallied_contest_count: i64 = row.try_get("tallied_contest_count")?;
    let expected_elections = i64::try_from(election_ids.len())?;
    let expected_contests = i64::try_from(contest_ids.len())?;

    if !valid_execution {
        return Err(anyhow!(
            "The tally session, execution, and results event do not belong together"
        ));
    }
    if election_count != expected_elections {
        return Err(anyhow!(
            "One or more publication elections are outside the tally event"
        ));
    }
    if contest_count != expected_contests {
        return Err(anyhow!(
            "One or more publication contests are outside the selected elections"
        ));
    }
    if tallied_contest_count != expected_contests {
        return Err(anyhow!(
            "Every selected contest must have results in the selected tally execution"
        ));
    }

    Ok(())
}

pub async fn next_publication_version(
    tx: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    route_scope: ResultsRouteScope,
    route_election_id: Option<&str>,
) -> Result<i32> {
    let route_election_uuid = route_election_id.map(parse_uuid_v4).transpose()?;
    let lock_key = format!(
        "tally-results-publication:{tenant_id}:{election_event_id}:{}:{}",
        route_scope.as_ref(),
        route_election_id.unwrap_or("event")
    );
    tx.query_one(
        "SELECT pg_advisory_xact_lock(hashtextextended($1, 0))",
        &[&lock_key],
    )
    .await?;
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
                &route_scope.as_ref(),
                &route_election_uuid,
            ],
        )
        .await?;

    Ok(row.try_get("version")?)
}

pub struct NewTallyResultsPublication<'a> {
    pub tenant_id: &'a str,
    pub election_event_id: &'a str,
    pub tally_session_id: &'a str,
    pub tally_session_execution_id: &'a str,
    pub results_event_id: &'a str,
    pub task_execution_id: &'a str,
    pub route_scope: ResultsRouteScope,
    pub route_election_id: Option<&'a str>,
    pub election_ids: &'a [String],
    pub access: ResultsWebsiteAccess,
    pub visibility_scope: ResultsWebsiteVisibilityScope,
    pub contest_ids: &'a [String],
    pub published_by_user_id: Option<&'a str>,
}

pub async fn insert_publishing_publication(
    tx: &Transaction<'_>,
    publication: NewTallyResultsPublication<'_>,
) -> Result<TallyResultsPublication> {
    let version = next_publication_version(
        tx,
        publication.tenant_id,
        publication.election_event_id,
        publication.route_scope,
        publication.route_election_id,
    )
    .await?;
    let election_uuids = publication
        .election_ids
        .iter()
        .map(|id| parse_uuid_v4(id))
        .collect::<std::result::Result<Vec<Uuid>, _>>()?;
    let contest_state = publication
        .contest_ids
        .iter()
        .map(|id| (id.clone(), ContestPublicationState::Published))
        .collect::<HashMap<_, _>>();
    let route_election_uuid = publication
        .route_election_id
        .map(parse_uuid_v4)
        .transpose()?;
    let published_by_uuid = publication
        .published_by_user_id
        .map(parse_uuid_v4)
        .transpose()?;
    let contest_ids_value = serde_json::to_value(publication.contest_ids)?;
    let contest_state_value = serde_json::to_value(contest_state)?;

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
                &parse_uuid_v4(publication.tenant_id)?,
                &parse_uuid_v4(publication.election_event_id)?,
                &parse_uuid_v4(publication.tally_session_id)?,
                &parse_uuid_v4(publication.tally_session_execution_id)?,
                &parse_uuid_v4(publication.results_event_id)?,
                &parse_uuid_v4(publication.task_execution_id)?,
                &publication.route_scope.as_ref(),
                &route_election_uuid,
                &election_uuids,
                &publication.access.to_string(),
                &publication.visibility_scope.to_string(),
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
    route_scope: ResultsRouteScope,
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
                &route_scope.as_ref(),
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

pub async fn list_superseded_publications(
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
                  AND publication_status = 'Superseded'
                ORDER BY updated_at;
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
            &publication.route_scope.as_ref(),
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
                    error_message = NULL,
                    published_at = now(),
                    updated_at = now()
                WHERE tenant_id = $1
                  AND election_event_id = $2
                  AND id = $3
                  AND publication_status IN ('Publishing', 'Failed');
            "#,
        )
        .await?;
    let affected_rows = tx
        .execute(
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

    if affected_rows != 1 {
        return Err(anyhow!(
            "Publication is not in a state that can be activated"
        ));
    }

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
                  AND id = $3
                  AND publication_status IN ('Publishing', 'Failed');
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
                &error_message,
            ],
        )
        .await?;

    if affected_rows != 1 {
        return Err(anyhow!(
            "Publication is not in a state that can be marked failed"
        ));
    }

    Ok(())
}

pub async fn set_publication_finalization_error(
    tx: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    publication_id: &str,
    error_message: Option<&str>,
) -> Result<()> {
    let statement = tx
        .prepare(
            r#"
                UPDATE sequent_backend.tally_results_publication
                SET error_message = $4,
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
                &error_message,
            ],
        )
        .await?;

    if affected_rows != 1 {
        return Err(anyhow!(
            "Published publication was not available to update its finalization error"
        ));
    }

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
