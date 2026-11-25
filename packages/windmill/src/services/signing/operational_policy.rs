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

/// Fixed by the dedicated application build, not by a tenant/event setting,
/// an imported value, or the availability of an installed signer.
pub const fn direct_hsm_approval_build() -> bool {
    cfg!(feature = "comelec-demo-direct-hsm")
}

/// Keep the ordinary DB policy available for installation admission and
/// existing non-approval output classification. The demo floor is not a claim
/// that a production managed device or official output signer is installed.
pub async fn approval_signing_mode(
    tx: &Transaction<'_>,
    tenant: Uuid,
) -> Result<OperationalSigningMode, LegacySigningBlocked> {
    if direct_hsm_approval_build() {
        lock_signing_policy(tx).await?;
        return Ok(OperationalSigningMode::HsmRequired);
    }
    signing_mode(tx, tenant).await
}

pub async fn require_legacy_signing(
    tx: &Transaction<'_>,
    tenant: Uuid,
) -> Result<(), LegacySigningBlocked> {
    match approval_signing_mode(tx, tenant).await? {
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
        || (direct_hsm_approval_build() && std::env::var_os(PROFILE).is_some())
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

/// Read-only effective approval policy. The genuine installation is required
/// to sign, never to select or opt out of the provider.
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DirectScheduleScope {
    Standard,
    Unavailable,
    ExactPost { post: Uuid, approved: bool },
}

pub(crate) async fn direct_schedule_scope(
    tx: &Transaction<'_>,
    tenant: Uuid,
    event: Uuid,
) -> anyhow::Result<DirectScheduleScope> {
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
        #[cfg(feature = "comelec-demo-direct-hsm")]
        if let Some((post, approved)) =
            super::direct_multi_event::scheduled_scope(tx, tenant, event).await?
        {
            return Ok(DirectScheduleScope::ExactPost { post, approved });
        }
        return Ok(if direct_hsm_approval_build() {
            DirectScheduleScope::Unavailable
        } else {
            DirectScheduleScope::Standard
        });
    };
    // Never let a synthetic installation answer the production hardware
    // policy, even if a mounted descriptor is still present.
    if read_tenant_mode(tx, tenant).await? != OperationalSigningMode::LegacyP12 {
        return Err(LegacySigningBlocked::ScopeUnavailable.into());
    }
    local_scope::verify_action_scope(tx, tenant, event, latch).await?;
    // The original anchored scope is unchanged; individual V2 configuration
    // proofs and the signed occurrence are still checked by the scheduler.
    Ok(DirectScheduleScope::ExactPost {
        post: latch.post,
        approved: true,
    })
}

/// In the dedicated profile, an automated zero report is a downstream
/// effect of an already executed Initialize, not a human custody operation.
/// No legacy request or stored tally ID alone supplies that authority.
pub(crate) async fn verified_automated_zero_report(
    tx: &Transaction<'_>,
    tenant: Uuid,
    event: Uuid,
    post: Uuid,
    session: Uuid,
) -> anyhow::Result<bool> {
    use crate::postgres::signing::{get_signing_request, get_signing_rule, list_signing_approvals};
    use sequent_core::signing::{SigningAction, SigningRequestStatus};

    if !direct_hsm_approval_build() || [tenant, event, post, session].iter().any(Uuid::is_nil) {
        return Ok(false);
    }
    lock_signing_policy(tx).await?;
    let lane_open: bool = tx
        .query_one(
            "SELECT pg_try_advisory_xact_lock_shared(hashtextextended($1,0))",
            &[&format!("signing-event:{tenant}:{event}")],
        )
        .await?
        .try_get(0)?;
    if !lane_open {
        // Revisited by the ordinary worker on the next pass.
        return Ok(false);
    }
    let tally_key =
        get_signing_rule(tx, tenant, event, SigningAction::ContributeCustodyProof).await?;
    check_automated_zero_report_rule(tally_key.as_ref().map(|row| &row.rule))?;

    let rows = tx
        .query(
            "SELECT id FROM sequent_backend.signing_request
             WHERE (tenant_id,election_event_id,election_id)=($1,$2,$3)
               AND action='initialize-voting' AND status='executed'
               AND area_id IS NULL AND trustee_id IS NULL AND executed_at IS NOT NULL
               AND execution_result->>'tally_session_id'=$4
             LIMIT 2",
            &[&tenant, &event, &post, &session.to_string()],
        )
        .await?;
    let [row] = rows.as_slice() else {
        anyhow::ensure!(
            rows.is_empty(),
            "Ambiguous Initialization approval for this report"
        );
        return Ok(false);
    };
    let request_id: Uuid = row.try_get(0)?;
    let Some(request) = get_signing_request(tx, tenant, event, request_id).await? else {
        return Ok(false);
    };
    if request.action != SigningAction::InitializeVoting
        || request.status != SigningRequestStatus::Executed
        || request.election_id != Some(post)
        || request.executed_at.is_none()
    {
        return Ok(false);
    }
    let (mode, latch) = local_scope::request_mode_and_latch(tx, &request).await?;
    if mode != OperationalSigningMode::HsmRequired || latch.is_none_or(|l| l.post != post) {
        return Ok(false);
    }
    let current = super::guard::effective_rule_in_mode(
        tx,
        tenant,
        event,
        SigningAction::InitializeVoting,
        mode,
    )
    .await?;
    let approvals = list_signing_approvals(tx, tenant, event, request_id).await?;
    super::operational_quorum::has_operational_quorum(
        tx,
        &request,
        &approvals,
        i64::from(current.required()),
    )
    .await
}

fn check_automated_zero_report_rule(
    rule: Option<&sequent_core::signing::SigningRule>,
) -> anyhow::Result<()> {
    use sequent_core::signing::SigningAction;
    if let Some(rule) = rule {
        anyhow::ensure!(
            rule.action == SigningAction::ContributeCustodyProof
                && rule.validate().is_ok()
                && !rule.is_required(),
            "Automated initialization cannot satisfy a required or invalid human tally-key rule"
        );
    }
    Ok(())
}

#[cfg(test)]
mod automated_zero_report_rule_tests {
    use super::*;
    use sequent_core::signing::{SigningAction, SigningRequirement, SigningRule};

    #[test]
    fn automated_zero_report_refuses_conflicting_or_invalid_human_custody() {
        let mut rule = SigningRule::default_for(SigningAction::ContributeCustodyProof);
        assert!(check_automated_zero_report_rule(None).is_ok());
        assert!(check_automated_zero_report_rule(Some(&rule)).is_ok());
        rule.requirement = SigningRequirement::Required;
        assert!(check_automated_zero_report_rule(Some(&rule)).is_err());
        rule.requirement = SigningRequirement::NotRequired;
        rule.signatures = 0;
        assert!(check_automated_zero_report_rule(Some(&rule)).is_err());
        assert!(
            check_automated_zero_report_rule(Some(&SigningRule::default_for(
                SigningAction::InitializeVoting
            )))
            .is_err()
        );
    }
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
