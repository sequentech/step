// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize_voter_election;
use anyhow::Result;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::types::date_time::DateFormat;
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::types::permissions::VoterPermissions;
use sequent_core::{services::jwt::JwtClaims, types::date_time::TimeZone};
use serde::{Deserialize, Serialize};
use tracing::{event, instrument, Level};
use uuid::Uuid;
use windmill::services::celery_app::get_celery_app;
use windmill::services::database::get_hasura_pool;
use windmill::services::reports::ballot_receipt::BallotTemplate;
use windmill::services::reports::template_renderer::{
    ReportOriginatedFrom, ReportOrigins, TemplateRenderer,
};
use windmill::services::tasks_execution::post;
use windmill::types::tasks::ETasksExecution;

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateBallotReceiptInput {
    pub ballot_id: String,
    pub ballot_tracker_url: String,
    pub election_event_id: String,
    pub election_id: String,
    pub time_zone: Option<TimeZone>,
    pub date_format: Option<DateFormat>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateBallotReceiptOutput {
    pub id: String,
    pub ballot_id: String,
    pub status: String,
    pub task_execution: TasksExecution,
}

#[instrument(skip_all)]
#[post("/create-ballot-receipt", format = "json", data = "<body>")]
pub async fn create_ballot_receipt(
    body: Json<CreateBallotReceiptInput>,
    claims: JwtClaims,
) -> Result<Json<CreateBallotReceiptOutput>, (Status, String)> {
    let input = body.into_inner();
    let tenant_id = claims.hasura_claims.tenant_id.clone();
    let executer_name = claims
        .name
        .clone()
        .unwrap_or_else(|| claims.hasura_claims.user_id.clone());

    let area_id = match authorize_voter_election(
        &claims,
        vec![VoterPermissions::CAST_VOTE],
        &input.election_id,
    ) {
        Ok((area_id, _)) => area_id,
        Err(error) => {
            return Err(error);
        }
    };

    let voter_id = claims.hasura_claims.user_id.clone();
    // Resolve the same effective assignment used by the receipt renderer.
    // The native worker rechecks freshness and mode after the queued handoff.
    let use_prerendered = {
        let mut client =
            get_hasura_pool().await.get().await.map_err(|error| {
                (
                    Status::InternalServerError,
                    format!(
                        "Failed to get receipt database connection: {error:?}"
                    ),
                )
            })?;
        let transaction = client.transaction().await.map_err(|error| {
            (
                Status::InternalServerError,
                format!("Failed to start receipt transaction: {error:?}"),
            )
        })?;
        let report = BallotTemplate::new(
            ReportOrigins {
                tenant_id: tenant_id.clone(),
                election_event_id: input.election_event_id.clone(),
                election_id: Some(input.election_id.clone()),
                template_alias: None,
                voter_id: None,
                report_origin: ReportOriginatedFrom::VotingPortal,
                executer_username: None,
                tally_session_id: None,
            },
            None,
        );
        report
            .get_custom_user_template_data(&transaction)
            .await
            .map_err(|error| {
                (
                    Status::InternalServerError,
                    format!(
                        "Failed to resolve ballot receipt template: {error:?}"
                    ),
                )
            })?
            .and_then(|template| template.pre_render)
            .is_some_and(|options| options.enabled)
    };
    let document_id: String = Uuid::new_v4().to_string();
    let celery_app = get_celery_app().await;

    // Insert the task execution record
    let task_execution = post(
        &tenant_id,
        Some(&input.election_event_id),
        ETasksExecution::CREATE_BALLOT_RECEIPT,
        &executer_name,
    )
    .await
    .map_err(|error| {
        (
            Status::InternalServerError,
            format!("Failed to insert task execution record: {error:?}"),
        )
    })?;

    let celery_task_result = if use_prerendered {
        celery_app.send_task(
            windmill::tasks::create_ballot_receipt::create_prerendered_ballot_receipt::new(
                document_id.clone(),
                input.ballot_id.clone(),
                input.ballot_tracker_url,
                tenant_id.clone(),
                input.election_event_id,
                input.election_id,
                area_id,
                voter_id,
                input.time_zone,
                input.date_format,
                task_execution.clone(),
            ),
        )
        .await.map(|_| ())
    } else {
        celery_app.send_task(
            windmill::tasks::create_ballot_receipt::create_ballot_receipt::new(
                document_id.clone(),
                input.ballot_id.clone(),
                input.ballot_tracker_url,
                tenant_id.clone(),
                input.election_event_id,
                input.election_id,
                area_id,
                voter_id,
                input.time_zone,
                input.date_format,
                task_execution.clone(),
            ),
        )
        .await.map(|_| ())
    };

    match celery_task_result {
        Ok(task) => task,
        Err(error) => {
            return Err((
                Status::InternalServerError,
                format!("Error sending create_ballot_receipt task: {error:?}"),
            ));
        }
    };

    info!("Sent ballot receipt task successfully");

    Ok(Json(CreateBallotReceiptOutput {
        id: document_id,
        ballot_id: input.ballot_id,
        status: "pending".to_string(),
        task_execution: task_execution,
    }))
}
