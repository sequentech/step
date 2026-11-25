// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! One signature of a request, in one transaction (design §5).
//!
//! The event's signing lock, then the request's row lock, serialize the
//! approvals of a request, so the signature that completes it is taken
//! once and its action runs once. A refused signature, whether the signer
//! may not sign or a check refused the certificate, is logged and
//! committed (at most once every few seconds per person and request), and
//! answered. The per-request uniqueness of account, certificate, key and
//! holder (§5a) is checked here before the approval is inserted; the
//! table's constraints are the backstop. The certificates of every counted
//! signature are share-locked, so a revocation waits for the signature or
//! the signature sees it: a revoked one cancels the request
//! (`certificate-revoked`).

use super::certificates::{
    register_first_use, CertificateIdentity, CertificateVerification, CertificateVerificationInput,
    CertificateVerifier, DocumentSignatureInput, RegistrationState, SignatureInputs,
};
#[cfg(feature = "comelec-demo-direct-hsm")]
use super::direct_actions::{self, VerifiedActionProof};
#[cfg(feature = "comelec-demo-direct-hsm")]
use super::direct_initialize::{self, VerifiedDirectProof};
use super::executors::{ExecutionOutcome, PostCommit, SigningExecutorRegistry};
use super::log::{Actor, SystemOutcome};
use super::operational_approval::{OperationalApprovalInput, OperationalEvidence};
use super::operational_policy::local_scope::request_mode_and_latch;
use super::operational_policy::{signing_mode, OperationalSigningMode};
use super::pdf::{PanelDocument, PdfPrepared};
use super::requests::{
    cancel_request, execution_failure, expire_request, is_overdue, is_the_trustee, read_request,
    relock, stage_executed, stage_request_step, throttled, SigningExportStore,
};
use super::{InvalidReason, SigningCaller, SigningError, SigningResult};
use crate::postgres::signing::*;
use crate::services::audit_log::{act_for, AuditActor, AuditOrigin};
use crate::tasks::audit_log_outbox::kick_audit_log_outbox;
use crate::tasks::signing_log_outbox::kick_signing_log_outbox;
use anyhow::Context;
use async_trait::async_trait;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use chrono::{DateTime, Utc};
use deadpool_postgres::{Client, Transaction};
use electoral_log::messages::newtypes::SigningStatementKind;
use sequent_core::signing::{
    sha256_hex, CancelReason, CertificateCheckId, DocumentKind, ExecutionMode, RequesterSigning,
    SignatureAlgorithm, SigningRequestStatus, SigningRule, StaffCertificateStatus,
};
use serde::Serialize;
use serde_json::{json, Value};
use std::sync::Arc;
use strum_macros::Display;
use tracing::{instrument, warn};
use uuid::Uuid;

/// The document side of a signature: the bytes an EML signature covers,
/// and the PDF revision a CMS signs.
#[async_trait]
pub trait DocumentSigner: Send + Sync {
    /// Whether this signer takes document signatures of `kind`. An action
    /// whose document kind it doesn't take can't be signed: it fails
    /// closed.
    fn supports(&self, kind: DocumentKind) -> bool;

    /// The bytes of the request's document, for actions whose approval
    /// also signs them.
    async fn document_bytes(
        &self,
        hasura_transaction: &Transaction<'_>,
        request: &SigningRequestRow,
    ) -> anyhow::Result<Option<Vec<u8>>>;

    /// Prepares the PDF revision a signer's CMS will sign (see
    /// [`super::pdf::prepare_pdf`]). Without PDF signatures there is none.
    /// Owns its transaction on `client`, so a refusal's log entry commits.
    async fn prepare_pdf(
        &self,
        _client: &mut Client,
        _caller: &SigningCaller,
        _tenant_id: Uuid,
        _request_id: Uuid,
        _chain_pem: &[String],
        _now: DateTime<Utc>,
    ) -> SigningResult<PdfPrepared> {
        Err(SigningError::invalid(
            InvalidReason::Document,
            "PDF signatures are not available.",
        ))
    }

    /// The request's document as the signing panel links it. Without a
    /// document store there is none.
    async fn panel_document(
        &self,
        _hasura_transaction: &Transaction<'_>,
        _request: &SigningRequestRow,
    ) -> SigningResult<PanelDocument> {
        Ok(PanelDocument::default())
    }

    /// Checks and keeps the approval's document signature. Runs after the
    /// certificate checks passed and the approval was recorded, in a
    /// savepoint. A `Refused` answer undoes both and is logged like the
    /// other refused signatures.
    async fn embed(
        &self,
        hasura_transaction: &Transaction<'_>,
        request: &SigningRequestRow,
        signer: &Actor,
        certificate: &CertificateIdentity,
        pdf_cms: Option<&[u8]>,
        revision: Option<i32>,
    ) -> SigningResult<()>;
}

/// No document side: no EML bytes and no PDF revisions, so actions with a
/// document can't be signed with it.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoDocumentSigner;

#[async_trait]
impl DocumentSigner for NoDocumentSigner {
    fn supports(&self, kind: DocumentKind) -> bool {
        kind == DocumentKind::NoDocument
    }

    async fn document_bytes(
        &self,
        _hasura_transaction: &Transaction<'_>,
        _request: &SigningRequestRow,
    ) -> anyhow::Result<Option<Vec<u8>>> {
        Ok(None)
    }

    async fn embed(
        &self,
        _hasura_transaction: &Transaction<'_>,
        _request: &SigningRequestRow,
        _signer: &Actor,
        _certificate: &CertificateIdentity,
        _pdf_cms: Option<&[u8]>,
        _revision: Option<i32>,
    ) -> SigningResult<()> {
        Ok(())
    }
}

/// What signing steps reach outside their transaction.
#[derive(Clone)]
pub struct SigningServices {
    pub verifier: Arc<dyn CertificateVerifier>,
    pub executors: SigningExecutorRegistry,
    pub documents: Arc<dyn DocumentSigner>,
    pub exports: Arc<dyn SigningExportStore>,
}

/// One signature, as the browser sends it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApproveInput {
    pub request_id: Uuid,
    /// The signing certificate first, then intermediates.
    pub chain_pem: Vec<String>,
    pub algorithm: SignatureAlgorithm,
    pub payload_signature: Vec<u8>,
    pub document_signature: Option<Vec<u8>>,
    pub pdf_cms: Option<Vec<u8>>,
    pub revision: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ApproveOutcome {
    pub status: SigningRequestStatus,
    pub count: i64,
    pub required: i32,
}

/// Why a person may not sign a request, as its refusal is logged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display)]
#[strum(serialize_all = "kebab-case")]
pub enum SignRefusal {
    /// Without the action's sign permission.
    Forbidden,
    /// The request is for a Post they don't sign for.
    OutsidePost,
    /// They started it, and the rule doesn't let them sign it.
    RequesterNotAllowed,
    /// A trustee's request, and they are not that trustee.
    NotTheTrustee,
    /// The request no longer takes signatures.
    NotWaiting,
    /// The request's time was up.
    Expired,
    /// The document signature doesn't fit the action, or this server
    /// can't take signatures of its document.
    Document,
}

/// The step of a request a refusal refused, as its log entry names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display)]
#[strum(serialize_all = "kebab-case")]
pub enum RefusedStep {
    Sign,
    Cancel,
    Handover,
    /// Reporting a certificate file that didn't open.
    OpenFailure,
}

/// How the transaction ends.
enum Step {
    /// Commit, then run the post-commit work.
    Signed(ApproveOutcome, Vec<PostCommit>),
    /// Commit what was logged, then answer the error.
    Refused(SigningError),
}

/// Records `caller`'s signature of a request; see the module documentation.
/// Owns its transaction on `client`. `now` is the time the certificate
/// must be valid at.
#[instrument(skip(client, services, input), fields(request_id = %input.request_id), err)]
pub async fn approve(
    client: &mut Client,
    services: &SigningServices,
    caller: &SigningCaller,
    tenant_id: Uuid,
    input: &ApproveInput,
    now: DateTime<Utc>,
) -> SigningResult<ApproveOutcome> {
    approve_ingress(
        client,
        services,
        caller,
        tenant_id,
        Ingress::Legacy(input),
        now,
    )
    .await
}

/// Records exact, already-settled device receipts. No direct route is
/// installed; the private operational flow must use server enrollment.
pub async fn approve_operational(
    client: &mut Client,
    services: &SigningServices,
    caller: &SigningCaller,
    tenant_id: Uuid,
    input: &OperationalApprovalInput,
    now: DateTime<Utc>,
) -> SigningResult<ApproveOutcome> {
    approve_ingress(
        client,
        services,
        caller,
        tenant_id,
        Ingress::Operational(input),
        now,
    )
    .await
}

#[derive(Clone, Copy)]
enum Ingress<'a> {
    Legacy(&'a ApproveInput),
    Operational(&'a OperationalApprovalInput),
}

impl Ingress<'_> {
    fn id(self) -> Uuid {
        match self {
            Self::Legacy(i) => i.request_id,
            Self::Operational(i) => i.request_id,
        }
    }

    fn is_operational(self) -> bool {
        matches!(self, Self::Operational(_))
    }

    fn method_denied() -> SigningError {
        SigningError::Forbidden("This signing method is not allowed for the tenant.".into())
    }

    fn request_id(self, mode: OperationalSigningMode) -> SigningResult<Uuid> {
        match (self, mode) {
            (Self::Legacy(input), OperationalSigningMode::LegacyP12) => Ok(input.request_id),
            (Self::Operational(input), OperationalSigningMode::HsmRequired) => Ok(input.request_id),
            _ => Err(Self::method_denied()),
        }
    }
}

async fn approve_ingress(
    client: &mut Client,
    services: &SigningServices,
    caller: &SigningCaller,
    tenant_id: Uuid,
    input: Ingress<'_>,
    now: DateTime<Utc>,
) -> SigningResult<ApproveOutcome> {
    let mut transaction = client
        .transaction()
        .await
        .context("Error starting the approval")?;
    attribute_approval(&transaction, caller, tenant_id).await?;
    let step = approve_in(&mut transaction, services, caller, tenant_id, input, now).await?;
    finish_approval(transaction, step).await
}

async fn attribute_approval(
    transaction: &Transaction<'_>,
    caller: &SigningCaller,
    tenant_id: Uuid,
) -> SigningResult<()> {
    // The action the last signature runs is logged as that signer's (SW-F-0364).
    act_for(
        transaction,
        &AuditActor {
            user_id: Some(caller.user_id.clone()),
            username: Some(caller.username.clone()),
            role: None,
        },
        AuditOrigin::Harvest,
        tenant_id,
    )
    .await?;
    Ok(())
}

async fn finish_approval(
    transaction: Transaction<'_>,
    step: Step,
) -> SigningResult<ApproveOutcome> {
    match step {
        Step::Signed(outcome, post_commit) => {
            transaction
                .commit()
                .await
                .context("Error committing the approval")?;
            kick_signing_log_outbox();
            kick_audit_log_outbox();
            for work in post_commit {
                if let Err(error) = work.run().await {
                    warn!("post-commit work of a signing request failed: {error:?}");
                }
            }
            Ok(outcome)
        }
        Step::Refused(error) => {
            transaction
                .commit()
                .await
                .context("Error committing the refusal")?;
            kick_signing_log_outbox();
            Err(error)
        }
    }
}

fn certificate_details(certificate: &CertificateIdentity) -> Value {
    json!({
        "subject": certificate.subject,
        "issuer": certificate.issuer,
        "serial": certificate.serial,
        "fingerprint": certificate.fingerprint_sha256,
    })
}

/// The document signature a signature entry carries: an EML's signature
/// itself, or a PDF CMS's SHA-256 and the revision it signs.
fn document_signature_details(request: &SigningRequestRow, input: &ApproveInput) -> Value {
    match request.action.document() {
        DocumentKind::Eml => json!({
            "kind": DocumentKind::Eml,
            "signature": input.document_signature.as_deref().map(|signature| BASE64.encode(signature)),
        }),
        DocumentKind::Pdf => json!({
            "kind": DocumentKind::Pdf,
            "cms_sha256": input.pdf_cms.as_deref().map(sha256_hex),
            "revision": input.revision,
        }),
        DocumentKind::NoDocument => Value::Null,
    }
}

/// Logs a refused signature, unless the caller had one logged on the
/// request within the throttle, and answers `error`.
async fn refusal(
    transaction: &Transaction<'_>,
    request: &SigningRequestRow,
    caller: &SigningCaller,
    check: &str,
    detail: &str,
    certificate: Option<&CertificateIdentity>,
    error: SigningError,
) -> SigningResult<Step> {
    log_refusal(transaction, request, caller, check, detail, certificate).await?;
    Ok(Step::Refused(error))
}

/// Logs a refused signing step (SigningSignatureRefused, SYSTEM = ERROR),
/// unless the caller had one logged on the request within the throttle.
pub(crate) async fn log_refusal(
    transaction: &Transaction<'_>,
    request: &SigningRequestRow,
    caller: &SigningCaller,
    check: &str,
    detail: &str,
    certificate: Option<&CertificateIdentity>,
) -> SigningResult<()> {
    log_step_refusal(
        transaction,
        request,
        caller,
        RefusedStep::Sign,
        check,
        detail,
        certificate,
    )
    .await
}

/// Logs that `step` on the request was refused (SigningSignatureRefused,
/// SYSTEM = ERROR), unless the caller had a refusal logged on the request
/// within the throttle ([`super::requests::LOG_THROTTLE_SECONDS`]). The
/// caller commits its transaction to keep the entry.
pub(crate) async fn log_step_refusal(
    transaction: &Transaction<'_>,
    request: &SigningRequestRow,
    caller: &SigningCaller,
    step: RefusedStep,
    check: &str,
    detail: &str,
    certificate: Option<&CertificateIdentity>,
) -> SigningResult<()> {
    if throttled(
        transaction,
        request,
        SigningStatementKind::SigningSignatureRefused,
        caller,
    )
    .await?
    {
        return Ok(());
    }
    let description = match step {
        RefusedStep::Sign => format!("Signature refused on {}: {}", request.code, check),
        RefusedStep::Cancel => format!("Cancelling {} refused: {}", request.code, check),
        RefusedStep::Handover => format!("Handover of {} refused: {}", request.code, check),
        RefusedStep::OpenFailure => format!(
            "Certificate file report on {} refused: {}",
            request.code, check
        ),
    };
    stage_request_step(
        transaction,
        request,
        SigningStatementKind::SigningSignatureRefused,
        caller.actor(),
        SystemOutcome::Error,
        description,
        json!({
            "step": step.to_string(),
            "check": check,
            "detail": detail,
            "certificate": certificate.map(certificate_details),
        }),
    )
    .await?;
    Ok(())
}

/// A certificate check refused the signature: 422 with the check.
async fn refuse(
    transaction: &Transaction<'_>,
    request: &SigningRequestRow,
    caller: &SigningCaller,
    check: CertificateCheckId,
    detail: String,
    certificate: Option<&CertificateIdentity>,
) -> SigningResult<Step> {
    refusal(
        transaction,
        request,
        caller,
        &check.to_string(),
        &detail,
        certificate,
        SigningError::Refused {
            check,
            message: detail.clone(),
            other_holder: None,
        },
    )
    .await
}

/// Which identity of `certificate` already signed the request.
fn signed_identity(
    approvals: &[SigningApprovalRow],
    certificate: &CertificateIdentity,
) -> Option<&'static str> {
    approvals.iter().find_map(|approval| {
        if approval.fingerprint_sha256.as_deref() == Some(certificate.fingerprint_sha256.as_str()) {
            Some("This certificate already signed this request.")
        } else if approval.spki_sha256 == certificate.spki_sha256 {
            Some("A certificate with this key already signed this request.")
        } else if approval.holder_sha256.as_deref() == Some(certificate.holder_sha256.as_str()) {
            Some("The holder of this certificate already signed this request.")
        } else {
            None
        }
    })
}

/// Why the caller may not sign the request, if they may not. Reads only
/// what never changes on a request, so it runs before any lock.
pub(crate) async fn sign_refusal(
    transaction: &Transaction<'_>,
    caller: &SigningCaller,
    request: &SigningRequestRow,
) -> SigningResult<Option<(SignRefusal, &'static str)>> {
    if !caller.has(request.action.sign_permission()) {
        return Ok(Some((
            SignRefusal::Forbidden,
            "You don't have the permission to sign this action.",
        )));
    }
    if !caller.signs_for(request.permission_label.as_deref()) {
        return Ok(Some((
            SignRefusal::OutsidePost,
            "This request is for a Post you don't sign for.",
        )));
    }
    let rule: SigningRule = serde_json::from_value(request.rule_snapshot.clone())
        .context("Error reading the request's rule")?;
    if caller.user_id == request.requested_by
        && rule.requester_signing_effective() == RequesterSigning::NotAllowed
    {
        return Ok(Some((
            SignRefusal::RequesterNotAllowed,
            "The person who started this request can't sign it.",
        )));
    }
    if request.action.is_trustee() && !is_the_trustee(transaction, caller, request).await? {
        return Ok(Some((
            SignRefusal::NotTheTrustee,
            "Only the request's trustee can sign it.",
        )));
    }
    Ok(None)
}

/// Refuses a document signature the action doesn't take, and an action
/// whose document this server can't take signatures of yet.
fn check_document_inputs(
    request: &SigningRequestRow,
    input: &ApproveInput,
    services: &SigningServices,
) -> SigningResult<()> {
    let kind = request.action.document();
    let invalid = |message: &str| SigningError::invalid(InvalidReason::Document, message);
    if input.document_signature.is_some() && kind != DocumentKind::Eml {
        return Err(invalid(
            "A document signature only goes with an action that signs an EML document.",
        ));
    }
    if (input.pdf_cms.is_some() || input.revision.is_some()) && kind != DocumentKind::Pdf {
        return Err(invalid(
            "A PDF signature only goes with an action that signs a PDF document.",
        ));
    }
    if !services.documents.supports(kind) {
        return Err(invalid(
            "This action signs a document, which this server can't take signatures of yet.",
        ));
    }
    match kind {
        DocumentKind::Eml if input.document_signature.is_none() => {
            Err(invalid("This action also needs the document's signature."))
        }
        DocumentKind::Pdf if input.pdf_cms.is_none() || input.revision.is_none() => Err(invalid(
            "This action also needs the PDF signature and its revision.",
        )),
        _ => Ok(()),
    }
}

async fn approve_in(
    transaction: &mut Transaction<'_>,
    services: &SigningServices,
    caller: &SigningCaller,
    tenant_id: Uuid,
    ingress: Ingress<'_>,
    now: DateTime<Utc>,
) -> SigningResult<Step> {
    let blocked = |error: super::operational_policy::LegacySigningBlocked| {
        SigningError::Forbidden(error.to_string())
    };
    // The dedicated path has its own verified and audited ingress. Neither
    // ordinary browser certificates nor an ordinary managed receipt qualify.
    if super::operational_policy::direct_hsm_approval_build() {
        return Err(Ingress::method_denied());
    }
    let baseline = signing_mode(transaction, tenant_id).await.map_err(blocked)?;
    if baseline == OperationalSigningMode::HsmRequired {
        // Preserve tenant-wide refusal without revealing request existence.
        ingress.request_id(OperationalSigningMode::HsmRequired)?;
    }
    let found = match read_request(transaction, tenant_id, ingress.id()).await {
        Err(SigningError::NotFound(_))
            if baseline == OperationalSigningMode::LegacyP12 && ingress.is_operational() =>
        {
            // A managed caller cannot distinguish an unknown request from
            // an ordinary legacy request by guessing its UUID.
            return Err(Ingress::method_denied());
        }
        result => result?,
    };
    let authorization = sign_refusal(transaction, caller, &found).await?;
    if baseline == OperationalSigningMode::LegacyP12
        && ingress.is_operational()
        && authorization.is_some()
    {
        // Do not disclose that this actor guessed the one managed scope and
        // do not write a refusal against a guessed legacy request.
        return Err(Ingress::method_denied());
    }
    let checked_mode = request_mode_and_latch(transaction, &found).await;
    let (mode, direct) =
        if baseline == OperationalSigningMode::LegacyP12 && ingress.is_operational() {
            checked_mode.map_err(|_| Ingress::method_denied())?
        } else {
            checked_mode.map_err(blocked)?
        };
    if direct.is_some() {
        return Err(Ingress::method_denied());
    }
    ingress.request_id(mode)?;

    // 1. Who may sign: decided before any event/row lock and before
    // revealing request state. Ordinary eligible legacy refusals are logged.
    if let Some((refused, message)) = authorization {
        let request = relock(transaction, &found).await?;
        return refusal(
            transaction,
            &request,
            caller,
            &refused.to_string(),
            message,
            None,
            SigningError::Forbidden(message.into()),
        )
        .await;
    }
    let request = relock(transaction, &found).await?;
    let (mode, direct) = request_mode_and_latch(transaction, &request)
        .await
        .map_err(blocked)?;
    if direct.is_some() {
        return Err(Ingress::method_denied());
    }
    ingress.request_id(mode)?;
    let event_id = request.election_event_id;

    // 2. Only a waiting request within its time takes signatures.
    if request.status != SigningRequestStatus::Waiting {
        let message = format!(
            "Signing request {} is {}; it takes no more signatures.",
            request.code, request.status
        );
        return refusal(
            transaction,
            &request,
            caller,
            &SignRefusal::NotWaiting.to_string(),
            &message,
            None,
            SigningError::Closed {
                status: request.status,
                message: message.clone(),
            },
        )
        .await;
    }
    if is_overdue(transaction, &request).await? {
        if let Some(expired) = expire_request(transaction, &request).await? {
            let message = format!(
                "Signing request {} expired; it takes no more signatures.",
                request.code
            );
            return refusal(
                transaction,
                &expired,
                caller,
                &SignRefusal::Expired.to_string(),
                &message,
                None,
                SigningError::Closed {
                    status: SigningRequestStatus::Expired,
                    message: message.clone(),
                },
            )
            .await;
        }
    }
    let approvals = list_signing_approvals(transaction, tenant_id, event_id, request.id).await?;
    if approvals.iter().any(SigningApprovalRow::is_direct_demo) {
        return Err(Ingress::method_denied());
    }
    if approvals
        .iter()
        .any(|approval| approval.user_id == caller.user_id)
    {
        return refuse(
            transaction,
            &request,
            caller,
            CertificateCheckId::AlreadySigned,
            "You already signed this request.".into(),
            None,
        )
        .await;
    }

    let evidence = match ingress {
        Ingress::Legacy(_) => None,
        Ingress::Operational(input) => Some(
            OperationalEvidence::load(transaction, services, &request, caller, input, &approvals)
                .await?,
        ),
    };
    let input = match (ingress, &evidence) {
        (Ingress::Legacy(input), None) => input,
        (Ingress::Operational(_), Some(evidence)) => &evidence.input,
        _ => {
            return Err(SigningError::Conflict(
                "Signing evidence is unavailable.".into(),
            ))
        }
    };
    if let Err(error) = check_document_inputs(&request, input, services) {
        let detail = error.to_string();
        return refusal(
            transaction,
            &request,
            caller,
            &SignRefusal::Document.to_string(),
            &detail,
            None,
            error,
        )
        .await;
    }

    // 3. The certificate and the signatures.
    let document = match request.action.document() {
        DocumentKind::Eml => {
            let bytes = services
                .documents
                .document_bytes(transaction, &request)
                .await?
                .ok_or_else(|| {
                    SigningError::invalid(
                        InvalidReason::Document,
                        "The request's document is not available.",
                    )
                })?;
            input
                .document_signature
                .clone()
                .map(|signature| DocumentSignatureInput {
                    document: bytes,
                    signature,
                })
        }
        _ => None,
    };
    let verification_input = CertificateVerificationInput {
        request: &request,
        signer: caller.actor(),
        chain_pem: input.chain_pem.clone(),
        signatures: Some(SignatureInputs {
            algorithm: input.algorithm,
            payload_signature: input.payload_signature.clone(),
            document,
        }),
        now,
    };
    let verification = services
        .verifier
        .verify(transaction, &verification_input)
        .await?;
    let certificate = match (&verification.certificate, verification.passed()) {
        (Some(certificate), true) => certificate.clone(),
        _ => {
            let (check, detail) = verification
                .refusal()
                .map(|refusal| (refusal.id, refusal.detail.clone()))
                .unwrap_or((CertificateCheckId::TrustedIssuer, None));
            let detail = detail.unwrap_or_else(|| format!("The {check} check failed."));
            let other_holder = (check == CertificateCheckId::RegisteredToOther)
                .then(|| verification.other_holder.clone())
                .flatten();
            return refusal(
                transaction,
                &request,
                caller,
                &check.to_string(),
                &detail,
                verification.certificate.as_ref(),
                SigningError::Refused {
                    check,
                    message: detail.clone(),
                    other_holder,
                },
            )
            .await;
        }
    };

    // §5a: each certificate, key and holder signs a request once.
    if let Some(detail) = signed_identity(&approvals, &certificate) {
        return refuse(
            transaction,
            &request,
            caller,
            CertificateCheckId::AlreadySigned,
            detail.into(),
            Some(&certificate),
        )
        .await;
    }

    // 4. The signer's registered certificate, or its first use.
    let certificate_id = match &verification.registration {
        RegistrationState::Registered(row)
            if evidence
                .as_ref()
                .is_none_or(|item| item.certificate_id == row.id) =>
        {
            row.id
        }
        RegistrationState::FirstUse if evidence.is_none() => {
            register_first_use(
                transaction,
                &verification_input,
                &verification,
                Some(&caller.display_name),
            )
            .await?
            .id
        }
        RegistrationState::Registered(_)
        | RegistrationState::FirstUse
        | RegistrationState::NotRegistered => {
            return refuse(
                transaction,
                &request,
                caller,
                CertificateCheckId::Registered,
                "This certificate is not registered to you.".into(),
                Some(&certificate),
            )
            .await;
        }
    };

    // Every counted signature's certificate, and this one, still active:
    // share-locked in id order, so a revocation waits for this step.
    let mut certificate_ids: Vec<Uuid> = approvals
        .iter()
        .filter_map(|approval| approval.certificate_id)
        .chain(std::iter::once(certificate_id))
        .collect();
    certificate_ids.sort();
    certificate_ids.dedup();
    for id in certificate_ids {
        let locked = lock_staff_certificate_for_share(transaction, tenant_id, event_id, id).await?;
        let active = locked
            .as_ref()
            .is_some_and(|row| row.status == StaffCertificateStatus::Active);
        if !active && id != certificate_id {
            // A signature it counts no longer stands: the request ends, as
            // the officer who revoked the key would have ended it. The
            // registration keeps the officer's id and display name.
            let officer = locked
                .as_ref()
                .and_then(|row| {
                    row.revoked_by.as_ref().map(|user_id| Actor {
                        user_id: user_id.clone(),
                        username: row
                            .revoked_by_name
                            .clone()
                            .unwrap_or_else(|| user_id.clone()),
                    })
                })
                .unwrap_or_else(|| caller.actor());
            cancel_request(
                transaction,
                &request,
                CancelReason::CertificateRevoked,
                officer,
                None,
            )
            .await?;
            return Ok(Step::Refused(SigningError::Closed {
                status: SigningRequestStatus::Cancelled,
                message: format!(
                    "Signing request {} was cancelled: the certificate of a signature it counted was revoked.",
                    request.code
                ),
            }));
        }
        let refusal = match &locked {
            Some(row) if row.status != StaffCertificateStatus::Active => Some((
                CertificateCheckId::NotRevoked,
                "This certificate was revoked.",
            )),
            Some(row) if row.user_id != caller.user_id => Some((
                CertificateCheckId::Registered,
                "This certificate is not registered to you.",
            )),
            Some(_) => None,
            None => Some((
                CertificateCheckId::Registered,
                "This certificate is not registered to you.",
            )),
        }
        .filter(|_| id == certificate_id);
        if let Some((check, detail)) = refusal {
            return refuse(
                transaction,
                &request,
                caller,
                check,
                detail.into(),
                Some(&certificate),
            )
            .await;
        }
    }

    // 5. The approval and its document signature, together: when either is
    // refused, neither is kept (only the refusal's log entry).
    let savepoint = transaction
        .savepoint("signing_approval")
        .await
        .context("Error starting the signature")?;
    let recorded = record_signature(
        &savepoint,
        services,
        &request,
        caller,
        &certificate,
        certificate_id,
        &verification,
        input,
    )
    .await;
    match recorded {
        Ok(()) => {
            if let Some(evidence) = &evidence {
                evidence.mark_applied(&savepoint).await?;
            }
            savepoint
                .commit()
                .await
                .context("Error keeping the signature")?;
        }
        Err(SigningError::Refused { check, message, .. }) => {
            savepoint
                .rollback()
                .await
                .context("Error undoing the signature")?;
            return refuse(
                transaction,
                &request,
                caller,
                check,
                message,
                Some(&certificate),
            )
            .await;
        }
        Err(error) => return Err(error),
    }
    let count = approvals.len() as i64 + 1;
    let required = request.required;
    stage_request_step(
        transaction,
        &request,
        SigningStatementKind::SigningRequestSigned,
        caller.actor(),
        SystemOutcome::Info,
        format!(
            "Signature verified on {}: {} of {}",
            request.code, count, required
        ),
        json!({
            "certificate": certificate_details(&certificate),
            "signature": BASE64.encode(&input.payload_signature),
            "document_signature": document_signature_details(&request, input),
            "algorithm": input.algorithm.to_string(),
            "revocation_status": verification.revocation_status.to_string(),
            "checks": verification.checks,
            "count": count,
            "required": required,
        }),
    )
    .await?;
    advance_approval(transaction, services, &request, caller, count).await
}

async fn advance_approval(
    transaction: &mut Transaction<'_>,
    services: &SigningServices,
    request: &SigningRequestRow,
    caller: &SigningCaller,
    count: i64,
) -> SigningResult<Step> {
    let required = request.required;
    let tenant_id = request.tenant_id;
    let event_id = request.election_event_id;
    if count < i64::from(required) {
        return Ok(Step::Signed(
            ApproveOutcome {
                status: SigningRequestStatus::Waiting,
                count,
                required,
            },
            vec![],
        ));
    }

    // 6. The last signature: complete, and run a deferred action.
    let completed = update_signing_request_status(
        transaction,
        tenant_id,
        event_id,
        request.id,
        &SigningRequestTransition::Complete,
    )
    .await?
    .ok_or_else(|| SigningError::Conflict("The request is no longer waiting.".into()))?;
    stage_request_step(
        transaction,
        &completed,
        SigningStatementKind::SigningRequestCompleted,
        caller.actor(),
        SystemOutcome::Info,
        format!(
            "Signing request {} has every signature: {} of {}",
            completed.code, count, required
        ),
        json!({ "count": count, "required": required }),
    )
    .await?;
    if completed.action.mode() == ExecutionMode::Gate {
        return Ok(Step::Signed(
            ApproveOutcome {
                status: completed.status,
                count,
                required,
            },
            vec![],
        ));
    }
    let (status, post_commit) = execute(transaction, &services.executors, &completed).await?;
    Ok(Step::Signed(
        ApproveOutcome {
            status,
            count,
            required,
        },
        post_commit,
    ))
}

// Deliberately separate from the old X.509/P12 routing. This is not a generic
// certificate-verifier bypass: only the opaque exact Initialize proof enters.
#[cfg(feature = "comelec-demo-direct-hsm")]
pub(super) async fn approve_direct(
    client: &mut Client,
    services: &SigningServices,
    caller: &SigningCaller,
    proof: VerifiedDirectProof,
    now: DateTime<Utc>,
) -> SigningResult<ApproveOutcome> {
    let mut transaction = client
        .transaction()
        .await
        .context("Start direct Initialize approval")?;
    attribute_approval(&transaction, caller, proof.scope().tenant()).await?;
    let request = direct_initialize::lock_authorized(&transaction, caller, proof.scope()).await?;
    let approvals = list_signing_approvals(
        &transaction,
        request.tenant_id,
        request.election_event_id,
        request.id,
    )
    .await?;
    if !direct_initialize::verified_prior(&transaction, &request, &approvals).await? {
        return Err(direct_initialize::unavailable());
    }
    if approvals.iter().any(|row| row.user_id == caller.user_id) {
        // Lost response / simultaneous retry: the existing checked proof is
        // the approval. Do not re-consume, emit another audit or dispatch.
        return finish_approval(
            transaction,
            Step::Signed(
                ApproveOutcome {
                    status: request.status,
                    count: approvals.len() as i64,
                    required: request.required,
                },
                vec![],
            ),
        )
        .await;
    }
    if request.status != SigningRequestStatus::Waiting
        || request.created_at > now
        || is_overdue(&transaction, &request).await?
        || approvals.len() as i64 >= i64::from(request.required)
    {
        return Err(direct_initialize::unavailable());
    }
    let signed = proof.consume_and_insert(&transaction, caller).await?;
    let count = approvals.len() as i64 + 1;
    stage_request_step(
        &transaction,
        &request,
        SigningStatementKind::SigningRequestSigned,
        caller.actor(),
        SystemOutcome::Info,
        format!(
            "Direct demo HSM signature verified on {}: {} of {}",
            request.code, count, request.required
        ),
        json!({
            "approval_id": signed.id,
            "proof_kind": "initialize-voting-demo-pkcs11",
            "device_profile": direct_initialize::DEVICE_PROFILE,
            "installation_id": proof.scope().installation(),
            "post_id": proof.scope().post(),
            "publication_id": proof.scope().publication(),
            "reservation_nonce": proof.nonce(),
            "payload_sha256": proof.scope().payload_sha256(),
            "spki_sha256": signed.spki_sha256,
            "signature_der_b64": BASE64.encode(&signed.payload_signature),
            "algorithm": signed.algorithm.to_string(),
            "count": count,
            "required": request.required,
        }),
    )
    .await?;
    let step = advance_approval(&mut transaction, services, &request, caller, count).await?;
    finish_approval(transaction, step).await
}

#[cfg(feature = "comelec-demo-direct-hsm")]
pub(super) async fn approve_direct_action(
    client: &mut Client,
    services: &SigningServices,
    caller: &SigningCaller,
    proof: VerifiedActionProof,
    now: DateTime<Utc>,
) -> SigningResult<ApproveOutcome> {
    let mut transaction = client
        .transaction()
        .await
        .context("Start certificate-free action approval")?;
    attribute_approval(&transaction, caller, proof.operation.anchor().tenant).await?;
    let request = direct_actions::lock_authorized(&transaction, caller, &proof.operation).await?;
    let approvals = list_signing_approvals(
        &transaction,
        request.tenant_id,
        request.election_event_id,
        request.id,
    )
    .await?;
    if !direct_actions::verified_prior(&transaction, &request, &approvals).await? {
        return Err(direct_actions::unavailable());
    }
    if approvals.iter().any(|row| row.user_id == caller.user_id) {
        // Lost response / simultaneous retry: the existing checked proof is
        // the approval. Do not re-consume, emit another audit or dispatch.
        return finish_approval(
            transaction,
            Step::Signed(
                ApproveOutcome {
                    status: request.status,
                    count: approvals.len() as i64,
                    required: request.required,
                },
                vec![],
            ),
        )
        .await;
    }
    if request.status != SigningRequestStatus::Waiting
        || request.created_at > now
        || is_overdue(&transaction, &request).await?
        || approvals.len() as i64 >= i64::from(request.required)
    {
        return Err(direct_actions::unavailable());
    }
    let signed = proof.consume_and_insert(&transaction, caller).await?;
    let count = approvals.len() as i64 + 1;
    stage_request_step(
        &transaction,
        &request,
        SigningStatementKind::SigningRequestSigned,
        caller.actor(),
        SystemOutcome::Info,
        format!(
            "Direct demo HSM signature verified on {}: {} of {}",
            request.code, count, request.required
        ),
        json!({
            "approval_id": signed.id,
            "proof_kind": "direct-action-demo-pkcs11-v2",
            "action": request.action.to_string(),
            "document_sha256": request.document_sha256,
            "device_profile": direct_initialize::DEVICE_PROFILE,
            "installation_id": proof.operation.anchor().installation,
            "post_id": proof.operation.anchor().post,
            "publication_id": proof.operation.anchor().publication,
            "reservation_nonce": proof.nonce,
            "payload_sha256": hex::encode(proof.operation.payload_digest()),
            "spki_sha256": signed.spki_sha256,
            "signature_der_b64": BASE64.encode(&signed.payload_signature),
            "algorithm": signed.algorithm.to_string(),
            "count": count,
            "required": request.required,
        }),
    )
    .await?;
    let step = advance_approval(&mut transaction, services, &request, caller, count).await?;
    finish_approval(transaction, step).await
}

/// Records the approval, then keeps its document signature (a PDF
/// revision). `Refused` when either is refused; the caller's savepoint then
/// keeps neither.
#[allow(clippy::too_many_arguments)]
async fn record_signature(
    transaction: &Transaction<'_>,
    services: &SigningServices,
    request: &SigningRequestRow,
    caller: &SigningCaller,
    certificate: &CertificateIdentity,
    certificate_id: Uuid,
    verification: &CertificateVerification,
    input: &ApproveInput,
) -> SigningResult<()> {
    let inserted = insert_signing_approval(
        transaction,
        &NewSigningApproval {
            tenant_id: request.tenant_id,
            election_event_id: request.election_event_id,
            request_id: request.id,
            user_id: caller.user_id.clone(),
            username: caller.username.clone(),
            auth_time: caller.auth_time,
            certificate_id,
            certificate_pem: certificate.pem.clone(),
            chain_pem: certificate.chain_pem.clone(),
            fingerprint_sha256: certificate.fingerprint_sha256.clone(),
            spki_sha256: certificate.spki_sha256.clone(),
            holder_sha256: certificate.holder_sha256.clone(),
            algorithm: input.algorithm,
            payload_signature: input.payload_signature.clone(),
            document_signature: input.document_signature.clone(),
            pdf_cms: input.pdf_cms.clone(),
            revocation_status: verification.revocation_status,
            display_name: Some(caller.display_name.clone()),
        },
    )
    .await?;
    if let Err(conflict) = inserted {
        return Err(SigningError::Refused {
            check: CertificateCheckId::AlreadySigned,
            message: format!("{conflict:?}: this person already signed this request."),
            other_holder: None,
        });
    }
    // After the approval, so an approval that conflicts stores no document.
    services
        .documents
        .embed(
            transaction,
            request,
            &caller.actor(),
            certificate,
            input.pdf_cms.as_deref(),
            input.revision,
        )
        .await
}

/// Runs a completed deferred request's executor inside a savepoint: the
/// request is executed, stays completed for a dispatched task, or fails
/// with nothing the executor wrote. The executor's error goes to the
/// service's log; the request keeps a stable code.
async fn execute(
    transaction: &mut Transaction<'_>,
    executors: &SigningExecutorRegistry,
    completed: &SigningRequestRow,
) -> SigningResult<(SigningRequestStatus, Vec<PostCommit>)> {
    let tenant_id = completed.tenant_id;
    let event_id = completed.election_event_id;
    let approvals = list_signing_approvals(transaction, tenant_id, event_id, completed.id).await?;
    let outcome = match executors.get(completed.action) {
        Some(executor) => {
            let savepoint = transaction
                .savepoint("signing_execution")
                .await
                .context("Error starting the execution")?;
            let outcome = executor.execute(&savepoint, completed, &approvals).await;
            match outcome {
                Ok(outcome) => {
                    savepoint
                        .commit()
                        .await
                        .context("Error keeping the execution")?;
                    Ok(outcome)
                }
                Err(error) => {
                    savepoint
                        .rollback()
                        .await
                        .context("Error undoing the execution")?;
                    warn!(request_id = %completed.id, "the signing executor failed: {error:#}");
                    Err("execution-failed")
                }
            }
        }
        None => {
            warn!(request_id = %completed.id, action = %completed.action, "no signing executor");
            Err("no-executor")
        }
    };
    let code = match outcome {
        Ok(ExecutionOutcome::Executed {
            result,
            post_commit,
        }) => {
            let executed = update_signing_request_status(
                transaction,
                tenant_id,
                event_id,
                completed.id,
                &SigningRequestTransition::Execute { result },
            )
            .await?
            .ok_or_else(|| SigningError::Conflict("The request is not completed.".into()))?;
            stage_executed(transaction, &executed, SystemOutcome::Info).await?;
            return Ok((executed.status, post_commit));
        }
        Ok(ExecutionOutcome::Dispatched {
            task_execution_id,
            task,
        }) => {
            set_signing_request_task_execution(
                transaction,
                tenant_id,
                event_id,
                completed.id,
                task_execution_id,
            )
            .await?;
            return Ok((completed.status, vec![task]));
        }
        Err(code) => code,
    };
    let failed = update_signing_request_status(
        transaction,
        tenant_id,
        event_id,
        completed.id,
        &SigningRequestTransition::Fail {
            result: Some(execution_failure(code)),
        },
    )
    .await?
    .ok_or_else(|| SigningError::Conflict("The request is not completed.".into()))?;
    stage_executed(transaction, &failed, SystemOutcome::Error).await?;
    Ok((failed.status, vec![]))
}

#[cfg(test)]
mod ingress_tests {
    use super::*;

    #[test]
    fn operational_signing_has_no_p12_fallback() {
        let request_id = Uuid::from_u128(1);
        let receipt = OperationalApprovalInput {
            request_id,
            approval_challenge: Uuid::from_u128(2),
            artifact_challenge: None,
        };
        let legacy = ApproveInput {
            request_id,
            chain_pem: vec![],
            algorithm: SignatureAlgorithm::EcdsaP256Sha256,
            payload_signature: vec![],
            document_signature: None,
            pdf_cms: None,
            revision: None,
        };
        let device = Ingress::Operational(&receipt);
        let p12 = Ingress::Legacy(&legacy);
        let required = OperationalSigningMode::HsmRequired;
        let ordinary = OperationalSigningMode::LegacyP12;
        assert_eq!(device.request_id(required).unwrap(), request_id);
        assert!(matches!(
            p12.request_id(required),
            Err(SigningError::Forbidden(_))
        ));
        assert!(matches!(
            device.request_id(ordinary),
            Err(SigningError::Forbidden(_))
        ));
        assert_eq!(p12.request_id(ordinary).unwrap(), request_id);
    }
}
