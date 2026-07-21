// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! In-browser mixnet emulator (M3-C): run the full v0.6 protocol — DKG →
//! encrypt → mix → threshold-decrypt — entirely in the browser over an in-memory
//! board.
//!
//! This is the wasm counterpart of `native::test::protocol_test_memory`: all
//! trustees share one [`MemoryBoard`] (no b4), each drives the update-first cycle
//! (§6), and the manager posts a single `Ballots` set. It proves the whole v0.6
//! core (pure `SessionTrustee` + datalog + action-layer crypto + board client)
//! runs under `wasm32`.
//!
//! Two entry points:
//! - [`run_in_memory`] — one-shot: run the whole protocol and return the outcome
//!   (step (i)).
//! - [`Emulator`] — interactive: `create` / `step` / `post_ballots` / `state` /
//!   `verify` for round-by-round driving from a page (step (ii)).
//!
//! Persistence is [`NoOpPersistence`] here (the IndexedDB backend has its own
//! test); wiring it into the emulator is the next step.

use wasm_bindgen::prelude::*;

use anyhow::{anyhow, Result};
use serde::Serialize;
use std::collections::HashSet;
use std::marker::PhantomData;
use std::sync::Arc;

use cryptography::context::{Context, RistrettoCtx};
use cryptography::cryptosystem::elgamal::{Ciphertext, KeyPair, PublicKey};
use cryptography::traits::groups::CryptographicGroup;
use cryptography::utils::serialization::{VDeserializable, VSerializable};
use cryptography::utils::signatures::SignatureScheme;

use b4::messages::artifact::{Ballots, Configuration, DkgPublicKey, Plaintexts};
use b4::messages::newtypes::{ConfigurationHash, PublicKeyHash, Timestamp, TrusteeIndex};
use b4::messages::protocol_manager::ProtocolManager;
use b4::messages::wire::{MessageType, WireMessage};

use crate::board::persistence::{IndexedDbPersistence, NoOpPersistence, Persistence};
use crate::board::transport::{MemoryBoard, MemoryTransport};
use crate::board::BoardClient;
use crate::runtime::SessionTrustee;
use crate::session::Session;

/// Wire `date` for every emulator message (§3.1 — timestamps are wire-only).
const DATE: Timestamp = 0;

/// Safety cap on driver rounds; a healthy run converges in a handful of passes.
const MAX_ROUNDS: usize = 200;

/// The maximum committee size the dispatch macros monomorphize for.
const MAX_TRUSTEES: usize = 8;

/// A concrete emulator session: in-memory board, IndexedDB-backed persistence
/// (each trustee gets its own browser store).
type EmuSession = Session<RistrettoCtx, MemoryTransport<RistrettoCtx>, IndexedDbPersistence>;

///////////////////////////////////////////////////////////////////////////
// Shared setup / driver helpers (generic over the context to avoid the
// ambiguous-associated-type issue with a concrete `RistrettoCtx`).
///////////////////////////////////////////////////////////////////////////

/// The pieces produced by [`build_sessions`].
struct Setup<C: Context, P: Persistence> {
    board: Arc<MemoryBoard<C>>,
    pm: ProtocolManager<C>,
    cfg_hash: ConfigurationHash,
    sessions: Vec<Session<C, MemoryTransport<C>, P>>,
}

/// Generate key material + configuration, seed a shared board with the
/// `Configuration`, and connect one session per trustee. `persistences` supplies
/// one backend per trustee (length must equal `n_trustees`).
async fn build_sessions<C: Context, P: Persistence>(
    n_trustees: usize,
    n_threshold: usize,
    width: usize,
    persistences: Vec<P>,
) -> Result<Setup<C, P>> {
    if persistences.len() != n_trustees {
        return Err(anyhow!(
            "expected {} persistence backends, got {}",
            n_trustees,
            persistences.len()
        ));
    }
    let mut key_rng = C::get_rng();
    let pm = ProtocolManager::<C>::new(C::SignatureScheme::gen_signing_key(&mut key_rng));

    let mut signing_keys = Vec::with_capacity(n_trustees);
    let mut trustee_vks = Vec::with_capacity(n_trustees);
    let mut share_keypairs = Vec::with_capacity(n_trustees);
    let mut share_enc_keys = Vec::with_capacity(n_trustees);
    for _ in 0..n_trustees {
        let sk = C::SignatureScheme::gen_signing_key(&mut key_rng);
        trustee_vks.push(C::SignatureScheme::verifying_key(&sk));
        signing_keys.push(sk);
        let keypair = KeyPair::<C>::generate();
        share_enc_keys.push(keypair.pkey.y.clone());
        share_keypairs.push(keypair);
    }

    let cfg = Configuration::<C>::new(
        0,
        C::SignatureScheme::verifying_key(&pm.signing_key),
        trustee_vks,
        n_threshold,
        width,
        PhantomData,
    )
    .with_share_encryption_keys(share_enc_keys);
    let cfg_hash = ConfigurationHash::from_configuration(&cfg)?;
    let cfg_message = WireMessage::<C>::configuration(&pm, DATE, &cfg);

    let board = MemoryBoard::<C>::new();
    board.push(cfg_message);

    let mut sessions = Vec::with_capacity(n_trustees);
    for (i, ((signing_key, keypair), persistence)) in signing_keys
        .into_iter()
        .zip(share_keypairs)
        .zip(persistences)
        .enumerate()
    {
        let transport = MemoryTransport::new(board.clone());
        let client = BoardClient::connect(transport, persistence).await?;
        let trustee = SessionTrustee::new(
            (i + 1).to_string(),
            signing_key,
            keypair,
            client.configuration(),
        )?;
        sessions.push(Session::new(trustee, client));
    }

    Ok(Setup {
        board,
        pm,
        cfg_hash,
        sessions,
    })
}

/// One update-first round across all sessions (§6). Returns whether any trustee
/// produced a message.
async fn advance_round<C: Context, P: Persistence>(
    sessions: &mut [Session<C, MemoryTransport<C>, P>],
) -> Result<bool> {
    let mut produced_any = false;
    for session in sessions.iter_mut() {
        if session.advance().await? {
            produced_any = true;
        }
    }
    Ok(produced_any)
}

/// Drive to a fixpoint; returns the number of rounds taken.
async fn drive_to_fixpoint<C: Context, P: Persistence>(
    sessions: &mut [Session<C, MemoryTransport<C>, P>],
) -> Result<usize> {
    for round in 0..MAX_ROUNDS {
        if !advance_round(sessions).await? {
            return Ok(round);
        }
    }
    Err(anyhow!(
        "protocol did not reach a fixpoint within {} rounds",
        MAX_ROUNDS
    ))
}

/// The manager encrypts `ciphertexts` random plaintexts under the DKG public key
/// (read off `board`) and posts a `Ballots` set for `mixing_trustees`. Returns the
/// set of expected plaintexts as their serialized bytes (for later verification).
fn post_ballots_inner<C: Context, const W: usize>(
    board: &MemoryBoard<C>,
    pm: &ProtocolManager<C>,
    cfg_hash: ConfigurationHash,
    mixing_trustees: Vec<TrusteeIndex>,
    ciphertexts: u32,
) -> Result<HashSet<Vec<u8>>> {
    let messages = board.snapshot();
    let pk_body = messages
        .iter()
        .find(|m| m.message_type == MessageType::PublicKey)
        .and_then(|m| m.body.as_ref())
        .ok_or_else(|| anyhow!("no public key on the board yet (run DKG to a fixpoint first)"))?;
    let dkg_pk = DkgPublicKey::<C>::deser(pk_body)
        .map_err(|e| anyhow!("failed to deserialize public key: {:?}", e))?;
    let pk_hash = PublicKeyHash(b4::hash_bytes(pk_body));

    let pk = PublicKey::<C>::new(dkg_pk.pk.clone());
    let mut enc_rng = C::get_rng();
    let plaintexts_in: Vec<[C::Element; W]> = (0..ciphertexts)
        .map(|_| std::array::from_fn(|_| C::G::random_element(&mut enc_rng)))
        .collect();
    let encrypted: Vec<Ciphertext<C, W>> = plaintexts_in.iter().map(|p| pk.encrypt(p)).collect();
    let expected: HashSet<Vec<u8>> = plaintexts_in.iter().map(|p| p.ser()).collect();

    let ballots = Ballots::<C, W>::new(encrypted);
    let ballots_message =
        WireMessage::<C>::ballots(pm, DATE, cfg_hash, pk_hash, mixing_trustees, &ballots);
    board.push(ballots_message);

    Ok(expected)
}

/// Compare the decrypted plaintexts on `board` against the `expected` set.
fn verify_inner<C: Context, const W: usize>(
    board: &MemoryBoard<C>,
    expected: &HashSet<Vec<u8>>,
) -> Result<(bool, usize)> {
    let messages = board.snapshot();
    let pt_body = messages
        .iter()
        .find(|m| m.message_type == MessageType::Plaintexts)
        .and_then(|m| m.body.as_ref())
        .ok_or_else(|| anyhow!("no plaintexts on the board yet (finish the tally first)"))?;
    let plaintexts = Plaintexts::<C, W>::deser(pt_body)
        .map_err(|e| anyhow!("failed to deserialize plaintexts: {:?}", e))?;
    let actual: HashSet<Vec<u8>> = plaintexts.0.iter().map(|p| p.ser()).collect();
    let count = actual.len();
    Ok((*expected == actual, count))
}

/// Validate the committee parameters against the dispatch-macro ranges.
fn validate_params(
    trustees: usize,
    threshold: usize,
    width: usize,
) -> std::result::Result<(), String> {
    if !(1..=MAX_TRUSTEES).contains(&width) {
        return Err(format!(
            "unsupported ciphertext width {width} (expected 1..={MAX_TRUSTEES})"
        ));
    }
    if !(2..=MAX_TRUSTEES).contains(&trustees) {
        return Err(format!(
            "unsupported trustee count {trustees} (expected 2..={MAX_TRUSTEES})"
        ));
    }
    if !(2..=trustees).contains(&threshold) {
        return Err(format!(
            "unsupported threshold {threshold} (expected 2..={trustees})"
        ));
    }
    Ok(())
}

///////////////////////////////////////////////////////////////////////////
// One-shot entry point (step i)
///////////////////////////////////////////////////////////////////////////

/// The outcome of a one-shot emulator run, returned to JS.
#[derive(Serialize)]
struct EmulatorResult {
    success: bool,
    trustees: usize,
    threshold: usize,
    ciphertexts: u32,
    width: usize,
    dkg_rounds: usize,
    tally_rounds: usize,
    message: String,
}

/// Run the full in-memory protocol in the browser and return the outcome.
#[wasm_bindgen]
pub async fn run_in_memory(
    trustees: usize,
    threshold: usize,
    ciphertexts: u32,
    width: usize,
) -> Result<JsValue, JsValue> {
    validate_params(trustees, threshold, width).map_err(|e| JsValue::from_str(&e))?;

    let outcome = crate::dispatch_ciphertext_width!(width, {
        run_inner::<RistrettoCtx, W>(trustees, threshold, ciphertexts).await
    });

    match outcome {
        Ok(result) => serde_wasm_bindgen::to_value(&result)
            .map_err(|e| JsValue::from_str(&format!("failed to serialize result: {e}"))),
        Err(e) => Err(JsValue::from_str(&format!("{e:#}"))),
    }
}

async fn run_inner<C: Context, const W: usize>(
    n_trustees: usize,
    n_threshold: usize,
    ciphertexts: u32,
) -> Result<EmulatorResult> {
    // The one-shot run does not persist; a fresh NoOp backend per trustee.
    let persistences: Vec<NoOpPersistence> = (0..n_trustees).map(|_| NoOpPersistence).collect();
    let Setup {
        board,
        pm,
        cfg_hash,
        mut sessions,
    } = build_sessions::<C, NoOpPersistence>(n_trustees, n_threshold, W, persistences).await?;

    let dkg_rounds = drive_to_fixpoint(&mut sessions).await?;

    let mixing_trustees: Vec<TrusteeIndex> = (1..=n_threshold).collect();
    let expected = post_ballots_inner::<C, W>(&board, &pm, cfg_hash, mixing_trustees, ciphertexts)?;

    let tally_rounds = drive_to_fixpoint(&mut sessions).await?;

    let (success, _) = verify_inner::<C, W>(&board, &expected)?;

    Ok(EmulatorResult {
        success,
        trustees: n_trustees,
        threshold: n_threshold,
        ciphertexts,
        width: W,
        dkg_rounds,
        tally_rounds,
        message: if success {
            "decrypted plaintexts match the encrypted inputs".to_string()
        } else {
            "MISMATCH: decrypted plaintexts do not match the inputs".to_string()
        },
    })
}

///////////////////////////////////////////////////////////////////////////
// Interactive emulator (step ii)
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

/// One round's result.
#[derive(Serialize)]
struct StepReport {
    advanced: bool,
    round: usize,
    phase: String,
    board_messages: usize,
}

/// A snapshot of the board contents by message type.
#[derive(Serialize)]
struct StateReport {
    phase: String,
    round: usize,
    configuration: usize,
    shares: usize,
    public_key: usize,
    ballots: usize,
    mix: usize,
    mix_signature: usize,
    partial_decryptions: usize,
    plaintexts: usize,
}

/// Result of a verification.
#[derive(Serialize)]
struct VerifyReport {
    success: bool,
    expected: usize,
    actual: usize,
}

/// An interactive in-browser emulator: create a committee, then drive the
/// protocol one round at a time and inspect the board between rounds.
///
/// Fixed to [`RistrettoCtx`]; the ciphertext width is chosen at [`create`] and
/// dispatched at runtime, so the struct itself is width-agnostic. Mutable state
/// lives behind a `RefCell` so the exported methods can take `&self` (wasm-bindgen
/// does not allow `&mut self` on async methods); the driving page steps
/// sequentially, so there is no re-entrancy.
#[wasm_bindgen]
pub struct Emulator {
    board: Arc<MemoryBoard<RistrettoCtx>>,
    pm: ProtocolManager<RistrettoCtx>,
    cfg_hash: ConfigurationHash,
    mixing_trustees: Vec<TrusteeIndex>,
    width: usize,
    ciphertexts: u32,
    inner: std::cell::RefCell<Inner>,
}

struct Inner {
    sessions: Vec<EmuSession>,
    expected: Option<HashSet<Vec<u8>>>,
    phase: Phase,
    round: usize,
}

#[wasm_bindgen]
impl Emulator {
    /// Set up a committee and seed the board with the `Configuration`.
    pub async fn create(
        trustees: usize,
        threshold: usize,
        ciphertexts: u32,
        width: usize,
    ) -> Result<Emulator, JsValue> {
        validate_params(trustees, threshold, width).map_err(|e| JsValue::from_str(&e))?;

        // Each trustee gets its own IndexedDB store. A fresh, uniquely-named DB
        // per `create` keeps repeated runs isolated (the anti-rewrite demo will
        // later reuse names to simulate a restart).
        let run_id = js_sys::Date::now() as u64;
        let mut persistences = Vec::with_capacity(trustees);
        for i in 0..trustees {
            let name = format!("braid_emu_{run_id}_trustee_{i}");
            let persistence = IndexedDbPersistence::open(&name)
                .await
                .map_err(|e| JsValue::from_str(&format!("failed to open IndexedDB: {e:#}")))?;
            persistences.push(persistence);
        }

        let setup = build_sessions::<RistrettoCtx, IndexedDbPersistence>(
            trustees,
            threshold,
            width,
            persistences,
        )
        .await
        .map_err(|e| JsValue::from_str(&format!("{e:#}")))?;

        Ok(Emulator {
            board: setup.board,
            pm: setup.pm,
            cfg_hash: setup.cfg_hash,
            mixing_trustees: (1..=threshold).collect(),
            width,
            ciphertexts,
            inner: std::cell::RefCell::new(Inner {
                sessions: setup.sessions,
                expected: None,
                phase: Phase::Dkg,
                round: 0,
            }),
        })
    }

    /// Advance every trustee one update-first round. Returns whether the round
    /// produced anything (false ⇒ this phase has reached its fixpoint).
    pub async fn step(&self) -> Result<JsValue, JsValue> {
        // `persist()` (IndexedDB) does real async I/O, so this borrow is held
        // across an await. The driving page steps sequentially and disables its
        // controls while a step runs; `try_borrow_mut` turns any accidental
        // re-entrancy into a clean error instead of a panic.
        let advanced = {
            let mut inner = self
                .inner
                .try_borrow_mut()
                .map_err(|_| JsValue::from_str("emulator is busy (a step is already running)"))?;
            let advanced = advance_round(&mut inner.sessions)
                .await
                .map_err(|e| JsValue::from_str(&format!("{e:#}")))?;
            inner.round += 1;
            advanced
        };
        let inner = self.inner.borrow();
        let report = StepReport {
            advanced,
            round: inner.round,
            phase: inner.phase.as_str().to_string(),
            board_messages: self.board.snapshot().len(),
        };
        serde_wasm_bindgen::to_value(&report)
            .map_err(|e| JsValue::from_str(&format!("failed to serialize: {e}")))
    }

    /// The manager posts the ballots, moving the emulator into the tally phase.
    /// Requires the DKG to have produced a public key (step to its fixpoint first).
    pub fn post_ballots(&self) -> Result<JsValue, JsValue> {
        let expected = crate::dispatch_ciphertext_width!(self.width, {
            post_ballots_inner::<RistrettoCtx, W>(
                &self.board,
                &self.pm,
                self.cfg_hash,
                self.mixing_trustees.clone(),
                self.ciphertexts,
            )
        })
        .map_err(|e| JsValue::from_str(&format!("{e:#}")))?;

        {
            let mut inner = self.inner.borrow_mut();
            inner.expected = Some(expected);
            inner.phase = Phase::Tally;
        }
        self.state()
    }

    /// A snapshot of the board contents by message type.
    pub fn state(&self) -> Result<JsValue, JsValue> {
        let inner = self.inner.borrow();
        let messages = self.board.snapshot();
        let count = |t: MessageType| messages.iter().filter(|m| m.message_type == t).count();
        let report = StateReport {
            phase: inner.phase.as_str().to_string(),
            round: inner.round,
            configuration: count(MessageType::Configuration),
            shares: count(MessageType::Shares),
            public_key: count(MessageType::PublicKey),
            ballots: count(MessageType::Ballots),
            mix: count(MessageType::Mix),
            mix_signature: count(MessageType::MixSignature),
            partial_decryptions: count(MessageType::PartialDecryptions),
            plaintexts: count(MessageType::Plaintexts),
        };
        serde_wasm_bindgen::to_value(&report)
            .map_err(|e| JsValue::from_str(&format!("failed to serialize: {e}")))
    }

    /// Compare the decrypted plaintexts on the board with the encrypted inputs.
    pub fn verify(&self) -> Result<JsValue, JsValue> {
        let inner = self.inner.borrow();
        let expected = inner
            .expected
            .as_ref()
            .ok_or_else(|| JsValue::from_str("no ballots posted yet"))?;
        let (success, actual) = crate::dispatch_ciphertext_width!(self.width, {
            verify_inner::<RistrettoCtx, W>(&self.board, expected)
        })
        .map_err(|e| JsValue::from_str(&format!("{e:#}")))?;
        let report = VerifyReport {
            success,
            expected: expected.len(),
            actual,
        };
        serde_wasm_bindgen::to_value(&report)
            .map_err(|e| JsValue::from_str(&format!("failed to serialize: {e}")))
    }
}
