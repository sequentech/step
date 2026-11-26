// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The first activation-free orchestration slice: one Initialize voting
//! payload approval, with the real signing access and receipt rules.
//! This does not authorize the step-up client or contact a device.

use super::approve::sign_refusal;
#[cfg(feature = "felix3-local-initialize")]
pub use super::operational_device::local_initialize::LocalInitializeInstallation;
use super::operational_device::matches_canonical_request;
use super::operational_policy::local_scope::request_mode_and_latch;
use super::operational_policy::{lock_signing_policy, OperationalSigningMode};
use super::operational_quorum::checked_approval_device;
use super::requests::{is_overdue, lock_request, read_request};
use super::{SigningCaller, SigningError, SigningResult};
use crate::postgres::signing::{list_signing_approvals, SigningRequestRow};
use anyhow::Context;
use deadpool_postgres::Transaction;
use sequent_core::signing::{SigningAction, SigningRequestStatus, SigningRule};
use uuid::Uuid;

/// Only this service can construct a scope from the authorized request.
pub struct InitializeSigningScope {
    request: SigningRequestRow,
    actor: Uuid,
    #[cfg(feature = "felix3-local-initialize")]
    local_installation: Option<&'static LocalInitializeInstallation>,
}

impl InitializeSigningScope {
    pub fn tenant(&self) -> Uuid {
        self.request.tenant_id
    }

    pub fn event(&self) -> Uuid {
        self.request.election_event_id
    }

    pub fn request(&self) -> Uuid {
        self.request.id
    }

    /// The exact single Post bound to the canonical Initialize request.
    pub fn election(&self) -> Option<Uuid> {
        self.request.election_id
    }

    pub fn actor(&self) -> Uuid {
        self.actor
    }

    pub fn payload_sha256(&self) -> &str {
        &self.request.payload_sha256
    }

    /// Present only for the exact operator installation verified in the scope's
    /// transaction. Preparing a callback and applying its receipt recheck it.
    #[cfg(feature = "felix3-local-initialize")]
    pub fn local_installation(&self) -> Option<&LocalInitializeInstallation> {
        self.local_installation
    }
}

fn unavailable() -> SigningError {
    SigningError::Forbidden("Device signing for this request is not available.".into())
}

async fn scope(
    tx: &Transaction<'_>,
    caller: &SigningCaller,
    tenant: Uuid,
    request_id: Uuid,
    changes_state: bool,
) -> SigningResult<InitializeSigningScope> {
    lock_signing_policy(tx)
        .await
        .context("Lock signing policy")?;
    let initial = read_request(tx, tenant, request_id).await?;
    if initial.action != SigningAction::InitializeVoting {
        return Err(unavailable());
    }
    if sign_refusal(tx, caller, &initial).await?.is_some() {
        return Err(unavailable());
    }
    let request = if changes_state {
        lock_request(tx, tenant, request_id).await?
    } else {
        initial
    };
    let (mode, direct) = request_mode_and_latch(tx, &request)
        .await
        .map_err(|_| unavailable())?;
    if direct.is_some() || mode != OperationalSigningMode::HsmRequired {
        return Err(unavailable());
    }
    let rule: SigningRule = serde_json::from_value(request.rule_snapshot.clone())
        .context("Read frozen signing rule")?;
    if request.election_id.is_none_or(|post| post.is_nil())
        || rule.action != request.action
        || rule.validate().is_err()
        || !rule.is_required()
        || i32::from(rule.required()) != request.required
        || request.document_id.is_some()
        || request.document_sha256.is_some()
        || !matches_canonical_request(&request)
    {
        return Err(unavailable());
    }
    let actor = Uuid::parse_str(&caller.user_id)
        .ok()
        .filter(|id| !id.is_nil() && id.to_string() == caller.user_id)
        .ok_or_else(unavailable)?;
    Ok(InitializeSigningScope {
        #[cfg(feature = "felix3-local-initialize")]
        local_installation: super::operational_device::local_initialize::for_canonical_request(
            tx, &request,
        )
        .await?,
        request,
        actor,
    })
}

/// Same authorization and immutable scope without requiring a live attempt.
pub async fn initialize_signing_scope(
    tx: &Transaction<'_>,
    caller: &SigningCaller,
    tenant: Uuid,
    request: Uuid,
) -> SigningResult<InitializeSigningScope> {
    scope(tx, caller, tenant, request, false).await
}

/// Checked both when starting PAR and when a currently authenticated
/// browser returns from the step-up. The private orchestrator must
/// independently recheck the same immutable tuple when reserving.
pub async fn prepare_initialize_signing(
    tx: &Transaction<'_>,
    caller: &SigningCaller,
    tenant: Uuid,
    request_id: Uuid,
) -> SigningResult<InitializeSigningScope> {
    let scoped = scope(tx, caller, tenant, request_id, true).await?;
    let request = &scoped.request;
    if request.status != SigningRequestStatus::Waiting || is_overdue(tx, request).await? {
        return Err(SigningError::Closed {
            status: request.status,
            message: "This request can no longer receive another approval.".into(),
        });
    }
    Ok(scoped)
}

/// Returns true only for this currently authorized actor's exact persisted
/// approval, never other actors' receipts or an inferred successful response.
pub async fn initialize_signing_applied(
    tx: &Transaction<'_>,
    caller: &SigningCaller,
    tenant: Uuid,
    request_id: Uuid,
) -> SigningResult<bool> {
    let scoped = scope(tx, caller, tenant, request_id, false).await?;
    let request = &scoped.request;
    let rows = list_signing_approvals(tx, tenant, request.election_event_id, request.id).await?;
    let Some(approval) = rows.iter().find(|row| row.user_id == caller.user_id) else {
        return Ok(false);
    };
    Ok(
        checked_approval_device(tx, request, std::slice::from_ref(approval))
            .await?
            .is_some(),
    )
}
