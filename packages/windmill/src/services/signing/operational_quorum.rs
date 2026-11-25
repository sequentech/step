// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Consumption-side check for an already persisted operational quorum.
//! The original approval transaction verified the receipts cryptographically.

use super::operational_device::{
    matches_canonical_request, profile_for_canonical_request, InstalledDevice,
};
use super::operational_policy::local_scope::request_mode_and_latch;
#[cfg(feature = "comelec-demo-direct-hsm")]
use super::operational_policy::local_scope::DirectProtocol;
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
    if let (_, Some(latch)) = request_mode_and_latch(tx, request).await? {
        #[cfg(feature = "comelec-demo-direct-hsm")]
        {
            if latch.protocol == DirectProtocol::InitializeV1 {
                return super::direct_initialize::has_quorum(
                    tx, request, latch, approvals, minimum,
                )
                .await;
            }
            return super::direct_actions::has_quorum(tx, request, latch, approvals, minimum).await;
        }
        #[cfg(not(feature = "comelec-demo-direct-hsm"))]
        {
            let _ = latch;
            return Ok(false);
        }
    }
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
    Ok(checked_approval_device(tx, request, approvals)
        .await?
        .is_some())
}

/// Return the installed device only when every prior approval matches it.
/// An empty slice is valid when applying the first receipt. Consumption also
/// requires the frozen and current quorum in has_operational_quorum.
pub(super) async fn checked_approval_device(
    tx: &Transaction<'_>,
    request: &SigningRequestRow,
    approvals: &[SigningApprovalRow],
) -> Result<Option<InstalledDevice>> {
    let distinct: std::collections::HashSet<&str> = approvals
        .iter()
        .map(|approval| approval.user_id.as_str())
        .collect();
    if distinct.len() != approvals.len()
        || !matches_canonical_request(request)
        || approvals.iter().any(SigningApprovalRow::is_direct_demo)
    {
        return Ok(None);
    }
    if !approvals.is_empty() {
        // The consuming worker may already own the request row (rather than the
        // event lock). Never wait for a revoker holding the event/official lock
        // while it waits on that same request. Refuse a concurrent change;
        // otherwise retain shared locks through this transaction and use the
        // existing authoritative current-chain read without reversing lock order.
        let scopes: Vec<String> = std::iter::once(format!(
            "signing-event:{}:{}",
            request.tenant_id, request.election_event_id
        ))
        .chain(approvals.iter().map(|a| {
            format!(
                "operational-signing-official:{}:{}",
                request.tenant_id, a.user_id
            )
        }))
        .collect();
        let held: bool = tx.query_one(
            "SELECT coalesce(bool_and(pg_try_advisory_xact_lock_shared(hashtextextended(k,0))),false)
             FROM unnest($1::text[]) AS locks(k)",
            &[&scopes],
        ).await?.try_get(0)?;
        anyhow::ensure!(
            held,
            "Operational signing key or event is changing; retry the check"
        );
    }
    if request_mode_and_latch(tx, request).await?.1.is_some() {
        return Ok(None);
    }
    let Some(device) = profile_for_canonical_request(tx, request).await? else {
        return Ok(None);
    };
    let document_part = match request.action.document() {
        DocumentKind::NoDocument => None,
        DocumentKind::Pdf => Some("pdf-cms"),
        DocumentKind::Eml => Some("eml"),
    };
    // Approval rows are unique per request and user. Validate every row, not
    // just a sufficient subset: mixed legacy approvals cannot dispatch.
    for approval in approvals {
        let (Some(fingerprint), Some(certificate_id)) = (
            approval.fingerprint_sha256.as_deref(),
            approval.certificate_id,
        ) else {
            return Ok(None);
        };
        let matching: bool = tx
            .query_one(
                include_str!("sql/operational_quorum.sql"),
                &[
                    &request.tenant_id,
                    &request.election_event_id,
                    &request.id,
                    &approval.user_id,
                    &request.action.to_string(),
                    &request.payload_sha256,
                    &fingerprint,
                    &approval.spki_sha256,
                    &approval.payload_signature,
                    &document_part,
                    &request.document_sha256,
                    &approval.document_signature,
                    &approval.pdf_cms,
                    &device.as_str(),
                    &certificate_id,
                ],
            )
            .await
            .context("Cannot verify the applied operational signing receipts")?
            .try_get(0)
            .context("Invalid operational signing receipt check")?;
        if !matching {
            return Ok(None);
        }
    }
    Ok(Some(device))
}
