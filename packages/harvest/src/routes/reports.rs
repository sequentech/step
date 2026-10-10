// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::routes::signing::{signing_failure, SigningReply};
use crate::services::authorization::authorize;
use crate::services::dependencies::HarvestServices;
use crate::services::signing_http::SigningError as SigningFailure;
use anyhow::{anyhow, Result};
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use sequent_core::{
    services::{
        jwt::{self, JwtClaims},
        uuid_validation::parse_uuid_v4_field,
    },
    types::{hasura::core::TasksExecution, permissions::Permissions},
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use windmill::services::electoral_log::{
    ElectoralLogAdminContext, VoterSecretAttributeAction,
    VoterSecretAttributeAudit,
};
use windmill::services::signing::actions::reports::{
    held_report_requests, reads_report_requests, HeldReportRequest,
};
use windmill::services::signing::{SigningCaller, SigningError};

use strum_macros::{Display, EnumString};
use tracing::instrument;
use uuid::Uuid;
use windmill::postgres::reports::Report;
use windmill::postgres::reports::{get_report_by_type, ReportType};
use windmill::{
    postgres::{document::get_document, reports::get_report_by_id},
    services::reports::template_renderer::{
        EReportEncryption, GenerateReportMode,
    },
    tasks::generate_template::EGenerateTemplate,
    types::tasks::ETasksExecution,
};

#[derive(Serialize, Deserialize, Debug)]
pub struct RenderDocumentPdfInput {
    pub document_id: String,
    pub election_event_id: Option<String>,
    pub tally_session_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RenderDocumentPdfResponse {
    pub document_id: String,
    pub task_execution: TasksExecution,
}

#[instrument(skip(claims, services))]
#[post("/render-document-pdf", format = "json", data = "<body>")]
pub async fn render_document_pdf(
    claims: JwtClaims,
    body: Json<RenderDocumentPdfInput>,
    services: &State<HarvestServices>,
) -> Result<Json<RenderDocumentPdfResponse>, (Status, String)> {
    let input = body.into_inner();
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::REPORT_READ],
    )?;

    let mut hasura_db_client: DbClient =
        services.databases.hasura().await.get().await.map_err(|e| {
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

    let Some(found_document) = get_document(
        &hasura_transaction,
        &claims.hasura_claims.tenant_id,
        election_event_id.clone(),
        &document_id,
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
            format!("Document not found: {}", document_id),
        ));
    };

    if let Some(media_type) = found_document.media_type {
        if "text/html".to_string() != media_type {
            return Err((
                Status::InternalServerError,
                format!("Invalid document type: {}", media_type),
            ));
        }
    } else {
        return Err((
            Status::InternalServerError,
            format!("Document {}: missing media type", document_id),
        ));
    };

    let executer_name = claims
        .name
        .clone()
        .unwrap_or_else(|| claims.hasura_claims.user_id.clone());

    let executer_username = claims
        .preferred_username
        .clone()
        .unwrap_or_else(|| executer_name.clone());

    let output_document_id: String = Uuid::new_v4().to_string();

    let celery_app = services.tasks.connect().await;

    // Insert the task execution record
    let task_execution = services
        .ledger
        .post(
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
pub struct GenerateTemplateResponse {
    pub document_id: String,
    pub task_execution: TasksExecution,
}

#[instrument(skip(claims, services))]
#[post("/generate-template", format = "json", data = "<body>")]
pub async fn generate_template(
    claims: JwtClaims,
    body: Json<EGenerateTemplate>,
    services: &State<HarvestServices>,
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
        services.databases.hasura().await.get().await.map_err(|e| {
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
    let celery_app = services.tasks.connect().await;
    let EGenerateTemplate::BallotImages {
        election_event_id, ..
    } = input.clone();

    // Insert the task execution record
    let task_execution = services
        .ledger
        .post(
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
        document_id: document_id,
        task_execution: task_execution.clone(),
    }))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GenerateReportBody {
    pub report_id: String,
    pub tenant_id: String,
    pub report_mode: GenerateReportMode,
    pub election_event_id: Option<String>,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct GenerateReportResponse {
    pub document_id: String,
    pub encryption_policy: EReportEncryption,
    pub task_execution: TasksExecution,
}

const REPORT_NOT_FOUND: &str = "Report not found";

/// Checks REPORT_READ for the tenant the report is loaded from.
fn authorize_generate_report(
    claims: &JwtClaims,
    input: &GenerateReportBody,
) -> Result<(), (Status, String)> {
    authorize(
        claims,
        true,
        Some(input.tenant_id.clone()),
        vec![Permissions::REPORT_READ],
    )
}

/// Treats the report as not found when the request names an election event
/// other than the report's. The requested id must be a valid UUID and is
/// compared with the report's in its canonical form.
fn ensure_report_event(
    report_election_event_id: &str,
    requested_election_event_id: Option<&str>,
) -> Result<(), (Status, String)> {
    match requested_election_event_id {
        None => Ok(()),
        Some(election_event_id) => {
            let election_event_id =
                parse_uuid_v4_field(election_event_id, "election_event_id")
                    .map_err(|error| (Status::BadRequest, error.to_string()))?;
            if election_event_id.to_string() == report_election_event_id {
                Ok(())
            } else {
                Err((Status::NotFound, REPORT_NOT_FOUND.to_string()))
            }
        }
    }
}

/// Queues a report of the requested tenant, which the caller must be
/// authorized for.
#[instrument(skip(claims, services))]
#[post("/generate-report", format = "json", data = "<body>")]
pub async fn generate_report(
    claims: JwtClaims,
    body: Json<GenerateReportBody>,
    services: &State<HarvestServices>,
) -> Result<Json<GenerateReportResponse>, (Status, String)> {
    let input = body.into_inner();
    info!("Generating report: {input:?}");
    authorize_generate_report(&claims, &input)?;

    let mut hasura_db_client: DbClient =
        services.databases.hasura().await.get().await.map_err(|e| {
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
    let celery_app = services.tasks.connect().await;
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
    .ok_or_else(|| (Status::NotFound, REPORT_NOT_FOUND.to_string()))?;
    ensure_report_event(
        &report.election_event_id,
        input.election_event_id.as_deref(),
    )?;
    let report_type =
        ReportType::from_str(&report.report_type).map_err(|error| {
            (Status::BadRequest, format!("Invalid report type: {error}"))
        })?;
    if input.report_mode == GenerateReportMode::REAL {
        if let Some(refusal) = signed_generation_refusal(
            &hasura_transaction,
            &report,
            &report_type,
        )
        .await
        .map_err(|error| {
            (
                Status::InternalServerError,
                format!("Error reading the report's signing rule: {error:?}"),
            )
        })? {
            return Err(refusal);
        }
    }
    let declared_secret_names =
        windmill::services::reports::template_renderer::get_declared_report_secret_attribute_names(
            &hasura_transaction,
            &report.tenant_id,
            &report.election_event_id,
            &report_type,
            report.election_id.as_deref(),
        )
        .await
        .map_err(|error| {
            (
                Status::InternalServerError,
                format!("Error reading report template: {error:#}"),
            )
        })?;
    let may_read_secret_attributes = !declared_secret_names.is_empty();
    if may_read_secret_attributes {
        authorize(
            &claims,
            true,
            Some(report.tenant_id.clone()),
            vec![Permissions::VOTER_SECRET_ATTRIBUTE_READ],
        )?;
        let attribute_names: Vec<String> =
            declared_secret_names.iter().cloned().collect();
        services
            .electoral_log
            .voter_secret_attributes(
            &report.tenant_id,
            &report.election_event_id,
            &ElectoralLogAdminContext::from_claims(&claims),
            VoterSecretAttributeAction::Report,
            VoterSecretAttributeAudit {
                voter_id: None,
                voter_username: None,
                attribute_names: &attribute_names,
                document_id: None,
            },
        )
        .await
        .map_err(|error| {
            (
                Status::InternalServerError,
                format!("Failed to record the secret-attribute electoral-log entry: {error:#}"),
            )
        })?;
    }

    // Insert the task execution record
    let task_execution = services
        .ledger
        .post(
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
            may_read_secret_attributes,
            Some(SigningCaller::from_claims(&claims)),
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
        encryption_policy: report.encryption_policy,
        task_execution: task_execution.clone(),
    }))
}

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

#[instrument(skip(claims, services))]
#[post("/encrypt-report", format = "json", data = "<input>")]
pub async fn encrypt_report_route(
    claims: jwt::JwtClaims,
    input: Json<EncryptReportBody>,
    services: &State<HarvestServices>,
) -> Result<Json<ExportTemplateOutput>, (Status, String)> {
    let body = input.into_inner();
    let tenant_id = claims.hasura_claims.tenant_id.clone();

    authorize(
        &claims,
        true,
        Some(tenant_id.clone()),
        vec![Permissions::REPORT_WRITE],
    )?;

    let mut hasura_db_client: DbClient = services
        .databases
        .hasura()
        .await
        .get()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    services
        .vault
        .check_report_password(
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

#[cfg(test)]
#[path = "../../tests/support/report_routes.rs"]
mod route_tests;

#[cfg(test)]
mod generate_report_scope_tests {
    use super::*;
    use crate::test_claims::Claims;

    /// Request for report "report" of the given tenant in election event
    /// "event".
    fn request(tenant_id: &str) -> GenerateReportBody {
        GenerateReportBody {
            report_id: "report".into(),
            tenant_id: tenant_id.into(),
            report_mode: GenerateReportMode::REAL,
            election_event_id: Some("event".into()),
        }
    }

    /// Admin of the given tenant with REPORT_READ.
    fn report_reader(tenant_id: &str) -> JwtClaims {
        Claims::new(tenant_id, "admin")
            .roles([Permissions::REPORT_READ])
            .build()
    }

    /// Another tenant is refused for a caller who is not a super admin.
    #[test]
    fn generate_report_requires_access_to_the_requested_tenant() {
        assert_eq!(
            authorize_generate_report(
                &report_reader("tenant-a"),
                &request("tenant-b")
            )
            .unwrap_err()
            .0,
            Status::Unauthorized
        );
        assert!(authorize_generate_report(
            &report_reader("tenant-a"),
            &request("tenant-a")
        )
        .is_ok());
    }

    /// REPORT_READ is required also for the caller's own tenant.
    #[test]
    fn generate_report_requires_report_read() {
        assert!(authorize_generate_report(
            &Claims::new("tenant-a", "admin").build(),
            &request("tenant-a")
        )
        .is_err());
    }

    /// An omitted election event is accepted; another one is not found.
    #[test]
    fn generate_report_rejects_an_election_event_other_than_the_reports() {
        let event = Uuid::new_v4().to_string();
        let other_event = Uuid::new_v4().to_string();
        assert_eq!(
            ensure_report_event(&event, Some(&other_event))
                .unwrap_err()
                .0,
            Status::NotFound
        );
        assert!(ensure_report_event(&event, Some(&event)).is_ok());
        assert!(ensure_report_event(&event, None).is_ok());
    }

    /// The report's election event matches however its UUID is cased.
    #[test]
    fn generate_report_compares_election_event_ids_as_uuids() {
        let event = Uuid::new_v4().to_string();
        assert!(
            ensure_report_event(&event, Some(&event.to_uppercase())).is_ok()
        );
    }

    /// A malformed election event id is a client error.
    #[test]
    fn generate_report_rejects_a_malformed_election_event_id() {
        let event = Uuid::new_v4().to_string();
        assert_eq!(
            ensure_report_event(&event, Some("event")).unwrap_err().0,
            Status::BadRequest
        );
    }
}

/// Why a real report can't be generated here while its action needs
/// signatures: the election returns and the initialization report are
/// produced and signed by the tally, and a held report is a Post's.
async fn signed_generation_refusal(
    hasura_transaction: &deadpool_postgres::Transaction<'_>,
    report: &windmill::postgres::reports::Report,
    report_type: &ReportType,
) -> anyhow::Result<Option<(Status, String)>> {
    use windmill::services::signing::guard::effective_rule;
    use windmill::services::signing::pdf::{
        report_signing_action, tally_signing_action,
    };
    let required = |action| async move {
        let tenant_id = Uuid::parse_str(&report.tenant_id)?;
        let election_event_id = Uuid::parse_str(&report.election_event_id)?;
        anyhow::Ok(
            effective_rule(
                hasura_transaction,
                tenant_id,
                election_event_id,
                action,
            )
            .await?
            .is_required(),
        )
    };
    if let Some(action) = tally_signing_action(report_type) {
        if required(action).await? {
            return Ok(Some((
                Status::Conflict,
                "This report needs signatures: the tally produces it for each Post, to be signed there."
                    .into(),
            )));
        }
    }
    if let Some(action) = report_signing_action(report_type) {
        if report.election_id.is_none() && required(action).await? {
            return Ok(Some((
                Status::BadRequest,
                "This report needs signatures: generate it for a Post.".into(),
            )));
        }
    }
    Ok(None)
}

#[derive(Debug, Deserialize)]
pub struct HeldReportRequestsInput {
    election_event_id: String,
}

#[derive(Debug, Serialize)]
pub struct HeldReportRequestsOutput {
    requests: Vec<HeldReportRequest>,
}

/// Safe request links for Reports and Tally, and the tally of a released
/// report. The tenant, action permissions and Post labels are enforced by
/// the same authenticated caller as the request panel.
#[instrument(skip(claims, services, body))]
#[post("/signing-reports/requests", format = "json", data = "<body>")]
pub async fn signing_held_report_requests(
    claims: JwtClaims,
    services: &State<HarvestServices>,
    body: Json<HeldReportRequestsInput>,
) -> SigningReply<HeldReportRequestsOutput> {
    crate::services::signing_http::authorize_403(
        &claims,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![],
    )?;
    let caller = SigningCaller::from_claims(&claims);
    if !reads_report_requests(&caller) {
        return Err(signing_failure(SigningError::Forbidden(
            "Reading it needs a report or signing request permission.".into(),
        )));
    }
    let tenant_id =
        Uuid::parse_str(&claims.hasura_claims.tenant_id).map_err(|_| {
            SigningFailure::new(
                Status::Unauthorized,
                crate::services::signing_http::SigningErrorCode::Unauthorized,
                "The token's tenant is not a UUID.",
            )
        })?;
    let election_event_id =
        Uuid::parse_str(&body.election_event_id).map_err(|_| {
            SigningFailure::invalid("election_event_id is not a UUID")
        })?;
    let mut client = services
        .databases
        .hasura()
        .await
        .get()
        .await
        .map_err(SigningFailure::internal)?;
    let transaction = client
        .transaction()
        .await
        .map_err(SigningFailure::internal)?;
    let requests = held_report_requests(
        &transaction,
        &caller,
        tenant_id,
        election_event_id,
    )
    .await
    .map_err(signing_failure)?;
    Ok(Json(HeldReportRequestsOutput { requests }))
}
