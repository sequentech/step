// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Consumption-side check for an already persisted operational quorum.
//! The original approval transaction verified the receipts cryptographically.

use crate::postgres::signing::{SigningApprovalRow, SigningRequestRow};
use anyhow::{Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::signing::SigningRule;

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
    // The device-provenance registry this check needs is not part of this
    // checkpoint, so no operational approval can be counted yet.
    let _ = (tx, request);
    if !approvals.is_empty() {
        anyhow::bail!("Operational device provenance is not available");
    }
    Ok(true)
}
