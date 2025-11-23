// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use anyhow::Result;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::Permissions;

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Instant;
use tracing::{event, instrument, Level};
use uuid::Uuid;
use windmill::services::celery_app::get_celery_app;

#[derive(Serialize, Deserialize, Debug)]
pub struct GetManualVerificationPdfInput {
    tenant_id: String,
    election_event_id: String,
    voter_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GetManualVerificationPdfOutput {
    document_id: String,
    status: String,
}

#[instrument(skip_all)]
#[post("/get-manual-verification-pdf", format = "json", data = "<body>")]
pub async fn get_manual_verification_pdf(
    body: Json<GetManualVerificationPdfInput>,
    claims: JwtClaims,
) -> Result<Json<GetManualVerificationPdfOutput>, (Status, String)> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::VOTER_MANUALLY_VERIFY],
    )?;

    let input = body.into_inner();
    let document_id: String = Uuid::new_v4().to_string();
    let celery_app = get_celery_app().await;

    let task = celery_app
        .send_task(
            windmill::tasks::manual_verification_report::generate_manual_verification_report::new(
                document_id.clone(),
                input.tenant_id,
                input.election_event_id,
                input.voter_id,
                None,
            ),
        )
        .await
        .map_err(|e| {
            (
                Status::InternalServerError,
                format!("Error getting manual verification pdf: {e:?}"),
            )
        })?;
    event!(Level::INFO, "Sent task {task:?} successfully");

    Ok(Json(GetManualVerificationPdfOutput {
        document_id: document_id,
        status: "pending".to_string(),
    }))
}
