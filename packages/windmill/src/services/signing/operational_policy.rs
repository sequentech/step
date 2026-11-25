// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Authoritative deployment policy shared by Harvest and Windmill.
//! HSM-required denies legacy ingress. Receipt admission separately checks
//! the device installed for the request.

pub(super) mod local_scope;
pub use local_scope::{request_signing_mode, require_legacy_request};

use deadpool_postgres::{GenericClient, Transaction};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperationalSigningMode {
    LegacyP12,
    HsmRequired,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LegacySigningBlocked {
    HsmRequired,
    Unavailable,
    ScopeUnavailable,
    WholePostRequired,
    #[cfg(feature = "felix3-local-initialize")]
    LocalInstallationUnavailable,
}

impl std::fmt::Display for LegacySigningBlocked {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::HsmRequired => "This tenant requires a server-managed signing device.",
            Self::WholePostRequired => {
                "Initialize the selected Post by itself, without a country filter."
            }
            Self::ScopeUnavailable => "Device signing for this request is unavailable.",
            Self::Unavailable => "The official signing mode is unavailable.",
            #[cfg(feature = "felix3-local-initialize")]
            Self::LocalInstallationUnavailable => concat!(
                "Cannot verify /run/sequent-felix3-local-initialize/installation.json ",
                "against the application's signing policy and installation approval."
            ),
        })
    }
}

impl std::error::Error for LegacySigningBlocked {}

impl OperationalSigningMode {
    fn parse(mode: &str) -> Result<Self, LegacySigningBlocked> {
        match mode {
            "legacy-p12" => Ok(Self::LegacyP12),
            "hsm-required" => Ok(Self::HsmRequired),
            _ => Err(LegacySigningBlocked::Unavailable),
        }
    }
}

// Only transaction callers can retain the policy read lock through their
// consequential operation. Promotion takes the matching exclusive lock.
// Read under the usual READ COMMITTED isolation used by these services.
pub async fn signing_mode(
    tx: &Transaction<'_>,
    tenant: Uuid,
) -> Result<OperationalSigningMode, LegacySigningBlocked> {
    lock_signing_policy(tx).await?;
    read_tenant_mode(tx, tenant).await
}

pub(super) async fn lock_signing_policy(tx: &Transaction<'_>) -> Result<(), LegacySigningBlocked> {
    tx.query_one(
        "SELECT pg_advisory_xact_lock_shared(hashtextextended('operational-signing-policy:deployment', 0))",
        &[],
    )
    .await
    .map_err(|_| LegacySigningBlocked::Unavailable)?;
    Ok(())
}

// Startup also uses this read with its existing application login. Operations
// use signing_mode so the policy lock is retained through commit.
pub(super) async fn read_tenant_mode(
    client: &impl GenericClient,
    tenant: Uuid,
) -> Result<OperationalSigningMode, LegacySigningBlocked> {
    let row = client
        .query_one(
            "SELECT mode, EXISTS (
                 SELECT FROM sequent_backend.operational_signing_required_tenant WHERE tenant_id = $1
             ) AS tenant_required
             FROM sequent_backend.operational_signing_policy WHERE singleton",
            &[&tenant],
        )
        .await
        .map_err(|_| LegacySigningBlocked::Unavailable)?;
    let mode = row
        .try_get::<_, &str>("mode")
        .map_err(|_| LegacySigningBlocked::Unavailable)?;
    let tenant_required = row
        .try_get::<_, bool>("tenant_required")
        .map_err(|_| LegacySigningBlocked::Unavailable)?;
    let mode = OperationalSigningMode::parse(mode)?;
    Ok(if tenant_required {
        OperationalSigningMode::HsmRequired
    } else {
        mode
    })
}

pub async fn require_legacy_signing(
    tx: &Transaction<'_>,
    tenant: Uuid,
) -> Result<(), LegacySigningBlocked> {
    match signing_mode(tx, tenant).await? {
        OperationalSigningMode::LegacyP12 => Ok(()),
        OperationalSigningMode::HsmRequired => Err(LegacySigningBlocked::HsmRequired),
    }
}

/// Installation environment is an optional assertion, never a source of
/// authority. A stand-alone COMELEC installation must assert hsm-required and
/// seed that deployment policy before serving; legacy-only startup is not a
/// fallback. Reject the superseded env settings rather than ignore them.
pub async fn assert_startup_policy(
    client: &impl GenericClient,
) -> Result<(), LegacySigningBlocked> {
    const PROFILE: &str = "OPERATIONAL_SIGNING_INSTALLATION_PROFILE";
    if std::env::var_os("OPERATIONAL_SIGNING_MODE").is_some()
        || std::env::var_os("OPERATIONAL_SIGNING_HSM_REQUIRED_TENANTS").is_some()
    {
        return Err(LegacySigningBlocked::Unavailable);
    }
    let expected = match std::env::var(PROFILE) {
        Ok(value) => Some(OperationalSigningMode::parse(&value)?),
        Err(std::env::VarError::NotPresent) => None,
        Err(std::env::VarError::NotUnicode(_)) => return Err(LegacySigningBlocked::Unavailable),
    };
    #[cfg(feature = "felix3-local-initialize")]
    if let Err(error) = super::operational_device::local_initialize::assert_startup(client).await {
        tracing::error!("Local Initialize installation verification failed: {error:#}");
        return Err(LegacySigningBlocked::LocalInstallationUnavailable);
    }
    #[cfg(feature = "comelec-demo-direct-hsm")]
    if let Err(error) = super::direct_initialize::assert_startup(client).await {
        tracing::error!("Direct Initialize installation verification failed: {error:#}");
        return Err(LegacySigningBlocked::ScopeUnavailable);
    }
    let row = client
        .query_one(
            "SELECT mode, (SELECT count(*) FROM sequent_backend.operational_signing_required_tenant)
             FROM sequent_backend.operational_signing_policy WHERE singleton",
            &[],
        )
        .await
        .map_err(|_| LegacySigningBlocked::Unavailable)?;
    let mode = row
        .try_get::<_, &str>("mode")
        .map_err(|_| LegacySigningBlocked::Unavailable)?;
    let actual = OperationalSigningMode::parse(mode)?;
    if expected.is_some_and(|profile| profile != actual) {
        return Err(LegacySigningBlocked::Unavailable);
    }
    // The staged source does not yet install the real per-signature path.
    // A standalone COMELEC server must not report a healthy start just
    // because its DB mode/profile agree. Replace this barrier only with a
    // verified installed runtime check, never a second operator toggle.
    // A mixed deployment can still start for its legacy clients; enrolled
    // HSM-required tenants continue to fail closed at guarded operations.
    if actual == OperationalSigningMode::HsmRequired {
        return Err(LegacySigningBlocked::Unavailable);
    }
    Ok(())
}

/// Event policy includes the explicitly installed direct-demo marker, never
/// the browser's selected mode. Other events keep the existing tenant policy.
pub async fn signing_mode_for_event(
    tx: &Transaction<'_>,
    tenant: Uuid,
    event: Uuid,
) -> Result<OperationalSigningMode, LegacySigningBlocked> {
    local_scope::event_action_mode(tx, tenant, event).await
}

/// Older MIRU upload/download surfaces already know their trusted event.
pub async fn require_legacy_event(
    tx: &Transaction<'_>,
    tenant: Uuid,
    event: Uuid,
) -> Result<(), LegacySigningBlocked> {
    match signing_mode_for_event(tx, tenant, event).await? {
        OperationalSigningMode::LegacyP12 => Ok(()),
        OperationalSigningMode::HsmRequired => Err(LegacySigningBlocked::HsmRequired),
    }
}

/// The schedule service uses this only as a restrictive proof requirement.
pub(crate) async fn direct_schedule_post(
    tx: &Transaction<'_>,
    tenant: Uuid,
    event: Uuid,
) -> anyhow::Result<Option<Uuid>> {
    // The scheduler may already hold the event lock. Never wait for a
    // policy writer that might wait on that event.
    let held: bool = tx.query_one(
        "SELECT pg_try_advisory_xact_lock_shared(hashtextextended('operational-signing-policy:deployment',0))",
        &[],
    ).await?.try_get(0)?;
    anyhow::ensure!(
        held,
        "Signing policy is busy; retry scheduled configuration check"
    );
    let Some(latch) = local_scope::action_latch(tx, tenant, event).await? else {
        return Ok(None);
    };
    local_scope::event_action_mode(tx, tenant, event).await?;
    Ok(Some(latch.post))
}

/// An already executed configuration is not automatically authorizing the
/// schedule of a newly latched event. Check its exact original HSM quorum.
pub(crate) async fn verified_direct_scheduled_configuration(
    tx: &Transaction<'_>,
    tenant: Uuid,
    event: Uuid,
    id: Uuid,
) -> anyhow::Result<bool> {
    #[cfg(feature = "comelec-demo-direct-hsm")]
    {
        use crate::postgres::signing::{get_signing_request, list_signing_approvals};
        use sequent_core::signing::{SigningAction, SigningRequestStatus};
        let Some(request) = get_signing_request(tx, tenant, event, id).await? else {
            return Ok(false);
        };
        if request.action != SigningAction::ApproveConfiguration
            || request.status != SigningRequestStatus::Executed
            || request.executed_at.is_none()
        {
            return Ok(false);
        }
        let approvals = list_signing_approvals(tx, tenant, event, id).await?;
        if approvals.is_empty()
            || approvals
                .iter()
                .any(|row| row.action_direct_proof.is_none())
        {
            // Historical certificate or V1 rows never become V2 evidence.
            return Ok(false);
        }
        super::operational_quorum::has_operational_quorum(
            tx,
            &request,
            &approvals,
            i64::from(request.required),
        )
        .await
    }
    #[cfg(not(feature = "comelec-demo-direct-hsm"))]
    {
        let _ = (tx, tenant, event, id);
        Ok(false)
    }
}
