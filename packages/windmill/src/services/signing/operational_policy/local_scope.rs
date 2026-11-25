// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The durable per-Post latch can only make policy stricter. It is not a
//! credential: the exact immutable local installation and its separately
//! checked database approval are both required, in the caller's transaction.

use super::{signing_mode, LegacySigningBlocked as Blocked, OperationalSigningMode as Mode};
use crate::postgres::signing::SigningRequestRow;
use deadpool_postgres::{GenericClient, Transaction};
use sequent_core::signing::SigningAction;
use serde_json::Value;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::services::signing) struct Latch {
    pub post: Uuid,
    pub publication: Uuid,
    pub installation: Uuid,
}

type Result<T> = std::result::Result<T, Blocked>;

// Before optional migration 2280, a validated, exact one-column FK proved
// every local marker already belonged to a tenant-wide HSM policy. After
// 2280 neither a missing helper nor permission/row errors may restore P12.
// Check object/column identities, not constraint names or SQL text.
const FLOOR: &str = "SELECT
    pg_catalog.to_regprocedure(
        'sequent_backend.operational_signing_local_initialize_scope(uuid,uuid)') IS NOT NULL,
    NOT EXISTS (
        SELECT 1 FROM pg_catalog.pg_class r
        JOIN pg_catalog.pg_namespace n ON n.oid = r.relnamespace
        WHERE n.nspname = 'felix3_initialize_direct' AND r.relname = 'installation'
    ) AND (
    pg_catalog.to_regclass('sequent_backend.operational_initialize_local_profile') IS NULL OR EXISTS (
        SELECT 1 FROM pg_catalog.pg_constraint c
        JOIN pg_catalog.pg_attribute src ON src.attrelid = c.conrelid
           AND src.attname = 'tenant_id' AND NOT src.attisdropped AND src.attnotnull
        JOIN pg_catalog.pg_attribute dst ON dst.attrelid = c.confrelid
           AND dst.attname = 'tenant_id' AND NOT dst.attisdropped AND dst.attnotnull
        WHERE c.conrelid = pg_catalog.to_regclass('sequent_backend.operational_initialize_local_profile')
          AND c.confrelid = pg_catalog.to_regclass('sequent_backend.operational_signing_required_tenant')
          AND c.contype = 'f' AND c.convalidated
          AND c.conkey = ARRAY[src.attnum] AND c.confkey = ARRAY[dst.attnum]))";

pub(in crate::services::signing) async fn read_latch(
    client: &impl GenericClient,
    tenant: Uuid,
    event: Uuid,
) -> Result<Option<Latch>> {
    let floor = client
        .query_one(FLOOR, &[])
        .await
        .map_err(|_| Blocked::Unavailable)?;
    if !floor
        .try_get::<_, bool>(0)
        .map_err(|_| Blocked::Unavailable)?
    {
        return if floor
            .try_get::<_, bool>(1)
            .map_err(|_| Blocked::Unavailable)?
        {
            Ok(None)
        } else {
            Err(Blocked::ScopeUnavailable)
        };
    }
    let row = client
        .query_opt(
            "SELECT post_id, publication_id, installation_id
         FROM sequent_backend.operational_signing_local_initialize_scope($1::uuid,$2::uuid)",
            &[&tenant, &event],
        )
        .await
        .map_err(|_| Blocked::Unavailable)?;
    row.map(|r| {
        let s = Latch {
            post: r.try_get(0).map_err(|_| Blocked::ScopeUnavailable)?,
            publication: r.try_get(1).map_err(|_| Blocked::ScopeUnavailable)?,
            installation: r.try_get(2).map_err(|_| Blocked::ScopeUnavailable)?,
        };
        if [s.post, s.publication, s.installation]
            .iter()
            .any(Uuid::is_nil)
        {
            return Err(Blocked::ScopeUnavailable);
        }
        Ok(s)
    })
    .transpose()
}

pub(in crate::services::signing) enum Target<'a> {
    // A direct tally selects one or several Posts before its subject exists.
    Tally(&'a [String], bool),
    // Guard requests supply the server-built subject; all later uses supply
    // the persisted row. Never accept a client-supplied request scope.
    Intent(Option<Uuid>, Option<Uuid>, &'a Value),
}

impl Latch {
    fn applies(self, target: Target<'_>) -> Result<bool> {
        match target {
            Target::Tally(posts, has_country_selection) => {
                if posts.is_empty() {
                    return Err(Blocked::WholePostRequired);
                }
                let mut covered = false;
                for post in posts {
                    let id = Uuid::parse_str(post).map_err(|_| Blocked::ScopeUnavailable)?;
                    if id.is_nil() {
                        return Err(Blocked::ScopeUnavailable);
                    }
                    covered |= id == self.post;
                }
                if covered && (posts.len() != 1 || has_country_selection) {
                    return Err(Blocked::WholePostRequired);
                }
                Ok(covered)
            }
            Target::Intent(post, country, subject) => {
                let post = post
                    .filter(|p| !p.is_nil())
                    .ok_or(Blocked::ScopeUnavailable)?;
                if post != self.post {
                    return Ok(false);
                }
                // A changed publication on the named Post is *blocked*, not
                // eligible for a different provider or legacy signatures.
                if country.is_some()
                    || subject
                        != &serde_json::json!({"publication_id": self.publication.to_string()})
                {
                    return Err(Blocked::ScopeUnavailable);
                }
                Ok(true)
            }
        }
    }
}

#[cfg(feature = "comelec-demo-direct-hsm")]
async fn verify_device(
    tx: &Transaction<'_>,
    tenant: Uuid,
    event: Uuid,
    latch: Latch,
) -> Result<()> {
    crate::services::signing::direct_initialize::verify_installation(tx, tenant, event, latch)
        .await
        .map_err(|_| Blocked::ScopeUnavailable)
}

#[cfg(not(feature = "comelec-demo-direct-hsm"))]
async fn verify_device(_: &Transaction<'_>, _: Uuid, _: Uuid, _: Latch) -> Result<()> {
    Err(Blocked::ScopeUnavailable)
}

pub(in crate::services::signing) async fn for_target(
    tx: &Transaction<'_>,
    tenant: Uuid,
    event: Uuid,
    action: SigningAction,
    target: Target<'_>,
) -> Result<(Mode, Option<Latch>)> {
    // First lock, shared through commit on legacy AND managed paths. The
    // installation/DDL takes the same exclusive lock before its first change.
    let baseline = signing_mode(tx, tenant).await?;
    if action == SigningAction::InitializeVoting {
        if let Some(latch) = read_latch(tx, tenant, event).await? {
            if latch.applies(target)? {
                if baseline != Mode::LegacyP12 {
                    return Err(Blocked::ScopeUnavailable);
                }
                verify_device(tx, tenant, event, latch).await?;
                return Ok((Mode::HsmRequired, Some(latch)));
            }
        }
    }
    Ok((baseline, None))
}

/// Read using the database row, not URL/body hints. Every decision about
/// accepting a signature or releasing an effect rechecks its own transaction.
pub(in crate::services::signing) async fn request_mode_and_latch(
    tx: &Transaction<'_>,
    row: &SigningRequestRow,
) -> Result<(Mode, Option<Latch>)> {
    let (mode, latch) = for_target(
        tx,
        row.tenant_id,
        row.election_event_id,
        row.action,
        Target::Intent(row.election_id, row.area_id, &row.subject),
    )
    .await?;
    if latch.is_some()
        && !crate::services::signing::operational_device::matches_canonical_request(row)
    {
        return Err(Blocked::ScopeUnavailable);
    }
    Ok((mode, latch))
}

#[cfg(feature = "comelec-demo-direct-hsm")]
pub(in crate::services::signing) async fn request_latch(
    tx: &Transaction<'_>,
    row: &SigningRequestRow,
) -> Result<Option<Latch>> {
    Ok(request_mode_and_latch(tx, row).await?.1)
}

pub async fn request_signing_mode(tx: &Transaction<'_>, row: &SigningRequestRow) -> Result<Mode> {
    Ok(request_mode_and_latch(tx, row).await?.0)
}

pub async fn require_legacy_request(tx: &Transaction<'_>, row: &SigningRequestRow) -> Result<()> {
    match request_signing_mode(tx, row).await? {
        Mode::LegacyP12 => Ok(()),
        Mode::HsmRequired => Err(Blocked::HsmRequired),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn only_one_exact_post_and_publication_match_and_ambiguous_scopes_never_become_legacy() {
        let latch = Latch {
            post: Uuid::from_u128(1),
            publication: Uuid::from_u128(2),
            installation: Uuid::from_u128(3),
        };
        let other = Uuid::from_u128(4);
        let good = json!({"publication_id": latch.publication});
        let bad = json!({"publication_id": other});
        assert_eq!(
            latch.applies(Target::Intent(Some(latch.post), None, &good)),
            Ok(true)
        );
        assert_eq!(
            latch.applies(Target::Intent(Some(other), None, &bad)),
            Ok(false)
        );
        assert_eq!(
            latch.applies(Target::Intent(Some(latch.post), Some(other), &good)),
            Err(Blocked::ScopeUnavailable)
        );
        assert_eq!(
            latch.applies(Target::Tally(&[latch.post.to_string()], true)),
            Err(Blocked::WholePostRequired)
        );
        assert_eq!(
            latch.applies(Target::Tally(&[other.to_string()], true)),
            Ok(false)
        );
        for (post, subject) in [
            (Some(latch.post), &bad),
            (None, &good),
            (Some(Uuid::nil()), &good),
        ] {
            assert_eq!(
                latch.applies(Target::Intent(post, None, subject)),
                Err(Blocked::ScopeUnavailable)
            );
        }
        assert_eq!(
            latch.applies(Target::Tally(&[latch.post.to_string()], false)),
            Ok(true)
        );
        assert_eq!(
            latch.applies(Target::Tally(&[other.to_string()], false)),
            Ok(false)
        );
        assert_eq!(
            latch.applies(Target::Tally(
                &[other.to_string(), Uuid::from_u128(5).to_string()],
                false
            )),
            Ok(false)
        );
        assert_eq!(
            latch.applies(Target::Tally(&["bad".to_owned()], false)),
            Err(Blocked::ScopeUnavailable)
        );
        for posts in [
            vec![],
            vec![other.to_string(), latch.post.to_string()],
            vec![latch.post.to_string(), latch.post.to_string()],
        ] {
            assert_eq!(
                latch.applies(Target::Tally(&posts, false)),
                Err(Blocked::WholePostRequired)
            );
        }
    }
}
