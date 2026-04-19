// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use anyhow::{anyhow, Result};
use deadpool_postgres::Client as DbClient;
use deadpool_postgres::Transaction;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::{
    services::jwt::{self, JwtClaims},
    types::{hasura::core::TasksExecution, permissions::Permissions},
};
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};
use tracing::instrument;
use uuid::Uuid;
use windmill::{postgres::reports::Report, services::tasks_execution::post};
use windmill::{
    postgres::reports::{get_report_by_type, ReportType},
    services::reports_vault::get_report_key_pair,
};
use windmill::{
    postgres::{document::get_document, reports::get_report_by_id},
    services::{
        celery_app::get_celery_app,
        database::get_hasura_pool,
        reports::template_renderer::{EReportEncryption, GenerateReportMode},
    },
    tasks::generate_template::EGenerateTemplate,
    types::tasks::ETasksExecution,
};

#[derive(Serialize, Deserialize, Debug)]
#[allow(clippy::struct_field_names)] // enable same postfix for all fields
/// Request body for rendering a document PDF.
pub struct RenderDocumentPdfInput {
    /// The document ID.
    pub document_id: String,
    /// The election event ID.
    pub election_event_id: Option<String>,
    /// The tally session ID.
    pub tally_session_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
/// Response body for rendering a document PDF.
pub struct RenderDocumentPdfResponse {
    /// The document ID.
    document_id: String,
    /// The task execution.
    pub task_execution: TasksExecution,
}

/// Verifies that `document_id` resolves to an HTML document for the tenant.
async fn ensure_document_is_html(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: Option<String>,
    document_id: &str,
) -> Result<(), (Status, String)> {
    let Some(found_document) = get_document(
        hasura_transaction,
        tenant_id,
        election_event_id,
        document_id,
    )
    .await
    .map_err(|e| {
        (
            Status::InternalServerError,
            format!("Error fetching document: {e:?}"),
        )
    })?
    else {
        return Err((
            Status::NotFound,
            format!("Document not found: {document_id}"),
        ));
    };

    if let Some(media_type) = found_document.media_type {
        if media_type != "text/html" {
            return Err((
                Status::InternalServerError,
                format!("Invalid document type: {media_type}"),
            ));
        }
    } else {
        return Err((
            Status::InternalServerError,
            format!("Document {document_id}: missing media type"),
        ));
    }
    Ok(())
}

/// Render a document PDF.
#[instrument(skip(claims))]
#[post("/render-document-pdf", format = "json", data = "<body>")]
pub async fn render_document_pdf(
    claims: JwtClaims,
    body: Json<RenderDocumentPdfInput>,
) -> Result<Json<RenderDocumentPdfResponse>, (Status, String)> {
    let input = body.into_inner();
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::REPORT_READ],
    )?;

    let mut hasura_db_client: DbClient =
        get_hasura_pool().await.get().await.map_err(|e| {
            (
                Status::InternalServerError,
                format!("Error obtaining keycloak transaction: {e:?}"),
            )
        })?;
    let hasura_transaction =
        hasura_db_client.transaction().await.map_err(|e| {
            (
                Status::InternalServerError,
                format!("Error obtaining hasura transaction: {e:?}"),
            )
        })?;

    let election_event_id = input.election_event_id.clone();
    let document_id = input.document_id.clone();

    ensure_document_is_html(
        &hasura_transaction,
        &claims.hasura_claims.tenant_id,
        election_event_id.clone(),
        &document_id,
    )
    .await?;

    let executer_name = claims
        .name
        .clone()
        .unwrap_or_else(|| claims.hasura_claims.user_id.clone());

    let executer_username = claims
        .preferred_username
        .clone()
        .unwrap_or_else(|| executer_name.clone());

    let output_document_id: String = Uuid::new_v4().to_string();

    let celery_app = get_celery_app().await;

    // Insert the task execution record
    let task_execution = post(
        &claims.hasura_claims.tenant_id,
        election_event_id.as_deref(),
        ETasksExecution::RENDER_DOCUMENT_PDF,
        &executer_name,
    )
    .await
    .map_err(|error| {
        (
            Status::InternalServerError,
            format!("Failed to insert task execution record: {error:?}"),
        )
    })?;

    let _task = celery_app
        .send_task(
            windmill::tasks::render_document_pdf::render_document_pdf::new(
                claims.hasura_claims.tenant_id.clone(),
                document_id.clone(),
                election_event_id.clone(),
                task_execution.clone(),
                Some(executer_username),
                output_document_id.clone(),
                input.tally_session_id.clone(),
            ),
        )
        .await
        .map_err(|e| {
            (
                Status::InternalServerError,
                format!("Error publishing task render_document_pdf {e:?}"),
            )
        })?;

    Ok(Json(RenderDocumentPdfResponse {
        document_id: output_document_id,
        task_execution: task_execution.clone(),
    }))
}

////////////

#[derive(Serialize, Deserialize, Debug)]
/// Response body for generating a template.
pub struct GenerateTemplateResponse {
    /// The document ID.
    document_id: String,
    /// The task execution.
    pub task_execution: TasksExecution,
}

/// Generate a document based on a template.
#[instrument(skip(claims))]
#[post("/generate-template", format = "json", data = "<body>")]
pub async fn generate_template(
    claims: JwtClaims,
    body: Json<EGenerateTemplate>,
) -> Result<Json<GenerateTemplateResponse>, (Status, String)> {
    let input = body.into_inner();
    info!("Generating report: {input:?}");
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::REPORT_READ],
    )?;

    let mut hasura_db_client: DbClient =
        get_hasura_pool().await.get().await.map_err(|e| {
            (
                Status::InternalServerError,
                format!("Error obtaining keycloak transaction: {e:?}"),
            )
        })?;
    let hasura_transaction =
        hasura_db_client.transaction().await.map_err(|e| {
            (
                Status::InternalServerError,
                format!("Error obtaining hasura transaction: {e:?}"),
            )
        })?;

    let executer_name = claims
        .name
        .clone()
        .unwrap_or_else(|| claims.hasura_claims.user_id.clone());

    let executer_username = claims
        .preferred_username
        .clone()
        .unwrap_or_else(|| executer_name.clone());

    let document_id: String = Uuid::new_v4().to_string();
    let celery_app = get_celery_app().await;
    let EGenerateTemplate::BallotImages {
        election_event_id, ..
    } = input.clone();

    // Insert the task execution record
    let task_execution = post(
        &claims.hasura_claims.tenant_id,
        Some(&election_event_id),
        ETasksExecution::GENERATE_REPORT,
        &executer_name,
    )
    .await
    .map_err(|error| {
        (
            Status::InternalServerError,
            format!("Failed to insert task execution record: {error:?}"),
        )
    })?;

    let _task = celery_app
        .send_task(windmill::tasks::generate_template::generate_template::new(
            claims.hasura_claims.tenant_id.clone(),
            document_id.clone(),
            input,
            Some(task_execution.clone()),
            Some(executer_username),
        ))
        .await
        .map_err(|e| {
            (
                Status::InternalServerError,
                format!("Error generating template: {e:?}"),
            )
        })?;

    Ok(Json(GenerateTemplateResponse {
        document_id,
        task_execution: task_execution.clone(),
    }))
}

#[derive(Serialize, Deserialize, Debug)]
/// Request body for generating a report.
pub struct GenerateReportBody {
    /// The report ID.
    report_id: String,
    /// The tenant ID.
    tenant_id: String,
    /// The report mode.
    report_mode: GenerateReportMode,
    /// The election event ID.
    pub election_event_id: Option<String>,
}
#[derive(Serialize, Deserialize, Debug)]
/// Response body for generating a report.
pub struct GenerateReportResponse {
    /// The document ID.
    document_id: String,
    /// The encryption policy.
    pub encryption_policy: EReportEncryption,
    /// The task execution.
    pub task_execution: TasksExecution,
}

/// Generate a report.
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
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::REPORT_READ],
    )?;

    let mut hasura_db_client: DbClient =
        get_hasura_pool().await.get().await.map_err(|e| {
            (
                Status::InternalServerError,
                format!("Error obtaining keycloak transaction: {e:?}"),
            )
        })?;
    let hasura_transaction =
        hasura_db_client.transaction().await.map_err(|e| {
            (
                Status::InternalServerError,
                format!("Error obtaining hasura transaction: {e:?}"),
            )
        })?;

    let executer_name = claims
        .name
        .clone()
        .unwrap_or_else(|| claims.hasura_claims.user_id.clone());

    let executer_username = claims
        .preferred_username
        .clone()
        .unwrap_or_else(|| executer_name.clone());

    let document_id: String = Uuid::new_v4().to_string();
    let celery_app = get_celery_app().await;
    let report = get_report_by_id(
        &hasura_transaction,
        &input.tenant_id,
        &input.report_id,
    )
    .await
    .map_err(|e| {
        (
            Status::InternalServerError,
            format!("Error getting report by id: {e:?}"),
        )
    })?
    .ok_or_else(|| (Status::NotFound, "Report not found".to_string()))?;

    // Insert the task execution record
    let task_execution = post(
        &input.tenant_id,
        input.election_event_id.as_deref(),
        ETasksExecution::GENERATE_REPORT,
        &executer_name,
    )
    .await
    .map_err(|error| {
        (
            Status::InternalServerError,
            format!("Failed to insert task execution record: {error:?}"),
        )
    })?;

    let _task = celery_app
        .send_task(windmill::tasks::generate_report::generate_report::new(
            report.clone(),
            document_id.clone(),
            input.report_mode.clone(),
            false,
            Some(task_execution.clone()),
            Some(executer_username),
            None,
        ))
        .await
        .map_err(|e| {
            (
                Status::InternalServerError,
                format!("Error generating report: {e:?}"),
            )
        })?;

    Ok(Json(GenerateReportResponse {
        document_id,
        encryption_policy: report.encryption_policy,
        task_execution: task_execution.clone(),
    }))
}

#[derive(Serialize, Deserialize, Debug)]
/// Request body for encrypting a report.
pub struct EncryptReportBody {
    /// The election event ID.
    election_event_id: String,
    /// The report ID.
    report_id: Option<String>,
    /// The password.
    password: String,
}
#[derive(Serialize, Deserialize, Debug)]
/// Response body for encrypting a report.
pub struct ExportTemplateOutput {
    /// The document ID.
    document_id: String,
    /// The error message.
    error_msg: Option<String>,
}

/// Encrypt a report.
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

    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

    get_report_key_pair(
        &hasura_transaction,
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

    hasura_transaction
        .commit()
        .await
        .map_err(|err| (Status::InternalServerError, err.to_string()))?;

    Ok(Json(output))
}
