// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Converts server-owned device receipts into the existing approval. The
//! ingress identifies sealed receipts only, with no caller-supplied
//! signature, key, bytes, certificate selection, or mode. The private
//! signer must verify device provenance before settling its receipt; this
//! layer checks signatures independently and applies them under the lock.

use super::approve::{sign_refusal, ApproveInput, SigningServices};
use super::certificates::{
    parse_chain, verify_signature, CertificateIdentity, CertificateVerificationInput,
    RegistrationState,
};
use super::operational_device::{installed_profile, matches_canonical_request, InstalledDevice};
use super::operational_policy::{approval_mode_for_event, OperationalSigningMode};
use super::operational_quorum::check_with_installed_device;
use super::requests::{is_overdue, read_request, relock};
use super::{SigningCaller, SigningError, SigningResult};
use crate::postgres::signing::{list_signing_approvals, SigningApprovalRow, SigningRequestRow};
use anyhow::Context;
use chrono::{DateTime, Utc};
use deadpool_postgres::Transaction;
use sequent_core::signing::{
    sha256_hex, DocumentKind, SignatureAlgorithm, SigningAction, SigningRequestStatus,
};
use serde::Deserialize;
use uuid::Uuid;

/// Internal request, not a request-body DTO. The transaction reads the
/// complete certificate chain and current key enrollment itself; no API
/// client supplies or selects a certificate file.
pub struct OperationalApprovalInput {
    pub request_id: Uuid,
    pub approval_challenge: Uuid,
    pub artifact_challenge: Option<Uuid>,
}

#[derive(Deserialize)]
struct Binding {
    tenant_id: Uuid,
    event_id: Uuid,
    request_id: Uuid,
    actor: String,
    action: SigningAction,
    purpose: String,
    key_id: String,
    key_version: i32,
    cert_fingerprint: String,
    cert_spki: String,
    payload_hash: String,
    message_hash: String,
    artifact_hash: String,
    pdf_revision: Option<i32>,
    pdf_parent_hash: Option<String>,
    pdf_digest: Option<String>,
    challenge_id: Uuid,
}

struct Receipt {
    binding: Binding,
    reservation: Uuid,
    signature_der: Vec<u8>,
    certificate_pem: String,
}

pub(super) struct OperationalEvidence {
    pub(super) input: ApproveInput,
    pub(super) certificate_id: Uuid,
    approval: Receipt,
    artifact: Option<Receipt>,
}

fn refusal() -> SigningError {
    SigningError::Forbidden("No exact operational signing receipt is available.".into())
}

async fn load(
    tx: &Transaction<'_>,
    row: &SigningRequestRow,
    caller: &SigningCaller,
    id: Uuid,
    purpose: &str,
    installed: InstalledDevice,
) -> SigningResult<Receipt> {
    let found = tx
        .query_opt(
            "SELECT binding, phase, reservation_id, receipt, certificate_pem
         FROM sequent_backend.operational_signing_get_completed($1,$2,$3,$4,$5)",
            &[
                &row.tenant_id,
                &row.election_event_id,
                &row.id,
                &caller.user_id,
                &id,
            ],
        )
        .await
        .context("Error reading the operational signing receipt")?
        .ok_or_else(refusal)?;
    let binding: Binding =
        serde_json::from_value(found.try_get(0).context("Missing operational binding")?)
            .context("Invalid operational binding")?;
    let phase: &str = found.try_get(1).context("Missing operational phase")?;
    let expected_key = format!("official/{}/{}", row.tenant_id, caller.user_id);
    if phase != "completed"
        || binding.challenge_id != id
        || binding.tenant_id != row.tenant_id
        || binding.event_id != row.election_event_id
        || binding.request_id != row.id
        || binding.actor != caller.user_id
        || binding.action != row.action
        || binding.purpose != purpose
        || binding.key_id != expected_key
        || binding.key_version <= 0
        || binding.payload_hash != row.payload_sha256
    {
        return Err(refusal());
    }
    let reservation: Uuid = found
        .try_get(2)
        .context("Missing operational reservation")?;
    let device_profile: Option<String> = tx
        .query_one(
            "SELECT sequent_backend.operational_signing_recorded_device_profile($1,$2,$3,$4,$5)",
            &[
                &row.tenant_id,
                &row.election_event_id,
                &row.id,
                &caller.user_id,
                &reservation,
            ],
        )
        .await
        .context("Cannot check provenance of the applied device receipt")?
        .try_get(0)?;
    // The normal application cannot accept or count a disposable fixture
    // signature after database restore or profile reconfiguration.
    if !installed.admits(device_profile.as_deref()) {
        return Err(refusal());
    }
    Ok(Receipt {
        binding,
        reservation,
        signature_der: found.try_get(3).context("Missing device signature")?,
        certificate_pem: found
            .try_get::<_, Option<String>>(4)
            .context("Missing operational certificate field")?
            .ok_or_else(refusal)?,
    })
}

/// The typed purposes accepted by the durable device-intent ledger.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperationalSigningPurpose {
    ApprovalPayload,
    Eml,
    PdfCms,
}

impl OperationalSigningPurpose {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ApprovalPayload => "approval-payload",
            Self::Eml => "eml",
            Self::PdfCms => "pdf-cms",
        }
    }
}

fn purposes(
    action: SigningAction,
) -> (OperationalSigningPurpose, Option<OperationalSigningPurpose>) {
    let artifact = match action.document() {
        DocumentKind::NoDocument => None,
        DocumentKind::Eml => Some(OperationalSigningPurpose::Eml),
        DocumentKind::Pdf => Some(OperationalSigningPurpose::PdfCms),
    };
    (OperationalSigningPurpose::ApprovalPayload, artifact)
}

/// Internal broker input. Do not serialize or log the exact bytes, which
/// may include a not-yet-released EML or PDF signing attributes.
pub struct OperationalSigningMaterial {
    pub purpose: OperationalSigningPurpose,
    pub message_sha256: String,
    pub artifact_sha256: String,
    pub pdf_revision: Option<i32>,
    pub pdf_parent_sha256: Option<String>,
    pub pdf_byte_range_sha256: Option<String>,
    exact_message: Vec<u8>,
}

impl OperationalSigningMaterial {
    fn detached(purpose: OperationalSigningPurpose, exact_message: Vec<u8>) -> Self {
        let sha256 = sha256_hex(&exact_message);
        Self {
            purpose,
            message_sha256: sha256.clone(),
            artifact_sha256: sha256,
            pdf_revision: None,
            pdf_parent_sha256: None,
            pdf_byte_range_sha256: None,
            exact_message,
        }
    }

    pub fn exact_message(&self) -> &[u8] {
        &self.exact_message
    }
}

pub struct OperationalApprovalPlan {
    pub tenant_id: Uuid,
    pub event_id: Uuid,
    pub request_id: Uuid,
    pub actor: String,
    pub action: SigningAction,
    pub key_id: String,
    pub key_version: i32,
    pub cert_fingerprint: String,
    pub cert_spki: String,
    pub request_expires_at: Option<DateTime<Utc>>,
    pub payload: OperationalSigningMaterial,
    pub artifact: Option<OperationalSigningMaterial>,
}

/// Recheck the ordinary permission, requester rule, exact actor trustee ownership,
/// bound event policy and live waiting rule before continuing a bound device
/// attempt. No enrollment, Gold check or signature is implied.
pub async fn reauthorize_operational_approval(
    tx: &Transaction<'_>,
    caller: &SigningCaller,
    tenant_id: Uuid,
    request_id: Uuid,
) -> SigningResult<SigningRequestRow> {
    // The sealed binary has a different exact Initialize-only broker.
    if cfg!(feature = "operational-signing-demo-fixture") {
        return Err(refusal());
    }
    let found = read_request(tx, tenant_id, request_id).await?;
    if let Some((_, message)) = sign_refusal(tx, caller, &found).await? {
        return Err(SigningError::Forbidden(message.into()));
    }
    if approval_mode_for_event(tx, tenant_id, found.election_event_id)
        .await
        .map_err(|error| SigningError::Forbidden(error.to_string()))?
        != OperationalSigningMode::HsmRequired
    {
        return Err(refusal());
    }
    let row = relock(tx, &found).await?;
    if row.status != SigningRequestStatus::Waiting
        || is_overdue(tx, &row).await?
        || !matches_canonical_request(&row)
    {
        return Err(refusal());
    }
    Ok(row)
}

/// Build the ordinary product's exact device work from a permitted request.
/// A trusted ingress supplies the authenticated actor and the reviewed hash.
/// This is not authentication, a grant, or approval. The broker must bind
/// each returned purpose to its own fresh, one-use ceremony and validate
/// Gold authentication and device/CA evidence before hardware invocation.
pub async fn prepare_operational_approval(
    tx: &Transaction<'_>,
    services: &SigningServices,
    caller: &SigningCaller,
    tenant_id: Uuid,
    request_id: Uuid,
    reviewed_payload_sha256: &str,
    now: DateTime<Utc>,
) -> SigningResult<OperationalApprovalPlan> {
    let row = reauthorize_operational_approval(tx, caller, tenant_id, request_id).await?;
    if row.payload_sha256 != reviewed_payload_sha256
        || !services.documents.supports(row.action.document())
    {
        return Err(refusal());
    }
    let installed = installed_profile(tx, row.tenant_id, row.election_event_id, row.action)
        .await?
        .ok_or_else(refusal)?;
    let approvals =
        list_signing_approvals(tx, row.tenant_id, row.election_event_id, row.id).await?;
    if approvals.iter().any(|item| item.user_id == caller.user_id)
        || !check_with_installed_device(tx, &row, &approvals, installed).await?
    {
        return Err(refusal());
    }
    // Same lock order as certificate attachment: request/event, then
    // official. A key retirement cannot race selecting the material.
    let official_lock = format!(
        "operational-signing-official:{}:{}",
        row.tenant_id, caller.user_id
    );
    tx.query_one(
        "SELECT pg_advisory_xact_lock_shared(hashtextextended($1, 0))",
        &[&official_lock],
    )
    .await
    .context("Error locking the official's active key")?;
    let key = tx
        .query_opt(
            "SELECT key_id, key_version, cert_fingerprint, cert_spki, certificate_id, chain_pem,
                    device_profile
             FROM sequent_backend.operational_signing_current_approval_key($1, $2, $3, $4)",
            &[
                &row.tenant_id,
                &row.election_event_id,
                &row.id,
                &caller.user_id,
            ],
        )
        .await
        .context("Error reading the official's active approval key")?
        .ok_or_else(refusal)?;
    let key_id: String = key
        .try_get("key_id")
        .context("Missing operational key ID")?;
    let key_version: i32 = key
        .try_get("key_version")
        .context("Missing operational key version")?;
    let cert_fingerprint: String = key
        .try_get("cert_fingerprint")
        .context("Missing certificate fingerprint")?;
    let cert_spki: String = key
        .try_get("cert_spki")
        .context("Missing certificate public key")?;
    let certificate_id: Uuid = key
        .try_get("certificate_id")
        .context("Missing certificate ID")?;
    let chain: Vec<String> = key
        .try_get("chain_pem")
        .context("Missing server certificate chain")?;
    let profile: &str = key
        .try_get("device_profile")
        .context("Missing installed device profile")?;
    if !installed.admits(Some(profile)) {
        return Err(refusal());
    }
    let check = services
        .verifier
        .verify(
            tx,
            &CertificateVerificationInput {
                request: &row,
                signer: caller.actor(),
                chain_pem: chain.clone(),
                signatures: None,
                now,
            },
        )
        .await?;
    let certificate = check
        .certificate
        .as_ref()
        .filter(|cert| {
            cert.fingerprint_sha256 == cert_fingerprint
                && cert.spki_sha256 == cert_spki
                && cert.key_algorithm == Some(SignatureAlgorithm::EcdsaP256Sha256)
        })
        .ok_or_else(refusal)?;
    if !check.passed()
        || !matches!(&check.registration, RegistrationState::Registered(registration) if registration.id == certificate_id)
    {
        return Err(refusal());
    }

    services
        .documents
        .check_operational_certificate(tx, &row, &caller.actor(), certificate)
        .await?;

    let (purpose, artifact_purpose) = purposes(row.action);
    let payload =
        OperationalSigningMaterial::detached(purpose, row.canonical_payload.as_bytes().to_vec());
    let artifact = match artifact_purpose {
        None => None,
        Some(OperationalSigningPurpose::Eml) => {
            let eml = exact_eml(tx, services, &row).await?;
            Some(OperationalSigningMaterial::detached(
                OperationalSigningPurpose::Eml,
                eml,
            ))
        }
        Some(OperationalSigningPurpose::PdfCms) => {
            let prepared = services
                .documents
                .prepare_operational_pdf(tx, caller, &row, &chain, now)
                .await?;
            let artifact = services
                .documents
                .operational_pdf_artifact(tx, &row, &caller.user_id, prepared.revision, certificate)
                .await?;
            let exact_message = artifact.cms.exact_message().to_vec();
            Some(OperationalSigningMaterial {
                purpose: OperationalSigningPurpose::PdfCms,
                message_sha256: sha256_hex(&exact_message),
                artifact_sha256: hex::encode(artifact.prepared_pdf_sha256),
                pdf_revision: Some(artifact.revision),
                pdf_parent_sha256: Some(hex::encode(artifact.parent_sha256)),
                pdf_byte_range_sha256: Some(hex::encode(artifact.byte_range_sha256)),
                exact_message,
            })
        }
        Some(OperationalSigningPurpose::ApprovalPayload) => return Err(refusal()),
    };
    Ok(OperationalApprovalPlan {
        tenant_id: row.tenant_id,
        event_id: row.election_event_id,
        request_id: row.id,
        actor: caller.user_id.clone(),
        action: row.action,
        key_id,
        key_version,
        cert_fingerprint,
        cert_spki,
        request_expires_at: row.expires_at,
        payload,
        artifact,
    })
}

async fn exact_eml(
    tx: &Transaction<'_>,
    services: &SigningServices,
    request: &SigningRequestRow,
) -> SigningResult<Vec<u8>> {
    let exact = services
        .documents
        .document_bytes(tx, request)
        .await?
        .ok_or_else(refusal)?;
    let hash = sha256_hex(&exact);
    if request.document_sha256.as_deref() != Some(hash.as_str())
        || request
            .subject
            .get("eml_sha256")
            .and_then(serde_json::Value::as_str)
            != Some(hash.as_str())
    {
        return Err(refusal());
    }
    Ok(exact)
}

impl OperationalEvidence {
    pub(super) async fn load(
        tx: &Transaction<'_>,
        services: &SigningServices,
        row: &SigningRequestRow,
        caller: &SigningCaller,
        requested: &OperationalApprovalInput,
        existing: &[SigningApprovalRow],
    ) -> SigningResult<Self> {
        // A legacy signature from before a one-way promotion cannot be
        // counted in a still-waiting HSM quorum. Fail closed on mixed state.
        let (payload_purpose, artifact_purpose) = purposes(row.action);
        let installed = installed_profile(tx, row.tenant_id, row.election_event_id, row.action)
            .await?
            .ok_or_else(refusal)?;
        if !check_with_installed_device(tx, row, existing, installed).await? {
            return Err(SigningError::Conflict(
                "An earlier approval did not use the required signing device. Start a new request."
                    .into(),
            ));
        }
        let approval = load(
            tx,
            row,
            caller,
            requested.approval_challenge,
            payload_purpose.as_str(),
            installed,
        )
        .await?;
        let enrollment = tx
            .query_opt(
                "SELECT certificate_id, chain_pem, device_profile
             FROM sequent_backend.operational_signing_get_official_chain($1,$2,$3,$4,$5,$6,$7)",
                &[
                    &row.tenant_id,
                    &row.election_event_id,
                    &caller.user_id,
                    &approval.binding.key_id,
                    &approval.binding.key_version,
                    &approval.binding.cert_fingerprint,
                    &approval.binding.cert_spki,
                ],
            )
            .await
            .context("Error reading the current official enrollment")?
            .ok_or_else(refusal)?;
        let chain_pem: Vec<String> = enrollment
            .try_get(1)
            .context("Missing enrolled certificate chain")?;
        let certificate_id: Uuid = enrollment
            .try_get(0)
            .context("Missing enrolled certificate ID")?;
        let profile: &str = enrollment
            .try_get(2)
            .context("Missing enrolled device profile")?;
        if !installed.admits(Some(profile)) {
            return Err(refusal());
        }
        let chain = parse_chain(&chain_pem)?;
        let leaf = chain.first().ok_or_else(refusal)?;
        let persisted = openssl::x509::X509::from_pem(approval.certificate_pem.as_bytes())
            .context("Invalid stored operational certificate")?;
        let certificate = CertificateIdentity::of(leaf, &chain)?;
        if persisted
            .to_der()
            .context("Invalid stored operational certificate")?
            != leaf
                .to_der()
                .context("Invalid current operational certificate")?
            || certificate.fingerprint_sha256 != approval.binding.cert_fingerprint
            || certificate.spki_sha256 != approval.binding.cert_spki
        {
            return Err(refusal());
        }
        let algorithm = SignatureAlgorithm::EcdsaP256Sha256;
        let payload = row.canonical_payload.as_bytes();
        let payload_hash = sha256_hex(payload);
        if payload_hash != row.payload_sha256
            || payload_hash != approval.binding.message_hash
            || payload_hash != approval.binding.artifact_hash
            || !verify_signature(leaf, algorithm, payload, &approval.signature_der)?
        {
            return Err(refusal());
        }
        let mut input = ApproveInput {
            request_id: row.id,
            chain_pem,
            algorithm,
            payload_signature: approval.signature_der.clone(),
            document_signature: None,
            pdf_cms: None,
            revision: None,
        };
        let artifact = match (requested.artifact_challenge, artifact_purpose) {
            (None, None) => None,
            (Some(id), Some(purpose)) => {
                Some(load(tx, row, caller, id, purpose.as_str(), installed).await?)
            }
            _ => return Err(refusal()),
        };
        if let Some(receipt) = &artifact {
            if receipt.binding.key_version != approval.binding.key_version
                || receipt.binding.key_id != approval.binding.key_id
                || receipt.binding.cert_fingerprint != approval.binding.cert_fingerprint
                || receipt.binding.cert_spki != approval.binding.cert_spki
                || receipt.certificate_pem != approval.certificate_pem
            {
                return Err(refusal());
            }
            match row.action.document() {
                DocumentKind::Eml => {
                    let exact = exact_eml(tx, services, row).await?;
                    let hash = sha256_hex(&exact);
                    if hash != receipt.binding.message_hash
                        || hash != receipt.binding.artifact_hash
                        || !verify_signature(leaf, algorithm, &exact, &receipt.signature_der)?
                    {
                        return Err(refusal());
                    }
                    input.document_signature = Some(receipt.signature_der.clone());
                }
                DocumentKind::Pdf => {
                    let revision = receipt.binding.pdf_revision.ok_or_else(refusal)?;
                    let artifact = services
                        .documents
                        .operational_pdf_artifact(tx, row, &caller.user_id, revision, &certificate)
                        .await?;
                    let matches = receipt.binding.pdf_parent_hash.as_deref()
                        == Some(hex::encode(artifact.parent_sha256).as_str())
                        && receipt.binding.pdf_digest.as_deref()
                            == Some(hex::encode(artifact.byte_range_sha256).as_str())
                        && receipt.binding.artifact_hash
                            == hex::encode(artifact.prepared_pdf_sha256)
                        && receipt.binding.message_hash == sha256_hex(artifact.cms.exact_message());
                    if !matches {
                        return Err(refusal());
                    }
                    input.pdf_cms = Some(
                        artifact
                            .cms
                            .finish(&receipt.signature_der)
                            .context("Invalid device PDF signature")?,
                    );
                    input.revision = Some(revision);
                }
                DocumentKind::NoDocument => return Err(refusal()),
            }
        }
        Ok(Self {
            input,
            certificate_id,
            approval,
            artifact,
        })
    }

    /// Only after insert and exact PDF/EML validation succeeded, inside the
    /// SAME savepoint. Does not sign, dispatch, or permit a stale revision.
    pub(super) async fn mark_applied(&self, tx: &Transaction<'_>) -> SigningResult<()> {
        self.apply_one(tx, &self.approval).await?;
        if let Some(document) = &self.artifact {
            self.apply_one(tx, document).await?;
        }
        Ok(())
    }

    async fn apply_one(&self, tx: &Transaction<'_>, receipt: &Receipt) -> SigningResult<()> {
        let binding = &receipt.binding;
        let approved: &[u8] = match binding.purpose.as_str() {
            "approval-payload" => &self.input.payload_signature,
            "pdf-cms" => self.input.pdf_cms.as_deref().ok_or_else(refusal)?,
            "eml" => self
                .input
                .document_signature
                .as_deref()
                .ok_or_else(refusal)?,
            _ => return Err(refusal()),
        };
        let applied: bool = tx.query_one(
            "SELECT sequent_backend.operational_signing_mark_applied_from_evidence($1,$2,$3,$4,$5,$6,$7,$8)",
            &[&binding.tenant_id, &binding.event_id, &binding.request_id, &binding.actor,
              &binding.challenge_id, &receipt.reservation, &receipt.signature_der, &approved],
        ).await.context("Error applying the operational signing receipt")?.try_get(0)
            .context("Invalid applied signing result")?;
        if !applied {
            return Err(refusal());
        }
        Ok(())
    }
}

#[cfg(test)]
mod purpose_tests {
    use super::*;
    use strum::IntoEnumIterator;

    #[test]
    fn operational_parts_cover_every_protected_action() {
        use OperationalSigningPurpose::{ApprovalPayload, Eml, PdfCms};
        let mut count = 0;
        for action in SigningAction::iter() {
            let expected = match action {
                SigningAction::InitializeVoting
                | SigningAction::OpenVoting
                | SigningAction::CloseVoting
                | SigningAction::ApproveConfiguration
                | SigningAction::ApproveVoter
                | SigningAction::ConfirmCustodyProof
                | SigningAction::ContributeCustodyProof => None,
                SigningAction::GenerateElectionReturns | SigningAction::GenerateReports => {
                    Some(PdfCms)
                }
                SigningAction::TransmitResults => Some(Eml),
            };
            assert_eq!(purposes(action), (ApprovalPayload, expected));
            count += 1;
        }
        assert_eq!(count, 10);
    }
}
