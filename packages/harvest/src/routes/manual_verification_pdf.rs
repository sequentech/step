// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::audit;
use crate::services::authorization::authorize;
use crate::services::signing_gate::{caller, Guarded};
use crate::types::error_response::{ErrorCode, ErrorResponse, JsonError};
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::instrument;
use uuid::Uuid;
use windmill::postgres::audit_log_outbox::AuditLogTarget;
use windmill::postgres::reports::ReportType;
use windmill::services::audit_log::{self, AdminAction, AuditObject};
use windmill::services::celery_app::get_celery_app;
use windmill::services::database::get_hasura_pool;
use windmill::services::electoral_log::{
    post_voter_secret_attribute_audit, ElectoralLogAdminContext,
    VoterSecretAttributeAction, VoterSecretAttributeAudit,
};
use windmill::services::reports::template_renderer::get_declared_report_secret_attribute_names;
use windmill::services::signing::actions::manual_verification::{
    authorize_token, initiate, Delivery, Phase,
};
use windmill::services::signing::guard::SigningRequestSummary;
use windmill::tasks::audit_log_outbox::kick_audit_log_outbox;
use windmill::tasks::signing_log_outbox::kick_signing_log_outbox;

#[derive(Deserialize)]
pub struct GetManualVerificationPdfInput {
    tenant_id: String,
    election_event_id: String,
    voter_id: String,
    initiation_id: Option<Uuid>,
    recover_request_id: Option<Uuid>,
}

#[derive(Serialize)]
pub struct GetManualVerificationPdfOutput {
    document_id: Option<Uuid>,
    status: &'static str,
    signing_request: Option<SigningRequestSummary>,
}

fn unavailable<E>(_: E) -> (Status, String) {
    (
        Status::ServiceUnavailable,
        "Manual verification is unavailable.".into(),
    )
}

fn pdf_authorization_error((status, _): (Status, String)) -> JsonError {
    ErrorResponse::new(
        status,
        "Manual verification is not authorized.",
        ErrorCode::Unauthorized,
    )
}

fn pdf_scope_id(value: &str) -> Result<Uuid, JsonError> {
    Uuid::parse_str(value).map_err(|_| {
        ErrorResponse::new(
            Status::BadRequest,
            "Invalid election scope.",
            ErrorCode::UuidParseFailed,
        )
    })
}

fn pdf_unavailable<E>(_: E) -> JsonError {
    ErrorResponse::new(
        Status::ServiceUnavailable,
        "Manual verification is unavailable.",
        ErrorCode::ManualVerificationUnavailable,
    )
}

fn pdf_delivery_failed() -> JsonError {
    ErrorResponse::new(
        Status::Conflict,
        "This document could not be delivered. Start a new authorization.",
        ErrorCode::ManualVerificationDeliveryFailed,
    )
}

#[instrument(skip_all)]
#[post("/get-manual-verification-pdf", format = "json", data = "<body>")]
pub async fn get_manual_verification_pdf(
    body: Json<GetManualVerificationPdfInput>,
    claims: JwtClaims,
) -> Result<Json<GetManualVerificationPdfOutput>, Guarded<JsonError>> {
    let input = body.into_inner();
    authorize(
        &claims,
        true,
        Some(input.tenant_id.clone()),
        vec![Permissions::VOTER_MANUALLY_VERIFY],
    )
    .map_err(pdf_authorization_error)?;
    let tenant = pdf_scope_id(&input.tenant_id)?;
    let event = pdf_scope_id(&input.election_event_id)?;
    let initiation = input.initiation_id.unwrap_or_else(Uuid::new_v4);
    let mut client = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(pdf_unavailable)?;
    let tx = client.transaction().await.map_err(pdf_unavailable)?;
    let names = get_declared_report_secret_attribute_names(
        &tx,
        &input.tenant_id,
        &input.election_event_id,
        &ReportType::MANUAL_VERIFICATION,
        None,
    )
    .await
    .map_err(pdf_unavailable)?;
    let mut names: Vec<_> = names.into_iter().collect();
    names.sort();
    if !names.is_empty() {
        authorize(
            &claims,
            true,
            Some(input.tenant_id.clone()),
            vec![Permissions::VOTER_SECRET_ATTRIBUTE_READ],
        )
        .map_err(pdf_authorization_error)?;
    }
    let outcome = initiate(
        &tx,
        &caller(&claims),
        tenant,
        event,
        &input.voter_id,
        initiation,
        &names,
        input.recover_request_id,
    )
    .await?;
    tx.commit().await.map_err(pdf_unavailable)?;
    kick_audit_log_outbox();
    drop(client);
    match outcome {
        // Hasura must get the typed 409 (not a plain-text webhook error).
        // This attempt may already have minted its one-use link.
        Delivery::Failed => Err(pdf_delivery_failed().into()),
        Delivery::Waiting(summary) => {
            kick_signing_log_outbox();
            Ok(Json(GetManualVerificationPdfOutput {
                status: "awaiting-signatures",
                document_id: None,
                signing_request: Some(summary),
            }))
        }
        Delivery::Document { id, dispatch } => {
            if dispatch {
                if !names.is_empty() {
                    post_voter_secret_attribute_audit(
                        &input.tenant_id,
                        &input.election_event_id,
                        &ElectoralLogAdminContext::from_claims(&claims),
                        VoterSecretAttributeAction::Report,
                        VoterSecretAttributeAudit {
                            voter_id: Some(&input.voter_id),
                            voter_username: None,
                            attribute_names: &names,
                            document_id: None,
                        },
                    )
                    .await
                    .map_err(pdf_unavailable)?;
                }
                // Sending again after a lost acknowledgement is safe: Windmill
                // atomically claims the initiation before asking for the one-use link.
                get_celery_app().await.send_task(windmill::tasks::manual_verification_report::generate_manual_verification_report::new(id.to_string(),input.tenant_id,input.election_event_id,input.voter_id,None,!names.is_empty())).await.map_err(pdf_unavailable)?;
            }
            Ok(Json(GetManualVerificationPdfOutput {
                status: "pending",
                document_id: Some(id),
                signing_request: None,
            }))
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthorityInput {
    tenant_id: Uuid,
    election_event_id: Uuid,
    voter_id: String,
    phase: Phase,
    initiation_id: Option<Uuid>,
    proof: Option<Uuid>,
}

// Internal Keycloak callback; deliberately not exposed as a Hasura action.
// Never echo a capability, raw request or token, even on validation failure.
#[instrument(skip_all)]
#[post("/manual-verification-authority", format = "json", data = "<body>")]
pub async fn manual_verification_authority(
    body: Json<AuthorityInput>,
    claims: JwtClaims,
) -> Result<Status, (Status, String)> {
    let b = body.into_inner();
    authorize(
        &claims,
        false,
        Some(b.tenant_id.to_string()),
        vec![Permissions::SERVICE_ACCOUNT],
    )?;
    let mut client =
        get_hasura_pool().await.get().await.map_err(unavailable)?;
    let tx = client.transaction().await.map_err(unavailable)?;
    let transitioned = authorize_token(
        &tx,
        b.tenant_id,
        b.election_event_id,
        &b.voter_id,
        b.phase,
        b.initiation_id,
        b.proof,
    )
    .await
    .map_err(|_| {
        (
            Status::Forbidden,
            "Manual verification not authorized.".into(),
        )
    })?;
    if let Some((initiation, request)) = transitioned {
        let (phase, description) = match b.phase {
            Phase::Issue => (
                "issue_claimed",
                "Claimed a manual-verification one-use link for issuance",
            ),
            Phase::Redeem => (
                "redeem_claimed",
                "Claimed a manual-verification one-use link for redemption",
            ),
        };
        let change = audit::change(
            &claims,
            AuditLogTarget {
                tenant_id: b.tenant_id,
                election_event_id: Some(b.election_event_id),
            },
            AuditObject::new("manual_verification_issuance", initiation, None),
            AdminAction::Update,
        )
        .with_extra(json!({"phase": phase, "signing_request_id": request}))
        .with_description(description);
        audit_log::stage(&tx, &change).await.map_err(unavailable)?;
        tx.commit().await.map_err(unavailable)?;
        kick_audit_log_outbox();
    } else {
        tx.rollback().await.map_err(unavailable)?;
    }
    Ok(Status::NoContent)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rocket::http::ContentType;

    #[get("/probe/<case>")]
    fn public_error_probe(case: &str) -> Guarded<JsonError> {
        match case {
            "failed" => pdf_delivery_failed().into(),
            "scope" => pdf_scope_id("not-a-uuid").err().unwrap().into(),
            "auth" => pdf_authorization_error((
                Status::Forbidden,
                "private auth detail".into(),
            ))
            .into(),
            "unavailable" => pdf_unavailable("private service detail").into(),
            _ => unreachable!(),
        }
    }

    #[rocket::async_test]
    async fn manual_verification_refusals_keep_status_and_use_hasura_json_error_contract(
    ) {
        let client = rocket::local::asynchronous::Client::tracked(
            rocket::build().mount("/", routes![public_error_probe]),
        )
        .await
        .unwrap();
        for (case, status, code, message) in [
            (
                "failed",
                Status::Conflict,
                "manual-verification-delivery-failed",
                "This document could not be delivered. Start a new authorization.",
            ),
            (
                "scope",
                Status::BadRequest,
                "UuidParseFailed",
                "Invalid election scope.",
            ),
            (
                "auth",
                Status::Forbidden,
                "Unauthorized",
                "Manual verification is not authorized.",
            ),
            (
                "unavailable",
                Status::ServiceUnavailable,
                "manual-verification-unavailable",
                "Manual verification is unavailable.",
            ),
        ] {
            let response = client.get(format!("/probe/{case}")).dispatch().await;
            assert_eq!(response.status(), status);
            assert_eq!(response.content_type(), Some(ContentType::JSON));
            let body = response.into_json::<serde_json::Value>().await.unwrap();
            assert_eq!(body, serde_json::json!({"message": message, "extensions": {"code": code}}));
        }
    }
}
