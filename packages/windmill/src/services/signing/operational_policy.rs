// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Authoritative deployment policy shared by Harvest and Windmill.
//! HSM-required denies legacy ingress. The isolated replacement path admits
//! only the mounted, operator-selected Initialize installation.

use deadpool_postgres::{GenericClient, Transaction};
pub use sequent_core::signing::OperationalSigningMode;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LegacySigningBlocked {
    HsmRequired,
    Unavailable,
}

impl std::fmt::Display for LegacySigningBlocked {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::HsmRequired => "This tenant requires a server-managed signing device.",
            Self::Unavailable => "The official signing mode is unavailable.",
        })
    }
}

impl std::error::Error for LegacySigningBlocked {}

fn parse_mode(mode: &str) -> Result<OperationalSigningMode, LegacySigningBlocked> {
    mode.parse().map_err(|_| LegacySigningBlocked::Unavailable)
}

// Only transaction callers can retain the policy read lock through their
// consequential operation. Promotion takes the matching exclusive lock.
// Read under the usual READ COMMITTED isolation used by these services.
pub async fn signing_mode(
    tx: &Transaction<'_>,
    tenant: Uuid,
) -> Result<OperationalSigningMode, LegacySigningBlocked> {
    tx.query_one(
        "SELECT pg_advisory_xact_lock_shared(hashtextextended('operational-signing-policy:deployment', 0))",
        &[],
    )
    .await
    .map_err(|_| LegacySigningBlocked::Unavailable)?;
    let row = tx
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
    let mode = parse_mode(mode)?;
    Ok(mode.with_hsm_requirement(tenant_required))
}

fn resolve_event_mode(
    deployment_mode: &str,
    tenant_required: bool,
    explicit_mode: Option<&str>,
    product_active: bool,
) -> Result<OperationalSigningMode, LegacySigningBlocked> {
    // Validate both values even when the deployment floor requires HSM.
    // A malformed explicit value is not an omitted selection.
    let deployment = parse_mode(deployment_mode)?;
    let explicit = explicit_mode.map(parse_mode).transpose()?;
    let floor = deployment.with_hsm_requirement(tenant_required);
    // Explicitly staged installations preserve ordinary old behavior until
    // the migration preconditions are satisfied; active product defaults HSM.
    let selected = if product_active {
        explicit
    } else {
        explicit.or(Some(OperationalSigningMode::LegacyP12))
    };
    Ok(floor.for_event(selected))
}

/// Resolves an installed policy for precisely one existing tenant/event.
/// Like `signing_mode`, retain the shared policy lock in this transaction
/// through the protected operation. No event/row lock or device check is
/// performed here; callers still apply their action rules and device checks.
/// This function does not mount any ordinary approval ingress.
pub async fn signing_mode_for_event(
    tx: &Transaction<'_>,
    tenant: Uuid,
    election_event: Uuid,
) -> Result<OperationalSigningMode, LegacySigningBlocked> {
    // The facts query must take a fresh snapshot after waiting for a
    // writer. Even the first statement of a REPEATABLE READ transaction
    // could have taken its snapshot before obtaining this lock.
    tx.query_opt(
        "SELECT pg_advisory_xact_lock_shared(hashtextextended('operational-signing-policy:deployment', 0))
         WHERE current_setting('transaction_isolation') = 'read committed'",
        &[],
    )
    .await
    .map_err(|_| LegacySigningBlocked::Unavailable)?
    .ok_or(LegacySigningBlocked::Unavailable)?;
    let row = tx
        .query_opt(
            "SELECT deployment_mode, tenant_required, explicit_mode, product_active
             FROM sequent_backend.operational_signing_event_mode_facts($1, $2)",
            &[&tenant, &election_event],
        )
        .await
        .map_err(|_| LegacySigningBlocked::Unavailable)?
        .ok_or(LegacySigningBlocked::Unavailable)?;
    let deployment_mode = row
        .try_get::<_, &str>("deployment_mode")
        .map_err(|_| LegacySigningBlocked::Unavailable)?;
    let tenant_required = row
        .try_get::<_, bool>("tenant_required")
        .map_err(|_| LegacySigningBlocked::Unavailable)?;
    let explicit_mode = row
        .try_get::<_, Option<&str>>("explicit_mode")
        .map_err(|_| LegacySigningBlocked::Unavailable)?;
    let product_active = row
        .try_get::<_, bool>("product_active")
        .map_err(|_| LegacySigningBlocked::Unavailable)?;
    resolve_event_mode(
        deployment_mode,
        tenant_required,
        explicit_mode,
        product_active,
    )
}

/// Policy of the protected-action paths. The sealed installation keeps
/// its existing stricter admission: trust its exact manifest and DB
/// marker, never a dev flag or a fallback legacy mode.
pub async fn approval_mode_for_event(
    tx: &Transaction<'_>,
    tenant: Uuid,
    election_event: Uuid,
) -> Result<OperationalSigningMode, LegacySigningBlocked> {
    #[cfg(feature = "operational-signing-demo-fixture")]
    if super::operational_device::installed_profile(
        tx,
        tenant,
        election_event,
        sequent_core::signing::SigningAction::InitializeVoting,
    )
    .await
    .map_err(|_| LegacySigningBlocked::Unavailable)?
    .is_some()
    {
        return match signing_mode(tx, tenant).await? {
            OperationalSigningMode::HsmRequired => Ok(OperationalSigningMode::HsmRequired),
            OperationalSigningMode::LegacyP12 => Err(LegacySigningBlocked::Unavailable),
        };
    }
    signing_mode_for_event(tx, tenant, election_event).await
}

pub async fn require_legacy_signing_for_event(
    tx: &Transaction<'_>,
    tenant: Uuid,
    election_event: Uuid,
) -> Result<(), LegacySigningBlocked> {
    match approval_mode_for_event(tx, tenant, election_event).await? {
        OperationalSigningMode::LegacyP12 => Ok(()),
        OperationalSigningMode::HsmRequired => Err(LegacySigningBlocked::HsmRequired),
    }
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
        Ok(value) => Some(parse_mode(&value)?),
        Err(std::env::VarError::NotPresent) => None,
        Err(std::env::VarError::NotUnicode(_)) => return Err(LegacySigningBlocked::Unavailable),
    };
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
    let actual = parse_mode(mode)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installed_event_modes_require_valid_floor_and_explicit_values() {
        use OperationalSigningMode::{HsmRequired as Hsm, LegacyP12 as Legacy};
        for active in [false, true] {
            assert_eq!(
                resolve_event_mode("legacy-p12", false, None, active),
                Ok(if active { Hsm } else { Legacy })
            );
            assert_eq!(
                resolve_event_mode("legacy-p12", false, Some("legacy-p12"), active),
                Ok(Legacy)
            );
            assert_eq!(
                resolve_event_mode("legacy-p12", false, Some("hsm-required"), active),
                Ok(Hsm)
            );
            for (floor, promoted) in [("legacy-p12", true), ("hsm-required", false)] {
                assert_eq!(resolve_event_mode(floor, promoted, None, active), Ok(Hsm));
                assert_eq!(
                    resolve_event_mode(floor, promoted, Some("legacy-p12"), active),
                    Ok(Hsm)
                );
                assert_eq!(
                    resolve_event_mode(floor, promoted, Some("invalid"), active),
                    Err(LegacySigningBlocked::Unavailable)
                );
            }
            assert_eq!(
                resolve_event_mode("invalid", true, Some("hsm-required"), active),
                Err(LegacySigningBlocked::Unavailable)
            );
            assert_eq!(
                resolve_event_mode("legacy-p12", false, Some(""), active),
                Err(LegacySigningBlocked::Unavailable)
            );
        }
    }
}
