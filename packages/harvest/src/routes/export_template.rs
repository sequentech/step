// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use uuid::Uuid;
use windmill::services::celery_app::get_celery_app;
use windmill::tasks::export_templates;

#[derive(Serialize, Deserialize, Debug)]
pub struct ExportTemplateBody {
    tenant_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ExportTemplateOutput {
    document_id: String,
    error_msg: Option<String>,
}
#[instrument(skip(claims))]
#[post("/export-template", format = "json", data = "<input>")]
pub async fn export_template(
    claims: jwt::JwtClaims,
    input: Json<ExportTemplateBody>,
) -> Result<Json<ExportTemplateOutput>, (Status, String)> {
    let body = input.into_inner();
    let tenant_id = claims.hasura_claims.tenant_id.clone();

    authorize(
        &claims,
        true,
        Some(body.tenant_id.clone()),
        vec![Permissions::TEMPLATE_WRITE],
    )?;

    let document_id = Uuid::new_v4().to_string();
    let celery_app = get_celery_app().await;
    let celery_task = celery_app
        .send_task(export_templates::export_templates::new(
            tenant_id.clone(),
            document_id.clone(),
        ))
        .await
        .map_err(|err| {
            (
                Status::InternalServerError,
                format!("Error sending Export Tasks Execution task: ${err}"),
            )
        })?;

    let output = ExportTemplateOutput {
        document_id,
        error_msg: None,
    };

    Ok(Json(output))
}
