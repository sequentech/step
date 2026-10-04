// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Signing requests and rules (design §5). The portal reaches these routes
//! through Hasura actions, so each takes a JSON body with the ids it acts
//! on; the tenant is always the caller's.
//!
//! Errors are JSON `{message, extensions: {code}}`: `forbidden` (403, a
//! missing permission), `not-found` (404), `invalid` (400, with a `reason`
//! such as `over-capacity`), `locked-down` (409, a rule edit of a locked-down
//! event), `conflict` (409, a stale rule revision),
//! `request-closed` (409, with the request's `status`), `stale-revision`
//! (409, the PDF revision a signer prepared no longer extends the document:
//! prepare again), and for a refused
//! signature, which is logged, `signing-refused` or `already-signed` (422,
//! with the refusing `check`).
//!
//! | Route | Permission |
//! | --- | --- |
//! | `/signing-requests/get` | the requester, `sign-<action>` or `signing-requests-read` (Post by labels) |
//! | `/signing-requests/approve` | `sign-<action>` (Post by labels) |
//! | `/signing-requests/pdf-prepare` | `sign-<action>` (Post by labels) |
//! | `/signing-requests/open-failures` | `sign-<action>` (Post by labels) |
//! | `/signing-requests/handover` | the requester or `sign-<action>` |
//! | `/signing-requests/cancel` | the requester or `signing-requests-cancel` (Post by labels) |
//! | `/signing-requests/export` | `signing-requests-export` (Posts by labels) |
//! | `/signing-rules/put` | `signing-rules-write` (+ `role-read` and `role-write` to change who signs) |
//! | `/signing-rules/capacity` | `signing-rules-read` |
//! | `/signing-event-info` | a `signing-*-read` or `sign-<action>`; the titles `signing-certificates-read` |

use crate::services::dependencies::HarvestServices;
use crate::services::signing_http::{
    authorize_403, caller_display_name, SigningError as SigningFailure,
    SigningErrorCode,
};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use chrono::Utc;
use deadpool_postgres::{Client, Transaction};
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::signing::{
    CertificateCheckId, CertificateOpenFailure, RequesterSigning,
    SignatureAlgorithm, SigningAction, SigningRequestStatus,
    SigningRequirement,
};
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::{instrument, warn};
use uuid::Uuid;
use windmill::postgres::signing_certificates::get_signing_request_in_tenant;
use windmill::services::signing::approve::{
    approve, ApproveInput, ApproveOutcome,
};
use windmill::services::signing::crl::{refresh_chain_crls, HttpCrlFetcher};
use windmill::services::signing::directory::KeycloakUserDirectory;
use windmill::services::signing::pdf::PdfPrepared;
use windmill::services::signing::requests::{
    cancel, event_info, export_requests, get_panel, handover, reads_event_info,
    refusal_is_logged, report_open_failure, ExportFilter, SigningEventInfo,
    SigningPanel,
};
use windmill::services::signing::rules::{
    capacity, commit_rule, save_rule, SaveRuleInput, SaveRuleOutcome,
    SigningCapacity,
};
use windmill::services::signing::signers::GroupChange;
use windmill::services::signing::{InvalidReason, SigningCaller, SigningError};
use windmill::tasks::signing_log_outbox::kick_signing_log_outbox;

pub type SigningReply<T> = Result<Json<T>, SigningFailure>;

/// The answer to a signing step that wasn't taken, in the contract's
/// error body.
pub fn signing_failure(error: SigningError) -> SigningFailure {
    match error {
        SigningError::NotFound(message) => SigningFailure::not_found(message),
        SigningError::Forbidden(message) => SigningFailure::forbidden(message),
        SigningError::Invalid {
            reason: InvalidReason::LockedDown,
            message,
        } => SigningFailure::new(
            Status::Conflict,
            SigningErrorCode::LockedDown,
            message,
        ),
        SigningError::Invalid { reason, message } => {
            SigningFailure::invalid(message).with("reason", reason.to_string())
        }
        SigningError::Conflict(message) => SigningFailure::conflict(message),
        SigningError::StaleRevision(message) => SigningFailure::new(
            Status::Conflict,
            SigningErrorCode::StaleRevision,
            message,
        ),
        SigningError::Closed { status, message } => SigningFailure::new(
            Status::Conflict,
            SigningErrorCode::RequestClosed,
            message,
        )
        .with("status", status.to_string()),
        SigningError::Refused {
            check: CertificateCheckId::AlreadySigned,
            message,
            ..
        } => SigningFailure::new(
            Status::UnprocessableEntity,
            SigningErrorCode::AlreadySigned,
            message,
        )
        .with("check", CertificateCheckId::AlreadySigned.to_string()),
        SigningError::Refused {
            check,
            message,
            other_holder,
        } => {
            let refused = SigningFailure::refused(check, message);
            match other_holder {
                Some(holder) => refused
                    .with("user_id", holder.user_id)
                    .with("display_name", holder.display_name),
                None => refused,
            }
        }
        SigningError::Internal(error) => SigningFailure::internal(error),
    }
}

fn internal(error: impl std::fmt::Debug) -> SigningFailure {
    SigningFailure::internal(error)
}

/// The caller's tenant as a UUID.
fn tenant_of(claims: &JwtClaims) -> Result<Uuid, SigningFailure> {
    Uuid::parse_str(&claims.hasura_claims.tenant_id).map_err(|_| {
        SigningFailure::new(
            Status::Unauthorized,
            SigningErrorCode::Unauthorized,
            "The token's tenant is not a UUID.",
        )
    })
}

/// Checks `permissions` in the caller's own tenant: 403 when one is
/// missing.
fn require(
    claims: &JwtClaims,
    permissions: Vec<Permissions>,
) -> Result<(), SigningFailure> {
    authorize_403(
        claims,
        Some(claims.hasura_claims.tenant_id.clone()),
        permissions,
    )
}

fn uuid_of(text: &str, name: &str) -> Result<Uuid, SigningFailure> {
    Uuid::parse_str(text)
        .map_err(|_| SigningFailure::invalid(format!("{name} is not a UUID")))
}

async fn hasura(services: &HarvestServices) -> Result<Client, SigningFailure> {
    services
        .databases
        .hasura()
        .await
        .get()
        .await
        .map_err(internal)
}

async fn keycloak(
    services: &HarvestServices,
) -> Result<Client, SigningFailure> {
    services
        .databases
        .keycloak()
        .await
        .get()
        .await
        .map_err(internal)
}

/// The caller of a step that writes, with the display name the rows keep.
async fn writer(
    claims: &JwtClaims,
    tenant_id: Uuid,
    services: &HarvestServices,
) -> Result<SigningCaller, SigningFailure> {
    let mut caller = SigningCaller::from_claims(claims);
    let directory =
        KeycloakUserDirectory::on(services.databases.keycloak().await);
    caller.display_name =
        caller_display_name(claims, tenant_id, &directory).await?;
    Ok(caller)
}

/// Ends a cancel, handover or open-failure report: commits it when it was
/// taken, and when it was refused with a logged refusal (see
/// [`refusal_is_logged`]); any other failure rolls it back.
async fn finish_step<T>(
    transaction: Transaction<'_>,
    result: Result<T, SigningError>,
) -> Result<T, SigningFailure> {
    let keep = match &result {
        Ok(_) => true,
        Err(error) => refusal_is_logged(error),
    };
    if keep {
        transaction.commit().await.map_err(internal)?;
        kick_signing_log_outbox();
    }
    result.map_err(signing_failure)
}

#[derive(Debug, Deserialize)]
pub struct RequestIdInput {
    request_id: String,
}

#[derive(Debug, Serialize)]
pub struct RequestIdOutput {
    request_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct PanelOutput {
    panel: SigningPanel,
}

#[instrument(skip(claims, services))]
#[post("/signing-requests/get", format = "json", data = "<body>")]
pub async fn get_signing_request(
    claims: JwtClaims,
    services: &State<HarvestServices>,
    body: Json<RequestIdInput>,
) -> SigningReply<PanelOutput> {
    let tenant_id = tenant_of(&claims)?;
    let request_id = uuid_of(&body.request_id, "request_id")?;
    let caller = SigningCaller::from_claims(&claims);
    let mut hasura_client = hasura(services).await?;
    let mut keycloak_client = keycloak(services).await?;
    let hasura_transaction =
        hasura_client.transaction().await.map_err(internal)?;
    let keycloak_transaction =
        keycloak_client.transaction().await.map_err(internal)?;
    let panel = get_panel(
        &hasura_transaction,
        &keycloak_transaction,
        services.signing.documents.as_ref(),
        &caller,
        tenant_id,
        request_id,
    )
    .await
    .map_err(signing_failure)?;
    Ok(Json(PanelOutput { panel }))
}

#[derive(Debug, Deserialize)]
pub struct ApproveBody {
    request_id: String,
    chain_pem: Vec<String>,
    algorithm: SignatureAlgorithm,
    payload_signature_b64: String,
    #[serde(default)]
    document_signature_b64: Option<String>,
    #[serde(default)]
    pdf_cms_b64: Option<String>,
    #[serde(default)]
    revision: Option<i32>,
}

fn base64_of(text: &str, name: &str) -> Result<Vec<u8>, SigningFailure> {
    BASE64
        .decode(text.trim())
        .map_err(|_| SigningFailure::invalid(format!("{name} is not base64")))
}

#[instrument(skip(claims, services, body))]
#[post("/signing-requests/approve", format = "json", data = "<body>")]
pub async fn approve_signing_request(
    claims: JwtClaims,
    services: &State<HarvestServices>,
    body: Json<ApproveBody>,
) -> SigningReply<ApproveOutcome> {
    let tenant_id = tenant_of(&claims)?;
    let body = body.into_inner();
    let input = ApproveInput {
        request_id: uuid_of(&body.request_id, "request_id")?,
        chain_pem: body.chain_pem,
        algorithm: body.algorithm,
        payload_signature: base64_of(
            &body.payload_signature_b64,
            "payload_signature_b64",
        )?,
        document_signature: body
            .document_signature_b64
            .as_deref()
            .map(|text| base64_of(text, "document_signature_b64"))
            .transpose()?,
        pdf_cms: body
            .pdf_cms_b64
            .as_deref()
            .map(|text| base64_of(text, "pdf_cms_b64"))
            .transpose()?,
        revision: body.revision,
    };
    let caller = writer(&claims, tenant_id, services).await?;
    let mut client = hasura(services).await?;
    // The chain's revocation lists, outside any lock: a download failure
    // is left to the checks, which refuse or accept per the event.
    let request = {
        let transaction = client.transaction().await.map_err(internal)?;
        get_signing_request_in_tenant(&transaction, tenant_id, input.request_id)
            .await
            .map_err(internal)?
    };
    if let Some(request) = request {
        if let Err(error) = refresh_chain_crls(
            &mut client,
            tenant_id,
            request.election_event_id,
            &input.chain_pem,
            &HttpCrlFetcher::default(),
            Utc::now(),
        )
        .await
        {
            warn!("Refreshing the signer's revocation lists failed: {error:?}");
        }
    }
    let outcome = approve(
        &mut client,
        &services.signing,
        &caller,
        tenant_id,
        &input,
        Utc::now(),
    )
    .await
    .map_err(signing_failure)?;
    Ok(Json(outcome))
}

#[derive(Debug, Deserialize)]
pub struct OpenFailureBody {
    request_id: String,
    file_name: String,
    reason: CertificateOpenFailure,
}

#[instrument(skip(claims, services))]
#[post("/signing-requests/open-failures", format = "json", data = "<body>")]
pub async fn report_signing_open_failure(
    claims: JwtClaims,
    services: &State<HarvestServices>,
    body: Json<OpenFailureBody>,
) -> SigningReply<RequestIdOutput> {
    let tenant_id = tenant_of(&claims)?;
    let request_id = uuid_of(&body.request_id, "request_id")?;
    let caller = writer(&claims, tenant_id, services).await?;
    let mut client = hasura(services).await?;
    let transaction = client.transaction().await.map_err(internal)?;
    let result = report_open_failure(
        &transaction,
        &caller,
        tenant_id,
        request_id,
        &body.file_name,
        body.reason,
    )
    .await;
    finish_step(transaction, result).await?;
    Ok(Json(RequestIdOutput { request_id }))
}

#[instrument(skip(claims, services))]
#[post("/signing-requests/handover", format = "json", data = "<body>")]
pub async fn handover_signing_request(
    claims: JwtClaims,
    services: &State<HarvestServices>,
    body: Json<RequestIdInput>,
) -> SigningReply<RequestIdOutput> {
    let tenant_id = tenant_of(&claims)?;
    let request_id = uuid_of(&body.request_id, "request_id")?;
    let caller = writer(&claims, tenant_id, services).await?;
    let mut client = hasura(services).await?;
    let transaction = client.transaction().await.map_err(internal)?;
    let result = handover(&transaction, &caller, tenant_id, request_id).await;
    finish_step(transaction, result).await?;
    Ok(Json(RequestIdOutput { request_id }))
}

#[derive(Debug, Deserialize)]
pub struct CancelBody {
    request_id: String,
    /// A note for the log.
    #[serde(default)]
    reason: Option<String>,
}

#[instrument(skip(claims, services))]
#[post("/signing-requests/cancel", format = "json", data = "<body>")]
pub async fn cancel_signing_request(
    claims: JwtClaims,
    services: &State<HarvestServices>,
    body: Json<CancelBody>,
) -> SigningReply<RequestIdOutput> {
    let tenant_id = tenant_of(&claims)?;
    let request_id = uuid_of(&body.request_id, "request_id")?;
    let caller = writer(&claims, tenant_id, services).await?;
    let mut client = hasura(services).await?;
    let transaction = client.transaction().await.map_err(internal)?;
    let result = cancel(
        &transaction,
        &caller,
        tenant_id,
        request_id,
        body.reason.as_deref(),
    )
    .await;
    finish_step(transaction, result).await?;
    Ok(Json(RequestIdOutput { request_id }))
}

#[derive(Debug, Deserialize)]
pub struct ExportBody {
    election_event_id: String,
    #[serde(default)]
    status: Option<SigningRequestStatus>,
    #[serde(default)]
    action: Option<SigningAction>,
}

#[derive(Debug, Serialize)]
pub struct ExportOutput {
    document_id: String,
    sha256: String,
    rows: usize,
    /// A short-lived download link: readers who may export but not read
    /// documents download the file with it.
    url: Option<String>,
}

#[instrument(skip(claims, services))]
#[post("/signing-requests/export", format = "json", data = "<body>")]
pub async fn export_signing_requests(
    claims: JwtClaims,
    services: &State<HarvestServices>,
    body: Json<ExportBody>,
) -> SigningReply<ExportOutput> {
    require(&claims, vec![Permissions::SIGNING_REQUESTS_EXPORT])?;
    let tenant_id = tenant_of(&claims)?;
    let election_event_id =
        uuid_of(&body.election_event_id, "election_event_id")?;
    let caller = writer(&claims, tenant_id, services).await?;
    let mut client = hasura(services).await?;
    let transaction = client.transaction().await.map_err(internal)?;
    let export = export_requests(
        &transaction,
        &caller,
        tenant_id,
        election_event_id,
        &ExportFilter {
            status: body.status,
            action: body.action,
        },
        Utc::now(),
    )
    .await
    .map_err(signing_failure)?;
    let stored = services
        .signing
        .exports
        .store(
            &transaction,
            tenant_id,
            election_event_id,
            &export.file_name,
            export.content.as_bytes(),
        )
        .await
        .map_err(internal)?;
    transaction.commit().await.map_err(internal)?;
    kick_signing_log_outbox();
    Ok(Json(ExportOutput {
        document_id: stored.document_id,
        sha256: export.sha256,
        rows: export.rows,
        url: stored.url,
    }))
}

#[derive(Debug, Deserialize)]
pub struct RoleChangesBody {
    #[serde(default)]
    add: Vec<String>,
    #[serde(default)]
    remove: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct SaveRuleBody {
    election_event_id: String,
    action: SigningAction,
    requirement: SigningRequirement,
    signatures: u16,
    requester_signing: RequesterSigning,
    expires_minutes: Option<u32>,
    #[serde(default)]
    roles: Option<RoleChangesBody>,
    expected_revision: i64,
}

#[instrument(skip(claims, services))]
#[post("/signing-rules/put", format = "json", data = "<body>")]
pub async fn save_signing_rule(
    claims: JwtClaims,
    services: &State<HarvestServices>,
    body: Json<SaveRuleBody>,
) -> SigningReply<SaveRuleOutcome> {
    let body = body.into_inner();
    let roles = body.roles.map(|roles| GroupChange {
        add: roles.add,
        remove: roles.remove,
    });
    let changes_roles = roles
        .as_ref()
        .is_some_and(|roles| !roles.add.is_empty() || !roles.remove.is_empty());
    let mut permissions = vec![Permissions::SIGNING_RULES_WRITE];
    if changes_roles {
        permissions.push(Permissions::ROLE_READ);
        permissions.push(Permissions::ROLE_WRITE);
    }
    require(&claims, permissions)?;
    let tenant_id = tenant_of(&claims)?;
    let election_event_id =
        uuid_of(&body.election_event_id, "election_event_id")?;
    let caller = writer(&claims, tenant_id, services).await?;
    if changes_roles {
        // Sign in to Keycloak before any lock is taken.
        services.signing_roles.prepare().await.map_err(internal)?;
    }
    let mut hasura_client = hasura(services).await?;
    let mut keycloak_client = keycloak(services).await?;
    let hasura_transaction =
        hasura_client.transaction().await.map_err(internal)?;
    let keycloak_transaction =
        keycloak_client.transaction().await.map_err(internal)?;
    let saved = save_rule(
        &hasura_transaction,
        &keycloak_transaction,
        &caller,
        tenant_id,
        election_event_id,
        &SaveRuleInput {
            action: body.action,
            requirement: body.requirement,
            signatures: body.signatures,
            requester_signing: body.requester_signing,
            expires_minutes: body.expires_minutes,
            roles,
            expected_revision: body.expected_revision,
        },
    )
    .await
    .map_err(signing_failure)?;
    commit_rule(
        hasura_transaction,
        services.signing_roles.as_ref(),
        &saved.roles,
    )
    .await
    .map_err(signing_failure)?;
    kick_signing_log_outbox();
    Ok(Json(saved.outcome))
}

#[derive(Debug, Deserialize)]
pub struct CapacityBody {
    election_event_id: String,
    action: SigningAction,
    /// Short Posts have fewer signers than this; the saved rule's number
    /// by default.
    #[serde(default)]
    signatures: Option<u16>,
    /// Whether the requester may sign; the saved rule's by default.
    #[serde(default)]
    requester_signing: Option<RequesterSigning>,
}

#[instrument(skip(claims, services))]
#[post("/signing-rules/capacity", format = "json", data = "<body>")]
pub async fn signing_rule_capacity(
    claims: JwtClaims,
    services: &State<HarvestServices>,
    body: Json<CapacityBody>,
) -> SigningReply<SigningCapacity> {
    require(&claims, vec![Permissions::SIGNING_RULES_READ])?;
    let tenant_id = tenant_of(&claims)?;
    let election_event_id =
        uuid_of(&body.election_event_id, "election_event_id")?;
    let mut hasura_client = hasura(services).await?;
    let mut keycloak_client = keycloak(services).await?;
    let hasura_transaction =
        hasura_client.transaction().await.map_err(internal)?;
    let keycloak_transaction =
        keycloak_client.transaction().await.map_err(internal)?;
    let capacity = capacity(
        &hasura_transaction,
        &keycloak_transaction,
        tenant_id,
        election_event_id,
        body.action,
        &GroupChange::default(),
        body.signatures,
        body.requester_signing,
    )
    .await
    .map_err(|error| signing_failure(SigningError::Internal(error)))?;
    Ok(Json(capacity))
}

#[derive(Debug, Deserialize)]
pub struct EventInfoBody {
    election_event_id: String,
}

/// The event's time zone, and for a reader of the certificates the
/// signers' titles: what the Signatures tab and a signer's list of waiting
/// requests show beside the rows Hasura gives them.
#[instrument(skip(claims, services))]
#[post("/signing-event-info", format = "json", data = "<body>")]
pub async fn signing_event_info(
    claims: JwtClaims,
    services: &State<HarvestServices>,
    body: Json<EventInfoBody>,
) -> SigningReply<SigningEventInfo> {
    require(&claims, vec![])?;
    let caller = SigningCaller::from_claims(&claims);
    // Any one of several permissions reads it.
    if !reads_event_info(&caller) {
        return Err(signing_failure(SigningError::Forbidden(
            "Reading it needs a signing read or sign permission.".into(),
        )));
    }
    let tenant_id = tenant_of(&claims)?;
    let election_event_id =
        uuid_of(&body.election_event_id, "election_event_id")?;
    let mut hasura_client = hasura(services).await?;
    let mut keycloak_client = keycloak(services).await?;
    let hasura_transaction =
        hasura_client.transaction().await.map_err(internal)?;
    let keycloak_transaction =
        keycloak_client.transaction().await.map_err(internal)?;
    let info = event_info(
        &hasura_transaction,
        &keycloak_transaction,
        &caller,
        tenant_id,
        election_event_id,
    )
    .await
    .map_err(signing_failure)?;
    Ok(Json(info))
}

#[derive(Debug, Deserialize)]
pub struct PdfPrepareBody {
    request_id: String,
    /// The signing certificate first, then intermediates.
    chain_pem: Vec<String>,
}

/// PDF mode: the revision that fills the caller's signature field, whose
/// digest the browser signs. The `revision` goes back with the approval.
#[instrument(skip(claims, services, body))]
#[post("/signing-requests/pdf-prepare", format = "json", data = "<body>")]
pub async fn prepare_signing_pdf(
    claims: JwtClaims,
    services: &State<HarvestServices>,
    body: Json<PdfPrepareBody>,
) -> SigningReply<PdfPrepared> {
    let tenant_id = tenant_of(&claims)?;
    let request_id = uuid_of(&body.request_id, "request_id")?;
    let caller = writer(&claims, tenant_id, services).await?;
    let mut client = hasura(services).await?;
    let prepared = services
        .signing
        .documents
        .prepare_pdf(
            &mut client,
            &caller,
            tenant_id,
            request_id,
            &body.chain_pem,
            Utc::now(),
        )
        .await
        .map_err(signing_failure)?;
    Ok(Json(prepared))
}

#[cfg(test)]
#[path = "../../tests/support/signing_routes.rs"]
mod route_tests;
