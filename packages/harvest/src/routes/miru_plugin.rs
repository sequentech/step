// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::authorization::authorize;
use anyhow::Result;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};
use windmill::{
    services::celery_app::get_celery_app, tasks::send_eml::send_eml_task,
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
        vec![Permissions::TALLY_WRITE],
    )?;

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
        vec![Permissions::TALLY_WRITE],
    )?;

    Ok(Json(CreateTransmissionPackageOutput {}))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UploadSignatureInput {
    election_id: String,
    area_id: String,
    tally_session_id: String,
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
        vec![Permissions::TALLY_WRITE],
    )?;

    Ok(Json(CreateTransmissionPackageOutput {}))
}
