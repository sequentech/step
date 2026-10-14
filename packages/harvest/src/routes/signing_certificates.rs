// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The Certificates settings of signing (trusted issuers, checks, registered
//! staff certificates) and the signing dialog's certificate dry run. Every
//! route answers 403 without its permission and a typed JSON error (design
//! §5, the signing API contract).

use crate::services::authorization::authorize;
use crate::services::dependencies::HarvestServices;
use crate::services::signing_http::{
    authorize_403, caller_display_name, SigningError, SigningResult,
};
use chrono::{DateTime, Utc};
use deadpool_postgres::Client as DbClient;
use rocket::serde::json::Json;
use rocket::State;
use sequent_core::services::jwt::{decode_permission_labels, JwtClaims};
use sequent_core::signing::{
    CertificateCheckId, CertificateCheckResult, CertificatePostBinding,
    CertificateRegistration, CrlUnavailablePolicy, RevocationCheck,
    RevocationStatus, SignatureAlgorithm, SigningAction, SigningChecks,
    SigningRequestStatus, StaffCertificateRegistration, StaffCertificateStatus,
};
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use strum::IntoEnumIterator;
use tracing::instrument;
use uuid::Uuid;
use windmill::postgres::signing_certificates::get_signing_request_in_tenant;
use windmill::services::signing::certificates::{
    parse_pem_or_der, CertificateVerification, OpensslCertificateVerifier,
    RegistrationState,
};
use windmill::services::signing::crl::HttpCrlFetcher;
use windmill::services::signing::directory::KeycloakUserDirectory;
use windmill::services::signing::issuers::{
    import_staff_issuers, remove_staff_issuer, update_signing_checks,
};
use windmill::services::signing::log::Actor;
use windmill::services::signing::staff_certificates::{
    check_certificate, register_staff_certificate, revoke_registration,
    RegistrationRefusalReason, RegistrationRefused, RevokeOutcome,
    StaffCertificateRegistrationInput, MAX_REVOKE_REASON_CHARS,
};
use windmill::services::signing::Allowance;

type RouteResult<T> = SigningResult<Json<T>>;

fn parse_id(id: &str, what: &str) -> SigningResult<Uuid> {
    Uuid::parse_str(id)
        .map_err(|err| SigningError::invalid(format!("Invalid {what}: {err}")))
}

fn tenant_of(claims: &JwtClaims) -> SigningResult<Uuid> {
    parse_id(&claims.hasura_claims.tenant_id, "tenant_id")
}

/// The caller's tenant, checked as [`authorize_403`] does.
fn own_tenant(claims: &JwtClaims) -> Option<String> {
    Some(claims.hasura_claims.tenant_id.clone())
}

fn actor(claims: &JwtClaims) -> Actor {
    Actor {
        user_id: claims.hasura_claims.user_id.clone(),
        username: claims
            .preferred_username
            .clone()
            .unwrap_or_else(|| claims.hasura_claims.user_id.clone()),
    }
}

async fn hasura(services: &HarvestServices) -> SigningResult<DbClient> {
    services
        .databases
        .hasura()
        .await
        .get()
        .await
        .map_err(SigningError::internal)
}

async fn directory(services: &HarvestServices) -> KeycloakUserDirectory {
    KeycloakUserDirectory::on(services.databases.keycloak().await)
}

// Trusted issuers

#[derive(Deserialize, Debug)]
pub struct ImportSigningIssuersInput {
    election_event_id: Uuid,
    /// PEM text: one or more certificates.
    pem: Option<String>,
    /// Or the base64 of one DER certificate (a .cer/.der file).
    der_base64: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ImportSigningIssuersOutput {
    /// How many issuers were imported.
    imported: usize,
    /// How many the event already trusted.
    skipped: usize,
    /// One line per certificate refused (not a CA, expired, unreadable).
    errors: Vec<String>,
}

#[instrument(skip(claims, input, services))]
#[post("/signing-issuers", format = "json", data = "<input>")]
pub async fn import_signing_issuers(
    claims: JwtClaims,
    input: Json<ImportSigningIssuersInput>,
    services: &State<HarvestServices>,
) -> RouteResult<ImportSigningIssuersOutput> {
    authorize_403(
        &claims,
        own_tenant(&claims),
        vec![Permissions::SIGNING_ISSUERS_WRITE],
    )?;
    let input = input.into_inner();
    let tenant_id = tenant_of(&claims)?;
    let certificates =
        parse_pem_or_der(input.pem.as_deref(), input.der_base64.as_deref())
            .map_err(|err| SigningError::invalid(format!("{err:#}")))?;
    let mut client = hasura(services).await?;
    let transaction =
        client.transaction().await.map_err(SigningError::internal)?;
    let import = import_staff_issuers(
        &transaction,
        tenant_id,
        input.election_event_id,
        &certificates,
        &actor(&claims),
        Allowance::Permission(Permissions::SIGNING_ISSUERS_WRITE),
        Utc::now(),
    )
    .await
    .map_err(SigningError::internal)?;
    transaction.commit().await.map_err(SigningError::internal)?;
    Ok(Json(ImportSigningIssuersOutput {
        imported: import.imported.len(),
        skipped: import.skipped.len(),
        errors: import.errors,
    }))
}

#[derive(Deserialize, Debug)]
pub struct RemoveSigningIssuerInput {
    election_event_id: Uuid,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RemoveSigningIssuerOutput {
    issuer_id: Uuid,
}

#[instrument(skip(claims, input, services))]
#[delete("/signing-issuers/<id>", format = "json", data = "<input>")]
pub async fn remove_signing_issuer(
    claims: JwtClaims,
    id: &str,
    input: Json<RemoveSigningIssuerInput>,
    services: &State<HarvestServices>,
) -> RouteResult<RemoveSigningIssuerOutput> {
    authorize_403(
        &claims,
        own_tenant(&claims),
        vec![Permissions::SIGNING_ISSUERS_WRITE],
    )?;
    let tenant_id = tenant_of(&claims)?;
    let id = parse_id(id, "issuer id")?;
    let mut client = hasura(services).await?;
    let transaction =
        client.transaction().await.map_err(SigningError::internal)?;
    let removed = remove_staff_issuer(
        &transaction,
        tenant_id,
        input.election_event_id,
        id,
        &actor(&claims),
    )
    .await
    .map_err(SigningError::internal)?
    .ok_or_else(|| SigningError::not_found("No such trusted issuer"))?;
    transaction.commit().await.map_err(SigningError::internal)?;
    Ok(Json(RemoveSigningIssuerOutput {
        issuer_id: removed.id,
    }))
}

// Checks

#[derive(Deserialize, Debug)]
pub struct PutSigningChecksInput {
    election_event_id: Uuid,
    revocation_check: RevocationCheck,
    crl_unavailable: CrlUnavailablePolicy,
    registration: CertificateRegistration,
    post_binding: CertificatePostBinding,
    /// The revision the change was made on; 0 before the first save.
    expected_revision: i64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PutSigningChecksOutput {
    revision: i64,
}

#[instrument(skip(claims, input, services))]
#[put("/signing-checks", format = "json", data = "<input>")]
pub async fn put_signing_checks(
    claims: JwtClaims,
    input: Json<PutSigningChecksInput>,
    services: &State<HarvestServices>,
) -> RouteResult<PutSigningChecksOutput> {
    authorize_403(
        &claims,
        own_tenant(&claims),
        vec![Permissions::SIGNING_CHECKS_WRITE],
    )?;
    let input = input.into_inner();
    let tenant_id = tenant_of(&claims)?;
    let checks = SigningChecks {
        revocation_check: input.revocation_check,
        crl_unavailable: input.crl_unavailable,
        registration: input.registration,
        post_binding: input.post_binding,
        revision: input.expected_revision,
    };
    let name =
        caller_display_name(&claims, tenant_id, &directory(services).await)
            .await?;
    let mut client = hasura(services).await?;
    let transaction =
        client.transaction().await.map_err(SigningError::internal)?;
    let row = update_signing_checks(
        &transaction,
        tenant_id,
        input.election_event_id,
        &checks,
        input.expected_revision,
        &actor(&claims),
        Some(&name),
    )
    .await
    .map_err(SigningError::internal)?
    .ok_or_else(|| {
        SigningError::conflict(
            "The checks changed since they were read; reload them",
        )
    })?;
    transaction.commit().await.map_err(SigningError::internal)?;
    Ok(Json(PutSigningChecksOutput {
        revision: row.checks.revision,
    }))
}

// Staff certificates

#[derive(Deserialize, Debug)]
pub struct RegisterStaffCertificateInput {
    election_event_id: Uuid,
    user_id: String,
    /// The Post the certificate signs for; absent: every Post.
    election_id: Option<Uuid>,
    /// The certificate, optionally followed by its intermediates.
    pem: String,
    /// The account the certificate is already registered to, to link it to
    /// the same person's second account (decided O1).
    linked_to: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StaffCertificateIdOutput {
    certificate_id: Uuid,
}

/// A Security Officer's refused registration, as the contract's codes say it.
fn registration_error(refused: RegistrationRefused) -> SigningError {
    use RegistrationRefusalReason as Reason;
    let reason = refused.reason.to_string();
    let check = match refused.reason {
        Reason::UntrustedIssuer => CertificateCheckId::TrustedIssuer,
        Reason::NotValidNow => CertificateCheckId::ValidNow,
        Reason::NotForSigning => CertificateCheckId::SigningKeyUsage,
        Reason::Revoked => CertificateCheckId::Registered,
        Reason::RegisteredToOther => {
            // The contract names the account holding the key or holder.
            let mut error = SigningError::refused(
                CertificateCheckId::RegisteredToOther,
                refused.detail,
            )
            .with("reason", reason);
            if let Some(holder) = refused.other_holder {
                error = error
                    .with("user_id", holder.user_id)
                    .with("display_name", holder.display_name);
            }
            return error;
        }
        Reason::UnknownPost => return SigningError::not_found(refused.detail),
        Reason::AlreadyRegistered => {
            return SigningError::conflict(refused.detail)
        }
        Reason::Unreadable | Reason::NothingToLink => {
            return SigningError::invalid(refused.detail).with("reason", reason)
        }
    };
    SigningError::refused(check, refused.detail).with("reason", reason)
}

#[instrument(skip(claims, input, services))]
#[post("/staff-certificates", format = "json", data = "<input>")]
pub async fn register_staff_certificate_route(
    claims: JwtClaims,
    input: Json<RegisterStaffCertificateInput>,
    services: &State<HarvestServices>,
) -> RouteResult<StaffCertificateIdOutput> {
    authorize_403(
        &claims,
        own_tenant(&claims),
        vec![Permissions::SIGNING_CERTIFICATES_REGISTER],
    )?;
    let input = input.into_inner();
    let tenant_id = tenant_of(&claims)?;
    let directory = directory(services).await;
    let holder = directory
        .people(tenant_id, std::slice::from_ref(&input.user_id))
        .await
        .map_err(SigningError::internal)?
        .remove(&input.user_id)
        .ok_or_else(|| SigningError::not_found("No such user"))?;
    let officer_name =
        caller_display_name(&claims, tenant_id, &directory).await?;
    let mut client = hasura(services).await?;
    let transaction =
        client.transaction().await.map_err(SigningError::internal)?;
    let outcome = register_staff_certificate(
        &transaction,
        &StaffCertificateRegistrationInput {
            tenant_id,
            election_event_id: input.election_event_id,
            holder: Actor {
                user_id: input.user_id,
                username: holder.username,
            },
            holder_name: Some(holder.display_name),
            election_id: input.election_id,
            chain_pem: vec![input.pem],
            linked_to: input.linked_to,
        },
        &actor(&claims),
        Some(&officer_name),
        Utc::now(),
    )
    .await
    .map_err(SigningError::internal)?;
    let row = outcome.map_err(registration_error)?;
    transaction.commit().await.map_err(SigningError::internal)?;
    Ok(Json(StaffCertificateIdOutput {
        certificate_id: row.id,
    }))
}

#[derive(Deserialize, Debug)]
pub struct RevokeStaffCertificateInput {
    election_event_id: Uuid,
    reason: String,
}

#[instrument(skip(claims, input, services))]
#[post("/staff-certificates/<id>/revoke", format = "json", data = "<input>")]
pub async fn revoke_staff_certificate_route(
    claims: JwtClaims,
    id: &str,
    input: Json<RevokeStaffCertificateInput>,
    services: &State<HarvestServices>,
) -> RouteResult<StaffCertificateIdOutput> {
    authorize_403(
        &claims,
        own_tenant(&claims),
        vec![Permissions::SIGNING_CERTIFICATES_REVOKE],
    )?;
    let input = input.into_inner();
    let tenant_id = tenant_of(&claims)?;
    let id = parse_id(id, "certificate id")?;
    let reason = input.reason.trim();
    if reason.is_empty() || reason.chars().count() > MAX_REVOKE_REASON_CHARS {
        return Err(SigningError::invalid(format!(
            "A revocation needs a reason of at most {MAX_REVOKE_REASON_CHARS} characters"
        )));
    }
    let officer_name =
        caller_display_name(&claims, tenant_id, &directory(services).await)
            .await?;
    let mut client = hasura(services).await?;
    let transaction =
        client.transaction().await.map_err(SigningError::internal)?;
    let outcome = revoke_registration(
        &transaction,
        tenant_id,
        input.election_event_id,
        id,
        reason,
        &actor(&claims),
        Some(&officer_name),
    )
    .await
    .map_err(SigningError::internal)?;
    match outcome {
        RevokeOutcome::Revoked {
            certificate: row, ..
        } => {
            transaction.commit().await.map_err(SigningError::internal)?;
            Ok(Json(StaffCertificateIdOutput {
                certificate_id: row.id,
            }))
        }
        RevokeOutcome::NotFound => {
            Err(SigningError::not_found("No such registered certificate"))
        }
        RevokeOutcome::AlreadyRevoked(_) => {
            Err(SigningError::conflict("The certificate is already revoked"))
        }
    }
}

// The dry run

#[derive(Deserialize, Debug)]
pub struct CheckCertificateInput {
    chain_pem: Vec<String>,
}

/// The certificate card of the signing dialog.
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct CheckedCertificate {
    pub common_name: String,
    pub subject: String,
    pub issuer: String,
    pub serial: String,
    pub not_before: DateTime<Utc>,
    pub not_after: DateTime<Utc>,
    pub fingerprint_sha256: String,
    pub spki_sha256: String,
    pub key_algorithm: Option<SignatureAlgorithm>,
}

/// How the certificate stands with the signer.
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Copy)]
#[serde(rename_all = "kebab-case")]
pub enum CheckedRegistration {
    Registered,
    FirstUse,
    NotRegistered,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CheckCertificateOutput {
    /// Every check but the signature, in the dialog's order.
    checks: Vec<CertificateCheckResult>,
    certificate: Option<CheckedCertificate>,
    registration: CheckedRegistration,
    revocation_status: RevocationStatus,
}

impl From<CertificateVerification> for CheckCertificateOutput {
    fn from(verification: CertificateVerification) -> Self {
        CheckCertificateOutput {
            checks: verification.checks,
            certificate: verification.certificate.map(|identity| {
                CheckedCertificate {
                    common_name: identity.common_name,
                    subject: identity.subject,
                    issuer: identity.issuer,
                    serial: identity.serial,
                    not_before: identity.not_before,
                    not_after: identity.not_after,
                    fingerprint_sha256: identity.fingerprint_sha256,
                    spki_sha256: identity.spki_sha256,
                    key_algorithm: identity.key_algorithm,
                }
            }),
            registration: match verification.registration {
                RegistrationState::Registered(_) => {
                    CheckedRegistration::Registered
                }
                RegistrationState::FirstUse => CheckedRegistration::FirstUse,
                RegistrationState::NotRegistered => {
                    CheckedRegistration::NotRegistered
                }
            },
            revocation_status: verification.revocation_status,
        }
    }
}

/// Whether a person with `labels` sees a Post labelled `label`, as Hasura
/// shows Posts: the unlabelled ones and those with one of their labels. A
/// person without labels sees only the unlabelled ones.
fn sees_post(labels: &[String], label: Option<&str>) -> bool {
    match label {
        None => true,
        Some(label) => labels.iter().any(|own| own == label),
    }
}

/// Runs every certificate check but the signature for the signed-in signer.
/// Needs the request action's sign permission and access to its Post; the
/// request must be waiting.
#[instrument(skip(claims, input, services))]
#[post(
    "/signing-requests/<id>/check-certificate",
    format = "json",
    data = "<input>"
)]
pub async fn check_signing_certificate(
    claims: JwtClaims,
    id: &str,
    input: Json<CheckCertificateInput>,
    services: &State<HarvestServices>,
) -> RouteResult<CheckCertificateOutput> {
    authorize(&claims, true, own_tenant(&claims), vec![])?;
    // Someone who signs nothing learns nothing, not even that the request
    // exists.
    let roles = &claims.hasura_claims.allowed_roles;
    let signs_anything = SigningAction::iter()
        .any(|action| roles.contains(&action.sign_permission().to_string()));
    if !signs_anything {
        return Err(SigningError::not_found("No such signing request"));
    }
    let tenant_id = tenant_of(&claims)?;
    let id = parse_id(id, "request id")?;
    let mut client = hasura(services).await?;
    let request = {
        let transaction =
            client.transaction().await.map_err(SigningError::internal)?;
        get_signing_request_in_tenant(&transaction, tenant_id, id)
            .await
            .map_err(SigningError::internal)?
            .ok_or_else(|| SigningError::not_found("No such signing request"))?
    };
    authorize_403(
        &claims,
        own_tenant(&claims),
        vec![request.action.sign_permission()],
    )?;
    if !sees_post(
        &decode_permission_labels(&claims),
        request.permission_label.as_deref(),
    ) {
        return Err(SigningError::forbidden(
            "The request is outside your Posts",
        ));
    }
    let now = Utc::now();
    if request.status != SigningRequestStatus::Waiting {
        return Err(SigningError::request_closed(request.status));
    }
    if request
        .expires_at
        .is_some_and(|expires_at| expires_at <= now)
    {
        return Err(SigningError::request_closed(
            SigningRequestStatus::Expired,
        ));
    }
    let verifier = OpensslCertificateVerifier::with_directory(Arc::new(
        directory(services).await,
    ));
    let verification = check_certificate(
        &mut client,
        &verifier,
        &HttpCrlFetcher::default(),
        &request,
        actor(&claims),
        input.into_inner().chain_pem,
        now,
    )
    .await
    .map_err(SigningError::internal)?;
    Ok(Json(verification.into()))
}

#[cfg(test)]
mod tests {
    use super::sees_post;

    #[test]
    fn a_post_is_seen_when_unlabelled_or_with_its_label() {
        let labels = vec!["post-a".to_owned()];
        // Without labels, as in Hasura: only the unlabelled Posts.
        assert!(!sees_post(&[], Some("post-a")));
        assert!(sees_post(&[], None));
        assert!(sees_post(&labels, None));
        assert!(sees_post(&labels, Some("post-a")));
        assert!(!sees_post(&labels, Some("post-b")));
    }
}

#[cfg(test)]
#[path = "../../tests/support/signing_certificate_routes.rs"]
mod signing_certificate_routes;
