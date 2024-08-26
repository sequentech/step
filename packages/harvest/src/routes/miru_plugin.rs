// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::authorization::authorize;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};
use windmill::services::database::get_hasura_pool;
use windmill::tasks::miru_plugin_tasks::upload_signature_task;
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
    election_id: String,
    area_id: String,
    tally_session_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateTransmissionPackageOutput {}

#[instrument(skip(claims))]
#[post("/miru/create-transmission-package", format = "json", data = "<input>")]
pub async fn create_transmission_package(
    claims: jwt::JwtClaims,
    input: Json<CreateTransmissionPackageInput>,
) -> Result<Json<CreateTransmissionPackageOutput>, (Status, String)> {
    let body = input.into_inner();
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::MIRU_CREATE],
    )?;
    let celery_app = get_celery_app().await;
    let task = celery_app
        .send_task(create_transmission_package_task::new(
            claims.hasura_claims.tenant_id.clone(),
            body.election_id.clone(),
            body.area_id.clone(),
            body.tally_session_id.clone(),
        ))
        .await
        .map_err(|error| {
            (
                Status::InternalServerError,
                format!("Error sending create_transmission_package_task task: {error:?}"),
            )
        })?;
    info!(
        "Sent create_transmission_package_task task {}",
        task.task_id
    );

    Ok(Json(CreateTransmissionPackageOutput {}))
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
    private_key: String,
    public_key: String,
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

    let celery_app = get_celery_app().await;
    let task = celery_app
        .send_task(upload_signature_task::new(
            claims.hasura_claims.tenant_id.clone(),
            body.election_id.clone(),
            body.area_id.clone(),
            body.tally_session_id.clone(),
            username,
            body.private_key.clone(),
            body.public_key.clone(),
        ))
        .await
        .map_err(|error| {
            (
                Status::InternalServerError,
                format!("Error sending upload_signature_task task: {error:?}"),
            )
        })?;
    info!("Sent upload_signature_task task {}", task.task_id);

    Ok(Json(UploadSignatureOutput {}))
}
