// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The reads and writes of the protected actions' integrations: what their
//! signing requests sign, and the task row a signed action runs under.

use crate::services::serialize_tasks_logs::general_start_log;
use crate::types::tasks::ETasksExecution;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use deadpool_postgres::Transaction;
use sequent_core::signing::SigningRequestStatus;
use sequent_core::types::hasura::extra::TasksExecutionStatus;
use serde_json::Value;
use std::collections::HashMap;
use tracing::instrument;
use uuid::Uuid;

/// Inserts the `tasks_execution` row of a signed action's task, in the
/// caller's transaction, so it goes away with a rolled-back execution.
#[instrument(skip(hasura_transaction), err)]
pub async fn insert_signed_action_task(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    executed_by_user: &str,
    annotations: &Value,
) -> Result<Uuid> {
    let task_type = ETasksExecution::RUN_SIGNED_ACTION;
    let logs = serde_json::to_value(general_start_log())?;
    Ok(hasura_transaction
        .query_one(
            "INSERT INTO sequent_backend.tasks_execution
                 (tenant_id, election_event_id, name, type, execution_status, annotations, logs,
                  executed_by_user)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             RETURNING id",
            &[
                &tenant_id,
                &election_event_id,
                &task_type.to_name(),
                &task_type.to_string(),
                &TasksExecutionStatus::IN_PROGRESS.to_string(),
                annotations,
                &logs,
                &executed_by_user,
            ],
        )
        .await
        .context("Error recording the signed action's task")?
        .try_get(0)?)
}

/// Marks a signed action's task row.
#[instrument(skip(hasura_transaction), err)]
pub async fn set_signed_action_task_status(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    task_execution_id: Uuid,
    status: TasksExecutionStatus,
) -> Result<()> {
    hasura_transaction
        .execute(
            "UPDATE sequent_backend.tasks_execution
             SET execution_status = $3,
                 end_at = CASE WHEN $3 <> 'IN_PROGRESS' THEN now() ELSE end_at END
             WHERE tenant_id = $1 AND id = $2",
            &[&tenant_id, &task_execution_id, &status.to_string()],
        )
        .await
        .context("Error marking the signed action's task")?;
    Ok(())
}

/// The Post's published ballot publication, the newest first: one of the
/// Post itself or one of the event that includes it.
#[instrument(skip(hasura_transaction), err)]
pub async fn published_publication_of_post(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    election_id: Uuid,
) -> Result<Option<Uuid>> {
    Ok(hasura_transaction
        .query_opt(
            "SELECT id FROM sequent_backend.ballot_publication
             WHERE tenant_id = $1 AND election_event_id = $2
                 AND published_at IS NOT NULL AND deleted_at IS NULL
                 AND (election_id = $3 OR $3 = ANY(election_ids))
             ORDER BY published_at DESC, id
             LIMIT 1",
            &[&tenant_id, &election_event_id, &election_id],
        )
        .await
        .context("Error reading the Post's publication")?
        .map(|row| row.try_get(0))
        .transpose()?)
}

/// When the last publication of the same target (the event, or the Post
/// of an election-level publication) was published, if any was.
#[instrument(skip(hasura_transaction), err)]
pub async fn last_published_at(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    election_id: Option<Uuid>,
) -> Result<Option<DateTime<Utc>>> {
    Ok(hasura_transaction
        .query_one(
            "SELECT max(published_at) FROM sequent_backend.ballot_publication
             WHERE tenant_id = $1 AND election_event_id = $2
                 AND published_at IS NOT NULL
                 AND election_id IS NOT DISTINCT FROM $3",
            &[&tenant_id, &election_event_id, &election_id],
        )
        .await
        .context("Error reading the last publication")?
        .try_get(0)?)
}

/// How many active scheduled events of the event were created after
/// `since` (all of them without one).
#[instrument(skip(hasura_transaction), err)]
pub async fn count_scheduled_events_since(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    since: Option<DateTime<Utc>>,
) -> Result<i64> {
    Ok(hasura_transaction
        .query_one(
            "SELECT count(*) FROM sequent_backend.scheduled_event
             WHERE tenant_id = $1 AND election_event_id = $2
                 AND stopped_at IS NULL AND archived_at IS NULL
                 AND ($3::timestamptz IS NULL OR created_at > $3)",
            &[&tenant_id, &election_event_id, &since],
        )
        .await
        .context("Error counting the scheduled events")?
        .try_get(0)?)
}

/// What each action's signing rule was before its first change logged
/// after `since` (every logged change without one): the `old` rule of that
/// SigningRuleChanged entry, by action id.
#[instrument(skip(hasura_transaction), err)]
pub async fn signing_rules_before(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    since: Option<DateTime<Utc>>,
) -> Result<HashMap<String, Value>> {
    Ok(hasura_transaction
        .query(
            "SELECT DISTINCT ON (body->'details'->>'action')
                 body->'details'->>'action', body->'details'->'old'
             FROM sequent_backend.signing_log_outbox
             WHERE tenant_id = $1 AND election_event_id = $2
                 AND statement_kind = 'SigningRuleChanged' AND entry = 0
                 AND ($3::timestamptz IS NULL OR occurred_at > $3)
                 AND body->'details' ? 'old'
             ORDER BY body->'details'->>'action', id",
            &[&tenant_id, &election_event_id, &since],
        )
        .await
        .context("Error reading the signing rules' changes")?
        .into_iter()
        .map(|row| Ok((row.try_get(0)?, row.try_get(1)?)))
        .collect::<Result<_>>()?)
}

/// What a voter approval signs about an application.
#[derive(Debug, Clone, PartialEq)]
pub struct ApplicationForSigning {
    pub id: Uuid,
    pub area_id: Option<Uuid>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub annotations: Option<Value>,
    pub permission_label: Option<String>,
}

#[instrument(skip(hasura_transaction), err)]
pub async fn get_application_for_signing(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    application_id: Uuid,
) -> Result<Option<ApplicationForSigning>> {
    hasura_transaction
        .query_opt(
            "SELECT id, area_id, status, created_at, annotations, permission_label
             FROM sequent_backend.applications
             WHERE tenant_id = $1 AND election_event_id = $2 AND id = $3",
            &[&tenant_id, &election_event_id, &application_id],
        )
        .await
        .context("Error reading the application")?
        .map(|row| {
            Ok(ApplicationForSigning {
                id: row.try_get("id")?,
                area_id: row.try_get("area_id")?,
                status: row.try_get("status")?,
                created_at: row.try_get("created_at")?,
                annotations: row.try_get("annotations")?,
                permission_label: row.try_get("permission_label")?,
            })
        })
        .transpose()
}

/// A Keycloak user's last name, first name and username, from the realm's
/// tables.
#[instrument(skip(keycloak_transaction), err)]
pub async fn registry_user_names(
    keycloak_transaction: &Transaction<'_>,
    realm: &str,
    user_id: &str,
) -> Result<Option<(Option<String>, Option<String>, Option<String>)>> {
    keycloak_transaction
        .query_opt(
            "SELECT u.last_name, u.first_name, u.username
             FROM user_entity u JOIN realm r ON r.id = u.realm_id
             WHERE r.name = $1 AND u.id = $2",
            &[&realm, &user_id],
        )
        .await
        .context("Error reading the registry record")?
        .map(|row| Ok((row.try_get(0)?, row.try_get(1)?, row.try_get(2)?)))
        .transpose()
}

/// Keeps the result of a signed action's effect on its request, in the
/// effect's transaction: once it commits, the effect ran, and a later copy
/// of the task only reports it.
#[instrument(skip(hasura_transaction, result), err)]
pub async fn set_signing_effect_result(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    request_id: Uuid,
    result: &Value,
) -> Result<()> {
    let updated = hasura_transaction
        .execute(
            "UPDATE sequent_backend.signing_request SET execution_result = $4
             WHERE tenant_id = $1 AND election_event_id = $2 AND id = $3
                 AND status = $5 AND executed_at IS NULL",
            &[
                &tenant_id,
                &election_event_id,
                &request_id,
                result,
                &SigningRequestStatus::Completed.to_string(),
            ],
        )
        .await
        .context("Error keeping the signed action's result")?;
    if updated != 1 {
        anyhow::bail!("signing request {request_id} is no longer completed");
    }
    Ok(())
}

/// How many event-level publications the event ever published, soft-deleted
/// ones included: its configuration version.
#[instrument(skip(hasura_transaction), err)]
pub async fn count_published_configuration_versions(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
) -> Result<i64> {
    Ok(hasura_transaction
        .query_one(
            "SELECT count(*) FROM sequent_backend.ballot_publication
             WHERE tenant_id = $1 AND election_event_id = $2 AND election_id IS NULL
                 AND published_at IS NOT NULL",
            &[&tenant_id, &election_event_id],
        )
        .await
        .context("Error counting the configuration versions")?
        .try_get(0)?)
}

/// The elections whose contests an area takes part in, with whether each
/// carries `label`, by id.
#[instrument(skip(hasura_transaction), err)]
pub async fn posts_of_area(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    area_id: Uuid,
    label: Option<&str>,
) -> Result<Vec<(Uuid, bool)>> {
    hasura_transaction
        .query(
            "SELECT DISTINCT el.id, (el.permission_label IS NOT DISTINCT FROM $4) AS labelled
             FROM sequent_backend.area_contest ac
                 JOIN sequent_backend.contest con
                     ON con.id = ac.contest_id AND con.tenant_id = ac.tenant_id
                     AND con.election_event_id = ac.election_event_id
                 JOIN sequent_backend.election el
                     ON el.id = con.election_id AND el.tenant_id = con.tenant_id
                     AND el.election_event_id = con.election_event_id
             WHERE ac.tenant_id = $1 AND ac.election_event_id = $2 AND ac.area_id = $3
             ORDER BY el.id",
            &[&tenant_id, &election_event_id, &area_id, &label],
        )
        .await
        .context("Error reading the area's Posts")?
        .into_iter()
        .map(|row| Ok((row.try_get(0)?, row.try_get(1)?)))
        .collect()
}

/// Adds `annotations` to an application's annotations, keeping the others.
#[instrument(skip(hasura_transaction, annotations), err)]
pub async fn annotate_application(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    application_id: Uuid,
    annotations: &Value,
) -> Result<()> {
    hasura_transaction
        .execute(
            "UPDATE sequent_backend.applications
             SET annotations = COALESCE(annotations, '{}'::jsonb) || $4::jsonb
             WHERE tenant_id = $1 AND election_event_id = $2 AND id = $3",
            &[&tenant_id, &election_event_id, &application_id, annotations],
        )
        .await
        .context("Error annotating the application")?;
    Ok(())
}
