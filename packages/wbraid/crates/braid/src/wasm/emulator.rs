// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! In-browser mixnet emulator (M3-C).
//!
//! The emulator is **interactive** and runs against a **live b4** (over HTTP+S3
//! via [`WasmHttpTransport`], with per-trustee IndexedDB persistence) — the
//! production-shaped setting. It manages a single [`Setup`](Emulator) — one
//! `Configuration` and the keys behind it — and drives the protocol one round at
//! a time, letting a page inspect what the board and each trustee hold between
//! rounds.
//!
//! ## One DKG, many tallies (§8.2)
//!
//! The trustees run the DKG **once** on a hidden parent board; each **tally**
//! then runs on its own hidden child board, unioned with the DKG parent and
//! seeded with the trustees' own committed DKG digests (the anti-rewrite seed,
//! §8.2). This is the board-union batch mechanism: a single DKG backs many
//! tallies, each over a different ciphertext set. The trustees are pure and are
//! reused across the DKG and every tally; only the board clients change.
//!
//! ## Surviving a browser refresh
//!
//! To test persistence-based anti-rewrite across a page refresh the setup's
//! identity must survive it — a fresh `create` would mint new keys, a new
//! `Configuration`, and new board/IndexedDB names, orphaning everything persisted
//! before. So a `Setup` has a **stable id** from which every board name and every
//! per-trustee IndexedDB name is derived deterministically.
//!
//! The setup is kept in `localStorage`, so a reload just reconnects: same keys,
//! same `Configuration`, same boards and IndexedDB stores, and the same tally if
//! one was under way. Only the secrets and the setup are stored — the board
//! contents, the committed sets and the DKG public key all reload from b4 and
//! IndexedDB, and a tally's plaintexts are derived rather than kept (see
//! [`ballot_plaintexts`]).
//!
//! [`export`](Emulator::export) hands over the same bytes as a paste string, for
//! moving a setup to a *different* browser. It is not on the path back from a
//! refresh.
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
use crate::protocol_manager::ProtocolManager;
use crate::messages::wire::{MessageType, ProtocolMessage};

use crate::board::transport::Transport;
use crate::board::BoardClient;
use crate::messages::predicate::Predicate;
use crate::trustee::Trustee;
use crate::wasm::persistence::IndexedDbPersistence;
use crate::wasm::transport::WasmHttpTransport;

/// Wire `date` for every emulator message (§3.1 — timestamps are wire-only).
const DATE: Timestamp = 0;

use crate::messages::newtypes::{MAX_CIPHERTEXT_WIDTH, MAX_TRUSTEES};

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
// Key material, and the Configuration it describes.
///////////////////////////////////////////////////////////////////////////

/// Every private key in the emulated deployment.
///
/// No real participant holds these together — the manager's signing key and
/// each trustee's signing and share-decryption keys live on separate machines.
/// The emulator plays every role in one process, so it holds the lot.
///
/// The public counterpart is the [`Configuration`], which these keys determine:
/// see [`configuration_for`].
struct Keys {
    pm: ProtocolManager<RistrettoCtx>,
    signing: Vec<EmuSigner>,
    share: Vec<KeyPair<RistrettoCtx>>,
}

impl Keys {
    /// A fresh manager and `n_trustees` trustees.
    fn generate(n_trustees: usize) -> Self {
        let mut key_rng = RistrettoCtx::get_rng();
        let pm = ProtocolManager::<RistrettoCtx>::new(Sig::gen_signing_key(&mut key_rng));

        let mut signing = Vec::with_capacity(n_trustees);
        let mut share = Vec::with_capacity(n_trustees);
        for _ in 0..n_trustees {
            signing.push(Sig::gen_signing_key(&mut key_rng));
            share.push(KeyPair::<RistrettoCtx>::generate());
        }
        Keys { pm, signing, share }
    }
}

/// The `Configuration` a set of keys describes, and its hash: the public
/// verifying keys, the share-encryption keys, and the election parameters.
///
/// Deterministic in its inputs, which is what lets an imported setup rebuild
/// the identical `Configuration` — and therefore the same hash, which every
/// message on the board is bound to.
fn configuration_for(
    keys: &Keys,
    threshold: usize,
    width: usize,
) -> Result<(Configuration<RistrettoCtx>, ConfigurationHash)> {
    let cfg = Configuration::<RistrettoCtx>::new(
        0,
        Sig::verifying_key(&keys.pm.signing_key),
        keys.signing.iter().map(Sig::verifying_key).collect(),
        threshold,
        width,
        keys.share.iter().map(|kp| kp.pkey.y.clone()).collect(),
        PhantomData,
    );
    let cfg_hash = ConfigurationHash::from_configuration(&cfg)?;
    Ok((cfg, cfg_hash))
}

/// Build the (pure, reusable) trustees (§8.2 — the same trustees drive the DKG
/// and every tally; only the board client changes).
fn build_trustees(
    keys: &Keys,
    cfg: &Configuration<RistrettoCtx>,
) -> Result<Vec<Trustee<RistrettoCtx>>> {
    keys.signing
        .iter()
        .zip(&keys.share)
        .enumerate()
        .map(|(i, (signing_key, keypair))| {
            Trustee::new(
                (i + 1).to_string(),
                signing_key.clone(),
                keypair.clone(),
                cfg,
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

/// Domain separation for the derived ballot plaintexts. They are demo data, but
/// they must not collide with any element derived elsewhere.
const BALLOT_TAG: &[u8] = b"braid_emulator_ballot_plaintext";

/// The plaintext set for one tally, **derived** rather than drawn at random.
///
/// A tally's plaintexts are known only to the manager — the board carries the
/// ciphertexts — so verifying a tally after a page refresh would otherwise mean
/// persisting the whole set. Deriving them from `(setup_id, tally, index)`
/// instead makes the set reproducible from what the setup already knows, so
/// nothing about a tally in progress has to be stored to verify it later.
///
/// The *encryption* randomness stays random; only the plaintexts are determined.
fn ballot_plaintexts<C: Context, const W: usize>(
    setup_id: &str,
    tally: usize,
    ciphertexts: u32,
) -> Result<Vec<[C::Element; W]>> {
    (0..ciphertexts)
        .map(|i| {
            let mut components = Vec::with_capacity(W);
            for w in 0..W {
                components.push(
                    C::G::hash_to_element(
                        &[
                            setup_id.as_bytes(),
                            &tally.to_be_bytes(),
                            &i.to_be_bytes(),
                            &w.to_be_bytes(),
                        ],
                        &[BALLOT_TAG],
                    )
                    .map_err(|e| anyhow!("derive ballot plaintext: {:?}", e))?,
                );
            }
            Ok(std::array::from_fn(|w| components[w].clone()))
        })
        .collect()
}

/// The serialized form of a derived plaintext set, for comparison with a tally's
/// decrypted output.
fn expected_plaintexts<C: Context, const W: usize>(
    setup_id: &str,
    tally: usize,
    ciphertexts: u32,
) -> Result<HashSet<Vec<u8>>> {
    Ok(ballot_plaintexts::<C, W>(setup_id, tally, ciphertexts)?
        .iter()
        .map(|p| p.ser())
        .collect())
}

/// Encrypt this tally's derived plaintexts under the DKG public key (`pk_body`)
/// and build the manager's `Ballots` message.
fn encrypt_ballots<C: Context, const W: usize>(
    pk_body: &[u8],
    setup_id: &str,
    tally: usize,
    ciphertexts: u32,
    mixing_trustees: Vec<TrusteeIndex>,
    pm: &ProtocolManager<C>,
    cfg_hash: ConfigurationHash,
) -> Result<ProtocolMessage<C>> {
    let dkg_pk = DkgPublicKey::<C>::deser(pk_body)
        .map_err(|e| anyhow!("deserialize public key: {:?}", e))?;
    let pk_hash = PublicKeyHash(hash_bytes(pk_body));
    let pk = PublicKey::<C>::new(dkg_pk.pk.clone());

    let plaintexts_in = ballot_plaintexts::<C, W>(setup_id, tally, ciphertexts)?;
    let encrypted: Vec<Ciphertext<C, W>> = plaintexts_in.iter().map(|p| pk.encrypt(p)).collect();

    let ballots = Ballots::<C, W>::new(encrypted);
    Ok(ProtocolMessage::<C>::ballots(
        pm,
        DATE,
        cfg_hash,
        pk_hash,
        mixing_trustees,
        &ballots,
    ))
}

/// Compare a tally's decrypted plaintexts (`pt_body`) against the set its
/// parameters derive. Returns `(match, expected_count, actual_count)`.
fn plaintexts_match<C: Context, const W: usize>(
    pt_body: &[u8],
    setup_id: &str,
    tally: usize,
    ciphertexts: u32,
) -> Result<(bool, usize, usize)> {
    let expected = expected_plaintexts::<C, W>(setup_id, tally, ciphertexts)?;
    let plaintexts = Plaintexts::<C, W>::deser(pt_body)
        .map_err(|e| anyhow!("deserialize plaintexts: {:?}", e))?;
    let actual: HashSet<Vec<u8>> = plaintexts.0.iter().map(|p| p.ser()).collect();
    Ok((expected == actual, expected.len(), actual.len()))
}

/// Validate the election parameters against the dispatch-macro ranges.
fn validate_params(trustees: usize, threshold: usize, width: usize) -> Result<()> {
    if !(1..=MAX_CIPHERTEXT_WIDTH).contains(&width) {
        return Err(anyhow!(
            "unsupported ciphertext width {width} (expected 1..={MAX_CIPHERTEXT_WIDTH})"
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

/// Where the setup is kept between page loads. One at a time: creating or
/// importing a setup replaces whatever was here.
const STORAGE_KEY: &str = "braid_emulator_setup";

/// A `Setup`'s identity and progress, with no secrets in it.
///
/// Everything else — board contents, the persisted committed sets, the DKG
/// public key — is reconstructable from b4 + IndexedDB given the same `id`, so
/// none of it is recorded here. The ballot plaintexts are not recorded either:
/// they are derived from `id` and the tally index (see [`ballot_plaintexts`]).
#[derive(Serialize, Deserialize, Clone)]
struct Setup {
    id: String,
    trustees: usize,
    threshold: usize,
    width: usize,
    /// Every tally started, in order; the index into this is the child-board
    /// index. Recording them rather than counting them is what lets an earlier
    /// tally be reopened without disturbing where the next one goes.
    tallies: Vec<TallyInfo>,
    /// The tally currently attached, if any. This is what lets a refresh
    /// mid-tally reconnect to that tally instead of stranding it.
    active: Option<usize>,
}

/// What a tally needs beyond its index.
///
/// The ciphertext count is kept rather than read back from the tally's own
/// `Ballots` message: [`verify_plaintexts`](Emulator::verify_plaintexts) derives
/// the expected set from it, and a verifier must not take the size of what it
/// expects from the artifact it is checking.
#[derive(Serialize, Deserialize, Clone, Copy)]
struct TallyInfo {
    ciphertexts: u32,
}

/// A `Setup` plus the secret key material, which is the whole of what has to
/// survive a page load. Stored as base64(JSON) in `localStorage`, and the same
/// bytes are what [`export`](Emulator::export) hands over for moving a setup to
/// another browser.
#[derive(Serialize, Deserialize)]
struct SetupBlob {
    setup: Setup,
    /// Manager signing key (base64).
    manager_sk: String,
    /// Trustee signing keys (base64), in index order.
    trustee_sks: Vec<String>,
    /// Trustee share-decryption secret scalars (base64 `VSerializable`), in index
    /// order; the public side is recomputed as `g^sk`.
    share_sks: Vec<String>,
}

/// The browser's `localStorage`, or an error naming why it is unavailable.
fn storage() -> Result<web_sys::Storage> {
    web_sys::window()
        .ok_or_else(|| anyhow!("no window"))?
        .local_storage()
        .map_err(|e| anyhow!("localStorage unavailable: {e:?}"))?
        .ok_or_else(|| anyhow!("localStorage is disabled"))
}

/// Reconstruct the keys a blob carries. The public side of each share keypair is
/// recomputed rather than stored, so the blob holds only true secrets.
fn keys_from_blob(blob: &SetupBlob) -> Result<Keys> {
    let pm = ProtocolManager::<RistrettoCtx>::new(
        Sig::signer_from_base64_string(&blob.manager_sk)
            .map_err(|e| anyhow!("decode manager key: {e}"))?,
    );
    let mut signing = Vec::with_capacity(blob.trustee_sks.len());
    for s in &blob.trustee_sks {
        signing.push(Sig::signer_from_base64_string(s).map_err(|e| anyhow!("decode trustee key: {e}"))?);
    }
    let mut share = Vec::with_capacity(blob.share_sks.len());
    for s in &blob.share_sks {
        let raw = general_purpose::STANDARD_NO_PAD
            .decode(s)
            .map_err(|e| anyhow!("decode share key: {e}"))?;
        let skey = <RistrettoCtx as Context>::Scalar::deser(&raw)
            .map_err(|e| anyhow!("deserialize share key: {e:?}"))?;
        let pkey = <RistrettoCtx as Context>::G::g_exp(&skey);
        share.push(KeyPair::<RistrettoCtx>::new(skey, pkey));
    }
    Ok(Keys { pm, signing, share })
}

///////////////////////////////////////////////////////////////////////////
// Reports (serialized to JS).
///////////////////////////////////////////////////////////////////////////

/// What one trustee produced in a round (message types it posted).
#[derive(Serialize)]
struct TrusteeActivity {
    trustee: usize,
    produced: Vec<String>,
}

/// One trustee's result from being stepped on its own.
///
/// No round number: a round is every trustee having had a turn, and stepping
/// one is not that. The protocol has no notion of a round either — each trustee
/// runs update-first against whatever the board holds when it looks (§6), so
/// driving them one at a time is if anything the more faithful picture.
#[derive(Serialize)]
struct TrusteeStepReport {
    trustee: usize,
    advanced: bool,
    produced: Vec<String>,
    phase: String,
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

/// One tally, for the tally list.
#[derive(Serialize)]
struct TallyReport {
    index: usize,
    ciphertexts: u32,
    board: String,
    active: bool,
}

/// A snapshot of the board: which board, per-type counts, and the message list.
///
/// The board's *name* carries the rest — `emu_<id>_dkg` against
/// `emu_<id>_tally_<k>` says both which phase and which tally — so none of that
/// is repeated here. Nor is the round: only the whole-committee `step` advances
/// it, so once trustees are also stepped individually it counts something other
/// than how far the protocol has come.
#[derive(Serialize)]
struct StateReport {
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

/// Result of comparing a tally's decrypted plaintexts against the set its
/// parameters derive.
///
/// This is **not** proof verification — no proof is checked here. It answers
/// "did this tally output what was put into it", which is a different question
/// from "are the mixing and decryption proofs sound".
#[derive(Serialize)]
struct PlaintextsReport {
    success: bool,
    expected: usize,
    actual: usize,
}

///////////////////////////////////////////////////////////////////////////
// Emulator
///////////////////////////////////////////////////////////////////////////

/// An interactive in-browser emulator for a single `Setup` (one `Configuration`)
/// against a live b4.
///
/// `create` generates the keys, creates the DKG board on b4, posts the
/// `Configuration` (as manager), and connects one DKG board client per trustee over
/// [`WasmHttpTransport`] with its own IndexedDB store. The protocol is driven a
/// round at a time (`step`); once the DKG reaches its fixpoint, `new_tally`
/// creates a fresh child board (unioned with the DKG parent, §8.2), posts a fresh
/// ciphertext set, and rebuilds the clients to run that tally. `state`/`verify_plaintexts`
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
    keys: Keys,
    /// Only the hash is kept: the `Configuration` itself is posted to the board
    /// and consumed by [`build_trustees`] at construction, and is derivable from
    /// `keys` again should anything need it.
    cfg_hash: ConfigurationHash,
    mixing_trustees: Vec<TrusteeIndex>,
    trustees_n: usize,
    threshold: usize,
    width: usize,
    inner: RefCell<Inner>,
}

struct Inner {
    /// The pure, reusable trustees (§8.2): built once, shared across all phases.
    trustees: Vec<Trustee<RistrettoCtx>>,
    /// The active phase's board clients, trustee `i` paired with client `i`.
    clients: Vec<EmuClient>,
    round: usize,
    /// Every tally started, in order.
    tallies: Vec<TallyInfo>,
    /// Each trustee's committed DKG digests — the union anti-rewrite seed (§8.2),
    /// captured when the DKG completes and reused by every tally.
    seeds: Option<Vec<Vec<Predicate>>>,
    /// The DKG public-key body, captured when the DKG completes.
    pk_body: Option<Vec<u8>>,
    /// Which tally the clients are attached to, if any. There is no separate
    /// phase: the datalog has none either — it derives everything from the
    /// message store — so this is the only thing that says where we are.
    active: Option<usize>,
}

impl Inner {
    /// The phase label for the UI. Derived, not tracked.
    fn phase(&self) -> &'static str {
        if self.active.is_some() {
            "tally"
        } else {
            "dkg"
        }
    }
}

/// Drive trustee `index` one update-first cycle and report the message types it
/// produced.
///
/// Takes `&mut Inner` rather than the `RefCell` so the caller decides how long
/// the borrow is held: a full round holds it across every trustee, a single
/// step across one. Split borrows are why the trustee and its client are reached
/// through separate fields rather than a pair.
async fn drive(inner: &mut Inner, index: usize) -> Result<Vec<String>, JsValue> {
    let trustee = &inner.trustees[index];
    let client = &mut inner.clients[index];

    client.update().await.map_err(js)?;
    let produced = trustee.step(client.view()).map_err(js)?;
    let kinds: Vec<String> = produced
        .iter()
        .map(|m| format!("{:?}", m.message_type))
        .collect();
    client.post(produced).await.map_err(js)?;
    Ok(kinds)
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

    /// Write the current setup to `localStorage`, returning the same bytes so
    /// `export` can hand them over. Called whenever the setup changes, so a
    /// refresh always finds the setup as it stood.
    fn save(&self) -> Result<String> {
        let setup = {
            let inner = self.inner.try_borrow().map_err(|_| anyhow!("emulator is busy"))?;
            Setup {
                id: self.setup_id.clone(),
                trustees: self.trustees_n,
                threshold: self.threshold,
                width: self.width,
                tallies: inner.tallies.clone(),
                active: inner.active,
            }
        };
        let blob = SetupBlob {
            setup,
            manager_sk: Sig::signer_to_base64_string(&self.keys.pm.signing_key)
                .map_err(|e| anyhow!("encode manager key: {e}"))?,
            trustee_sks: self
                .keys
                .signing
                .iter()
                .map(|sk| Sig::signer_to_base64_string(sk))
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(|e| anyhow!("encode trustee key: {e}"))?,
            share_sks: self
                .keys
                .share
                .iter()
                .map(|kp| general_purpose::STANDARD_NO_PAD.encode(kp.skey.ser()))
                .collect(),
        };
        let json = serde_json::to_string(&blob).map_err(|e| anyhow!("serialize setup: {e}"))?;
        let encoded = general_purpose::STANDARD_NO_PAD.encode(json);
        storage()?
            .set_item(STORAGE_KEY, &encoded)
            .map_err(|e| anyhow!("save setup: {e:?}"))?;
        Ok(encoded)
    }

    /// Connect one union client per trustee against tally `tally`: child (tally)
    /// board ∪ parent (DKG) board, each seeded with that trustee's own committed
    /// DKG digests (§8.2). Captures the DKG seeds first if not already held.
    ///
    /// Shared by starting a tally and by reattaching to one after a refresh — the
    /// two differ only in whether the board and its ballots are created first.
    async fn attach_tally(&self, tally: usize) -> Result<(), JsValue> {
        let need_capture = {
            let inner = self.inner.try_borrow().map_err(|_| busy())?;
            inner.pk_body.is_none()
        };
        if need_capture {
            self.capture_dkg().await.map_err(js)?;
        }
        let seeds = {
            let inner = self.inner.try_borrow().map_err(|_| busy())?;
            inner
                .seeds
                .clone()
                .ok_or_else(|| js(anyhow!("no DKG seeds")))?
        };

        let child_board = child_board_name(&self.setup_id, tally);
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

        let mut inner = self.inner.try_borrow_mut().map_err(|_| busy())?;
        inner.clients = clients;
        inner.round = 0;
        inner.active = Some(tally);
        Ok(())
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
    /// Generate the keys, create the DKG board on b4, post the `Configuration`,
    /// and connect one DKG board client per trustee (each with its own IndexedDB store).
    ///
    /// Replaces whatever setup was saved: the emulator holds one at a time.
    pub async fn create(
        b4_url: String,
        trustees: usize,
        threshold: usize,
        width: usize,
    ) -> Result<Emulator, JsValue> {
        validate_params(trustees, threshold, width).map_err(js)?;
        let keys = Keys::generate(trustees);
        let setup = Setup {
            // A stable id so board names and IndexedDB names are deterministic,
            // and so a later restore reconnects to the very same stores.
            id: format!("{:x}", js_sys::Date::now() as u64),
            trustees,
            threshold,
            width,
            tallies: Vec::new(),
            active: None,
        };
        Self::start(b4_url, keys, setup, true).await
    }

    /// Whether a setup is saved and [`restore`](Self::restore) would find one.
    pub fn has_saved() -> bool {
        storage()
            .ok()
            .and_then(|s| s.get_item(STORAGE_KEY).ok().flatten())
            .is_some()
    }

    /// Reconnect to the saved setup, resuming a tally if one was under way.
    ///
    /// This is the ordinary way back after a page refresh — no paste involved.
    /// The keys and `Configuration` come from storage; the board contents, the
    /// committed sets and the DKG public key reload from b4 and IndexedDB.
    pub async fn restore(b4_url: String) -> Result<Emulator, JsValue> {
        let saved = storage()
            .map_err(js)?
            .get_item(STORAGE_KEY)
            .map_err(|e| JsValue::from_str(&format!("read saved setup: {e:?}")))?
            .ok_or_else(|| JsValue::from_str("no saved setup"))?;
        Self::from_blob(b4_url, &saved).await
    }

    /// Forget the saved setup. The boards and IndexedDB stores it named are left
    /// alone on b4 — this only drops the way back to them.
    pub fn forget() -> Result<(), JsValue> {
        storage()
            .map_err(js)?
            .remove_item(STORAGE_KEY)
            .map_err(|e| JsValue::from_str(&format!("clear saved setup: {e:?}")))
    }

    /// The saved setup as a paste string, for moving it to another browser.
    ///
    /// The same bytes storage holds, so this is a copy rather than a separate
    /// mechanism: a refresh needs no export, and [`restore`](Self::restore)
    /// happens without it.
    pub fn export(&self) -> Result<String, JsValue> {
        self.save().map_err(js)
    }

    /// Rebuild a setup from an [`export`](Self::export)ed string and reconnect it
    /// to the same b4 boards + IndexedDB stores, saving it as the current setup.
    pub async fn import(b4_url: String, blob: String) -> Result<Emulator, JsValue> {
        Self::from_blob(b4_url, blob.trim()).await
    }

    /// Rebuild a setup from a stored/exported blob and reconnect it.
    async fn from_blob(b4_url: String, blob: &str) -> Result<Emulator, JsValue> {
        let bytes = general_purpose::STANDARD_NO_PAD
            .decode(blob)
            .map_err(|e| JsValue::from_str(&format!("decode setup: {e}")))?;
        let blob: SetupBlob = serde_json::from_slice(&bytes)
            .map_err(|e| JsValue::from_str(&format!("parse setup: {e}")))?;
        let keys = keys_from_blob(&blob).map_err(js)?;
        Self::start(b4_url, keys, blob.setup, false).await
    }

    /// Shared constructor. When `fresh`, creates the DKG board on b4 and posts the
    /// `Configuration`; otherwise the board is assumed to exist. Then connects the
    /// per-trustee DKG clients, and reconnects an in-progress tally if the setup
    /// records one. Their message stores stay empty until the first `step`
    /// (update-first, §6): `connect` reloads only the persisted committed set (the
    /// anti-rewrite baseline) and the `Configuration`, not the board contents —
    /// those are shown on the global board panel instead.
    async fn start(
        b4_url: String,
        keys: Keys,
        setup: Setup,
        fresh: bool,
    ) -> Result<Emulator, JsValue> {
        let (cfg, cfg_hash) =
            configuration_for(&keys, setup.threshold, setup.width).map_err(js)?;
        let parent_board = parent_board_name(&setup.id);
        if fresh {
            WasmHttpTransport::create_board(&b4_url, &parent_board)
                .await
                .map_err(js)?;
            let cfg_message = ProtocolMessage::<RistrettoCtx>::configuration(&keys.pm, DATE, &cfg);
            let manager = WasmHttpTransport::new(&b4_url, &parent_board);
            Transport::<RistrettoCtx>::post(&manager, vec![cfg_message])
                .await
                .map_err(js)?;
        }

        let trustees = build_trustees(&keys, &cfg).map_err(js)?;
        let emulator = Emulator {
            b4_url,
            setup_id: setup.id.clone(),
            keys,
            cfg_hash,
            mixing_trustees: (1..=setup.threshold).collect(),
            trustees_n: setup.trustees,
            threshold: setup.threshold,
            width: setup.width,
            inner: RefCell::new(Inner {
                trustees,
                clients: Vec::new(),
                round: 0,
                tallies: setup.tallies.clone(),
                active: None,
                seeds: None,
                pk_body: None,
            }),
        };

        let clients = emulator.connect_dkg_clients().await.map_err(js)?;
        emulator.inner.borrow_mut().clients = clients;

        // A tally was under way when the page went away. The DKG clients above
        // have just reloaded their committed sets, so the anti-rewrite seed is
        // available again and the tally's own stores reload with its clients.
        if let Some(tally) = setup.active {
            emulator.attach_tally(tally).await?;
        }

        emulator.save().map_err(js)?;
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
            let phase = inner.phase().to_string();
            let mut advanced = false;
            let mut activity = Vec::with_capacity(inner.clients.len());
            {
                let inner = &mut *inner;
                for i in 0..inner.clients.len() {
                    let kinds = drive(inner, i).await?;
                    if !kinds.is_empty() {
                        advanced = true;
                    }
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

    /// Drive one trustee through update -> step -> post, leaving the others where
    /// they are.
    ///
    /// Trustees are independent: each runs the pure `step` over its own board
    /// client's view, so stepping one produces exactly what it would have
    /// produced in its turn of a full round. What this buys is the interleavings
    /// a lockstep round cannot show — one trustee running ahead, another left
    /// behind, a message observed by some and not yet by others.
    pub async fn step_trustee(&self, index: usize) -> Result<JsValue, JsValue> {
        let (produced, phase) = {
            let mut inner = self.inner.try_borrow_mut().map_err(|_| busy())?;
            if index >= inner.clients.len() {
                return Err(JsValue::from_str("trustee index out of range"));
            }
            let phase = inner.phase().to_string();
            let inner = &mut *inner;
            (drive(inner, index).await?, phase)
        };
        let report = TrusteeStepReport {
            trustee: index + 1,
            advanced: !produced.is_empty(),
            produced,
            phase,
        };
        serde_wasm_bindgen::to_value(&report)
            .map_err(|e| JsValue::from_str(&format!("failed to serialize: {e}")))
    }

    /// Start a new tally over the completed DKG (§8.2): create a fresh child board,
    /// post a fresh ciphertext set (as manager), and reconnect the clients as union
    /// clients (child ∪ DKG parent, seeded with the trustees' own DKG digests).
    /// Requires the DKG to have produced a public key (step it to a fixpoint first).
    pub async fn new_tally(&self, ciphertexts: u32) -> Result<JsValue, JsValue> {
        if ciphertexts == 0 {
            return Err(JsValue::from_str("a tally needs at least one ciphertext"));
        }
        // Capturing the DKG here rather than inside `attach_tally` because the
        // public key is needed to encrypt before any client is rebuilt.
        let need_capture = {
            let inner = self.inner.try_borrow().map_err(|_| busy())?;
            inner.pk_body.is_none()
        };
        if need_capture {
            self.capture_dkg().await.map_err(js)?;
        }
        let (pk_body, tally) = {
            let inner = self.inner.try_borrow().map_err(|_| busy())?;
            (
                inner
                    .pk_body
                    .clone()
                    .ok_or_else(|| js(anyhow!("no DKG public key")))?,
                inner.tallies.len(),
            )
        };

        // A fresh child board, and this tally's derived ciphertext set.
        let child_board = child_board_name(&self.setup_id, tally);
        WasmHttpTransport::create_board(&self.b4_url, &child_board)
            .await
            .map_err(js)?;
        let ballots_message = crate::dispatch_ciphertext_width!(self.width, {
            encrypt_ballots::<RistrettoCtx, W>(
                &pk_body,
                &self.setup_id,
                tally,
                ciphertexts,
                self.mixing_trustees.clone(),
                &self.keys.pm,
                self.cfg_hash,
            )
        })
        .map_err(js)?;
        let manager = WasmHttpTransport::new(&self.b4_url, &child_board);
        Transport::<RistrettoCtx>::post(&manager, vec![ballots_message])
            .await
            .map_err(js)?;

        self.inner
            .try_borrow_mut()
            .map_err(|_| busy())?
            .tallies
            .push(TallyInfo { ciphertexts });
        self.attach_tally(tally).await?;
        self.save().map_err(js)?;
        self.state().await
    }

    /// Reattach the trustees to a tally already started, leaving its board and
    /// per-trustee stores as they are.
    ///
    /// The child board and each trustee's IndexedDB store outlive the clients
    /// pointing at them, so returning to an earlier tally is a reconnect rather
    /// than a rebuild — it resumes where that tally was left, and does not
    /// disturb where the next new one goes.
    pub async fn open_tally(&self, index: usize) -> Result<JsValue, JsValue> {
        let count = self.inner.try_borrow().map_err(|_| busy())?.tallies.len();
        if index >= count {
            return Err(JsValue::from_str(&format!(
                "no tally {index} (started {count})"
            )));
        }
        self.attach_tally(index).await?;
        self.save().map_err(js)?;
        self.state().await
    }

    /// Every tally started: index, ciphertext count, board, and which is
    /// attached.
    pub fn tallies(&self) -> Result<JsValue, JsValue> {
        let inner = self.inner.try_borrow().map_err(|_| busy())?;
        let list: Vec<TallyReport> = inner
            .tallies
            .iter()
            .enumerate()
            .map(|(i, info)| TallyReport {
                index: i,
                ciphertexts: info.ciphertexts,
                board: child_board_name(&self.setup_id, i),
                active: inner.active == Some(i),
            })
            .collect();
        serde_wasm_bindgen::to_value(&list)
            .map_err(|e| JsValue::from_str(&format!("failed to serialize: {e}")))
    }

    /// A snapshot of the current board's contents by message type (fetched from b4):
    /// the DKG parent while in the DKG phase, or the active tally's child board.
    pub async fn state(&self) -> Result<JsValue, JsValue> {
        let (board_name, is_child) = {
            let inner = self.inner.try_borrow().map_err(|_| busy())?;
            match inner.active {
                Some(tally) => (child_board_name(&self.setup_id, tally), true),
                None => (parent_board_name(&self.setup_id), false),
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

    /// Compare the current tally's decrypted plaintexts with the set its
    /// parameters derive.
    ///
    /// Deliberately not called `verify`: this checks an *outcome*, not a proof.
    /// The trustees' proofs are checked by each other during the protocol; this
    /// asks only whether the plaintexts that came out are the ones that went in.
    pub async fn verify_plaintexts(&self) -> Result<JsValue, JsValue> {
        let (tally, ciphertexts, board_name) = {
            let inner = self.inner.try_borrow().map_err(|_| busy())?;
            let tally = inner
                .active
                .ok_or_else(|| JsValue::from_str("no tally is open"))?;
            let info = inner
                .tallies
                .get(tally)
                .ok_or_else(|| JsValue::from_str("the open tally has no record"))?;
            (
                tally,
                info.ciphertexts,
                child_board_name(&self.setup_id, tally),
            )
        };
        let transport = WasmHttpTransport::new(&self.b4_url, &board_name);
        let messages = Transport::<RistrettoCtx>::fetch(&transport)
            .await
            .map_err(js)?;
        let pt_body = find_body(&messages, MessageType::Plaintexts).ok_or_else(|| {
            JsValue::from_str("no plaintexts on the board yet (finish the tally first)")
        })?;
        let (success, expected, actual) = crate::dispatch_ciphertext_width!(self.width, {
            plaintexts_match::<RistrettoCtx, W>(pt_body, &self.setup_id, tally, ciphertexts)
        })
        .map_err(js)?;
        let report = PlaintextsReport {
            success,
            expected,
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
