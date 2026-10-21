// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use anyhow::Result;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt;
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tracing::instrument;
use uuid::Uuid;
use windmill::services::celery_app::get_celery_app;
use windmill::services::reports::activity_log::ReportFormat;
use windmill::services::tasks_execution::*;
use windmill::types::tasks::ETasksExecution;

#[derive(Serialize, Deserialize, Debug)]
pub struct ExportElectionEventInput {
    election_event_id: String,
    format: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ExportElectionEventOutput {
    document_id: String,
    task_execution: TasksExecution,
}

fn authorize_logs_export(
    claims: &jwt::JwtClaims,
) -> Result<(), (Status, String)> {
    authorize(
        claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::LOGS_EXPORT],
    )
}

#[instrument(skip(claims))]
#[post("/export-election-event-logs", format = "json", data = "<input>")]
pub async fn export_election_event_logs_route(
    claims: jwt::JwtClaims,
    input: Json<ExportElectionEventInput>,
) -> Result<Json<ExportElectionEventOutput>, (Status, String)> {
    authorize_logs_export(&claims)?;

    let body = input.into_inner();

    info!("Format: {}", &body.format);
    let report_fmt = ReportFormat::from_str(&body.format).map_err(|error| {
        (
            Status::InternalServerError,
            format!("Error sending export_election_event task: {error:?}"),
        )
    })?;
    info!("{:?}", report_fmt);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::authorization::test_claims::{
        admin_claims, CALLER_TENANT_ID,
    };

    #[test]
    fn exporting_logs_needs_logs_export_not_report_write() {
        let writer = admin_claims(CALLER_TENANT_ID, &["report-write"]);
        assert_eq!(
            authorize_logs_export(&writer).unwrap_err().0,
            Status::Unauthorized
        );
        let exporter = admin_claims(CALLER_TENANT_ID, &["logs-export"]);
        assert!(authorize_logs_export(&exporter).is_ok());
    }
}
