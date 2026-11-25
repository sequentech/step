// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Certificate-free, one-Post synthetic Initialize evidence. No browser
//! supplies an actor or pinned key; Harvest supplies its authenticated
//! caller and talks to the installed private PKCS#11 signer.

use super::approve::{approve_direct, sign_refusal, ApproveOutcome, SigningServices};
use super::operational_policy::local_scope::{read_latch, request_latch, Latch};
use super::operational_policy::lock_signing_policy;
use super::requests::{is_overdue, read_request, relock};
use super::{SigningCaller, SigningError, SigningResult};
use crate::postgres::signing::{
    get_signing_post, list_signing_approvals, InitializeDirectProofMetadata, SigningApprovalRow,
    SigningRequestRow,
};
use crate::postgres::signing_actions::published_publication_of_post;
use anyhow::{ensure, Context, Result};
use chrono::{DateTime, Utc};
use deadpool_postgres::{Client, GenericClient, Transaction};
use openssl::{ecdsa::EcdsaSig, nid::Nid, pkey::PKey, sha::sha256};
use sequent_core::signing::{SignatureAlgorithm, SigningAction, SigningRequestStatus, SigningRule};
use serde::Deserialize;
use std::{collections::HashSet, sync::OnceLock};
use uuid::Uuid;

pub const DIRECT_INITIALIZE_INSTALLATION_ROOT: &str = "/run/sequent-comelec-demo-direct-hsm";
const DIRECTORY: &str = "sequent-comelec-demo-direct-hsm";
pub const DEVICE_PROFILE: &str = "softhsm-test";
const DOMAIN: &[u8] = b"sequent.local.initialize.direct/v1\0";

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Installation {
    installation: Uuid,
    tenant: Uuid,
    event: Uuid,
    post: Uuid,
    publication: Uuid,
    profile: String,
}

impl Installation {
    fn validate(&self) -> Result<()> {
        ensure!(
            self.profile == DEVICE_PROFILE
                && [
                    self.installation,
                    self.tenant,
                    self.event,
                    self.post,
                    self.publication
                ]
                .iter()
                .all(|id| !id.is_nil()),
            "Bad direct Initialize descriptor"
        );
        Ok(())
    }

    fn identifies(&self, tenant: Uuid, event: Uuid, latch: Latch) -> bool {
        self.tenant == tenant
            && self.event == event
            && self.post == latch.post
            && self.publication == latch.publication
            && self.installation == latch.installation
    }
}

#[cfg(target_os = "linux")]
fn read_installation() -> Result<Option<Installation>> {
    use rustix::fs::{fstatvfs, open, openat, Mode, OFlags, StatVfsMountFlags};
    use rustix::io::Errno;
    use std::{fs::File, io::Read, os::unix::fs::MetadataExt};
    let directory_flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let run = match open("/run", directory_flags, Mode::empty()) {
        Ok(fd) => File::from(fd),
        Err(Errno::NOENT) => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let dir = match openat(&run, DIRECTORY, directory_flags, Mode::empty()) {
        Ok(fd) => File::from(fd),
        Err(Errno::NOENT) => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let protected = |f: &File| -> Result<bool> {
        Ok(f.metadata()?.uid() == 0 || fstatvfs(f)?.f_flag.contains(StatVfsMountFlags::RDONLY))
    };
    ensure!(
        protected(&run)?
            && run.metadata()?.mode() & 0o022 == 0
            && protected(&dir)?
            && dir.metadata()?.mode() & 0o022 == 0,
        "Unsafe direct Initialize directory"
    );
    let file = File::from(openat(
        &dir,
        "installation.json",
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
        Mode::empty(),
    )?);
    let meta = file.metadata()?;
    ensure!(
        meta.is_file()
            && meta.mode() & 0o222 == 0
            && protected(&file)?
            && (1..=4096).contains(&meta.len()),
        "Unsafe direct Initialize descriptor"
    );
    let mut bytes = Vec::new();
    file.take(4097).read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() <= 4096 && bytes.iter().find(|b| !b.is_ascii_whitespace()) == Some(&b'{'),
        "Invalid direct Initialize descriptor bytes"
    );
    let installation: Installation = serde_json::from_slice(&bytes)?;
    installation.validate()?;
    Ok(Some(installation))
}

#[cfg(not(target_os = "linux"))]
fn read_installation() -> Result<Option<Installation>> {
    // The feature can be checked/tested elsewhere; it is never active off Linux.
    Ok(None)
}

fn installed() -> Result<Option<&'static Installation>> {
    static INSTALLED: OnceLock<Result<Option<Installation>>> = OnceLock::new();
    INSTALLED
        .get_or_init(read_installation)
        .as_ref()
        .map(Option::as_ref)
        .map_err(|error| anyhow::anyhow!("{error:#}"))
}

pub(super) async fn verify_installation(
    client: &impl GenericClient,
    tenant: Uuid,
    event: Uuid,
    latch: Latch,
) -> Result<()> {
    ensure!(
        installed()?.is_some_and(|i| i.identifies(tenant, event, latch)),
        "No matching mounted direct Initialize installation"
    );
    let approved: bool = client
        .query_one(
            "SELECT felix3_initialize_direct.scope_enabled($1,$2,$3,$4,$5) AND EXISTS (
            SELECT 1 FROM pg_catalog.pg_attribute
            WHERE attrelid = 'sequent_backend.signing_approval'::regclass
              AND attname = 'initialize_direct_proof' AND NOT attisdropped
              AND atttypid = 'pg_catalog.jsonb'::regtype)",
            &[
                &latch.installation,
                &tenant,
                &event,
                &latch.post,
                &latch.publication,
            ],
        )
        .await?
        .try_get(0)?;
    ensure!(approved, "Direct Initialize installation disabled");
    Ok(())
}

pub(super) async fn assert_startup(client: &impl GenericClient) -> Result<()> {
    if let Some(i) = installed()? {
        let latch = read_latch(client, i.tenant, i.event)
            .await?
            .context("Direct scope not installed")?;
        verify_installation(client, i.tenant, i.event, latch).await?;
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirectScope {
    installation: Uuid,
    tenant: Uuid,
    event: Uuid,
    post: Uuid,
    publication: Uuid,
    request: Uuid,
    actor: Uuid,
    payload_sha256: String,
}

impl DirectScope {
    pub fn installation(&self) -> Uuid {
        self.installation
    }
    pub fn tenant(&self) -> Uuid {
        self.tenant
    }
    pub fn event(&self) -> Uuid {
        self.event
    }
    pub fn post(&self) -> Uuid {
        self.post
    }
    pub fn publication(&self) -> Uuid {
        self.publication
    }
    pub fn request(&self) -> Uuid {
        self.request
    }
    pub fn actor(&self) -> Uuid {
        self.actor
    }
    pub fn payload_sha256(&self) -> &str {
        &self.payload_sha256
    }

    fn digest(&self) -> SigningResult<[u8; 32]> {
        let mut digest = [0; 32];
        hex::decode_to_slice(&self.payload_sha256, &mut digest).map_err(|_| unavailable())?;
        if hex::encode(digest) != self.payload_sha256 {
            return Err(unavailable());
        }
        Ok(digest)
    }

    pub fn message(&self, nonce: Uuid) -> SigningResult<Vec<u8>> {
        if nonce.is_nil() {
            return Err(unavailable());
        }
        let mut message = Vec::with_capacity(DOMAIN.len() + 8 * 16 + 32);
        message.extend_from_slice(DOMAIN);
        for id in [
            self.installation,
            self.tenant,
            self.event,
            self.post,
            self.publication,
            self.request,
            self.actor,
            nonce,
        ] {
            message.extend_from_slice(id.as_bytes());
        }
        message.extend_from_slice(&self.digest()?);
        Ok(message)
    }

    fn proof_metadata(&self, nonce: Uuid) -> InitializeDirectProofMetadata {
        InitializeDirectProofMetadata {
            installation: self.installation,
            post: self.post,
            publication: self.publication,
            nonce,
            payload_sha256: self.payload_sha256.clone(),
            profile: DEVICE_PROFILE.into(),
        }
    }

    fn for_request(request: &SigningRequestRow, latch: Latch, actor: Uuid) -> Self {
        Self {
            installation: latch.installation,
            tenant: request.tenant_id,
            event: request.election_event_id,
            post: latch.post,
            publication: latch.publication,
            request: request.id,
            actor,
            payload_sha256: request.payload_sha256.clone(),
        }
    }
}

pub(super) fn unavailable() -> SigningError {
    SigningError::Forbidden("Direct Initialize approval is unavailable for this request.".into())
}

fn actor_uuid(user: &str) -> SigningResult<Uuid> {
    Uuid::parse_str(user)
        .ok()
        .filter(|id| !id.is_nil() && id.to_string() == user)
        .ok_or_else(unavailable)
}

fn frozen_is_valid(request: &SigningRequestRow) -> bool {
    serde_json::from_value::<SigningRule>(request.rule_snapshot.clone()).is_ok_and(|rule| {
        rule.action == SigningAction::InitializeVoting
            && rule.is_required()
            && rule.validate().is_ok()
            && i32::from(rule.required()) == request.required
    })
}

async fn checked_request(
    tx: &Transaction<'_>,
    request: &SigningRequestRow,
) -> SigningResult<Latch> {
    let latch = request_latch(tx, request)
        .await
        .map_err(|_| unavailable())?
        .ok_or_else(unavailable)?;
    if request.action != SigningAction::InitializeVoting
        || request.election_id != Some(latch.post)
        || request.area_id.is_some()
        || request.document_id.is_some()
        || request.document_sha256.is_some()
        || !frozen_is_valid(request)
        || published_publication_of_post(
            tx,
            request.tenant_id,
            request.election_event_id,
            latch.post,
        )
        .await?
            != Some(latch.publication)
    {
        return Err(unavailable());
    }
    super::guard::effective_rule_in_mode(
        tx,
        request.tenant_id,
        request.election_event_id,
        request.action,
        super::operational_policy::OperationalSigningMode::HsmRequired,
    )
    .await?;
    Ok(latch)
}

async fn authorize_row(
    tx: &Transaction<'_>,
    caller: &SigningCaller,
    row: &SigningRequestRow,
    reviewed_payload_sha256: &str,
) -> SigningResult<DirectScope> {
    if row.payload_sha256 != reviewed_payload_sha256
        || sign_refusal(tx, caller, row).await?.is_some()
    {
        return Err(unavailable());
    }
    let latch = checked_request(tx, row).await?;
    let post = get_signing_post(tx, row.tenant_id, row.election_event_id, latch.post)
        .await?
        .ok_or_else(unavailable)?;
    if !caller.signs_for(post.permission_label.as_deref()) {
        return Err(unavailable());
    }
    let scope = DirectScope::for_request(row, latch, actor_uuid(&caller.user_id)?);
    scope.digest()?;
    Ok(scope)
}

/// The route is responsible for a real, authenticated current actor. The
/// canonical request, scope and labelled permissions are re-read here.
pub async fn authorize(
    tx: &Transaction<'_>,
    caller: &SigningCaller,
    tenant: Uuid,
    request: Uuid,
    reviewed_payload_sha256: &str,
) -> SigningResult<DirectScope> {
    lock_signing_policy(tx).await.map_err(|_| unavailable())?;
    let row = read_request(tx, tenant, request).await?;
    let scope = authorize_row(tx, caller, &row, reviewed_payload_sha256).await?;
    if row.status == SigningRequestStatus::Waiting && !is_overdue(tx, &row).await? {
        return Ok(scope);
    }
    if approved(tx, &scope).await?.is_some() {
        return Ok(scope);
    }
    Err(unavailable())
}

pub(super) async fn lock_authorized(
    tx: &Transaction<'_>,
    caller: &SigningCaller,
    expected: &DirectScope,
) -> SigningResult<SigningRequestRow> {
    lock_signing_policy(tx).await.map_err(|_| unavailable())?;
    let row = read_request(tx, expected.tenant, expected.request).await?;
    authorize_row(tx, caller, &row, &expected.payload_sha256).await?;
    let locked = relock(tx, &row).await?;
    if authorize_row(tx, caller, &locked, &expected.payload_sha256).await? != *expected {
        return Err(unavailable());
    }
    Ok(locked)
}

/// Compare the exact on-wire format, require P-256, and reject non-canonical
/// DER encodings. This first check is only for pinning the authenticated device
/// response. The database—not that response—selects the key for approval.
pub fn verify_signature(
    scope: &DirectScope,
    nonce: Uuid,
    spki_der: &[u8],
    signature_der: &[u8],
) -> SigningResult<()> {
    if !(8..=72).contains(&signature_der.len()) || !(1..=160).contains(&spki_der.len()) {
        return Err(unavailable());
    }
    let verify = || -> Result<bool> {
        let public = PKey::public_key_from_der(spki_der)?;
        if public.public_key_to_der()?.as_slice() != spki_der {
            return Ok(false);
        }
        let key = public.ec_key()?;
        if key.group().curve_name() != Some(Nid::X9_62_PRIME256V1) {
            return Ok(false);
        }
        key.check_key()?;
        let sig = EcdsaSig::from_der(signature_der)?;
        if sig.to_der()?.as_slice() != signature_der {
            return Ok(false);
        }
        Ok(sig.verify(&sha256(&scope.message(nonce)?), &key)?)
    };
    if verify().unwrap_or(false) {
        Ok(())
    } else {
        Err(unavailable())
    }
}

async fn pinned_spki(
    tx: &Transaction<'_>,
    scope: &DirectScope,
    nonce: Uuid,
) -> SigningResult<Vec<u8>> {
    let digest = scope.digest()?;
    let spki: Option<Vec<u8>> = tx
        .query_one(
            "SELECT felix3_initialize_direct.proof_key($1,$2,$3,$4,$5,$6,$7,$8,$9)",
            &[
                &scope.installation,
                &scope.tenant,
                &scope.event,
                &scope.post,
                &scope.publication,
                &scope.request,
                &scope.actor,
                &nonce,
                &&digest[..],
            ],
        )
        .await?
        .try_get(0)?;
    spki.ok_or_else(unavailable)
}

/// Opaque, process-local evidence; not deserializable. The approval transaction
/// locks and repeats all checks before it consumes the actual reservation.
pub struct VerifiedDirectProof {
    scope: DirectScope,
    nonce: Uuid,
    signature_der: Vec<u8>,
    spki_sha256: String,
}

pub async fn verify_proof(
    tx: &Transaction<'_>,
    scope: &DirectScope,
    nonce: Uuid,
    signature_der: &[u8],
) -> SigningResult<VerifiedDirectProof> {
    lock_signing_policy(tx).await.map_err(|_| unavailable())?;
    let spki = pinned_spki(tx, scope, nonce).await?;
    verify_signature(scope, nonce, &spki, signature_der)?;
    Ok(VerifiedDirectProof {
        scope: scope.clone(),
        nonce,
        signature_der: signature_der.to_vec(),
        spki_sha256: hex::encode(sha256(&spki)),
    })
}

impl VerifiedDirectProof {
    pub(super) fn scope(&self) -> &DirectScope {
        &self.scope
    }
    pub(super) fn nonce(&self) -> Uuid {
        self.nonce
    }

    pub(super) async fn consume_and_insert(
        &self,
        tx: &Transaction<'_>,
        caller: &SigningCaller,
    ) -> SigningResult<SigningApprovalRow> {
        let scope = &self.scope;
        let digest = scope.digest()?;
        let pinned = pinned_spki(tx, scope, self.nonce).await?;
        verify_signature(scope, self.nonce, &pinned, &self.signature_der)?;
        if hex::encode(sha256(&pinned)) != self.spki_sha256 {
            return Err(unavailable());
        }
        let consumed: bool = tx
            .query_one(
                "SELECT felix3_initialize_direct.consume($1,$2,$3,$4,$5,$6,$7,$8,$9)",
                &[
                    &scope.installation,
                    &scope.tenant,
                    &scope.event,
                    &scope.post,
                    &scope.publication,
                    &scope.request,
                    &scope.actor,
                    &self.nonce,
                    &&digest[..],
                ],
            )
            .await?
            .try_get(0)?;
        if !consumed {
            return Err(unavailable());
        }
        let meta =
            serde_json::to_value(scope.proof_metadata(self.nonce)).map_err(anyhow::Error::from)?;
        let row = tx.query_opt(
            "INSERT INTO sequent_backend.signing_approval
                 (tenant_id, election_event_id, request_id, user_id, username, display_name, auth_time,
                  spki_sha256, algorithm, payload_signature, initialize_direct_proof)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,'ecdsa-p256-sha256',$9,$10)
             ON CONFLICT DO NOTHING RETURNING *",
            &[&scope.tenant, &scope.event, &scope.request, &caller.user_id, &caller.username,
              &caller.display_name, &caller.auth_time, &self.spki_sha256, &self.signature_der, &meta],
        ).await?.ok_or_else(|| SigningError::Conflict("This actor, key, or reservation already approved.".into()))?;
        Ok(SigningApprovalRow::try_from(row)?)
    }
}

async fn check_approval_rows(
    tx: &Transaction<'_>,
    request: &SigningRequestRow,
    latch: Latch,
    approvals: &[SigningApprovalRow],
) -> anyhow::Result<bool> {
    let mut actors = HashSet::new();
    let mut keys = HashSet::new();
    for signed in approvals {
        let Some(proof) = &signed.initialize_direct_proof else {
            return Ok(false);
        };
        let Ok(actor) = actor_uuid(&signed.user_id) else {
            return Ok(false);
        };
        let scope = DirectScope::for_request(request, latch, actor);
        if signed.tenant_id != scope.tenant
            || signed.election_event_id != scope.event
            || signed.request_id != scope.request
            || signed.algorithm != SignatureAlgorithm::EcdsaP256Sha256
            || signed.document_signature.is_some()
            || signed.pdf_cms.is_some()
            || !actors.insert(actor)
            || !keys.insert(signed.spki_sha256.as_str())
            || *proof != scope.proof_metadata(proof.nonce)
        {
            return Ok(false);
        }
        let Ok(spki) = pinned_spki(tx, &scope, proof.nonce).await else {
            return Ok(false);
        };
        if hex::encode(sha256(&spki)) != signed.spki_sha256
            || verify_signature(&scope, proof.nonce, &spki, &signed.payload_signature).is_err()
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
    let latch = checked_request(tx, request).await?;
    Ok(check_approval_rows(tx, request, latch, approvals).await?)
}

pub(super) async fn has_quorum(
    tx: &Transaction<'_>,
    request: &SigningRequestRow,
    latch: Latch,
    approvals: &[SigningApprovalRow],
    minimum: i64,
) -> anyhow::Result<bool> {
    if !frozen_is_valid(request)
        || request.document_id.is_some()
        || request.document_sha256.is_some()
        || i64::try_from(approvals.len())? < minimum.max(i64::from(request.required)).max(1)
    {
        return Ok(false);
    }
    if checked_request(tx, request).await? != latch {
        return Ok(false);
    }
    check_approval_rows(tx, request, latch, approvals).await
}

/// Latest truthful outcome for this actor's exact previously consumed
/// signature. A lost response does not trigger another device operation.
pub async fn approved(
    tx: &Transaction<'_>,
    scope: &DirectScope,
) -> SigningResult<Option<ApproveOutcome>> {
    let request = read_request(tx, scope.tenant, scope.request).await?;
    let latch = checked_request(tx, &request).await?;
    if DirectScope::for_request(&request, latch, scope.actor) != *scope {
        return Err(unavailable());
    }
    let approvals = list_signing_approvals(tx, scope.tenant, scope.event, scope.request).await?;
    if !approvals
        .iter()
        .any(|row| row.user_id == scope.actor.to_string())
    {
        return Ok(None);
    }
    if !check_approval_rows(tx, &request, latch, &approvals).await? {
        return Err(unavailable());
    }
    Ok(Some(ApproveOutcome {
        status: request.status,
        count: approvals.len() as i64,
        required: request.required,
    }))
}

pub async fn commit(
    client: &mut Client,
    services: &SigningServices,
    caller: &SigningCaller,
    proof: VerifiedDirectProof,
    now: DateTime<Utc>,
) -> SigningResult<ApproveOutcome> {
    approve_direct(client, services, caller, proof, now).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use openssl::ec::{EcGroup, EcKey};

    #[test]
    fn synthetic_device_proof_is_bound_to_each_uuid_nonce_and_reviewed_digest() {
        let id = Uuid::from_u128;
        let scope = DirectScope {
            installation: id(1),
            tenant: id(2),
            event: id(3),
            post: id(4),
            publication: id(5),
            request: id(6),
            actor: id(7),
            payload_sha256: "ab".repeat(32),
        };
        let nonce = id(8);
        let key =
            EcKey::generate(&EcGroup::from_curve_name(Nid::X9_62_PRIME256V1).unwrap()).unwrap();
        let public = key.public_key_to_der().unwrap();
        let bytes = scope.message(nonce).unwrap();
        assert_eq!(&bytes[..DOMAIN.len()], DOMAIN);
        for n in 1..=8 {
            let offset = DOMAIN.len() + (n - 1) * 16;
            assert_eq!(&bytes[offset..offset + 16], id(n as u128).as_bytes());
        }
        assert_eq!(&bytes[bytes.len() - 32..], &[0xab; 32]);
        let sig = EcdsaSig::sign(&sha256(&bytes), &key)
            .unwrap()
            .to_der()
            .unwrap();
        verify_signature(&scope, nonce, &public, &sig).unwrap();
        for other in [
            DirectScope {
                actor: id(9),
                ..scope.clone()
            },
            DirectScope {
                request: id(9),
                ..scope.clone()
            },
            DirectScope {
                publication: id(9),
                ..scope.clone()
            },
            DirectScope {
                post: id(9),
                ..scope.clone()
            },
            DirectScope {
                event: id(9),
                ..scope.clone()
            },
            DirectScope {
                tenant: id(9),
                ..scope.clone()
            },
            DirectScope {
                installation: id(9),
                ..scope.clone()
            },
            DirectScope {
                payload_sha256: "cd".repeat(32),
                ..scope.clone()
            },
        ] {
            assert!(verify_signature(&other, nonce, &public, &sig).is_err());
        }
        assert!(verify_signature(&scope, id(9), &public, &sig).is_err());
        let mut surplus = sig.clone();
        surplus.push(0);
        assert!(verify_signature(&scope, nonce, &public, &surplus).is_err());
    }
}
