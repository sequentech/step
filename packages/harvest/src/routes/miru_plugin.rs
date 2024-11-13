// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::authorization::authorize;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt;
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};
use windmill::services::tasks_execution::*;
use windmill::tasks::miru_plugin_tasks::upload_signature_task;
use windmill::types::tasks::ETasksExecution;
use windmill::{
    services::{
        celery_app::get_celery_app,
        consolidation::upload_signature_service::upload_transmission_package_signature_service,
    },
    tasks::miru_plugin_tasks::{
        create_transmission_package_task, send_transmission_package_task,
    },
};

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateTransmissionPackageInput {
    election_event_id: String,
    election_id: String,
    area_id: String,
    tally_session_id: String,
    force: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateTransmissionPackageOutput {
    task_execution: Option<TasksExecution>,
    error_msg: Option<String>,
}

#[instrument(skip(claims))]
#[post("/miru/create-transmission-package", format = "json", data = "<input>")]
pub async fn create_transmission_package(
    claims: jwt::JwtClaims,
    input: Json<CreateTransmissionPackageInput>,
) -> Result<Json<CreateTransmissionPackageOutput>, (Status, String)> {
    let body = input.into_inner();
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
        ETasksExecution::CREATE_TRANSMISSION_PACKAGE,
        &executer_name,
    )
    .await
    .map_err(|error| {
        (
            Status::InternalServerError,
            format!("Failed to insert task execution record: {error:?}"),
        )
    })?;

    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::MIRU_CREATE],
    )?;
    let celery_app = get_celery_app().await;
    let celery_task = match celery_app
        .send_task(create_transmission_package_task::new(
            tenant_id,
            body.election_id.clone(),
            body.area_id.clone(),
            body.tally_session_id.clone(),
            body.force,
            task_execution.clone(),
        ))
        .await
    {
        Ok(celery_task) => celery_task,
        Err(err) => {
            return Ok(Json(CreateTransmissionPackageOutput {
                    error_msg: Some(format!(
                        "Error sending create_transmission_package_task task: ${err}"
                    )),
                    task_execution: Some(task_execution.clone()),
                }));
        }
    };

    info!(
        "Sent create_transmission_package_task task {}",
        task_execution.id
    );

    let output = CreateTransmissionPackageOutput {
        error_msg: None,
        task_execution: Some(task_execution.clone()),
    };

    Ok(Json(output))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SendTransmissionPackageInput {
    election_id: String,
    area_id: String,
    tally_session_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SendTransmissionPackageOutput {}

#[instrument(skip(claims))]
#[post("/miru/send-transmission-package", format = "json", data = "<input>")]
pub async fn send_transmission_package(
    claims: jwt::JwtClaims,
    input: Json<SendTransmissionPackageInput>,
) -> Result<Json<SendTransmissionPackageOutput>, (Status, String)> {
    let body = input.into_inner();
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::MIRU_SEND],
    )?;
    let celery_app = get_celery_app().await;
    let task = celery_app
        .send_task(send_transmission_package_task::new(
            claims.hasura_claims.tenant_id.clone(),
            body.election_id.clone(),
            body.area_id.clone(),
            body.tally_session_id.clone(),
        ))
        .await
        .map_err(|error| {
            (
                Status::InternalServerError,
                format!("Error sending send_transmission_package_task task: {error:?}"),
            )
        })?;
    info!("Sent send_transmission_package_task task {}", task.task_id);

    Ok(Json(SendTransmissionPackageOutput {}))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UploadSignatureInput {
    election_id: String,
    area_id: String,
    tally_session_id: String,
    document_id: String,
    password: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UploadSignatureOutput {}

#[instrument(skip(claims))]
#[post("/miru/upload-signature", format = "json", data = "<input>")]
pub async fn upload_signature(
    claims: jwt::JwtClaims,
    input: Json<UploadSignatureInput>,
) -> Result<Json<UploadSignatureOutput>, (Status, String)> {
    let body = input.into_inner();
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::MIRU_SIGN],
    )?;

    let Some(username) = claims.preferred_username.clone() else {
        return Err((
            Status::InternalServerError,
            "missing username in claims".into(),
        ));
    };

    upload_signature_task(
        claims.hasura_claims.tenant_id.clone(),
        body.election_id.clone(),
        body.area_id.clone(),
        body.tally_session_id.clone(),
        username,
        body.document_id.clone(),
        body.password.clone(),
    )
    .await
    .map_err(|err| {
        (
            Status::InternalServerError,
            format!("Error creating signature {}", err),
        )
    })?;

    Ok(Json(UploadSignatureOutput {}))
}
