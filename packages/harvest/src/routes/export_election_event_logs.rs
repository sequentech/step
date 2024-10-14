// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use anyhow::Result;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tracing::{instrument, Level};
use uuid::Uuid;
use windmill::services::celery_app::get_celery_app;
use windmill::services::reports::electoral_log::ReportFormat;

#[derive(Serialize, Deserialize, Debug)]
pub struct ExportElectionEventInput {
    election_event_id: String,
    format: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ExportElectionEventOutput {
    document_id: String,
    task_id: String,
}

#[instrument(skip(claims))]
#[post("/export-election-event-logs", format = "json", data = "<input>")]
pub async fn export_election_event_logs_route(
    claims: jwt::JwtClaims,
    input: Json<ExportElectionEventInput>,
) -> Result<Json<ExportElectionEventOutput>, (Status, String)> {
    let body = input.into_inner();

    let report_fmt = ReportFormat::from_str(&body.format).map_err(|error| {
        (
            Status::InternalServerError,
            format!("Error sending export_election_event task: {error:?}"),
        )
    })?;

    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::LOGS_READ],
    )?;
    let document_id = Uuid::new_v4().to_string();
    let celery_app = get_celery_app().await;
    let task = celery_app
        .send_task(
            windmill::tasks::activity_logs_report::generate_activity_logs_report::new(
                claims.hasura_claims.tenant_id.clone(),
                body.election_event_id.clone(),
                document_id.clone(),
                report_fmt,
            ),
        )
        .await
        .map_err(|error| {
            (
                Status::InternalServerError,
                format!("Error sending export_election_event task: {error:?}"),
            )
        })?;
    let output = ExportElectionEventOutput {
        document_id: document_id,
        task_id: task.task_id.clone(),
    };
    info!("Sent EXPORT_ELECTION_EVENT_LOGS task {}", task.task_id);

    Ok(Json(output))
}
