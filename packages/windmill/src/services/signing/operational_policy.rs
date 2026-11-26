// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Authoritative deployment policy shared by Harvest and Windmill.
//! HSM-required denies legacy ingress. The isolated replacement path admits
//! only the mounted, operator-selected Initialize installation.

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
