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
pub struct DecryptReportBody {
    election_event_id: String,
    report_id: Option<String>,
    password: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DecryptReportOutput {
    document_id: String,
    error_msg: Option<String>,
}

#[instrument(err)]
pub async fn validate_report_decryption(
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
    let existing_key = vault::read_secret(secret_key.clone()).await;

    if let Ok(Some(existing_secret)) = existing_key {
        let existing_password: String = existing_secret;
        if existing_password.to_string() == password.to_string() {
            info!("Password matches for secret_key {:?}", secret_key);
            return Ok(());
        } else {
            return Err(anyhow!("incorrect password"));
        }
    }

    Ok(())
}

#[instrument(skip(claims))]
#[post("/decrypt-report", format = "json", data = "<input>")]
pub async fn decrypt_report_route(
    claims: jwt::JwtClaims,
    input: Json<DecryptReportBody>,
) -> Result<Json<DecryptReportOutput>, (Status, String)> {
    let body = input.into_inner();
    let tenant_id = claims.hasura_claims.tenant_id.clone();

    authorize(
        &claims,
        true,
        Some(tenant_id.clone()),
        vec![Permissions::REPORT_WRITE],
    )?;

    validate_report_decryption(
        tenant_id,
        body.election_event_id.clone(),
        body.report_id.clone(),
        body.password.clone().to_string(),
    )
    .await
    .map_err(|err| (Status::InternalServerError, err.to_string()))?;

    info!("body {:?}", body);

    let document_id = Uuid::new_v4().to_string();

    let output = DecryptReportOutput {
        document_id,
        error_msg: None,
    };

    Ok(Json(output))
}
