// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! In-browser mixnet emulator (M3-C, step (i)): run the full v0.6 protocol —
//! DKG → encrypt → mix → threshold-decrypt — entirely in the browser over an
//! in-memory board.
//!
//! This is the wasm counterpart of `native::test::protocol_test_memory`: all
//! trustees share one [`MemoryBoard`] (no b4), each drives the update-first cycle
//! (§6), and the manager posts a single `Ballots` set. It proves the whole v0.6
//! core (pure `SessionTrustee` + datalog + action-layer crypto + board client)
//! runs under `wasm32`.
//!
//! Step (i) is a single one-shot [`run_in_memory`] call that returns the outcome;
//! the interactive per-trustee stepping UI is a follow-on. Persistence is
//! [`NoOpPersistence`] here (the IndexedDB backend is exercised by its own test).

use wasm_bindgen::prelude::*;

use anyhow::{anyhow, Result};
use serde::Serialize;
use std::collections::HashSet;
use std::marker::PhantomData;

use cryptography::context::{Context, RistrettoCtx};
use cryptography::cryptosystem::elgamal::{Ciphertext, KeyPair, PublicKey};
use cryptography::traits::groups::CryptographicGroup;
use cryptography::utils::serialization::VDeserializable;
use cryptography::utils::signatures::SignatureScheme;

use b4::messages::artifact::{Ballots, Configuration, DkgPublicKey, Plaintexts};
use b4::messages::newtypes::{ConfigurationHash, PublicKeyHash, Timestamp, TrusteeIndex};
use b4::messages::protocol_manager::ProtocolManager;
use b4::messages::wire::{MessageType, WireMessage};

use crate::board::persistence::NoOpPersistence;
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

/// The outcome of an emulator run, returned to JS.
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
///
/// `width` (ciphertext width) must be 1..=8; `threshold` must be 2..=`trustees`
/// and `trustees` 2..=8 (the range the dispatch macros cover).
#[wasm_bindgen]
pub async fn run_in_memory(
    trustees: usize,
    threshold: usize,
    ciphertexts: u32,
    width: usize,
) -> Result<JsValue, JsValue> {
    if !(1..=MAX_TRUSTEES).contains(&width) {
        return Err(JsValue::from_str(&format!(
            "unsupported ciphertext width {width} (expected 1..={MAX_TRUSTEES})"
        )));
    }
    if !(2..=MAX_TRUSTEES).contains(&trustees) {
        return Err(JsValue::from_str(&format!(
            "unsupported trustee count {trustees} (expected 2..={MAX_TRUSTEES})"
        )));
    }
    if !(2..=trustees).contains(&threshold) {
        return Err(JsValue::from_str(&format!(
            "unsupported threshold {threshold} (expected 2..={trustees})"
        )));
    }

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
    // The first `threshold` trustees are the mixing/decrypting subset.
    let mixing_trustees: Vec<TrusteeIndex> = (1..=n_threshold).collect();

    // --- manager, per-trustee key material, and the configuration ---
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
        W,
        PhantomData,
    )
    .with_share_encryption_keys(share_enc_keys);
    let cfg_hash = ConfigurationHash::from_configuration(&cfg)?;
    let cfg_message = WireMessage::<C>::configuration(&pm, DATE, &cfg);

    // --- the shared in-memory board, seeded with the Configuration ---
    let board = MemoryBoard::<C>::new();
    board.push(cfg_message);

    // --- one Session (trustee + board client) per configured trustee ---
    let mut sessions: Vec<Session<C, MemoryTransport<C>, NoOpPersistence>> =
        Vec::with_capacity(n_trustees);
    for (i, (signing_key, keypair)) in signing_keys.into_iter().zip(share_keypairs).enumerate() {
        let transport = MemoryTransport::new(board.clone());
        let client = BoardClient::connect(transport, NoOpPersistence).await?;
        let trustee = SessionTrustee::new(
            (i + 1).to_string(),
            signing_key,
            keypair,
            client.configuration(),
        )?;
        sessions.push(Session::new(trustee, client));
    }

    // --- phase 1: DKG ---
    let dkg_rounds = drive(&mut sessions).await?;

    let dkg_messages = board.snapshot();
    let pk_body = dkg_messages
        .iter()
        .find(|m| m.message_type == MessageType::PublicKey)
        .and_then(|m| m.body.as_ref())
        .ok_or_else(|| anyhow!("DKG did not produce a public key"))?;
    let dkg_pk = DkgPublicKey::<C>::deser(pk_body)
        .map_err(|e| anyhow!("failed to deserialize public key: {:?}", e))?;
    let pk_hash = PublicKeyHash(b4::hash_bytes(pk_body));

    // --- manager encrypts a set of plaintexts and posts the ballots ---
    let pk = PublicKey::<C>::new(dkg_pk.pk.clone());
    let mut enc_rng = C::get_rng();
    let plaintexts_in: Vec<[C::Element; W]> = (0..ciphertexts)
        .map(|_| std::array::from_fn(|_| C::G::random_element(&mut enc_rng)))
        .collect();
    let encrypted: Vec<Ciphertext<C, W>> = plaintexts_in.iter().map(|p| pk.encrypt(p)).collect();
    let ballots = Ballots::<C, W>::new(encrypted);
    let ballots_message =
        WireMessage::<C>::ballots(&pm, DATE, cfg_hash, pk_hash, mixing_trustees, &ballots);
    board.push(ballots_message);

    // --- phase 2: mixing + threshold decryption ---
    let tally_rounds = drive(&mut sessions).await?;

    let final_messages = board.snapshot();
    let pt_body = final_messages
        .iter()
        .find(|m| m.message_type == MessageType::Plaintexts)
        .and_then(|m| m.body.as_ref())
        .ok_or_else(|| anyhow!("protocol did not produce plaintexts"))?;
    let plaintexts = Plaintexts::<C, W>::deser(pt_body)
        .map_err(|e| anyhow!("failed to deserialize plaintexts: {:?}", e))?;

    let expected: HashSet<[C::Element; W]> = plaintexts_in.into_iter().collect();
    let actual: HashSet<[C::Element; W]> = plaintexts.0.into_iter().collect();
    let success = expected == actual;

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

/// Drive the sessions to a protocol fixpoint (§6), sequentially on the single
/// wasm thread. Returns the number of rounds taken. A round that produces nothing
/// is the fixpoint.
async fn drive<C: Context>(
    sessions: &mut [Session<C, MemoryTransport<C>, NoOpPersistence>],
) -> Result<usize> {
    for round in 0..MAX_ROUNDS {
        let mut produced_any = false;
        for session in sessions.iter_mut() {
            if session.advance().await? {
                produced_any = true;
            }
        }
        if !produced_any {
            return Ok(round);
        }
    }
    Err(anyhow!(
        "protocol did not reach a fixpoint within {} rounds",
        MAX_ROUNDS
    ))
}
