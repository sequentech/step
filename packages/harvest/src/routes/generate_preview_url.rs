// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use anyhow::Result;
use deadpool_postgres::{Client as DbClient, Transaction};
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt;
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};
use uuid::Uuid;
use windmill::services::celery_app::get_celery_app;
use windmill::services::database::get_hasura_pool;
use windmill::services::tasks_execution::*;
use windmill::types::tasks::ETasksExecution;

#[derive(Serialize, Deserialize, Debug)]
pub struct GeneratePreviewUrlInput {
    tenant_id: String,
    document_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GeneratePreviewUrlOutput {
    preview_url: String,
}

#[instrument(skip(claims))]
#[post("/generate-preview-url", format = "json", data = "<input>")]
pub async fn generate_preview_url(
    claims: jwt::JwtClaims,
    input: Json<GeneratePreviewUrlInput>,
) -> Result<Json<GeneratePreviewUrlOutput>, (Status, String)> {
    let body = input.into_inner();
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::GENERATE_PREVIEW],
    )?;

    let executer_name = claims
        .name
        .clone()
        .unwrap_or_else(|| claims.hasura_claims.user_id.clone());

    let mut hasura_db_client: DbClient =
        get_hasura_pool().await.get().await.map_err(|err| {
            (
                Status::InternalServerError,
                format!("Error getting hasura db pool: {err}"),
            )
        })?;

    let hasura_transaction = hasura_db_client.transaction().await.map_err(
        |err: tokio_postgres::Error| {
            (
                Status::InternalServerError,
                format!("Error starting hasura transaction: {err}"),
            )
        },
    )?;

    let preview_url =
        windmill::services::generate_preview_url::generate_preview_url(
            &hasura_transaction,
            &body.tenant_id,
            &body.document_id.clone(),
            &executer_name,
        )
        .await
        .map_err(|err| {
            (
                Status::InternalServerError,
                format!("Error generating preview url: {err}"),
            )
        })?;

    hasura_transaction.commit().await.map_err(|e| {
        (Status::InternalServerError, format!("Commit failed: {e}"))
    })?;

    Ok(Json(GeneratePreviewUrlOutput { preview_url }))
}
