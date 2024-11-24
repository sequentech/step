// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use anyhow::anyhow;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::types::permissions::Permissions;
use sequent_core::{
    serialization::deserialize_with_path::deserialize_str, services::jwt,
};
use serde::{Deserialize, Serialize};
use tracing::instrument;
use uuid::Uuid;
use windmill::services::vault::{self, save_secret};

#[derive(Serialize, Deserialize, Debug)]
pub struct EncryptReportBody {
    election_event_id: String,
    report_id: Option<String>,
    password: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ExportTemplateOutput {
    document_id: String,
    error_msg: Option<String>,
}

#[instrument(err)]
pub async fn get_report_key_pair(
    tenant_id: String,
    election_event_id: String,
    report_id: Option<String>,
    password: String,
) -> Result<(), anyhow::Error> {
    let secret_key = format!(
        "tenant-{}-event-{}-report_id-{}",
        &tenant_id,
        election_event_id,
        report_id.unwrap_or_else(|| "default".to_string())
    );

    info!("secret_key {:?}", secret_key);
    save_secret(secret_key.clone(), password.clone()).await?;

    Ok(())
}

#[instrument(skip(claims))]
#[post("/encrypt-report", format = "json", data = "<input>")]
pub async fn encrypt_report_route(
    claims: jwt::JwtClaims,
    input: Json<EncryptReportBody>,
) -> Result<Json<ExportTemplateOutput>, (Status, String)> {
    let body = input.into_inner();
    let tenant_id = claims.hasura_claims.tenant_id.clone();

    authorize(
        &claims,
        true,
        Some(tenant_id.clone()),
        vec![Permissions::REPORT_WRITE],
    )?;

    get_report_key_pair(
        tenant_id,
        body.election_event_id.clone(),
        body.report_id.clone(),
        body.password.clone(),
    )
    .await
    .map_err(|err| (Status::InternalServerError, err.to_string()))?;

    info!("body {:?}", body);

    let document_id = Uuid::new_v4().to_string();

    let output = ExportTemplateOutput {
        document_id,
        error_msg: None,
    };

    Ok(Json(output))
}
