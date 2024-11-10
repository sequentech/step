// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use anyhow::Result;
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::{
    services::jwt::{self, JwtClaims},
    types::permissions::Permissions,
};
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};
use tracing::instrument;
use uuid::Uuid;
use windmill::{
    postgres::reports::get_report_by_id,
    services::{
        celery_app::get_celery_app, database::get_hasura_pool,
        reports::template_renderer::GenerateReportMode,
    },
};

#[derive(Serialize, Deserialize, Debug)]
pub struct GenerateReportBody {
    pub report_id: String,
    pub tenant_id: String,
    pub report_mode: GenerateReportMode,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct GenerateReportResponse {
    pub document_id: String,
}

#[instrument(skip(claims))]
#[post("/generate-report", format = "json", data = "<body>")]
pub async fn generate_report(
    claims: JwtClaims,
    body: Json<GenerateReportBody>,
) -> Result<Json<GenerateReportResponse>, (Status, String)> {
    let input = body.into_inner();
    info!("Generating report: {input:?}");
    authorize(
        &claims,
        true,
        Some(input.tenant_id.clone()),
        vec![Permissions::REPORT_READ],
    )?;

    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    let document_id: String = Uuid::new_v4().to_string();
    let celery_app = get_celery_app().await;
    let report = get_report_by_id(
        &hasura_transaction,
        &input.tenant_id,
        &input.report_id,
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?
    .ok_or_else(|| (Status::NotFound, "Report not found".to_string()))?;

    let cron_config = report.cron_config.clone().ok_or_else(|| {
        (
            Status::InternalServerError,
            "Cron config not found".to_string(),
        )
    })?;

    let task = celery_app
        .send_task(windmill::tasks::generate_report::generate_report::new(
            report,
            document_id.clone(),
            input.report_mode.clone(),
            cron_config.is_active,
        ))
        .await
        .map_err(|e| {
            (
                Status::InternalServerError,
                format!("Error generating report: {e:?}"),
            )
        })?;

    Ok(Json(GenerateReportResponse {
        document_id: document_id,
    }))
}
