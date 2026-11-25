// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Certificate-free actor authorization for the dedicated COMELEC demo.
//! This is a continuation of the existing private DB-free signer. The
//! approval proof is not a public-facing document certificate or CMS.

use super::actions::transmission::SbeiDirectory;
use super::approve::{approve_direct_action, sign_refusal, ApproveOutcome, SigningServices};
use super::direct_initialize::{self, DEVICE_PROFILE};
use super::direct_protocol::{DirectAnchor, DirectOperation, DirectPurpose, DirectRequestScope};
use super::operational_policy::local_scope::{request_latch, DirectProtocol, Latch};
use super::operational_policy::{lock_signing_policy, OperationalSigningMode};
use super::pdf::{RevisionStore, S3RevisionStore};
use super::requests::{is_overdue, read_request, relock};
use super::{SigningCaller, SigningError, SigningResult};
use crate::postgres::signing::{
    get_signing_post, get_signing_request, list_signing_approvals, DirectActionProofMetadata,
    SigningApprovalRow, SigningRequestRow,
};
use crate::services::consolidation::signed_transmission_package::AnnotatedSbeis;
use chrono::{DateTime, Utc};
use deadpool_postgres::{Client, GenericClient, Transaction};
use openssl::sha::sha256;
use sequent_core::signing::{SignatureAlgorithm, SigningAction, SigningRequestStatus, SigningRule};
use std::collections::HashSet;
use uuid::Uuid;

pub(super) fn unavailable() -> SigningError {
    SigningError::Forbidden(
        "Direct demo HSM approval is unavailable for this exact request.".into(),
    )
}

fn actor_uuid(value: &str) -> SigningResult<Uuid> {
    Uuid::parse_str(value)
        .ok()
        .filter(|id| !id.is_nil() && id.to_string() == value)
        .ok_or_else(unavailable)
}

fn valid_frozen_rule(request: &SigningRequestRow) -> bool {
    serde_json::from_value::<SigningRule>(request.rule_snapshot.clone()).is_ok_and(|rule| {
        rule.action == request.action
            && rule.is_required()
            && rule.validate().is_ok()
            && i32::from(rule.required()) == request.required
    })
}

async fn request_scope(
    tx: &Transaction<'_>,
    request: &SigningRequestRow,
) -> SigningResult<DirectRequestScope> {
    if request.action == SigningAction::InitializeVoting {
        let publication = request
            .subject
            .get("publication_id")
            .and_then(serde_json::Value::as_str)
            .and_then(|s| Uuid::parse_str(s).ok().filter(|u| u.to_string() == s))
            .ok_or_else(unavailable)?;
        // request_latch already re-resolved the latest actual publication
        // for this exact Post and matched it to the canonical subject.
        return Ok(DirectRequestScope {
            election: request.election_id,
            area: request.area_id,
            trustee: request.trustee_id,
            publication: Some(publication),
        });
    }
    if request.action != SigningAction::ApproveConfiguration {
        return Ok(DirectRequestScope {
            election: request.election_id,
            area: request.area_id,
            trustee: request.trustee_id,
            publication: None,
        });
    }
    // Configuration is event-scoped in the ordinary request table. Its
    // immutable target key and the actual new publication jointly identify
    // the Post (if any); never reuse the old installation publication here.
    let publication = request
        .subject
        .get("ballot_publication_id")
        .and_then(serde_json::Value::as_str)
        .and_then(|s| Uuid::parse_str(s).ok().filter(|u| u.to_string() == s))
        .filter(|u| !u.is_nil())
        .ok_or_else(unavailable)?;
    let row = tx
        .query_opt(
            "SELECT election_id FROM sequent_backend.ballot_publication
             WHERE (tenant_id,election_event_id,id)=($1,$2,$3)
               AND is_generated AND deleted_at IS NULL",
            &[&request.tenant_id, &request.election_event_id, &publication],
        )
        .await?
        .ok_or_else(unavailable)?;
    let election: Option<Uuid> = row.try_get(0)?;
    let target = election.map(|id| id.to_string());
    if request.election_id.is_some()
        || request.area_id.is_some()
        || request.trustee_id.is_some()
        || request.scope_key != super::actions::configuration::target_scope_key(target.as_deref())
    {
        return Err(unavailable());
    }
    Ok(DirectRequestScope {
        election,
        publication: Some(publication),
        ..Default::default()
    })
}

async fn known_operation(
    tx: &Transaction<'_>,
    request: &SigningRequestRow,
    latch: Latch,
    actor: Uuid,
) -> SigningResult<DirectOperation> {
    if latch.protocol != DirectProtocol::ActionV2 || !valid_frozen_rule(request) {
        return Err(unavailable());
    }
    let request_scope = request_scope(tx, request).await?;
    use sequent_core::signing::DocumentKind;
    let signed_document = match request.action.document() {
        DocumentKind::NoDocument => None,
        DocumentKind::Pdf => request.subject.get("document_sha256"),
        DocumentKind::Eml => request.subject.get("eml_sha256"),
    };
    if (request.action.document() != DocumentKind::NoDocument) != request.document_id.is_some()
        || signed_document.and_then(serde_json::Value::as_str) != request.document_sha256.as_deref()
    {
        return Err(unavailable());
    }
    DirectOperation::new(
        DirectAnchor {
            installation: latch.installation,
            tenant: request.tenant_id,
            event: request.election_event_id,
            post: latch.post,
            publication: latch.publication,
        },
        request.id,
        actor,
        request.action,
        request_scope,
        &request.payload_sha256,
        request.document_sha256.as_deref(),
    )
    .ok_or_else(unavailable)
}

async fn authorize_row(
    tx: &Transaction<'_>,
    caller: &SigningCaller,
    request: &SigningRequestRow,
    reviewed_payload: &str,
) -> SigningResult<DirectOperation> {
    if request.payload_sha256 != reviewed_payload
        || sign_refusal(tx, caller, request).await?.is_some()
    {
        return Err(unavailable());
    }
    let latch = request_latch(tx, request)
        .await
        .map_err(|_| unavailable())?
        .ok_or_else(unavailable)?;
    let operation = known_operation(tx, request, latch, actor_uuid(&caller.user_id)?).await?;
    let mut post = operation.scope().election;
    if post.is_none() && super::direct_multi_event::root(tx, request.tenant_id, request.election_event_id)
        .await?.is_some() {
        // An event-scoped action in this deployment still concerns the one
        // exact eligible Post. Do not bypass its labels via an empty scope.
        post = Some(operation.anchor().post);
    }
    if let Some(post) = post {
        let post = get_signing_post(tx, request.tenant_id, request.election_event_id, post)
            .await?
            .ok_or_else(unavailable)?;
        if !caller.signs_for(post.permission_label.as_deref()) {
            return Err(unavailable());
        }
    }
    if request.action == SigningAction::TransmitResults
        && AnnotatedSbeis
            .post_sbeis(tx, request)
            .await?
            .is_some_and(|members| !members.contains_key(&caller.username))
    {
        // Stock Post/country membership still gates approval. A separately
        // pinned official certificate is not asserted by the demo proof.
        return Err(unavailable());
    }
    super::guard::effective_rule_in_mode(
        tx,
        request.tenant_id,
        request.election_event_id,
        request.action,
        OperationalSigningMode::HsmRequired,
    )
    .await?;
    Ok(operation)
}

pub async fn authorize(
    tx: &Transaction<'_>,
    caller: &SigningCaller,
    tenant: Uuid,
    request: Uuid,
    reviewed_payload: &str,
) -> SigningResult<DirectOperation> {
    lock_signing_policy(tx).await.map_err(|_| unavailable())?;
    let row = read_request(tx, tenant, request).await?;
    let operation = authorize_row(tx, caller, &row, reviewed_payload).await?;
    if (row.status == SigningRequestStatus::Waiting && !is_overdue(tx, &row).await?)
        || already_approved(tx, &operation).await?
    {
        return Ok(operation);
    }
    Err(unavailable())
}

pub(super) async fn lock_authorized(
    tx: &Transaction<'_>,
    caller: &SigningCaller,
    expected: &DirectOperation,
) -> SigningResult<SigningRequestRow> {
    lock_signing_policy(tx).await.map_err(|_| unavailable())?;
    let row = read_request(tx, expected.anchor().tenant, expected.request()).await?;
    let reviewed = hex::encode(expected.payload_digest());
    if authorize_row(tx, caller, &row, &reviewed).await? != *expected {
        return Err(unavailable());
    }
    // Stock relock takes the exclusive event lock before the request row;
    // final approval never upgrades a shared-only lock while consuming.
    let locked = relock(tx, &row).await?;
    if authorize_row(tx, caller, &locked, &reviewed).await? != *expected {
        return Err(unavailable());
    }
    Ok(locked)
}

/// Review exactly the stored, immutable bytes for a document action.
pub async fn verify_review_bytes(
    tx: &Transaction<'_>,
    operation: &DirectOperation,
) -> SigningResult<()> {
    let Some(purpose) = DirectPurpose::for_document(operation.action()) else {
        return Ok(());
    };
    let anchor = operation.anchor();
    let request = read_request(tx, anchor.tenant, operation.request()).await?;
    let document = request.document_id.ok_or_else(unavailable)?;
    if request.election_event_id != anchor.event || request.action != operation.action() {
        return Err(unavailable());
    }
    let bytes = S3RevisionStore
        .load(tx, anchor.tenant, anchor.event, document)
        .await?;
    if Some(sha256(&bytes)) != operation.artifact_digest(purpose) {
        return Err(unavailable());
    }
    Ok(())
}

pub struct DirectReservation {
    pub id: Uuid,
    pub nonce: Uuid,
    pub expires_at: DateTime<Utc>,
}

pub async fn reserve(
    tx: &Transaction<'_>,
    operation: &DirectOperation,
) -> SigningResult<DirectReservation> {
    let a = operation.anchor();
    let row = tx
        .query_opt(
            "SELECT id, nonce, expires_at FROM felix3_initialize_direct.reserve_action($1,$2,$3,$4,$5,$6,$7,$8,$9)",
            &[&a.installation, &a.tenant, &a.event, &a.post, &a.publication,
              &operation.request(), &operation.actor(), &operation.action().to_string(),
              &&operation.payload_digest()[..]],
        )
        .await?
        .ok_or_else(unavailable)?;
    Ok(DirectReservation {
        id: row.try_get("id")?,
        nonce: row.try_get("nonce")?,
        expires_at: row.try_get("expires_at")?,
    })
}

pub fn verify_signature(
    operation: &DirectOperation,
    nonce: Uuid,
    key: &[u8],
    signature: &[u8],
) -> SigningResult<()> {
    let bytes = operation
        .evidence(nonce, DirectPurpose::ApprovalPayload)
        .ok_or_else(unavailable)?;
    direct_initialize::verify_signature_bytes(&bytes, key, signature).map_err(|_| unavailable())
}

async fn pinned_key(
    tx: &Transaction<'_>,
    op: &DirectOperation,
    nonce: Uuid,
) -> SigningResult<Vec<u8>> {
    let a = op.anchor();
    let key: Option<Vec<u8>> = tx
        .query_one(
            "SELECT felix3_initialize_direct.proof_action_key($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
            &[
                &a.installation,
                &a.tenant,
                &a.event,
                &a.post,
                &a.publication,
                &op.request(),
                &op.actor(),
                &nonce,
                &op.action().to_string(),
                &&op.payload_digest()[..],
            ],
        )
        .await?
        .try_get(0)?;
    key.ok_or_else(unavailable)
}

fn metadata(operation: &DirectOperation, nonce: Uuid) -> DirectActionProofMetadata {
    let a = operation.anchor();
    DirectActionProofMetadata {
        installation: a.installation,
        post: a.post,
        publication: a.publication,
        nonce,
        payload_sha256: hex::encode(operation.payload_digest()),
        profile: DEVICE_PROFILE.into(),
        action: operation.action(),
    }
}

/// Opaque proof checked against the database-owned actor key. It cannot be
/// made from JSON. Its reservation is consumed only during final approval.
pub struct VerifiedActionProof {
    pub(super) operation: DirectOperation,
    pub(super) nonce: Uuid,
    signature: Vec<u8>,
    spki_sha256: String,
}

pub async fn verify_and_pin(
    tx: &Transaction<'_>,
    operation: &DirectOperation,
    reservation: DirectReservation,
    key: &[u8],
    signature: &[u8],
) -> SigningResult<VerifiedActionProof> {
    verify_signature(operation, reservation.nonce, key, signature)?;
    lock_signing_policy(tx).await.map_err(|_| unavailable())?;
    let recorded: bool = tx
        .query_one(
            "SELECT felix3_initialize_direct.record_action_key($1,$2)",
            &[&reservation.id, &key],
        )
        .await?
        .try_get(0)?;
    if !recorded {
        return Err(unavailable());
    }
    let pinned = pinned_key(tx, operation, reservation.nonce).await?;
    if pinned != key {
        return Err(unavailable());
    }
    Ok(VerifiedActionProof {
        operation: operation.clone(),
        nonce: reservation.nonce,
        signature: signature.to_vec(),
        spki_sha256: hex::encode(sha256(&pinned)),
    })
}

impl VerifiedActionProof {
    pub(super) async fn consume_and_insert(
        &self,
        tx: &Transaction<'_>,
        caller: &SigningCaller,
    ) -> SigningResult<SigningApprovalRow> {
        let op = &self.operation;
        let pinned = pinned_key(tx, op, self.nonce).await?;
        verify_signature(op, self.nonce, &pinned, &self.signature)?;
        if self.spki_sha256 != hex::encode(sha256(&pinned))
            || op.actor() != actor_uuid(&caller.user_id)?
        {
            return Err(unavailable());
        }
        let a = op.anchor();
        let consumed: bool = tx
            .query_one(
                "SELECT felix3_initialize_direct.consume_action($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
                &[
                    &a.installation,
                    &a.tenant,
                    &a.event,
                    &a.post,
                    &a.publication,
                    &op.request(),
                    &op.actor(),
                    &self.nonce,
                    &op.action().to_string(),
                    &&op.payload_digest()[..],
                ],
            )
            .await?
            .try_get(0)?;
        if !consumed {
            return Err(unavailable());
        }
        let proof = serde_json::to_value(metadata(op, self.nonce)).map_err(anyhow::Error::from)?;
        let row = tx
            .query_opt(
                "INSERT INTO sequent_backend.signing_approval
                 (tenant_id,election_event_id,request_id,user_id,username,display_name,auth_time,
                  spki_sha256,algorithm,payload_signature,action_direct_proof)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,'ecdsa-p256-sha256',$9,$10)
             ON CONFLICT DO NOTHING RETURNING *",
                &[
                    &a.tenant,
                    &a.event,
                    &op.request(),
                    &caller.user_id,
                    &caller.username,
                    &caller.display_name,
                    &caller.auth_time,
                    &self.spki_sha256,
                    &self.signature,
                    &proof,
                ],
            )
            .await?
            .ok_or_else(|| {
                SigningError::Conflict("This actor, key, or reservation already approved.".into())
            })?;
        Ok(SigningApprovalRow::try_from(row)?)
    }
}

async fn check_approvals(
    tx: &Transaction<'_>,
    request: &SigningRequestRow,
    latch: Latch,
    approvals: &[SigningApprovalRow],
) -> SigningResult<bool> {
    // A task may already own the request row. Do not wait for an
    // event/rotation transaction that might itself wait on that row.
    // When acquired, the shared event lock persists through its effect.
    let in_scope: bool = tx
        .query_one(
            "SELECT pg_try_advisory_xact_lock_shared(hashtextextended($1,0))",
            &[&format!(
                "signing-event:{}:{}",
                request.tenant_id, request.election_event_id
            )],
        )
        .await?
        .try_get(0)?;
    if !in_scope {
        return Err(
            anyhow::anyhow!("Direct HSM approval event is busy; retry the transaction").into(),
        );
    }
    let mut actors = HashSet::new();
    let mut keys = HashSet::new();
    for signed in approvals {
        let (Some(proof), Ok(actor)) = (&signed.action_direct_proof, actor_uuid(&signed.user_id))
        else {
            return Ok(false);
        };
        let op = match known_operation(tx, request, latch, actor).await {
            // Unusable historical proof is not a global scheduler error.
            Ok(operation) => operation,
            Err(SigningError::Forbidden(_)) => return Ok(false),
            Err(error) => return Err(error),
        };
        if (
            signed.tenant_id,
            signed.election_event_id,
            signed.request_id,
        ) != (request.tenant_id, request.election_event_id, request.id)
            || signed.algorithm != SignatureAlgorithm::EcdsaP256Sha256
            || signed.initialize_direct_proof.is_some()
            || signed.document_signature.is_some()
            || signed.pdf_cms.is_some()
            || !actors.insert(actor)
            || !keys.insert(signed.spki_sha256.as_str())
            || *proof != metadata(&op, proof.nonce)
        {
            return Ok(false);
        }
        let Ok(pinned) = pinned_key(tx, &op, proof.nonce).await else {
            return Ok(false);
        };
        if hex::encode(sha256(&pinned)) != signed.spki_sha256
            || verify_signature(&op, proof.nonce, &pinned, &signed.payload_signature).is_err()
        {
            return Ok(false);
        }
    }
    Ok(true)
}

pub(super) async fn verified_prior(
    tx: &Transaction<'_>,
    request: &SigningRequestRow,
    approvals: &[SigningApprovalRow],
) -> SigningResult<bool> {
    let latch = request_latch(tx, request)
        .await
        .map_err(|_| unavailable())?
        .ok_or_else(unavailable)?;
    check_approvals(tx, request, latch, approvals).await
}

pub(super) async fn has_quorum(
    tx: &Transaction<'_>,
    request: &SigningRequestRow,
    latch: Latch,
    approvals: &[SigningApprovalRow],
    minimum: i64,
) -> anyhow::Result<bool> {
    if !valid_frozen_rule(request)
        || i64::try_from(approvals.len())? < minimum.max(i64::from(request.required)).max(1)
    {
        return Ok(false);
    }
    Ok(check_approvals(tx, request, latch, approvals).await?)
}

pub async fn already_approved(tx: &Transaction<'_>, op: &DirectOperation) -> SigningResult<bool> {
    let a = op.anchor();
    let request = read_request(tx, a.tenant, op.request()).await?;
    let latch = request_latch(tx, &request)
        .await
        .map_err(|_| unavailable())?
        .ok_or_else(unavailable)?;
    if known_operation(tx, &request, latch, op.actor()).await? != *op {
        return Err(unavailable());
    }
    let approvals = list_signing_approvals(tx, a.tenant, a.event, op.request()).await?;
    if !approvals
        .iter()
        .any(|r| r.user_id == op.actor().to_string())
    {
        return Ok(false);
    }
    if !check_approvals(tx, &request, latch, &approvals).await? {
        return Err(unavailable());
    }
    Ok(true)
}

pub async fn commit(
    client: &mut Client,
    services: &SigningServices,
    caller: &SigningCaller,
    proof: VerifiedActionProof,
    now: DateTime<Utc>,
) -> SigningResult<ApproveOutcome> {
    approve_direct_action(client, services, caller, proof, now).await
}

/// A newer ballot publication can only be initialized after precisely its
/// executed V2 configuration. Use the already verified installation pose
/// directly to avoid recursively resolving Initialize's own scope.
pub(in crate::services::signing) async fn verified_configuration_publication(
    tx: &Transaction<'_>,
    tenant: Uuid,
    event: Uuid,
    latch: Latch,
    publication: Uuid,
) -> SigningResult<bool> {
    let Some(row) = tx
        .query_opt(
            "SELECT id FROM sequent_backend.signing_request
           WHERE (tenant_id,election_event_id,action,status) =
               ($1,$2,'approve-configuration','executed')
             AND executed_at IS NOT NULL AND subject->>'ballot_publication_id'=$3
           ORDER BY executed_at DESC, id LIMIT 1",
            &[&tenant, &event, &publication.to_string()],
        )
        .await?
    else {
        return Ok(false);
    };
    let request_id: Uuid = row.try_get(0)?;
    let Some(request) = get_signing_request(tx, tenant, event, request_id).await? else {
        return Ok(false);
    };
    let digest = request
        .subject
        .get("digest")
        .and_then(serde_json::Value::as_str);
    let target = publication.to_string();
    let result = request.execution_result.as_ref().ok_or_else(unavailable)?;
    if digest.is_none()
        || result.get("digest").and_then(serde_json::Value::as_str) != digest
        || result
            .get("ballot_publication_id")
            .and_then(serde_json::Value::as_str)
            != Some(target.as_str())
        || !super::operational_device::matches_canonical_request(&request)
    {
        return Ok(false);
    }
    let approvals = list_signing_approvals(tx, tenant, event, request_id).await?;
    // Active proof keys and exact original signatures, not a stored count,
    // are what let an executed configuration release the next Initialize.
    Ok(has_quorum(tx, &request, latch, &approvals, i64::from(request.required)).await?)
}
