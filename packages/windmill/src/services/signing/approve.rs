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
    register_first_use, CertificateIdentity, CertificateVerificationInput, CertificateVerifier,
    DocumentSignatureInput, RegistrationState, SignatureInputs,
};
use super::executors::{ExecutionOutcome, PostCommit, SigningExecutorRegistry};
use super::log::{Actor, SystemOutcome};
use super::requests::{
    cancel_request, execution_failure, expire_request, is_overdue, is_the_trustee, read_request,
    relock, stage_executed, stage_request_step, throttled, SigningExportStore,
};
use super::{InvalidReason, SigningCaller, SigningError, SigningResult};
use crate::postgres::signing::*;
use crate::tasks::signing_log_outbox::kick_signing_log_outbox;
use anyhow::Context;
use async_trait::async_trait;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use chrono::{DateTime, Utc};
use deadpool_postgres::{Client, Transaction};
use electoral_log::messages::newtypes::SigningStatementKind;
use sequent_core::signing::{
    CancelReason, CertificateCheckId, DocumentKind, ExecutionMode, RequesterSigning,
    SignatureAlgorithm, SigningRequestStatus, SigningRule, StaffCertificateStatus,
};
use serde::Serialize;
use serde_json::{json, Value};
use std::sync::Arc;
use strum_macros::Display;
use tracing::{instrument, warn};
use uuid::Uuid;

/// The document side of a signature: the bytes an EML signature covers,
/// and the PDF revision a CMS signs (PR 7 embeds it).
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

    /// Checks and keeps the approval's document signature. Runs after the
    /// certificate checks passed, before the approval is recorded.
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

/// No document side yet: EML bytes and PDF revisions come with the PRs
/// that sign reports and transmissions, so actions with a document can't
/// be signed until then.
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
    let mut transaction = client
        .transaction()
        .await
        .context("Error starting the approval")?;
    match approve_in(&mut transaction, services, caller, tenant_id, input, now).await? {
        Step::Signed(outcome, post_commit) => {
            transaction
                .commit()
                .await
                .context("Error committing the approval")?;
            kick_signing_log_outbox();
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
    if !throttled(
        transaction,
        request,
        SigningStatementKind::SigningSignatureRefused,
        caller,
    )
    .await?
    {
        stage_request_step(
            transaction,
            request,
            SigningStatementKind::SigningSignatureRefused,
            caller.actor(),
            SystemOutcome::Error,
            format!("Signature refused on {}: {}", request.code, check),
            json!({
                "check": check,
                "detail": detail,
                "certificate": certificate.map(certificate_details),
            }),
        )
        .await?;
    }
    Ok(Step::Refused(error))
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
        if approval.fingerprint_sha256 == certificate.fingerprint_sha256 {
            Some("This certificate already signed this request.")
        } else if approval.spki_sha256 == certificate.spki_sha256 {
            Some("A certificate with this key already signed this request.")
        } else if approval.holder_sha256 == certificate.holder_sha256 {
            Some("The holder of this certificate already signed this request.")
        } else {
            None
        }
    })
}

/// Why the caller may not sign the request, if they may not. Reads only
/// what never changes on a request, so it runs before any lock.
async fn sign_refusal(
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
    input: &ApproveInput,
    now: DateTime<Utc>,
) -> SigningResult<Step> {
    let found = read_request(transaction, tenant_id, input.request_id).await?;

    // 1. Who may sign: decided before any lock, and before anything of
    // the request's state is told. A refusal is logged.
    if let Some((refused, message)) = sign_refusal(transaction, caller, &found).await? {
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
    check_document_inputs(&found, input, services)?;
    let request = relock(transaction, &found).await?;
    let event_id = request.election_event_id;

    // 2. Only a waiting request within its time takes signatures.
    if request.status != SigningRequestStatus::Waiting {
        return Err(SigningError::Closed {
            status: request.status,
            message: format!(
                "Signing request {} is {}; it takes no more signatures.",
                request.code, request.status
            ),
        });
    }
    if is_overdue(transaction, &request).await?
        && expire_request(transaction, &request).await?.is_some()
    {
        return Ok(Step::Refused(SigningError::Closed {
            status: SigningRequestStatus::Expired,
            message: format!(
                "Signing request {} expired; it takes no more signatures.",
                request.code
            ),
        }));
    }
    let approvals = list_signing_approvals(transaction, tenant_id, event_id, request.id).await?;
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
        RegistrationState::Registered(row) => row.id,
        RegistrationState::FirstUse => {
            register_first_use(
                transaction,
                &verification_input,
                &verification,
                Some(&caller.display_name),
            )
            .await?
            .id
        }
        RegistrationState::NotRegistered => {
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
        .map(|approval| approval.certificate_id)
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
            // A signature it counts no longer stands: the request ends.
            cancel_request(
                transaction,
                &request,
                CancelReason::CertificateRevoked,
                caller.actor(),
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

    services
        .documents
        .embed(
            transaction,
            &request,
            &caller.actor(),
            &certificate,
            input.pdf_cms.as_deref(),
            input.revision,
        )
        .await?;

    // 5. The signature.
    let inserted = insert_signing_approval(
        transaction,
        &NewSigningApproval {
            tenant_id,
            election_event_id: event_id,
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
        return refuse(
            transaction,
            &request,
            caller,
            CertificateCheckId::AlreadySigned,
            format!("{conflict:?}: this person already signed this request."),
            Some(&certificate),
        )
        .await;
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
            "algorithm": input.algorithm.to_string(),
            "revocation_status": verification.revocation_status.to_string(),
            "count": count,
            "required": required,
        }),
    )
    .await?;
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
