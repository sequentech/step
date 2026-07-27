// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! In-browser mixnet emulator (M3-C).
//!
//! The emulator is **interactive** and runs against a **live b4** (over HTTP+S3
//! via [`WasmHttpTransport`], with per-trustee IndexedDB persistence) — the
//! production-shaped setting. It manages a single [`Setup`](Emulator) (one
//! committee: manager + trustee keys + `Configuration`) and drives the protocol
//! one round at a time, letting a page inspect what the board and each trustee
//! hold between rounds.
//!
//! ## One DKG, many tallies (§8.2)
//!
//! The committee runs the DKG **once** on a hidden parent board; each **tally**
//! then runs on its own hidden child board, unioned with the DKG parent and
//! seeded with the trustees' own committed DKG digests (the anti-rewrite seed,
//! §8.2). This is the board-union batch mechanism: a single DKG backs many
//! tallies, each over a different ciphertext set. The trustees are pure and are
//! reused across the DKG and every tally; only the board clients change.
//!
//! ## Bridging a browser refresh
//!
//! To test persistence-based anti-rewrite across a page refresh the committee
//! identity must survive it — a fresh `create` would mint new keys, a new
//! `Configuration`, and new board/IndexedDB names, orphaning everything persisted
//! before. So a `Setup` has a **stable id** from which every board name and every
//! per-trustee IndexedDB name is derived deterministically, and it can be
//! [`export`](Emulator::export)ed to / [`import`](Emulator::import)ed from a paste
//! string. Import rebuilds the exact same committee and reconnects to the same
//! boards + IndexedDB stores, so persisted state reloads.
//!
//! One-shot pass/fail runs belong in tests, not the emulator. Coverage: the
//! protocol logic is covered natively by `protocol_test_memory[_union]` /
//! `protocol_test_http[_union]`; the wasm build + serialization + async I/O by the
//! headless IndexedDB test (`tests/wasm_indexeddb.rs`); and the protocol running
//! correctly *under wasm* end-to-end by this emulator in a real browser. (A
//! headless wasm *protocol* test is infeasible: the crypto needs the rayon thread
//! pool, which needs the atomics/shared-memory build, whose async test executor
//! needs SharedArrayBuffer/COOP that the wasm-bindgen test runner cannot provide.)

use wasm_bindgen::prelude::*;

use anyhow::{anyhow, Result};
use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::HashSet;
use std::marker::PhantomData;

use cryptography::context::{Context, RistrettoCtx};
use cryptography::cryptosystem::elgamal::{Ciphertext, KeyPair, PublicKey};
use cryptography::traits::groups::CryptographicGroup;
use cryptography::utils::serialization::{VDeserializable, VSerializable};
use cryptography::utils::signatures::SignatureScheme;

use crate::messages::artifact::{Ballots, Configuration, DkgPublicKey, Plaintexts};
use crate::messages::newtypes::{
    hash_bytes, ConfigurationHash, PublicKeyHash, Timestamp, TrusteeIndex,
};
use crate::messages::protocol_manager::ProtocolManager;
use crate::messages::wire::{MessageType, ProtocolMessage};

use crate::board::transport::Transport;
use crate::board::BoardClient;
use crate::messages::predicate::Predicate;
use crate::runtime::SessionTrustee;
use crate::wasm::persistence::IndexedDbPersistence;
use crate::wasm::transport::WasmHttpTransport;

/// Wire `date` for every emulator message (§3.1 — timestamps are wire-only).
const DATE: Timestamp = 0;

use crate::messages::newtypes::MAX_TRUSTEES;

/// The context's signature scheme (Ed25519) and its signing-key type.
type Sig = <RistrettoCtx as Context>::SignatureScheme;
type EmuSigner = <Sig as SignatureScheme<<RistrettoCtx as Context>::Rng>>::Signer;

/// One trustee's board client for this emulator: browser HTTP+S3 transport with
/// IndexedDB persistence.
type EmuClient = BoardClient<RistrettoCtx, WasmHttpTransport, IndexedDbPersistence>;

///////////////////////////////////////////////////////////////////////////
// Deterministic names derived from the Setup id (§8.2 board union; bridge).
///////////////////////////////////////////////////////////////////////////

/// The hidden parent (DKG) board for a Setup.
fn parent_board_name(id: &str) -> String {
    format!("emu_{id}_dkg")
}

/// The hidden child (tally `k`) board for a Setup.
fn child_board_name(id: &str, tally: usize) -> String {
    format!("emu_{id}_tally_{tally}")
}

/// Trustee `i`'s IndexedDB store for the DKG phase.
fn dkg_db_name(id: &str, trustee: usize) -> String {
    format!("emu_{id}_dkg_t{trustee}")
}

/// Trustee `i`'s IndexedDB store for tally `k`.
fn tally_db_name(id: &str, tally: usize, trustee: usize) -> String {
    format!("emu_{id}_tally_{tally}_t{trustee}")
}

///////////////////////////////////////////////////////////////////////////
// Committee (key material + configuration).
///////////////////////////////////////////////////////////////////////////

/// A committee's key material and its shared `Configuration`.
struct Committee {
    pm: ProtocolManager<RistrettoCtx>,
    signing_keys: Vec<EmuSigner>,
    share_keypairs: Vec<KeyPair<RistrettoCtx>>,
    cfg: Configuration<RistrettoCtx>,
    cfg_hash: ConfigurationHash,
}

/// Assemble a committee from existing key material: derive the trustee verifying
/// keys and share-encryption keys, build the `Configuration`, and hash it. Shared
/// by fresh generation and import (deterministic given the same material).
fn build_committee(
    pm: ProtocolManager<RistrettoCtx>,
    signing_keys: Vec<EmuSigner>,
    share_keypairs: Vec<KeyPair<RistrettoCtx>>,
    threshold: usize,
    width: usize,
) -> Result<Committee> {
    let trustee_vks = signing_keys.iter().map(Sig::verifying_key).collect();
    let share_enc_keys = share_keypairs.iter().map(|kp| kp.pkey.y.clone()).collect();
    let cfg = Configuration::<RistrettoCtx>::new(
        0,
        Sig::verifying_key(&pm.signing_key),
        trustee_vks,
        threshold,
        width,
        PhantomData,
    )
    .with_share_encryption_keys(share_enc_keys);
    let cfg_hash = ConfigurationHash::from_configuration(&cfg)?;
    Ok(Committee {
        pm,
        signing_keys,
        share_keypairs,
        cfg,
        cfg_hash,
    })
}

/// Generate a fresh manager + `n_trustees` key pairs and the shared configuration.
fn generate_committee(n_trustees: usize, n_threshold: usize, width: usize) -> Result<Committee> {
    let mut key_rng = RistrettoCtx::get_rng();
    let pm = ProtocolManager::<RistrettoCtx>::new(Sig::gen_signing_key(&mut key_rng));

    let mut signing_keys = Vec::with_capacity(n_trustees);
    let mut share_keypairs = Vec::with_capacity(n_trustees);
    for _ in 0..n_trustees {
        signing_keys.push(Sig::gen_signing_key(&mut key_rng));
        share_keypairs.push(KeyPair::<RistrettoCtx>::generate());
    }
    build_committee(pm, signing_keys, share_keypairs, n_threshold, width)
}

/// Build the (pure, reusable) trustees from a committee's material (§8.2 — the
/// same trustees drive the DKG and every tally; only the board client changes).
fn build_trustees(committee: &Committee) -> Result<Vec<SessionTrustee<RistrettoCtx>>> {
    committee
        .signing_keys
        .iter()
        .zip(&committee.share_keypairs)
        .enumerate()
        .map(|(i, (signing_key, keypair))| {
            SessionTrustee::new(
                (i + 1).to_string(),
                signing_key.clone(),
                keypair.clone(),
                &committee.cfg,
            )
        })
        .collect()
}

///////////////////////////////////////////////////////////////////////////
// Shared helpers (generic over the context/width for the dispatch macro).
///////////////////////////////////////////////////////////////////////////

/// The body of the first message of `kind` in `messages`, if any.
fn find_body<C: Context>(messages: &[ProtocolMessage<C>], kind: MessageType) -> Option<&Vec<u8>> {
    messages
        .iter()
        .find(|m| m.message_type == kind)
        .and_then(|m| m.body.as_ref())
}

/// Encrypt `ciphertexts` random plaintexts under the DKG public key (`pk_body`)
/// and build the manager's `Ballots` message. Returns the message plus the set of
/// expected plaintexts (as serialized bytes) for later verification.
fn encrypt_ballots<C: Context, const W: usize>(
    pk_body: &[u8],
    ciphertexts: u32,
    mixing_trustees: Vec<TrusteeIndex>,
    pm: &ProtocolManager<C>,
    cfg_hash: ConfigurationHash,
) -> Result<(ProtocolMessage<C>, HashSet<Vec<u8>>)> {
    let dkg_pk = DkgPublicKey::<C>::deser(pk_body)
        .map_err(|e| anyhow!("deserialize public key: {:?}", e))?;
    let pk_hash = PublicKeyHash(hash_bytes(pk_body));
    let pk = PublicKey::<C>::new(dkg_pk.pk.clone());

    let mut enc_rng = C::get_rng();
    let plaintexts_in: Vec<[C::Element; W]> = (0..ciphertexts)
        .map(|_| std::array::from_fn(|_| C::G::random_element(&mut enc_rng)))
        .collect();
    let encrypted: Vec<Ciphertext<C, W>> = plaintexts_in.iter().map(|p| pk.encrypt(p)).collect();
    let expected: HashSet<Vec<u8>> = plaintexts_in.iter().map(|p| p.ser()).collect();

    let ballots = Ballots::<C, W>::new(encrypted);
    let message =
        ProtocolMessage::<C>::ballots(pm, DATE, cfg_hash, pk_hash, mixing_trustees, &ballots);
    Ok((message, expected))
}

/// Compare decrypted plaintexts (`pt_body`) against the `expected` set. Returns
/// `(match, actual_count)`.
fn plaintexts_match<C: Context, const W: usize>(
    pt_body: &[u8],
    expected: &HashSet<Vec<u8>>,
) -> Result<(bool, usize)> {
    let plaintexts = Plaintexts::<C, W>::deser(pt_body)
        .map_err(|e| anyhow!("deserialize plaintexts: {:?}", e))?;
    let actual: HashSet<Vec<u8>> = plaintexts.0.iter().map(|p| p.ser()).collect();
    Ok((*expected == actual, actual.len()))
}

/// Validate the committee parameters against the dispatch-macro ranges.
fn validate_params(trustees: usize, threshold: usize, width: usize) -> Result<()> {
    if !(1..=MAX_TRUSTEES).contains(&width) {
        return Err(anyhow!(
            "unsupported ciphertext width {width} (expected 1..={MAX_TRUSTEES})"
        ));
    }
    if !(2..=MAX_TRUSTEES).contains(&trustees) {
        return Err(anyhow!(
            "unsupported trustee count {trustees} (expected 2..={MAX_TRUSTEES})"
        ));
    }
    if !(2..=trustees).contains(&threshold) {
        return Err(anyhow!(
            "unsupported threshold {threshold} (expected 2..={trustees})"
        ));
    }
    Ok(())
}

///////////////////////////////////////////////////////////////////////////
// Export / import blob (the bridge across a browser refresh).
///////////////////////////////////////////////////////////////////////////

/// The serializable identity of a `Setup`: its stable id, parameters, and secret
/// key material. Everything else (boards, persisted committed sets, the DKG public
/// key) is reconstructable from b4 + IndexedDB given the same id, so it is not in
/// the blob. Exported base64(JSON); pasted back on another page load.
#[derive(Serialize, Deserialize)]
struct SetupBlob {
    id: String,
    trustees: usize,
    threshold: usize,
    ciphertexts: u32,
    width: usize,
    /// How many tallies have been started, so re-imported runs allocate fresh
    /// child boards instead of colliding with pre-refresh ones.
    tally_index: usize,
    /// Manager signing key (base64).
    manager_sk: String,
    /// Trustee signing keys (base64), in index order.
    trustee_sks: Vec<String>,
    /// Trustee share-decryption secret scalars (base64 `VSerializable`), in index
    /// order; the public side is recomputed as `g^sk`.
    share_sks: Vec<String>,
}

///////////////////////////////////////////////////////////////////////////
// Reports (serialized to JS).
///////////////////////////////////////////////////////////////////////////

/// Which phase the emulator is in (a UI label; the datalog itself is stateless).
#[derive(Clone, Copy, PartialEq)]
enum Phase {
    Dkg,
    Tally,
}

impl Phase {
    fn as_str(self) -> &'static str {
        match self {
            Phase::Dkg => "dkg",
            Phase::Tally => "tally",
        }
    }
}

/// What one trustee produced in a round (message types it posted).
#[derive(Serialize)]
struct TrusteeActivity {
    trustee: usize,
    produced: Vec<String>,
}

/// One round's result, including per-trustee activity.
#[derive(Serialize)]
struct StepReport {
    advanced: bool,
    round: usize,
    phase: String,
    activity: Vec<TrusteeActivity>,
}

/// A board message summary (type, sender, short body digest).
#[derive(Serialize)]
struct MessageSummary {
    kind: String,
    sender: String,
    digest: String,
}

/// A snapshot of the board: per-type counts plus the message list.
#[derive(Serialize)]
struct StateReport {
    phase: String,
    round: usize,
    tally: usize,
    board: String,
    configuration: usize,
    shares: usize,
    public_key: usize,
    ballots: usize,
    mix: usize,
    mix_signature: usize,
    partial_decryptions: usize,
    plaintexts: usize,
    messages: Vec<MessageSummary>,
}

/// Result of a verification.
#[derive(Serialize)]
struct VerifyReport {
    success: bool,
    expected: usize,
    actual: usize,
}

///////////////////////////////////////////////////////////////////////////
// Emulator
///////////////////////////////////////////////////////////////////////////

/// An interactive in-browser emulator for a single `Setup` (one committee) against
/// a live b4.
///
/// `create` generates a committee, creates its DKG board on b4, posts the
/// `Configuration` (as manager), and connects one DKG session per trustee over
/// [`WasmHttpTransport`] with its own IndexedDB store. The protocol is driven a
/// round at a time (`step`); once the DKG reaches its fixpoint, `new_tally`
/// creates a fresh child board (unioned with the DKG parent, §8.2), posts a fresh
/// ciphertext set, and rebuilds the sessions to run that tally. `state`/`verify`
/// inspect the current board; `export`/`import` bridge the Setup across a refresh.
///
/// All methods do real HTTP, so they are async; mutable state sits behind a
/// `RefCell` so they can take `&self` (wasm-bindgen disallows `&mut self` on
/// async), and the driving page disables its controls while an op runs so there is
/// no re-entrancy.
#[wasm_bindgen]
pub struct Emulator {
    b4_url: String,
    setup_id: String,
    committee: Committee,
    mixing_trustees: Vec<TrusteeIndex>,
    trustees_n: usize,
    threshold: usize,
    width: usize,
    ciphertexts: u32,
    inner: RefCell<Inner>,
}

struct Inner {
    /// The pure, reusable trustees (§8.2): built once, shared across all phases.
    trustees: Vec<SessionTrustee<RistrettoCtx>>,
    /// The active phase's board clients, trustee `i` paired with client `i`.
    clients: Vec<EmuClient>,
    phase: Phase,
    round: usize,
    /// Next child-board index (also the count of tallies started).
    tally_index: usize,
    /// Each trustee's committed DKG digests — the union anti-rewrite seed (§8.2),
    /// captured when the DKG completes and reused by every tally.
    seeds: Option<Vec<Vec<Predicate>>>,
    /// The DKG public-key body, captured when the DKG completes.
    pk_body: Option<Vec<u8>>,
    /// The current tally's child board name (for `state`/`verify`).
    child_board: Option<String>,
    /// The current tally's expected plaintexts (for `verify`).
    expected: Option<HashSet<Vec<u8>>>,
}

/// Map an `anyhow::Error` to a JS error.
fn js(e: anyhow::Error) -> JsValue {
    JsValue::from_str(&format!("{e:#}"))
}

/// The busy error (a `RefCell` borrow failed — a concurrent op is running).
fn busy() -> JsValue {
    JsValue::from_str("emulator is busy (an operation is already running)")
}

impl Emulator {
    /// Connect one DKG board client per trustee against the parent board, each with
    /// its own IndexedDB store. Used by both `create` and `import`.
    async fn connect_dkg_clients(&self) -> Result<Vec<EmuClient>> {
        let parent_board = parent_board_name(&self.setup_id);
        let mut clients = Vec::with_capacity(self.trustees_n);
        for i in 0..self.trustees_n {
            let transport = WasmHttpTransport::new(&self.b4_url, &parent_board);
            let persistence = IndexedDbPersistence::open(&dkg_db_name(&self.setup_id, i))
                .await
                .map_err(|e| anyhow!("failed to open IndexedDB: {e:#}"))?;
            clients.push(BoardClient::connect(transport, persistence).await?);
        }
        Ok(clients)
    }

    /// Capture the DKG anti-rewrite seeds and public key, transitioning out of the
    /// DKG phase. The seeds are each DKG client's own committed digests (§8.2 — the
    /// trustee's own memory, never a b4 re-fetch); the public key is read off the
    /// parent board. Errors if the DKG has not yet produced a public key.
    async fn capture_dkg(&self) -> Result<()> {
        let parent = WasmHttpTransport::new(&self.b4_url, &parent_board_name(&self.setup_id));
        let messages = Transport::<RistrettoCtx>::fetch(&parent).await?;
        let pk_body = find_body(&messages, MessageType::PublicKey)
            .ok_or_else(|| {
                anyhow!("no public key on the board yet (step the DKG to a fixpoint first)")
            })?
            .clone();
        let mut inner = self
            .inner
            .try_borrow_mut()
            .map_err(|_| anyhow!("emulator is busy"))?;
        let seeds = inner
            .clients
            .iter()
            .map(|c| c.committed().to_vec())
            .collect();
        inner.seeds = Some(seeds);
        inner.pk_body = Some(pk_body);
        Ok(())
    }
}

#[wasm_bindgen]
impl Emulator {
    /// Create a committee, create its DKG board on b4, post the `Configuration`,
    /// and connect one DKG session per trustee (each with its own IndexedDB store).
    pub async fn create(
        b4_url: String,
        trustees: usize,
        threshold: usize,
        ciphertexts: u32,
        width: usize,
    ) -> Result<Emulator, JsValue> {
        validate_params(trustees, threshold, width).map_err(js)?;
        let committee = generate_committee(trustees, threshold, width).map_err(js)?;
        // A stable id so board names and IndexedDB names are deterministic (and so
        // a re-import reconnects to the very same stores). Random per fresh Setup.
        let setup_id = format!("{:x}", js_sys::Date::now() as u64);
        Self::start(
            b4_url,
            setup_id,
            committee,
            trustees,
            threshold,
            ciphertexts,
            width,
            0,
            true,
        )
        .await
    }

    /// Serialize this Setup's identity + secret key material to a paste string
    /// (base64 of JSON) so it can survive a browser refresh — the bridge needed to
    /// test persistence-based anti-rewrite (§6.2–6.3).
    pub fn export(&self) -> Result<String, JsValue> {
        let tally_index = {
            let inner = self.inner.try_borrow().map_err(|_| busy())?;
            inner.tally_index
        };
        let manager_sk = Sig::signer_to_base64_string(&self.committee.pm.signing_key)
            .map_err(|e| JsValue::from_str(&format!("encode manager key: {e}")))?;
        let trustee_sks = self
            .committee
            .signing_keys
            .iter()
            .map(|sk| Sig::signer_to_base64_string(sk))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| JsValue::from_str(&format!("encode trustee key: {e}")))?;
        let share_sks = self
            .committee
            .share_keypairs
            .iter()
            .map(|kp| general_purpose::STANDARD_NO_PAD.encode(kp.skey.ser()))
            .collect();
        let blob = SetupBlob {
            id: self.setup_id.clone(),
            trustees: self.trustees_n,
            threshold: self.threshold,
            ciphertexts: self.ciphertexts,
            width: self.width,
            tally_index,
            manager_sk,
            trustee_sks,
            share_sks,
        };
        let json = serde_json::to_string(&blob)
            .map_err(|e| JsValue::from_str(&format!("serialize setup: {e}")))?;
        Ok(general_purpose::STANDARD_NO_PAD.encode(json))
    }

    /// Rebuild a Setup from an [`export`](Self::export)ed string and reconnect it to
    /// the same b4 boards + IndexedDB stores. The committee (keys, `Configuration`,
    /// hash) is deterministic in the blob; the DKG public key, the anti-rewrite
    /// seeds, and the board contents reload from b4/IndexedDB when the DKG clients
    /// reconnect. Lands in the DKG phase; if the DKG is already complete, `new_tally`
    /// starts fresh tallies (continuing the child-board index).
    pub async fn import(b4_url: String, blob: String) -> Result<Emulator, JsValue> {
        let bytes = general_purpose::STANDARD_NO_PAD
            .decode(blob.trim())
            .map_err(|e| JsValue::from_str(&format!("decode setup: {e}")))?;
        let blob: SetupBlob = serde_json::from_slice(&bytes)
            .map_err(|e| JsValue::from_str(&format!("parse setup: {e}")))?;

        let manager_sk = Sig::signer_from_base64_string(&blob.manager_sk)
            .map_err(|e| JsValue::from_str(&format!("decode manager key: {e}")))?;
        let pm = ProtocolManager::<RistrettoCtx>::new(manager_sk);

        let mut signing_keys = Vec::with_capacity(blob.trustees);
        for s in &blob.trustee_sks {
            signing_keys.push(
                Sig::signer_from_base64_string(s)
                    .map_err(|e| JsValue::from_str(&format!("decode trustee key: {e}")))?,
            );
        }
        let mut share_keypairs = Vec::with_capacity(blob.trustees);
        for s in &blob.share_sks {
            let raw = general_purpose::STANDARD_NO_PAD
                .decode(s)
                .map_err(|e| JsValue::from_str(&format!("decode share key: {e}")))?;
            let skey = <RistrettoCtx as Context>::Scalar::deser(&raw)
                .map_err(|e| JsValue::from_str(&format!("deserialize share key: {e:?}")))?;
            let pkey = <RistrettoCtx as Context>::G::g_exp(&skey);
            share_keypairs.push(KeyPair::<RistrettoCtx>::new(skey, pkey));
        }

        let committee =
            build_committee(pm, signing_keys, share_keypairs, blob.threshold, blob.width)
                .map_err(js)?;
        Self::start(
            b4_url,
            blob.id,
            committee,
            blob.trustees,
            blob.threshold,
            blob.ciphertexts,
            blob.width,
            blob.tally_index,
            false,
        )
        .await
    }

    /// Shared constructor for `create`/`import`. When `fresh`, creates the DKG
    /// board on b4 and posts the `Configuration`; otherwise the board is assumed to
    /// exist. Then connects the per-trustee DKG clients. Their message stores stay
    /// empty until the first `step` (update-first, §6): `connect` reloads only the
    /// persisted committed set (the anti-rewrite baseline) and the `Configuration`,
    /// not the board contents — those are shown on the global board panel instead.
    #[allow(clippy::too_many_arguments)]
    async fn start(
        b4_url: String,
        setup_id: String,
        committee: Committee,
        trustees: usize,
        threshold: usize,
        ciphertexts: u32,
        width: usize,
        tally_index: usize,
        fresh: bool,
    ) -> Result<Emulator, JsValue> {
        let parent_board = parent_board_name(&setup_id);
        if fresh {
            WasmHttpTransport::create_board(&b4_url, &parent_board)
                .await
                .map_err(js)?;
            let cfg_message =
                ProtocolMessage::<RistrettoCtx>::configuration(&committee.pm, DATE, &committee.cfg);
            let manager = WasmHttpTransport::new(&b4_url, &parent_board);
            Transport::<RistrettoCtx>::post(&manager, vec![cfg_message])
                .await
                .map_err(js)?;
        }

        let trustee_sessions = build_trustees(&committee).map_err(js)?;
        let emulator = Emulator {
            b4_url,
            setup_id,
            committee,
            mixing_trustees: (1..=threshold).collect(),
            trustees_n: trustees,
            threshold,
            width,
            ciphertexts,
            inner: RefCell::new(Inner {
                trustees: trustee_sessions,
                clients: Vec::new(),
                phase: Phase::Dkg,
                round: 0,
                tally_index,
                seeds: None,
                pk_body: None,
                child_board: None,
                expected: None,
            }),
        };

        let clients = emulator.connect_dkg_clients().await.map_err(js)?;
        emulator.inner.borrow_mut().clients = clients;
        Ok(emulator)
    }

    /// Advance every trustee one update-first round over the active clients. Returns
    /// whether the round produced anything (false ⇒ this phase reached its fixpoint).
    pub async fn step(&self) -> Result<JsValue, JsValue> {
        // The board client does real HTTP here, so this borrow is held across an
        // await. The driving page steps sequentially and disables its controls
        // while an op runs; `try_borrow_mut` turns any accidental re-entrancy into
        // a clean error instead of a panic. Each trustee is driven update -> step
        // -> post so we can report what it produced this round.
        let (advanced, round, phase, activity) = {
            let mut inner = self.inner.try_borrow_mut().map_err(|_| busy())?;
            let phase = inner.phase.as_str().to_string();
            let mut advanced = false;
            let mut activity = Vec::with_capacity(inner.clients.len());
            {
                let inner = &mut *inner;
                for (i, (trustee, client)) in inner
                    .trustees
                    .iter()
                    .zip(inner.clients.iter_mut())
                    .enumerate()
                {
                    client.update().await.map_err(js)?;
                    let produced = trustee.step(client.view()).map_err(js)?;
                    let kinds: Vec<String> = produced
                        .iter()
                        .map(|m| format!("{:?}", m.message_type))
                        .collect();
                    if !produced.is_empty() {
                        advanced = true;
                    }
                    client.post(produced).await.map_err(js)?;
                    activity.push(TrusteeActivity {
                        trustee: i + 1,
                        produced: kinds,
                    });
                }
            }
            inner.round += 1;
            (advanced, inner.round, phase, activity)
        };
        let report = StepReport {
            advanced,
            round,
            phase,
            activity,
        };
        serde_wasm_bindgen::to_value(&report)
            .map_err(|e| JsValue::from_str(&format!("failed to serialize: {e}")))
    }

    /// Start a new tally over the completed DKG (§8.2): create a fresh child board,
    /// post a fresh ciphertext set (as manager), and rebuild the sessions as union
    /// clients (child ∪ DKG parent, seeded with the trustees' own DKG digests).
    /// Requires the DKG to have produced a public key (step it to a fixpoint first);
    /// the first call captures the DKG seeds + public key.
    pub async fn new_tally(&self) -> Result<JsValue, JsValue> {
        let need_capture = {
            let inner = self.inner.try_borrow().map_err(|_| busy())?;
            inner.pk_body.is_none()
        };
        if need_capture {
            self.capture_dkg().await.map_err(js)?;
        }

        let (seeds, pk_body, tally) = {
            let inner = self.inner.try_borrow().map_err(|_| busy())?;
            (
                inner
                    .seeds
                    .clone()
                    .ok_or_else(|| js(anyhow!("no DKG seeds")))?,
                inner
                    .pk_body
                    .clone()
                    .ok_or_else(|| js(anyhow!("no DKG public key")))?,
                inner.tally_index,
            )
        };

        // A fresh child board + ciphertext set for this tally.
        let child_board = child_board_name(&self.setup_id, tally);
        WasmHttpTransport::create_board(&self.b4_url, &child_board)
            .await
            .map_err(js)?;
        let (ballots_message, expected) = crate::dispatch_ciphertext_width!(self.width, {
            encrypt_ballots::<RistrettoCtx, W>(
                &pk_body,
                self.ciphertexts,
                self.mixing_trustees.clone(),
                &self.committee.pm,
                self.committee.cfg_hash,
            )
        })
        .map_err(js)?;
        let manager = WasmHttpTransport::new(&self.b4_url, &child_board);
        Transport::<RistrettoCtx>::post(&manager, vec![ballots_message])
            .await
            .map_err(js)?;

        // One union client per trustee: child (tally) ∪ parent (DKG), seeded with
        // that trustee's own committed DKG digests (§8.2).
        let parent_board = parent_board_name(&self.setup_id);
        let mut clients = Vec::with_capacity(self.trustees_n);
        for (i, seed) in seeds.into_iter().enumerate() {
            let child_transport = WasmHttpTransport::new(&self.b4_url, &child_board);
            let parent_transport = WasmHttpTransport::new(&self.b4_url, &parent_board);
            let persistence = IndexedDbPersistence::open(&tally_db_name(&self.setup_id, tally, i))
                .await
                .map_err(|e| JsValue::from_str(&format!("failed to open IndexedDB: {e:#}")))?;
            clients.push(
                BoardClient::connect_union(child_transport, parent_transport, persistence, seed)
                    .await
                    .map_err(js)?,
            );
        }

        {
            let mut inner = self.inner.try_borrow_mut().map_err(|_| busy())?;
            inner.clients = clients;
            inner.phase = Phase::Tally;
            inner.round = 0;
            inner.child_board = Some(child_board);
            inner.expected = Some(expected);
            inner.tally_index = tally + 1;
        }
        self.state().await
    }

    /// A snapshot of the current board's contents by message type (fetched from b4):
    /// the DKG parent while in the DKG phase, or the active tally's child board.
    pub async fn state(&self) -> Result<JsValue, JsValue> {
        let (phase, round, tally, board_name, is_child) = {
            let inner = self.inner.try_borrow().map_err(|_| busy())?;
            match (&inner.child_board, inner.phase) {
                (Some(child), Phase::Tally) => (
                    inner.phase.as_str().to_string(),
                    inner.round,
                    inner.tally_index.saturating_sub(1),
                    child.clone(),
                    true,
                ),
                _ => (
                    inner.phase.as_str().to_string(),
                    inner.round,
                    inner.tally_index,
                    parent_board_name(&self.setup_id),
                    false,
                ),
            }
        };
        let transport = WasmHttpTransport::new(&self.b4_url, &board_name);
        let messages = Transport::<RistrettoCtx>::fetch(&transport)
            .await
            .map_err(js)?;
        let count = |t: MessageType| messages.iter().filter(|m| m.message_type == t).count();
        let list: Vec<MessageSummary> = messages
            .iter()
            .map(|m| MessageSummary {
                kind: format!("{:?}", m.message_type),
                sender: m.sender.name.clone(),
                digest: match &m.body {
                    Some(body) => hex::encode(&hash_bytes(body)[..])
                        .chars()
                        .take(12)
                        .collect(),
                    None => "-".to_string(),
                },
            })
            .collect();
        let report = StateReport {
            phase,
            round,
            tally,
            board: board_name,
            // The Configuration lives on the parent (DKG) board; `fetch` excludes it.
            configuration: if is_child { 0 } else { 1 },
            shares: count(MessageType::Shares),
            public_key: count(MessageType::PublicKey),
            ballots: count(MessageType::Ballots),
            mix: count(MessageType::Mix),
            mix_signature: count(MessageType::MixSignature),
            partial_decryptions: count(MessageType::PartialDecryptions),
            plaintexts: count(MessageType::Plaintexts),
            messages: list,
        };
        serde_wasm_bindgen::to_value(&report)
            .map_err(|e| JsValue::from_str(&format!("failed to serialize: {e}")))
    }

    /// Compare the current tally's decrypted plaintexts with its encrypted inputs.
    pub async fn verify(&self) -> Result<JsValue, JsValue> {
        let (expected, board_name) = {
            let inner = self.inner.try_borrow().map_err(|_| busy())?;
            let expected = inner
                .expected
                .clone()
                .ok_or_else(|| JsValue::from_str("no tally started yet"))?;
            let board = inner
                .child_board
                .clone()
                .ok_or_else(|| JsValue::from_str("no tally started yet"))?;
            (expected, board)
        };
        let transport = WasmHttpTransport::new(&self.b4_url, &board_name);
        let messages = Transport::<RistrettoCtx>::fetch(&transport)
            .await
            .map_err(js)?;
        let pt_body = find_body(&messages, MessageType::Plaintexts).ok_or_else(|| {
            JsValue::from_str("no plaintexts on the board yet (finish the tally first)")
        })?;
        let (success, actual) = crate::dispatch_ciphertext_width!(self.width, {
            plaintexts_match::<RistrettoCtx, W>(pt_body, &expected)
        })
        .map_err(js)?;
        let report = VerifyReport {
            success,
            expected: expected.len(),
            actual,
        };
        serde_wasm_bindgen::to_value(&report)
            .map_err(|e| JsValue::from_str(&format!("failed to serialize: {e}")))
    }

    /// This Setup's stable id (for display; also the root of every board/DB name).
    pub fn setup_id(&self) -> String {
        self.setup_id.clone()
    }

    /// The number of trustees (for the page's trustee selector).
    pub fn trustee_count(&self) -> usize {
        self.trustees_n
    }

    /// The message-store predicates the given trustee (0-based) currently holds
    /// — the datalog EDB it runs on (§6.1). Returned as readable `Debug` strings.
    pub fn trustee_predicates(&self, index: usize) -> Result<JsValue, JsValue> {
        let inner = self.inner.try_borrow().map_err(|_| busy())?;
        let client = inner
            .clients
            .get(index)
            .ok_or_else(|| JsValue::from_str("trustee index out of range"))?;
        let predicates: Vec<String> = client
            .view()
            .get_predicates()
            .iter()
            .map(|p| format!("{p:?}"))
            .collect();
        serde_wasm_bindgen::to_value(&predicates)
            .map_err(|e| JsValue::from_str(&format!("failed to serialize: {e}")))
    }

    /// The given trustee's persisted **committed** predicate set (§6.2) — the
    /// anti-rewrite baseline. Identical to the store in the happy path; diverges
    /// under anti-rewrite. Returned as readable `Debug` strings.
    pub fn trustee_committed(&self, index: usize) -> Result<JsValue, JsValue> {
        let inner = self.inner.try_borrow().map_err(|_| busy())?;
        let client = inner
            .clients
            .get(index)
            .ok_or_else(|| JsValue::from_str("trustee index out of range"))?;
        let committed: Vec<String> = client
            .committed()
            .iter()
            .map(|p| format!("{p:?}"))
            .collect();
        serde_wasm_bindgen::to_value(&committed)
            .map_err(|e| JsValue::from_str(&format!("failed to serialize: {e}")))
    }
}
