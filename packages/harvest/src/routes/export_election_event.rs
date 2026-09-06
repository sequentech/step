// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use anyhow::{anyhow, Context, Result};
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::{event, instrument, Level};
use uuid::Uuid;
use windmill::services::celery_app::get_celery_app;
use windmill::services::electoral_log::{
    post_voter_secret_attribute_audit, ElectoralLogAdminContext,
    VoterSecretAttributeAction, VoterSecretAttributeAudit,
};
use windmill::services::voter_secret_attributes::get_secret_attribute_config;
use windmill::services::{password, tasks_execution::*};
use windmill::tasks::export_election_event::{self, ExportOptions};
use windmill::types::tasks::ETasksExecution;

#[derive(Serialize, Deserialize, Debug)]
pub struct ExportElectionEventInput {
    election_event_id: String,
    export_configurations: ExportOptions,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ExportElectionEventOutput {
    document_id: String,
    password: Option<String>,
    task_execution: TasksExecution,
}

#[instrument(skip(claims, input))]
#[post("/export-election-event", format = "json", data = "<input>")]
pub async fn export_election_event_route(
    claims: jwt::JwtClaims,
    input: Json<ExportElectionEventInput>,
) -> Result<Json<ExportElectionEventOutput>, (Status, String)> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::ELECTION_EVENT_READ],
    )?;

    let body = input.into_inner();
    let tenant_id = claims.hasura_claims.tenant_id.clone();
    let election_event_id = body.election_event_id.clone();
    let executer_name = claims
        .name
        .clone()
        .unwrap_or_else(|| claims.hasura_claims.user_id.clone());
    let mut export_config = body.export_configurations.clone();
    export_config.password = None;
    export_config.contains_voter_secrets = false;
    export_config.is_encrypted = export_config.is_encrypted
        || export_config.encrypt_with_password
        || export_config.bulletin_board
        || export_config.reports
        || export_config.applications;

    if (export_config.include_voters || export_config.s3_files)
        && export_config.encrypt_with_password
    {
        let has_secret_attributes = export_config.s3_files
            || !get_secret_attribute_config(&tenant_id, &election_event_id)
                .await
                .map_err(|error| {
                    (Status::InternalServerError, format!("{error:#}"))
                })?
                .validated_names()
                .map_err(|error| (Status::BadRequest, error.to_string()))?
                .is_empty();

        if has_secret_attributes {
            export_config.contains_voter_secrets = authorize(
                &claims,
                true,
                Some(tenant_id.clone()),
                vec![Permissions::VOTER_SECRET_ATTRIBUTE_READ],
            )
            .is_ok();
        }
    }

    let document_id = Uuid::new_v4().to_string();
    if export_config.contains_voter_secrets {
        post_voter_secret_attribute_audit(
            &tenant_id,
            &election_event_id,
            &ElectoralLogAdminContext::from_claims(&claims),
            VoterSecretAttributeAction::Export,
            VoterSecretAttributeAudit {
                voter_id: None,
                voter_username: None,
                attribute_names: &[],
                document_id: Some(&document_id),
            },
        )
        .await
        .map_err(|error| {
            (
                Status::InternalServerError,
                format!("Failed to record the secret-attribute electoral-log entry: {error:#}"),
            )
        })?;
    }

    // Insert the task execution record
    let task_execution = post_with_annotations(
        &tenant_id,
        Some(&election_event_id),
        ETasksExecution::EXPORT_ELECTION_EVENT,
        &executer_name,
        secret_export_task_annotations(
            &document_id,
            export_config.contains_voter_secrets,
        ),
    )
    .await
    .map_err(|error| {
        (
            Status::InternalServerError,
            format!("Failed to insert task execution record: {error:?}"),
        )
    })?;

    let celery_app = get_celery_app().await;

    // todo: generarate only if encrypted
    let charset: String =
        "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_."
            .into();
    let password: Option<String> = if export_config.is_encrypted
        || export_config.bulletin_board
        || export_config.reports
        || export_config.applications
    {
        Some(password::generate_random_string_with_charset(64, &charset))
    } else {
        None
    };
    export_config.password = password.clone();

    let _celery_task = match celery_app
        .send_task(export_election_event::export_election_event::new(
            tenant_id,
            election_event_id,
            export_config,
            document_id.clone(),
            task_execution.clone(),
        ))
        .await
    {
        Ok(task) => task,
        Err(error) => {
            update_fail(
                &task_execution,
                &format!("Failed to enqueue export: {error:?}"),
            )
            .await
            .map_err(|update_error| {
                (
                    Status::InternalServerError,
                    format!("Failed to revoke export authorization: {update_error:?}"),
                )
            })?;
            return Err((
                Status::InternalServerError,
                format!("Error sending export_election_event task: {error:?}"),
            ));
        }
    };

    let output = ExportElectionEventOutput {
        document_id,
        password,
        task_execution: task_execution.clone(),
    };

    info!("Sent EXPORT_ELECTION_EVENT task  {:?}", &task_execution);

    Ok(Json(output))
}
