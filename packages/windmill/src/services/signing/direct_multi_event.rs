// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! A separate event does not get another device or its own provider switch.
//! The event's ordinary request transaction freezes its exact V2 context.
//! The original installation and its historical V1 protocol remain unchanged.

use super::operational_policy::local_scope::{DirectProtocol, Latch, LatchOrigin, Target};
use crate::postgres::signing::{get_signing_post, lock_signing_event, SigningRequestRow};
use crate::postgres::signing_actions::published_publication_of_post;
use anyhow::{ensure, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::signing::{SigningAction, SigningRequestStatus};
use serde_json::Value;
use uuid::Uuid;

pub(super) async fn root(tx: &Transaction<'_>, tenant: Uuid, event: Uuid) -> Result<Option<Uuid>> {
    let available: bool = tx.query_one(
        "SELECT pg_catalog.to_regprocedure('felix3_initialize_direct.action_device_root(uuid,uuid)') IS NOT NULL",
        &[],
    ).await?.try_get(0)?;
    if !available {
        return Ok(None);
    }
    let Some(row) = tx
        .query_opt(
            "SELECT installation_id FROM felix3_initialize_direct.action_device_root($1,$2)",
            &[&tenant, &event],
        )
        .await?
    else {
        return Ok(None);
    };
    let installation: Uuid = row.try_get(0)?;
    ensure!(!installation.is_nil(), "Invalid direct HSM device identity");
    super::direct_initialize::verify_multi_event_device(tenant, event, installation)?;
    Ok(Some(installation))
}

async fn unique_post(tx: &Transaction<'_>, tenant: Uuid, event: Uuid) -> Result<Option<Uuid>> {
    let rows = tx
        .query(
            "SELECT id FROM sequent_backend.election
         WHERE (tenant_id,election_event_id)=($1,$2) LIMIT 2",
            &[&tenant, &event],
        )
        .await?;
    if rows.len() != 1 {
        return Ok(None);
    }
    let post: Uuid = rows[0].try_get(0)?;
    ensure!(!post.is_nil(), "Invalid direct HSM Post");
    Ok(Some(post))
}

async fn single_post(tx: &Transaction<'_>, tenant: Uuid, event: Uuid) -> Result<Uuid> {
    unique_post(tx, tenant, event)
        .await?
        .context("Direct approval requires exactly one active Post")
}

fn subject_uuid(subject: &serde_json::Value, field: &str) -> Result<Uuid> {
    let value = subject
        .get(field)
        .and_then(serde_json::Value::as_str)
        .context("Missing exact direct HSM publication")?;
    Uuid::parse_str(value)
        .ok()
        .filter(|id| !id.is_nil() && id.to_string() == value)
        .context("Invalid exact direct HSM publication")
}

async fn candidate(
    tx: &Transaction<'_>,
    tenant: Uuid,
    event: Uuid,
    publication: Uuid,
) -> Result<(Uuid, String)> {
    let row = tx.query_opt(
        "SELECT post_id,files_root FROM felix3_initialize_direct.action_generated_candidate($1,$2,$3)",
        &[&tenant, &event, &publication],
    ).await?.context("Generate a new configuration in Publish before requesting HSM approval")?;
    let post = row.try_get(0)?;
    let files_root: String = row.try_get(1)?;
    crate::domain::publication_files::validate_publication_root(
        &files_root,
        tenant,
        event,
        publication,
    )?;
    Ok((post, files_root))
}

pub(super) async fn configuration_files_root(
    tx: &Transaction<'_>,
    tenant: Uuid,
    event: Uuid,
    publication: Uuid,
) -> Result<Option<String>> {
    if root(tx, tenant, event).await?.is_none() {
        return Ok(None);
    }
    Ok(Some(candidate(tx, tenant, event, publication).await?.1))
}

/// Mark a real generation in the transaction that installs the files; an
/// archive restore does not pass this entry point or mint its provenance.
pub(crate) async fn lock_generation_if_dynamic(
    tx: &Transaction<'_>,
    tenant: Uuid,
    event: Uuid,
) -> Result<bool> {
    if root(tx, tenant, event).await?.is_none() {
        return Ok(false);
    }
    super::operational_policy::lock_signing_policy(tx).await?;
    lock_signing_event(tx, tenant, event).await?;
    ensure!(
        root(tx, tenant, event).await?.is_some(),
        "Direct HSM device changed"
    );
    Ok(true)
}

pub(crate) async fn record_generation(
    tx: &Transaction<'_>,
    tenant: Uuid,
    event: Uuid,
    publication: Uuid,
    files_root: &str,
) -> Result<()> {
    let recorded: bool = tx
        .query_one(
            "SELECT felix3_initialize_direct.record_action_generation($1,$2,$3,$4)",
            &[&tenant, &event, &publication, &files_root],
        )
        .await?
        .try_get(0)?;
    if !recorded {
        // Ordinary generation is still usable; it cannot authorize direct
        // signing when its protected provenance is missing.
        tracing::warn!("Direct HSM generation has no eligible verified package or whole Post");
    }
    Ok(())
}

async fn current_approved_publication(
    tx: &Transaction<'_>,
    tenant: Uuid,
    event: Uuid,
    installation: Uuid,
    post: Uuid,
) -> Result<Option<Latch>> {
    let Some(publication) = published_publication_of_post(tx, tenant, event, post).await? else {
        return Ok(None);
    };
    let latch = Latch {
        protocol: DirectProtocol::ActionV2,
        origin: LatchOrigin::FrozenRequest,
        installation,
        post,
        publication,
    };
    let verified = super::direct_actions::verified_configuration_publication(
        tx,
        tenant,
        event,
        latch,
        publication,
    )
    .await?;
    Ok(verified.then_some(latch))
}

async fn approved_active_publication(
    tx: &Transaction<'_>,
    tenant: Uuid,
    event: Uuid,
    installation: Uuid,
    post: Uuid,
) -> Result<Latch> {
    current_approved_publication(tx, tenant, event, installation, post)
        .await?
        .context("Publish and approve this Post's configuration before requesting HSM signing")
}

/// Missing proof is a readable pending configuration, not a legacy or
/// unsigned scheduler path. Operational execution still needs exact proof.
pub(super) async fn scheduled_scope(
    tx: &Transaction<'_>,
    tenant: Uuid,
    event: Uuid,
) -> Result<Option<(Uuid, bool)>> {
    let Some(installation) = root(tx, tenant, event).await? else {
        return Ok(None);
    };
    let Some(post) = unique_post(tx, tenant, event).await? else {
        return Ok(None);
    };
    let approved = current_approved_publication(tx, tenant, event, installation, post)
        .await?
        .is_some();
    Ok(Some((post, approved)))
}

pub(super) async fn for_target(
    tx: &Transaction<'_>,
    tenant: Uuid,
    event: Uuid,
    action: SigningAction,
    target: Target<'_>,
) -> Result<Option<Latch>> {
    let Some(installation) = root(tx, tenant, event).await? else {
        return Ok(None);
    };
    if action == SigningAction::ApproveConfiguration {
        let Target::Intent(None, None, subject) = target else {
            anyhow::bail!("Configuration approval requires the original whole publication");
        };
        let publication = subject_uuid(subject, "ballot_publication_id")?;
        let (post, files_root) = candidate(tx, tenant, event, publication).await?;
        ensure!(
            subject
                .get("ballot_files_root")
                .and_then(serde_json::Value::as_str)
                == Some(files_root.as_str()),
            "Configuration approval does not cover the exact generated ballot files"
        );
        return Ok(Some(Latch {
            protocol: DirectProtocol::ActionV2,
            origin: LatchOrigin::FrozenRequest,
            installation,
            post,
            publication,
        }));
    }
    let post = match target {
        Target::Tally(posts, false)
            if action == SigningAction::InitializeVoting && posts.len() == 1 =>
        {
            Uuid::parse_str(&posts[0])
                .ok()
                .filter(|id| !id.is_nil() && id.to_string() == posts[0])
                .context("Invalid direct Initialize Post")?
        }
        Target::Intent(Some(post), _, _) if !post.is_nil() => post,
        Target::Intent(None, None, _)
            if matches!(
                action,
                SigningAction::ConfirmCustodyProof | SigningAction::ContributeCustodyProof
            ) =>
        {
            single_post(tx, tenant, event).await?
        }
        _ => anyhow::bail!("This action does not identify the exact approved Post"),
    };
    ensure!(
        get_signing_post(tx, tenant, event, post).await?.is_some(),
        "This Post is not part of the event"
    );
    let latch = approved_active_publication(tx, tenant, event, installation, post).await?;
    if let Target::Intent(request_post, area, subject) = target {
        if action == SigningAction::InitializeVoting {
            ensure!(
                request_post == Some(post)
                    && area.is_none()
                    && subject == &serde_json::json!({"publication_id": latch.publication}),
                "Direct Initialize requires the exact whole approved Post and publication"
            );
        }
    }
    Ok(Some(latch))
}

/// Commit the exact server-derived authority only for a fresh ordinary
/// request. No existing waiting request or certificate evidence is converted.
pub(super) async fn bind(
    tx: &Transaction<'_>,
    row: &SigningRequestRow,
    latch: Option<Latch>,
) -> Result<()> {
    let Some(installation) = root(tx, row.tenant_id, row.election_event_id).await? else {
        return Ok(());
    };
    let latch = latch.context("Missing direct HSM request scope")?;
    let digest = hex::decode(&row.payload_sha256)?;
    ensure!(
        latch.protocol == DirectProtocol::ActionV2
            && latch.origin == LatchOrigin::FrozenRequest
            && latch.installation == installation,
        "Wrong direct HSM request scope"
    );
    let bound: bool = tx
        .query_one(
            "SELECT felix3_initialize_direct.bind_action_request($1,$2,$3,$4,$5,$6,$7,$8)",
            &[
                &installation,
                &row.tenant_id,
                &row.election_event_id,
                &latch.post,
                &latch.publication,
                &row.id,
                &row.action.to_string(),
                &digest,
            ],
        )
        .await?
        .try_get(0)?;
    ensure!(bound, "The exact direct HSM request could not be bound");
    Ok(())
}

/// Compare the committed stock effect to the exact immutable reviewed subject.
fn matches_executed_configuration(
    publication: Uuid,
    subject: &Value,
    execution: Option<&Value>,
) -> bool {
    let Some(execution) = execution else {
        return false;
    };
    let Some(digest) = subject.get("digest").and_then(Value::as_str) else {
        return false;
    };
    digest.len() == 64
        && digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        && subject_uuid(subject, "ballot_publication_id").ok() == Some(publication)
        && subject_uuid(execution, "ballot_publication_id").ok() == Some(publication)
        && execution.get("digest").and_then(Value::as_str) == Some(digest)
}

pub(super) async fn persisted(
    tx: &Transaction<'_>,
    row: &SigningRequestRow,
) -> Result<Option<Latch>> {
    let Some(installation) = root(tx, row.tenant_id, row.election_event_id).await? else {
        return Ok(None);
    };
    let saved = tx.query_opt(
        "SELECT installation_id,post_id,publication_id,action,payload_sha256
           FROM felix3_initialize_direct.action_request_scope($1,$2,$3)",
        &[&row.tenant_id,&row.election_event_id,&row.id],
    ).await?.context("This request predates the direct HSM scope; start a new request after configuration approval")?;
    let post: Uuid = saved.try_get(1)?;
    let publication: Uuid = saved.try_get(2)?;
    ensure!(
        saved.try_get::<_, Uuid>(0)? == installation
            && !post.is_nil()
            && !publication.is_nil()
            && saved.try_get::<_, String>(3)? == row.action.to_string()
            && hex::encode(saved.try_get::<_, Vec<u8>>(4)?) == row.payload_sha256,
        "Wrong frozen direct HSM request scope"
    );
    let latch = Latch {
        protocol: DirectProtocol::ActionV2,
        origin: LatchOrigin::FrozenRequest,
        installation,
        post,
        publication,
    };
    if row.action == SigningAction::ApproveConfiguration {
        let files_root = row
            .subject
            .get("ballot_files_root")
            .and_then(Value::as_str)
            .context("Missing the frozen configuration's ballot files")?;
        if row.status == SigningRequestStatus::Executed {
            // A settled effect is no longer an unpublished candidate. This
            // reconstructs only its frozen envelope; has_quorum still checks
            // each signature with proof_action_key's exact historical
            // generation, consumed reservation, and actor-key checks.
            ensure!(
                row.executed_at.is_some()
                    && matches_executed_configuration(
                        publication,
                        &row.subject,
                        row.execution_result.as_ref()
                    ),
                "The executed configuration result does not match its frozen request"
            );
            crate::domain::publication_files::validate_publication_root(
                files_root,
                row.tenant_id,
                row.election_event_id,
                publication,
            )?;
        } else {
            // All paths that could still sign or commit an effect require
            // the genuine current unpublished candidate, as before.
            let (candidate_post, candidate_root) =
                candidate(tx, row.tenant_id, row.election_event_id, publication).await?;
            ensure!(
                candidate_post == post
                    && subject_uuid(&row.subject, "ballot_publication_id")? == publication
                    && files_root == candidate_root.as_str(),
                "The frozen configuration is no longer the same generation"
            );
        }
    } else {
        ensure!(
            approved_active_publication(
                tx,
                row.tenant_id,
                row.election_event_id,
                installation,
                post
            )
            .await?
                == latch,
            "The approved publication for this request has changed"
        );
    }
    Ok(Some(latch))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn settled_configuration_result_must_match_the_frozen_publication_and_digest() {
        let pub_id = Uuid::from_u128(1);
        let other = Uuid::from_u128(2);
        let digest = "aa".repeat(32);
        let signed = json!({"ballot_publication_id": pub_id, "digest": digest});
        let result = signed.clone();

        assert!(matches_executed_configuration(
            pub_id,
            &signed,
            Some(&result)
        ));
        assert!(!matches_executed_configuration(
            other,
            &signed,
            Some(&result)
        ));
        assert!(!matches_executed_configuration(pub_id, &signed, None));
        for invalid in [
            json!({"ballot_publication_id": other, "digest": digest}),
            json!({"ballot_publication_id": pub_id, "digest": "bb".repeat(32)}),
            json!({"ballot_publication_id": pub_id, "digest": 42}),
            json!({"ballot_publication_id": pub_id}),
        ] {
            assert!(!matches_executed_configuration(
                pub_id,
                &signed,
                Some(&invalid)
            ));
        }
        let upper = json!({"ballot_publication_id": pub_id, "digest": "AA".repeat(32)});
        assert!(!matches_executed_configuration(
            pub_id,
            &upper,
            Some(&upper)
        ));
    }
}
