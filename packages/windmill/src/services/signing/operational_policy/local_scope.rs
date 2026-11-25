// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The durable per-Post latch can only make policy stricter. It is not a
//! credential: the exact immutable local installation and its separately
//! checked database approval are both required, in the caller's transaction.

use super::{
    direct_hsm_approval_build, lock_signing_policy, signing_mode,
    LegacySigningBlocked as Blocked, OperationalSigningMode as Mode,
};
use crate::postgres::signing::SigningRequestRow;
use crate::postgres::signing_actions::published_publication_of_post;
use deadpool_postgres::{GenericClient, Transaction};
use sequent_core::signing::{SigningAction, SigningRequestStatus};
use serde_json::Value;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::services::signing) enum DirectProtocol {
    InitializeV1,
    ActionV2,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::services::signing) enum LatchOrigin {
    InstallationAnchor,
    #[cfg(feature = "comelec-demo-direct-hsm")]
    FrozenRequest,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::services::signing) struct Latch {
    pub protocol: DirectProtocol,
    pub origin: LatchOrigin,
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
            protocol: DirectProtocol::InitializeV1,
            origin: LatchOrigin::InstallationAnchor,
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

/// Stickiness is independent of runtime health. If the optional marker
/// exists but the projection is missing/inaccessible, never restore P12.
pub(in crate::services::signing) async fn action_latch(
    client: &impl GenericClient,
    tenant: Uuid,
    event: Uuid,
) -> Result<Option<Latch>> {
    let present = client
        .query_one(
            "SELECT pg_catalog.to_regclass('felix3_initialize_direct.action_installation') IS NOT NULL,
                    pg_catalog.to_regprocedure('felix3_initialize_direct.action_scope(uuid,uuid)') IS NOT NULL",
            &[],
        )
        .await
        .map_err(|_| Blocked::Unavailable)?;
    let table = present
        .try_get::<_, bool>(0)
        .map_err(|_| Blocked::Unavailable)?;
    let function = present
        .try_get::<_, bool>(1)
        .map_err(|_| Blocked::Unavailable)?;
    if !table && !function {
        return Ok(None);
    }
    if !(table && function) {
        return Err(Blocked::ScopeUnavailable);
    }
    let row = client
        .query_opt(
            "SELECT post_id, publication_id, installation_id
           FROM felix3_initialize_direct.action_scope($1,$2)",
            &[&tenant, &event],
        )
        .await
        .map_err(|_| Blocked::ScopeUnavailable)?;
    row.map(|r| {
        let latch = Latch {
            protocol: DirectProtocol::ActionV2,
            origin: LatchOrigin::InstallationAnchor,
            post: r.try_get(0).map_err(|_| Blocked::ScopeUnavailable)?,
            publication: r.try_get(1).map_err(|_| Blocked::ScopeUnavailable)?,
            installation: r.try_get(2).map_err(|_| Blocked::ScopeUnavailable)?,
        };
        if [latch.post, latch.publication, latch.installation]
            .iter()
            .any(Uuid::is_nil)
        {
            return Err(Blocked::ScopeUnavailable);
        }
        Ok(latch)
    })
    .transpose()
}

#[cfg(feature = "comelec-demo-direct-hsm")]
pub(in crate::services::signing) async fn verify_action_scope(
    tx: &Transaction<'_>,
    tenant: Uuid,
    event: Uuid,
    latch: Latch,
) -> Result<()> {
    crate::services::signing::direct_initialize::verify_action_installation(
        tx, tenant, event, latch,
    )
    .await
    .map_err(|_| Blocked::ScopeUnavailable)
}

#[cfg(not(feature = "comelec-demo-direct-hsm"))]
pub(in crate::services::signing) async fn verify_action_scope(
    _: &Transaction<'_>,
    _: Uuid,
    _: Uuid,
    _: Latch,
) -> Result<()> {
    Err(Blocked::ScopeUnavailable)
}

/// Early event rule read must see the all-action scope, before a route
/// decides to skip its guard and execute the old unsigned path.
pub(in crate::services::signing) async fn event_action_mode(
    tx: &Transaction<'_>,
    tenant: Uuid,
    event: Uuid,
) -> Result<Mode> {
    if direct_hsm_approval_build() {
        lock_signing_policy(tx).await?;
        return Ok(Mode::HsmRequired);
    }
    let baseline = signing_mode(tx, tenant).await?;
    if let Some(latch) = action_latch(tx, tenant, event).await? {
        if baseline != Mode::LegacyP12 {
            return Err(Blocked::ScopeUnavailable);
        }
        verify_action_scope(tx, tenant, event, latch).await?;
        return Ok(Mode::HsmRequired);
    }
    Ok(baseline)
}

#[derive(Clone, Copy)]
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

/// The original publication retains its actual V1 flow. A replacement
/// publication approved through the extended flow can still be initialized,
/// but its new requests use V2 and never inherit the original V1 signature.
async fn extended_initialize(
    tx: &Transaction<'_>,
    tenant: Uuid,
    event: Uuid,
    mut latch: Latch,
    target: Target<'_>,
) -> Result<Latch> {
    let publication = published_publication_of_post(tx, tenant, event, latch.post)
        .await
        .map_err(|_| Blocked::ScopeUnavailable)?
        .ok_or(Blocked::ScopeUnavailable)?;
    let exact = match target {
        Target::Tally(posts, country) => latch.applies(Target::Tally(posts, country))?,
        Target::Intent(post, country, subject) => {
            post == Some(latch.post)
                && country.is_none()
                && subject == &serde_json::json!({"publication_id": publication.to_string()})
        }
    };
    if !exact {
        return Err(Blocked::ScopeUnavailable);
    }
    if publication == latch.publication {
        latch.protocol = DirectProtocol::InitializeV1;
        verify_device(tx, tenant, event, latch).await?;
    } else {
        // The currently published snapshot is not its own authority. Only
        // its exact executed V2 configuration and original proof count.
        #[cfg(feature = "comelec-demo-direct-hsm")]
        if !crate::services::signing::direct_actions::verified_configuration_publication(
            tx,
            tenant,
            event,
            latch,
            publication,
        )
        .await
        .map_err(|_| Blocked::ScopeUnavailable)?
        {
            return Err(Blocked::ScopeUnavailable);
        }
        #[cfg(not(feature = "comelec-demo-direct-hsm"))]
        return Err(Blocked::ScopeUnavailable);
    }
    Ok(latch)
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
    let action_latch = action_latch(tx, tenant, event).await?;
    if let Some(latch) = action_latch {
        if baseline != Mode::LegacyP12 {
            return Err(Blocked::ScopeUnavailable);
        }
        verify_action_scope(tx, tenant, event, latch).await?;
        if action == SigningAction::InitializeVoting {
            let exact = extended_initialize(tx, tenant, event, latch, target).await?;
            return Ok((Mode::HsmRequired, Some(exact)));
        }
        if !matches!(target, Target::Intent(post, _, _) if post.is_none_or(|p| p == latch.post)) {
            return Err(Blocked::ScopeUnavailable);
        }
        return Ok((Mode::HsmRequired, Some(latch)));
    }
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
    if direct_hsm_approval_build() {
        #[cfg(feature = "comelec-demo-direct-hsm")]
        if baseline == Mode::LegacyP12 {
            if let Some(latch) = crate::services::signing::direct_multi_event::for_target(
                tx, tenant, event, action, target,
            )
            .await
            .map_err(|_| Blocked::ScopeUnavailable)?
            {
                return Ok((Mode::HsmRequired, Some(latch)));
            }
        }
        // No matching identity is a setup/scope failure, not permission to
        // use a certificate, a different HSM provider, or no approval.
        return Err(Blocked::ScopeUnavailable);
    }
    Ok((baseline, None))
}

// Older V1-only installations have no V2 column. The exact proof keys
// are selected by the verified latch protocol, never by a browser input.
const PENDING_DIRECT_PROOFS_MATCH: &str = "SELECT NOT EXISTS (
    SELECT FROM sequent_backend.signing_approval a
     WHERE (a.tenant_id,a.election_event_id,a.request_id)=($1,$2,$3)
       AND (jsonb_typeof(to_jsonb(a)->$4::text) IS DISTINCT FROM 'object'
            OR to_jsonb(a)->>$5::text IS NOT NULL OR a.certificate_pem IS NOT NULL)
)";

async fn pending_direct_proofs_match(
    tx: &Transaction<'_>,
    row: &SigningRequestRow,
    protocol: DirectProtocol,
) -> Result<bool> {
    // Preserve historical evidence. Never combine an unfinished legacy or
    // different-protocol approval with a new HSM approval on the same request.
    if !matches!(row.status, SigningRequestStatus::Waiting | SigningRequestStatus::Completed) {
        return Ok(true);
    }
    let (required, different) = match protocol {
        DirectProtocol::InitializeV1 => ("initialize_direct_proof", "action_direct_proof"),
        DirectProtocol::ActionV2 => ("action_direct_proof", "initialize_direct_proof"),
    };
    tx.query_one(
        PENDING_DIRECT_PROOFS_MATCH,
        &[&row.tenant_id, &row.election_event_id, &row.id, &required, &different],
    )
    .await
    .map_err(|_| Blocked::ScopeUnavailable)?
    .try_get(0)
    .map_err(|_| Blocked::ScopeUnavailable)
}

/// Read using the database row, not URL/body hints. Every decision about
/// accepting a signature or releasing an effect rechecks its own transaction.
pub(in crate::services::signing) async fn request_mode_and_latch(
    tx: &Transaction<'_>,
    row: &SigningRequestRow,
) -> Result<(Mode, Option<Latch>)> {
    #[cfg(feature = "comelec-demo-direct-hsm")]
    let dynamic = if signing_mode(tx, row.tenant_id).await? == Mode::LegacyP12 {
        crate::services::signing::direct_multi_event::persisted(tx, row)
            .await
            .map_err(|_| Blocked::ScopeUnavailable)?
    } else {
        None
    };
    #[cfg(not(feature = "comelec-demo-direct-hsm"))]
    let dynamic: Option<Latch> = None;
    let (mode, latch) = if let Some(latch) = dynamic {
        (Mode::HsmRequired, Some(latch))
    } else {
        for_target(
        tx,
        row.tenant_id,
        row.election_event_id,
        row.action,
        Target::Intent(row.election_id, row.area_id, &row.subject),
        )
        .await?
    };
    if let Some(latch) = latch {
        if !crate::services::signing::operational_device::matches_canonical_request(row)
            || !pending_direct_proofs_match(tx, row, latch.protocol).await?
        {
            return Err(Blocked::ScopeUnavailable);
        }
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
    if direct_hsm_approval_build() {
        // An older persisted request never opts the HSM-only build into P12.
        return Err(Blocked::HsmRequired);
    }
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
            protocol: DirectProtocol::InitializeV1,
            origin: LatchOrigin::InstallationAnchor,
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
