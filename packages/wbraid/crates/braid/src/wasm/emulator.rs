// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! In-browser mixnet emulator (M3-C).
//!
//! The emulator is **interactive** and runs against a **live b4** (over HTTP+S3
//! via [`WasmHttpTransport`], with per-trustee IndexedDB persistence) — the
//! production-shaped setting. It lets a page drive the protocol one round at a
//! time and inspect what the board and each trustee hold between rounds.
//!
//! One-shot pass/fail runs belong in tests, not the emulator. Coverage: the
//! protocol logic is covered natively by `protocol_test_memory` /
//! `protocol_test_http`; the wasm build + serialization + async I/O by the
//! headless IndexedDB test (`tests/wasm_indexeddb.rs`); and the protocol running
//! correctly *under wasm* end-to-end by this emulator in a real browser. (A
//! headless wasm *protocol* test is infeasible: the crypto needs the rayon thread
//! pool, which needs the atomics/shared-memory build, whose async test executor
//! needs SharedArrayBuffer/COOP that the wasm-bindgen test runner cannot provide.)

use wasm_bindgen::prelude::*;

use anyhow::{anyhow, Result};
use serde::Serialize;
use std::cell::RefCell;
use std::collections::HashSet;
use std::marker::PhantomData;

use cryptography::context::{Context, RistrettoCtx};
use cryptography::cryptosystem::elgamal::{Ciphertext, KeyPair, PublicKey};
use cryptography::traits::groups::CryptographicGroup;
use cryptography::utils::serialization::{VDeserializable, VSerializable};
use cryptography::utils::signatures::SignatureScheme;

use b4::messages::artifact::{Ballots, Configuration, DkgPublicKey, Plaintexts};
use b4::messages::newtypes::{ConfigurationHash, PublicKeyHash, Timestamp, TrusteeIndex};
use b4::messages::protocol_manager::ProtocolManager;
use b4::messages::wire::{MessageType, WireMessage};

use crate::board::persistence::IndexedDbPersistence;
use crate::board::transport::Transport;
use crate::board::wasm_transport::WasmHttpTransport;
use crate::board::BoardClient;
use crate::runtime::SessionTrustee;
use crate::session::Session;

/// Wire `date` for every emulator message (§3.1 — timestamps are wire-only).
const DATE: Timestamp = 0;

/// The maximum committee size the dispatch macros monomorphize for.
const MAX_TRUSTEES: usize = 8;

///////////////////////////////////////////////////////////////////////////
// Shared helpers (generic over the context to avoid the ambiguous
// associated-type issue with a concrete `RistrettoCtx`).
///////////////////////////////////////////////////////////////////////////

/// Freshly generated committee key material + configuration.
struct Committee<C: Context> {
    pm: ProtocolManager<C>,
    signing_keys: Vec<<C::SignatureScheme as SignatureScheme<C::Rng>>::Signer>,
    share_keypairs: Vec<KeyPair<C>>,
    cfg: Configuration<C>,
    cfg_hash: ConfigurationHash,
}

/// Generate the manager + `n_trustees` key pairs and the shared `Configuration`.
fn generate_committee<C: Context>(
    n_trustees: usize,
    n_threshold: usize,
    width: usize,
) -> Result<Committee<C>> {
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

    Ok(Committee {
        pm,
        signing_keys,
        share_keypairs,
        cfg,
        cfg_hash,
    })
}

/// The body of the first message of `kind` in `messages`, if any.
fn find_body<C: Context>(messages: &[WireMessage<C>], kind: MessageType) -> Option<&Vec<u8>> {
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
) -> Result<(WireMessage<C>, HashSet<Vec<u8>>)> {
    let dkg_pk = DkgPublicKey::<C>::deser(pk_body)
        .map_err(|e| anyhow!("deserialize public key: {:?}", e))?;
    let pk_hash = PublicKeyHash(b4::hash_bytes(pk_body));
    let pk = PublicKey::<C>::new(dkg_pk.pk.clone());

    let mut enc_rng = C::get_rng();
    let plaintexts_in: Vec<[C::Element; W]> = (0..ciphertexts)
        .map(|_| std::array::from_fn(|_| C::G::random_element(&mut enc_rng)))
        .collect();
    let encrypted: Vec<Ciphertext<C, W>> = plaintexts_in.iter().map(|p| pk.encrypt(p)).collect();
    let expected: HashSet<Vec<u8>> = plaintexts_in.iter().map(|p| p.ser()).collect();

    let ballots = Ballots::<C, W>::new(encrypted);
    let message = WireMessage::<C>::ballots(pm, DATE, cfg_hash, pk_hash, mixing_trustees, &ballots);
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
// Interactive emulator (against a live b4)
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

/// An interactive in-browser emulator running against a live b4.
///
/// `create` generates a committee, creates a fresh board on b4, posts the
/// `Configuration` (as manager), and connects one session per trustee over
/// [`WasmHttpTransport`] with its own IndexedDB store. The protocol is then driven
/// a round at a time (`step`), the manager posts ballots (`post_ballots`), and the
/// board/outcome are inspected (`state`, `verify`). All methods do real HTTP, so
/// they are async; mutable state sits behind a `RefCell` so they can take `&self`
/// (wasm-bindgen disallows `&mut self` on async), and the driving page disables
/// its controls while an op runs so there is no re-entrancy.
#[wasm_bindgen]
pub struct Emulator {
    manager: WasmHttpTransport,
    pm: ProtocolManager<RistrettoCtx>,
    cfg_hash: ConfigurationHash,
    mixing_trustees: Vec<TrusteeIndex>,
    width: usize,
    ciphertexts: u32,
    inner: RefCell<Inner>,
}

struct Inner {
    sessions: Vec<Session<RistrettoCtx, WasmHttpTransport, IndexedDbPersistence>>,
    expected: Option<HashSet<Vec<u8>>>,
    phase: Phase,
    round: usize,
}

/// Map an `anyhow::Error` to a JS error.
fn js(e: anyhow::Error) -> JsValue {
    JsValue::from_str(&format!("{e:#}"))
}

/// The busy error (a `RefCell` borrow failed — a concurrent op is running).
fn busy() -> JsValue {
    JsValue::from_str("emulator is busy (an operation is already running)")
}

#[wasm_bindgen]
impl Emulator {
    /// Create a committee, create a fresh board on b4, post the `Configuration`,
    /// and connect one session per trustee (each with its own IndexedDB store).
    pub async fn create(
        b4_url: String,
        trustees: usize,
        threshold: usize,
        ciphertexts: u32,
        width: usize,
    ) -> Result<Emulator, JsValue> {
        validate_params(trustees, threshold, width).map_err(js)?;

        let committee =
            generate_committee::<RistrettoCtx>(trustees, threshold, width).map_err(js)?;
        let cfg_hash = committee.cfg_hash;
        let cfg_message =
            WireMessage::<RistrettoCtx>::configuration(&committee.pm, DATE, &committee.cfg);

        // A fresh board per run so re-runs never collide on b4's persistent store.
        let board_name = format!("emu_{}", js_sys::Date::now() as u64);
        WasmHttpTransport::create_board(&b4_url, &board_name)
            .await
            .map_err(js)?;
        let manager = WasmHttpTransport::new(&b4_url, &board_name);
        Transport::<RistrettoCtx>::post(&manager, vec![cfg_message])
            .await
            .map_err(js)?;

        let mut sessions = Vec::with_capacity(trustees);
        for (i, (signing_key, keypair)) in committee
            .signing_keys
            .into_iter()
            .zip(committee.share_keypairs)
            .enumerate()
        {
            let transport = WasmHttpTransport::new(&b4_url, &board_name);
            let db_name = format!("{board_name}_trustee_{i}");
            let persistence = IndexedDbPersistence::open(&db_name)
                .await
                .map_err(|e| JsValue::from_str(&format!("failed to open IndexedDB: {e:#}")))?;
            let client = BoardClient::connect(transport, persistence)
                .await
                .map_err(js)?;
            let trustee = SessionTrustee::new(
                (i + 1).to_string(),
                signing_key,
                keypair,
                client.configuration(),
            )
            .map_err(js)?;
            sessions.push(Session::new(trustee, client));
        }

        Ok(Emulator {
            manager,
            pm: committee.pm,
            cfg_hash,
            mixing_trustees: (1..=threshold).collect(),
            width,
            ciphertexts,
            inner: RefCell::new(Inner {
                sessions,
                expected: None,
                phase: Phase::Dkg,
                round: 0,
            }),
        })
    }

    /// Advance every trustee one update-first round. Returns whether the round
    /// produced anything (false ⇒ this phase has reached its fixpoint).
    pub async fn step(&self) -> Result<JsValue, JsValue> {
        // The board client does real HTTP here, so this borrow is held across an
        // await. The driving page steps sequentially and disables its controls
        // while an op runs; `try_borrow_mut` turns any accidental re-entrancy into
        // a clean error instead of a panic. Each trustee is driven update -> step
        // -> post so we can report what it produced this round.
        let (advanced, round, phase, activity) = {
            let mut inner = self.inner.try_borrow_mut().map_err(|_| busy())?;
            let mut advanced = false;
            let mut activity = Vec::with_capacity(inner.sessions.len());
            for (i, session) in inner.sessions.iter_mut().enumerate() {
                session.update().await.map_err(js)?;
                let produced = session.step().map_err(js)?;
                let kinds: Vec<String> = produced
                    .iter()
                    .map(|m| format!("{:?}", m.message_type))
                    .collect();
                if !produced.is_empty() {
                    advanced = true;
                }
                session.post(produced).await.map_err(js)?;
                activity.push(TrusteeActivity {
                    trustee: i + 1,
                    produced: kinds,
                });
            }
            inner.round += 1;
            (
                advanced,
                inner.round,
                inner.phase.as_str().to_string(),
                activity,
            )
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

    /// The manager posts the ballots, moving the emulator into the tally phase.
    /// Requires the DKG to have produced a public key (step to its fixpoint first).
    pub async fn post_ballots(&self) -> Result<JsValue, JsValue> {
        let messages = Transport::<RistrettoCtx>::fetch(&self.manager)
            .await
            .map_err(js)?;
        let pk_body = find_body(&messages, MessageType::PublicKey).ok_or_else(|| {
            JsValue::from_str("no public key on the board yet (step the DKG to a fixpoint first)")
        })?;
        let (ballots_message, expected) = crate::dispatch_ciphertext_width!(self.width, {
            encrypt_ballots::<RistrettoCtx, W>(
                pk_body,
                self.ciphertexts,
                self.mixing_trustees.clone(),
                &self.pm,
                self.cfg_hash,
            )
        })
        .map_err(js)?;

        Transport::<RistrettoCtx>::post(&self.manager, vec![ballots_message])
            .await
            .map_err(js)?;

        {
            let mut inner = self.inner.try_borrow_mut().map_err(|_| busy())?;
            inner.expected = Some(expected);
            inner.phase = Phase::Tally;
        }
        self.state().await
    }

    /// A snapshot of the board contents by message type (fetched from b4).
    pub async fn state(&self) -> Result<JsValue, JsValue> {
        let (phase, round) = {
            let inner = self.inner.try_borrow().map_err(|_| busy())?;
            (inner.phase.as_str().to_string(), inner.round)
        };
        let messages = Transport::<RistrettoCtx>::fetch(&self.manager)
            .await
            .map_err(js)?;
        let count = |t: MessageType| messages.iter().filter(|m| m.message_type == t).count();
        let list: Vec<MessageSummary> = messages
            .iter()
            .map(|m| MessageSummary {
                kind: format!("{:?}", m.message_type),
                sender: m.sender.name.clone(),
                digest: match &m.body {
                    Some(body) => hex::encode(&b4::hash_bytes(body)[..])
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
            // The Configuration is posted at construction; `fetch` excludes it.
            configuration: 1,
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

    /// Compare the decrypted plaintexts on the board with the encrypted inputs.
    pub async fn verify(&self) -> Result<JsValue, JsValue> {
        let expected = {
            let inner = self.inner.try_borrow().map_err(|_| busy())?;
            inner
                .expected
                .clone()
                .ok_or_else(|| JsValue::from_str("no ballots posted yet"))?
        };
        let messages = Transport::<RistrettoCtx>::fetch(&self.manager)
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

    /// The number of trustees (for the page's trustee selector).
    pub fn trustee_count(&self) -> usize {
        self.inner
            .try_borrow()
            .map(|i| i.sessions.len())
            .unwrap_or(0)
    }

    /// The message-store predicates the given trustee (0-based) currently holds
    /// — the datalog EDB it runs on (§6.1). Returned as readable `Debug` strings.
    pub fn trustee_predicates(&self, index: usize) -> Result<JsValue, JsValue> {
        let inner = self.inner.try_borrow().map_err(|_| busy())?;
        let session = inner
            .sessions
            .get(index)
            .ok_or_else(|| JsValue::from_str("trustee index out of range"))?;
        let predicates: Vec<String> = session
            .client
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
        let session = inner
            .sessions
            .get(index)
            .ok_or_else(|| JsValue::from_str("trustee index out of range"))?;
        let committed: Vec<String> = session
            .client
            .committed()
            .iter()
            .map(|p| format!("{p:?}"))
            .collect();
        serde_wasm_bindgen::to_value(&committed)
            .map_err(|e| JsValue::from_str(&format!("failed to serialize: {e}")))
    }
}
