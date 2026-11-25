// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use crate::services::signing_gate::{caller, Guarded};
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use uuid::Uuid;
use windmill::postgres::reports::ReportType;
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

#[instrument(skip_all)]
#[post("/get-manual-verification-pdf", format = "json", data = "<body>")]
pub async fn get_manual_verification_pdf(
    body: Json<GetManualVerificationPdfInput>,
    claims: JwtClaims,
) -> Result<Json<GetManualVerificationPdfOutput>, Guarded<(Status, String)>> {
    let input = body.into_inner();
    authorize(
        &claims,
        true,
        Some(input.tenant_id.clone()),
        vec![Permissions::VOTER_MANUALLY_VERIFY],
    )?;
    let id = |s: &str| {
        Uuid::parse_str(s).map_err(|_| {
            (Status::BadRequest, "Invalid election scope.".to_owned())
        })
    };
    let tenant = id(&input.tenant_id)?;
    let event = id(&input.election_event_id)?;
    let initiation = input.initiation_id.unwrap_or_else(Uuid::new_v4);
    let mut client =
        get_hasura_pool().await.get().await.map_err(unavailable)?;
    let tx = client.transaction().await.map_err(unavailable)?;
    let names = get_declared_report_secret_attribute_names(
        &tx,
        &input.tenant_id,
        &input.election_event_id,
        &ReportType::MANUAL_VERIFICATION,
        None,
    )
    .await
    .map_err(unavailable)?;
    let mut names: Vec<_> = names.into_iter().collect();
    names.sort();
    if !names.is_empty() {
        authorize(
            &claims,
            true,
            Some(input.tenant_id.clone()),
            vec![Permissions::VOTER_SECRET_ATTRIBUTE_READ],
        )?;
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
    tx.commit().await.map_err(unavailable)?;
    drop(client);
    match outcome {
        Delivery::Failed => Err((
            Status::Conflict,
            "This document could not be delivered. Start a new authorization."
                .to_owned(),
        )
            .into()),
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
                    .map_err(unavailable)?;
                }
                // Sending again after a lost acknowledgement is safe: Windmill
                // atomically claims the initiation before asking for the one-use link.
                get_celery_app().await.send_task(windmill::tasks::manual_verification_report::generate_manual_verification_report::new(id.to_string(),input.tenant_id,input.election_event_id,input.voter_id,None,!names.is_empty())).await.map_err(unavailable)?;
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
    authorize_token(
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
    tx.commit().await.map_err(unavailable)?;
    Ok(Status::NoContent)
}
