// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::{
    authorize, authorize_election_permission_labels,
};
use anyhow::Result;
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt;
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tracing::instrument;
use uuid::Uuid;
use windmill::services::audit_log::AuditActor;
use windmill::services::audit_log_export::{
    AuditExportFormat, AuditExportRequest, AuditExportScope,
};
use windmill::services::celery_app::get_celery_app;
use windmill::services::database::get_hasura_pool;
use windmill::services::reports::activity_log::{
    ActivityLogExportOptions, ReportFormat,
};
use windmill::services::tasks_execution::*;
use windmill::tasks::export_audit_logs::export_audit_logs;
use windmill::types::tasks::ETasksExecution;

#[derive(Serialize, Deserialize, Debug)]
pub struct ExportElectionEventInput {
    election_event_id: String,
    format: String,
    /// The first minute to export (RFC 3339 instant), inclusive.
    #[serde(default)]
    created_from: Option<String>,
    /// The last minute to export (RFC 3339 instant), inclusive.
    #[serde(default)]
    created_to: Option<String>,
    /// The IANA zone the times are shown in; the log zone when absent.
    #[serde(default)]
    time_zone: Option<String>,
    /// `ALL` (also when absent), `ELECTION`, `AREA` or `ACCOUNT`.
    #[serde(default)]
    scope: Option<String>,
    #[serde(default)]
    election_id: Option<String>,
    #[serde(default)]
    area_id: Option<String>,
    #[serde(default)]
    username: Option<String>,
}

/// The format of the exports that carry each entry's signatures.
const SQL_FORMAT: &str = "SQL";

impl ExportElectionEventInput {
    /// The scoped or SQL export the request asks for, which the audit export
    /// writes; `None` for the whole log as CSV or PDF, which the activity
    /// logs report writes.
    pub fn audit_export(
        &self,
        claims: &jwt::JwtClaims,
    ) -> Result<Option<AuditExportRequest>, (Status, String)> {
        let bad_request = |error: String| (Status::BadRequest, error);
        let scope = match self
            .scope
            .as_deref()
            .map(str::trim)
            .filter(|scope| !scope.is_empty())
        {
            Some(scope) => AuditExportScope::from_str(scope)
                .map_err(|_| bad_request(format!("Unknown scope {scope:?}")))?,
            None => AuditExportScope::All,
        };
        let sql = self.format.eq_ignore_ascii_case(SQL_FORMAT);
        if scope == AuditExportScope::All && !sql {
            return Ok(None);
        }
        let request = AuditExportRequest {
            tenant_id: claims.hasura_claims.tenant_id.clone(),
            election_event_id: Some(self.election_event_id.clone()),
            format: AuditExportFormat::from_str(&self.format.to_uppercase())
                .map_err(|_| {
                    bad_request(format!("Unknown format {:?}", self.format))
                })?,
            scope,
            username: self.username.clone(),
            election_id: self.election_id.clone(),
            area_id: self.area_id.clone(),
            created_from: self.created_from.clone(),
            created_to: self.created_to.clone(),
            time_zone: self.time_zone.clone(),
            requested_by: AuditActor {
                user_id: Some(claims.hasura_claims.user_id.clone()),
                username: claims.preferred_username.clone(),
                role: None,
            },
        };
        request
            .validate()
            .map_err(|error| bad_request(format!("{error:#}")))?;
        Ok(Some(request))
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ExportElectionEventOutput {
    document_id: String,
    task_execution: TasksExecution,
}

/// Queues a scoped or SQL export of the event's log behind a task row.
async fn queue_audit_export(
    claims: &jwt::JwtClaims,
    request: AuditExportRequest,
) -> Result<ExportElectionEventOutput, (Status, String)> {
    let executer_name = claims
        .name
        .clone()
        .unwrap_or_else(|| claims.hasura_claims.user_id.clone());
    let task_execution = post(
        &request.tenant_id,
        request.election_event_id.as_deref(),
        ETasksExecution::EXPORT_ACTIVITY_LOGS_REPORT,
        &executer_name,
    )
    .await
    .map_err(|error| {
        (
            Status::InternalServerError,
            format!("Failed to insert task execution record: {error:?}"),
        )
    })?;
    let document_id = Uuid::new_v4().to_string();
    get_celery_app()
        .await
        .send_task(export_audit_logs::new(
            request,
            document_id.clone(),
            task_execution.clone(),
        ))
        .await
        .map_err(|error| {
            (
                Status::InternalServerError,
                format!("Error sending the logs export task: {error:?}"),
            )
        })?;
    Ok(ExportElectionEventOutput {
        document_id,
        task_execution,
    })
}

#[instrument(skip(claims))]
#[post("/export-election-event-logs", format = "json", data = "<input>")]
pub async fn export_election_event_logs_route(
    claims: jwt::JwtClaims,
    input: Json<ExportElectionEventInput>,
) -> Result<Json<ExportElectionEventOutput>, (Status, String)> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::LOGS_EXPORT],
    )?;

    let body = input.into_inner();
    {
        let mut hasura_db_client: DbClient = get_hasura_pool()
            .await
            .get()
            .await
            .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;
        let hasura_transaction = hasura_db_client
            .transaction()
            .await
            .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;
        authorize_election_permission_labels(
            &hasura_transaction,
            &claims,
            &body.election_event_id,
            None,
        )
        .await?;
    }

    if let Some(request) = body.audit_export(&claims)? {
        return queue_audit_export(&claims, request).await.map(Json);
    }

    info!("Format: {}", &body.format);
    let report_fmt = ReportFormat::from_str(&body.format).map_err(|error| {
        (
            Status::InternalServerError,
            format!("Error sending export_election_event task: {error:?}"),
        )
    })?;
    info!("{:?}", report_fmt);
    let options = ActivityLogExportOptions {
        created_from: body.created_from.clone(),
        created_to: body.created_to.clone(),
        time_zone: body.time_zone.clone(),
    };
    // Refuse a bad range or zone now rather than in the task.
    options
        .range()
        .and_then(|_| options.zone())
        .map_err(|error| (Status::BadRequest, format!("{error:?}")))?;
    let tenant_id = claims.hasura_claims.tenant_id.clone();
    let election_event_id = body.election_event_id.clone();

    let executer_name = claims
        .name
        .clone()
        .unwrap_or_else(|| claims.hasura_claims.user_id.clone());

    // Insert the task execution record
    let task_execution = post(
        &tenant_id,
        Some(&election_event_id),
        ETasksExecution::EXPORT_ACTIVITY_LOGS_REPORT,
        &executer_name,
    )
    .await
    .map_err(|error| {
        (
            Status::InternalServerError,
            format!("Failed to insert task execution record: {error:?}"),
        )
    })?;

    let document_id = Uuid::new_v4().to_string();
    let celery_app = get_celery_app().await;

    celery_app
        .send_task(
            windmill::tasks::activity_logs_report::generate_activity_logs_report::new(
                tenant_id,
                election_event_id,
                document_id.clone(),
                report_fmt,
                None,
                task_execution.clone(),
                Some(options),
                Some(AuditActor {
                    user_id: Some(claims.hasura_claims.user_id.clone()),
                    username: claims.preferred_username.clone(),
                    role: None,
                }),
            ),
        )
        .await
        .map_err(|error| {
            (
                Status::InternalServerError,
                format!(
                    "Error sending export_election_event_logs task: {error:?}"
                ),
            )
        })?;

    info!("Sent EXPORT_ELECTION_EVENT_LOGS task {task_execution:?}");
    let output = ExportElectionEventOutput {
        document_id,
        task_execution: task_execution.clone(),
    };

    Ok(Json(output))
}
