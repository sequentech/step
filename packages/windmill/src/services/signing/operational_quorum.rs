// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Consumption-side check for an already persisted operational quorum.
//! The original approval transaction verified the receipts cryptographically.

use crate::postgres::signing::{SigningApprovalRow, SigningRequestRow};
use anyhow::{Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::signing::{DocumentKind, SigningRule};

pub(super) async fn has_operational_quorum(
    tx: &Transaction<'_>,
    request: &SigningRequestRow,
    approvals: &[SigningApprovalRow],
    minimum: i64,
) -> Result<bool> {
    let snapshot: SigningRule = serde_json::from_value(request.rule_snapshot.clone())
        .context("Invalid frozen operational signing rule")?;
    if snapshot.action != request.action
        || snapshot.validate().is_err()
        || !snapshot.is_required()
        || i32::from(snapshot.required()) != request.required
        || i64::try_from(approvals.len())? < minimum.max(i64::from(request.required)).max(1)
    {
        return Ok(false);
    }
    check_operational_approvals(tx, request, approvals).await
}

/// Validate prior approvals with the very same evidence contract as the
/// consuming action. This deliberately accepts an empty slice: quorum
/// eligibility/threshold is checked separately by the authoritative caller.
pub(super) async fn check_operational_approvals(
    tx: &Transaction<'_>,
    request: &SigningRequestRow,
    approvals: &[SigningApprovalRow],
) -> Result<bool> {
    let document_part = match request.action.document() {
        DocumentKind::NoDocument => None,
        DocumentKind::Pdf => Some("pdf-cms"),
        DocumentKind::Eml => Some("eml"),
    };
    // Approval rows are unique per request and user. Validate every row, not
    // just a sufficient subset: mixed legacy approvals cannot dispatch.
    for approval in approvals {
        let matching: bool = tx
            .query_one(
                "SELECT EXISTS (
                SELECT FROM sequent_backend.operational_signing_intent p
                WHERE (p.tenant_id,p.event_id,p.request_id,p.actor) = ($1,$2,$3,$4)
                  AND p.phase='applied' AND p.purpose='approval-payload'
                  AND sequent_backend.operational_signing_recorded_device_profile(
                    p.tenant_id,p.event_id,p.request_id,p.actor,p.reservation_id
                  )='yubihsm'
                  AND (p.binding).action=$5 AND (p.binding).payload_hash=$6
                  AND (p.binding).cert_fingerprint=$7 AND (p.binding).cert_spki=$8
                  AND (p.binding).message_hash=$6 AND p.receipt=$9
                  AND p.applied_signature_sha256=encode(sha256($9::bytea),'hex')
                  AND ($10::text IS NULL OR EXISTS (
                    SELECT FROM sequent_backend.operational_signing_intent d
                    WHERE (d.tenant_id,d.event_id,d.request_id,d.actor) = ($1,$2,$3,$4)
                      AND d.phase='applied' AND d.purpose=$10
                      AND sequent_backend.operational_signing_recorded_device_profile(
                        d.tenant_id,d.event_id,d.request_id,d.actor,d.reservation_id
                      )='yubihsm'
                      AND (d.binding).action=$5 AND (d.binding).payload_hash=$6
                      AND (d.binding).cert_fingerprint=$7 AND (d.binding).cert_spki=$8
                      AND (d.binding).key_id=(p.binding).key_id
                      AND (d.binding).key_version=(p.binding).key_version
                      AND (
                        ($10='eml' AND (d.binding).message_hash=$11 AND d.receipt=$12
                            AND d.applied_signature_sha256=encode(sha256($12::bytea),'hex'))
                        OR ($10='pdf-cms' AND d.applied_signature_sha256=encode(sha256($13::bytea),'hex')
                            AND EXISTS (
                                SELECT FROM sequent_backend.signing_document_revision AS signed
                                WHERE (signed.tenant_id,signed.election_event_id,signed.request_id,signed.id)
                                    = (d.tenant_id,d.event_id,d.request_id,d.applied_pdf_revision_id)
                                  AND signed.state='signed' AND signed.document_id=d.applied_pdf_document_id
                                  AND signed.sha256=d.applied_pdf_sha256 AND signed.prepared_for_user=d.actor
                                  AND signed.parent_sha256=(d.binding).pdf_parent_hash
                                  AND signed.digest_sha256=(d.binding).pdf_digest
                            ))
                      )
                  ))
            )",
                &[
                    &request.tenant_id,
                    &request.election_event_id,
                    &request.id,
                    &approval.user_id,
                    &request.action.to_string(),
                    &request.payload_sha256,
                    &approval.fingerprint_sha256,
                    &approval.spki_sha256,
                    &approval.payload_signature,
                    &document_part,
                    &request.document_sha256,
                    &approval.document_signature,
                    &approval.pdf_cms,
                ],
            )
            .await
            .context("Cannot verify the applied operational signing receipts")?
            .try_get(0)
            .context("Invalid operational signing receipt check")?;
        if !matching {
            return Ok(false);
        }
    }
    Ok(true)
}
