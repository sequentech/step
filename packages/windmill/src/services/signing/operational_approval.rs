// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Converts server-owned device receipts into the existing approval. The
//! ingress identifies sealed receipts only, with no caller-supplied
//! signature, key, bytes, certificate selection, or mode. The private
//! signer must verify device provenance before settling its receipt; this
//! layer checks signatures independently and applies them under the lock.

use super::approve::{ApproveInput, SigningServices};
use super::certificates::{parse_chain, verify_signature, CertificateIdentity};
use super::operational_device::InstalledDevice;
use super::operational_quorum::checked_approval_device;
use super::pdf::{operational_pdf_artifact, S3RevisionStore};
use super::{SigningCaller, SigningError, SigningResult};
use crate::postgres::signing::{SigningApprovalRow, SigningRequestRow};
use anyhow::Context;
use deadpool_postgres::Transaction;
use sequent_core::signing::{sha256_hex, DocumentKind, SignatureAlgorithm, SigningAction};
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
    // The recorded PKCS#11 profile must match this request's installed device.
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
        let parts: &[&str] = match row.action.document() {
            DocumentKind::NoDocument => &["approval-payload"],
            DocumentKind::Eml => &["approval-payload", "eml"],
            DocumentKind::Pdf => &["approval-payload", "pdf-cms"],
        };
        let installed = checked_approval_device(tx, row, existing).await?.ok_or_else(|| {
            if existing.is_empty() {
                refusal()
            } else {
                SigningError::Conflict(
                    "An earlier approval did not use the required signing device. Start a new request."
                        .into(),
                )
            }
        })?;
        let approval = load(
            tx,
            row,
            caller,
            requested.approval_challenge,
            parts[0],
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
        let artifact = match (requested.artifact_challenge, parts.get(1)) {
            (None, None) => None,
            (Some(id), Some(purpose)) => Some(load(tx, row, caller, id, purpose, installed).await?),
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
                    let exact = services
                        .documents
                        .document_bytes(tx, row)
                        .await?
                        .ok_or_else(refusal)?;
                    let hash = sha256_hex(&exact);
                    if row.document_sha256.as_deref() != Some(hash.as_str())
                        || row
                            .subject
                            .get("eml_sha256")
                            .and_then(serde_json::Value::as_str)
                            != Some(hash.as_str())
                        || hash != receipt.binding.message_hash
                        || hash != receipt.binding.artifact_hash
                        || !verify_signature(leaf, algorithm, &exact, &receipt.signature_der)?
                    {
                        return Err(refusal());
                    }
                    input.document_signature = Some(receipt.signature_der.clone());
                }
                DocumentKind::Pdf => {
                    let revision = receipt.binding.pdf_revision.ok_or_else(refusal)?;
                    let artifact = operational_pdf_artifact(
                        tx,
                        row,
                        &caller.user_id,
                        revision,
                        &certificate,
                        &S3RevisionStore,
                    )
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
