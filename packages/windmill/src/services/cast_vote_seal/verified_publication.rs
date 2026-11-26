// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Opt-in receiver cast-key publication; no public event selection.
//!
//! A verified new event records an immutable authorization in its import
//! transaction. The site publisher stages just the public point. Only this
//! existing voting runtime can pin the typed electoral-log statement and
//! commit publication after the normal voting transition checks.

use super::{cast_key, pin_key, sealing, SealKey, Sealing};
use crate::services::browser_package_policy::BrowserPolicy;
use anyhow::{ensure, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::{
    election_config::package_verify::VerifiedPackage, services::uuid_validation::parse_uuid_v4,
    types::hasura::core::ElectionEvent,
};

const SITE_TENANT: &str = "WINDMILL_VERIFIED_CAST_VOTE_TENANT";
const DEFAULT_TENANT: &str = "SUPER_ADMIN_TENANT_ID";

fn matches_receiver(selected: Option<&str>, default: Option<&str>, tenant: &str) -> Result<bool> {
    let Some(selected) = selected else {
        return Ok(false);
    };
    ensure!(
        default == Some(selected) && parse_uuid_v4(selected)?.to_string() == selected,
        "{SITE_TENANT} must match the canonical Default tenant"
    );
    Ok(tenant == selected)
}

/// The environment only asserts which receiver to check. The database's
/// independently administered site registration is still mandatory.
pub(crate) fn selected(tenant: &str) -> Result<bool> {
    match std::env::var(SITE_TENANT) {
        Err(std::env::VarError::NotPresent) => Ok(false),
        Err(error) => Err(error).context("Invalid verified cast vote receiver setting"),
        Ok(value) => matches_receiver(
            Some(&value),
            std::env::var(DEFAULT_TENANT).ok().as_deref(),
            tenant,
        ),
    }
}

/// Only the importer calls this after recording a verified, new revision.
pub(crate) async fn authorize_import(
    tx: &Transaction<'_>,
    tenant: &str,
    event: &str,
    package: &VerifiedPackage,
) -> Result<()> {
    let BrowserPolicy::ManagedOnly(_, binding) = BrowserPolicy::load_with_db(tx, tenant).await
    else {
        anyhow::bail!(
            "Verified cast vote publication requires active portal-managed receiver trust"
        );
    };
    let revision = i64::try_from(package.manifest.configuration.revision)?;
    let accepted: bool = tx
        .query_one(
            "SELECT sequent_backend.verified_cast_key_authorize($1,$2,$3,$4,$5,$6)",
            &[
                &parse_uuid_v4(tenant)?,
                &parse_uuid_v4(event)?,
                &revision,
                &package.manifest_sha256,
                &binding.generation,
                &binding.sha256,
            ],
        )
        .await?
        .try_get(0)?;
    ensure!(
        accepted,
        "Verified cast vote key authorization was refused; check the receiver registration and new event status"
    );
    Ok(())
}

#[derive(Debug, thiserror::Error)]
#[error("This election event is waiting for its cast vote key. Retry once the receiver's key publisher is ready.")]
pub(crate) struct CastVoteKeyPending;

/// Finish publication before changing an authorized new event's status,
/// after the normal transition checks. An early close cannot strand the
/// job by violating the same no-previous-voting rule as the manual publisher.
/// The sidecar never receives a board key or may mark the job complete.
pub(crate) async fn complete_for_status_change(
    tx: &Transaction<'_>,
    event: &ElectionEvent,
    opening: bool,
) -> Result<()> {
    let tenant = &event.tenant_id;
    if !selected(tenant)? {
        return Ok(());
    }
    let tenant_id = parse_uuid_v4(tenant)?;
    let event_id = parse_uuid_v4(&event.id)?;
    let Some(job) = tx
        .query_opt(
            "SELECT public_key_sec1, completed
             FROM sequent_backend.verified_cast_key_for_open($1,$2)",
            &[&tenant_id, &event_id],
        )
        .await?
    else {
        // Including all events created before intending this opt-in.
        return Ok(());
    };
    let completed: bool = job.try_get(1)?;
    if completed && !opening {
        // Do not make closing a previously published event depend on the
        // opening path's pin read or on an available signing service.
        return Ok(());
    }
    let point: Option<Vec<u8>> = job.try_get(0)?;
    let point = point.ok_or(CastVoteKeyPending)?;
    let key = SealKey::new(
        electoral_log::cast_vote_seal::key_id(tenant, &event.id)?,
        1,
        point,
    );
    key.validate_public_point()
        .context("The receiver published an invalid cast vote key")?;
    if !completed {
        // A manually published or conflicting key can never be replaced.
        match sealing(tx, tenant, &event.id).await? {
            Sealing::Required(None) => {}
            Sealing::Required(Some(existing)) if existing == key => {}
            _ => anyhow::bail!("The event's cast vote key authority does not match the staged key"),
        }
        pin_key(tx, tenant, &event.id, &key).await?;
        let published: bool = tx
            .query_one(
                "SELECT sequent_backend.verified_cast_key_complete($1,$2,$3)",
                &[&tenant_id, &event_id, &key.fingerprint],
            )
            .await?
            .try_get(0)?;
        ensure!(
            published,
            "The verified cast vote key could not be published"
        );
    }
    // The opening gate still expects a published key and its matching
    // signed board pin, not merely a staged point or a successful HSM call.
    ensure!(
        cast_key(tx, tenant, event).await?.as_ref() == Some(&key),
        "The event has no matching published and pinned cast vote key"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const TENANT: &str = "431fe949-b833-4287-baa7-d3923ce70a29";
    const OTHER: &str = "843d58d2-2537-482b-9149-56a0d1014fdf";

    #[test]
    fn an_unselected_or_unrelated_tenant_does_not_enable_publication() {
        assert!(!matches_receiver(None, None, TENANT).unwrap());
        assert!(!matches_receiver(Some(TENANT), Some(TENANT), OTHER).unwrap());
        assert!(matches_receiver(Some(TENANT), Some(TENANT), TENANT).unwrap());
        assert!(matches_receiver(Some(TENANT), Some(OTHER), TENANT).is_err());
        let upper = TENANT.to_uppercase();
        assert!(matches_receiver(Some(&upper), Some(&upper), TENANT).is_err());
    }

    #[test]
    fn a_shape_compatible_non_curve_point_is_not_pinned() {
        use openssl::{
            bn::BigNumContext,
            ec::{EcGroup, EcKey},
            nid::Nid,
        };

        let mut invalid = [0; 65];
        invalid[0] = 4;
        assert!(SealKey::new("test".into(), 1, invalid.to_vec())
            .validate_public_point()
            .is_err());
        let group = EcGroup::from_curve_name(Nid::X9_62_PRIME256V1).unwrap();
        let key = EcKey::generate(&group).unwrap();
        let point = key
            .public_key()
            .to_bytes(
                &group,
                openssl::ec::PointConversionForm::UNCOMPRESSED,
                &mut BigNumContext::new().unwrap(),
            )
            .unwrap();
        assert!(SealKey::new("test".into(), 1, point)
            .validate_public_point()
            .is_ok());
    }
}
