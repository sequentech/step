// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! HTTP M2 protocol test: the same DKG → encrypt → mix → threshold-decrypt round
//! as the in-memory M1 harness, but each trustee runs a [`Session`] over a live
//! b4 via [`HttpTransport`] (real HTTP + S3).
//!
//! This test is `#[ignore]`d (see `tests/test_protocol.rs`): it requires a running
//! b4 server at [`HTTP_URL`] backed by S3/LocalStack. Persistence is
//! [`NoOpPersistence`] for now — a clean run does not exercise anti-rewrite /
//! restart (that is the SQLite persistence follow-up).

use anyhow::{anyhow, Result};
use log::info;
use rand::seq::IndexedRandom;
use rand::Rng;
use rayon::prelude::*;
use std::collections::HashSet;
use std::marker::PhantomData;
use std::time::Instant;

use cryptography::context::Context;
use cryptography::cryptosystem::elgamal::{Ciphertext, KeyPair, PublicKey};
use cryptography::traits::groups::CryptographicGroup;
use cryptography::utils::serialization::VDeserializable;
use cryptography::utils::signatures::SignatureScheme;

use crate::messages::artifact::{Ballots, Configuration, DkgPublicKey, Plaintexts};
use crate::messages::newtypes::{
    hash_bytes, ConfigurationHash, PublicKeyHash, Timestamp, TrusteeIndex, MAX_TRUSTEES,
};
use crate::messages::protocol_manager::ProtocolManager;
use crate::messages::wire::{MessageType, ProtocolMessage};

use crate::board::http_transport::HttpTransport;
use crate::board::persistence::NoOpPersistence;
use crate::board::transport::Transport;
use crate::board::BoardClient;
use crate::runtime::SessionTrustee;
use crate::session::Session;

/// b4 server endpoint the test drives against (must be running, with S3).
const HTTP_URL: &str = "http://127.0.0.1:3000";

/// Wire `date` for every message the harness posts (§3.1).
const DATE: Timestamp = 0;

/// Safety cap on driver rounds.
const MAX_ROUNDS: usize = 200;

/// A trustee session backed by the live HTTP transport, no persistence (M2 gate).
type HttpSession<C> = Session<C, HttpTransport, NoOpPersistence>;

/// Entry point (kept signature-compatible with the legacy harness). `batches` is
/// vestigial — M2 runs a single ballot set — and must be 1.
pub async fn run<C: Context>(ciphertexts: u32, batches: usize, ciphertext_width: usize) {
    assert_eq!(batches, 1, "M2 HTTP harness runs a single ballot set");
    crate::dispatch_ciphertext_width!(ciphertext_width, {
        run_with_width::<C, W>(ciphertexts).await.unwrap()
    });
}

async fn run_with_width<C: Context, const W: usize>(ciphertexts: u32) -> Result<()> {
    // --- pick a random committee size and threshold/mixing subset ---
    let mut setup_rng = rand::rng();
    let n_trustees = setup_rng.random_range(2..=MAX_TRUSTEES);
    let n_threshold = setup_rng.random_range(2..=n_trustees);
    let all: Vec<TrusteeIndex> = (1..=n_trustees).collect();
    let mixing_trustees: Vec<TrusteeIndex> = all
        .choose_multiple(&mut setup_rng, n_threshold)
        .cloned()
        .collect();

    // A fresh board per run so re-runs never collide on b4's persistent store.
    let board = format!("protocoltest_{}", setup_rng.random::<u64>());

    let now = Instant::now();

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
    let cfg_message = ProtocolMessage::<C>::configuration(&pm, DATE, &cfg);

    // --- create the board on b4 and post the Configuration (manager) ---
    info!("Creating board {} on b4", board);
    HttpTransport::create_board(HTTP_URL, &board).await?;
    let manager_tx = HttpTransport::new(HTTP_URL, &board);
    Transport::<C>::post(&manager_tx, vec![cfg_message]).await?;

    // --- one Session (trustee + board client) per configured trustee ---
    let mut sessions: Vec<HttpSession<C>> = Vec::with_capacity(n_trustees);
    for (i, (signing_key, keypair)) in signing_keys.into_iter().zip(share_keypairs).enumerate() {
        let transport = HttpTransport::new(HTTP_URL, &board);
        let client = BoardClient::connect(transport, NoOpPersistence).await?;
        let trustee = SessionTrustee::new(
            (i + 1).to_string(),
            signing_key,
            keypair,
            client.configuration(),
        )?;
        sessions.push(Session::new(trustee, client));
    }

    // --- phase 1: DKG (shares + joint public key) ---
    info!(
        "Running DKG for {} trustees (threshold {})",
        n_trustees, n_threshold
    );
    drive(&mut sessions).await?;

    // --- read the joint public key off b4 ---
    let dkg_messages = Transport::<C>::fetch(&manager_tx).await?;
    let pk_body = dkg_messages
        .iter()
        .find(|m| m.message_type == MessageType::PublicKey)
        .and_then(|m| m.body.as_ref())
        .ok_or_else(|| anyhow!("DKG did not produce a public key"))?;
    let dkg_pk = DkgPublicKey::<C>::deser(pk_body)
        .map_err(|e| anyhow!("failed to deserialize public key: {:?}", e))?;
    let pk_hash = PublicKeyHash(hash_bytes(pk_body));

    // --- manager encrypts a batch of plaintexts and posts the ballots ---
    let pk = PublicKey::<C>::new(dkg_pk.pk.clone());
    let mut enc_rng = C::get_rng();
    info!("Encrypting {} ciphertexts (width {})", ciphertexts, W);
    let plaintexts_in: Vec<[C::Element; W]> = (0..ciphertexts)
        .map(|_| std::array::from_fn(|_| C::G::random_element(&mut enc_rng)))
        .collect();
    let encrypted: Vec<Ciphertext<C, W>> =
        plaintexts_in.par_iter().map(|p| pk.encrypt(p)).collect();
    let ballots = Ballots::<C, W>::new(encrypted);
    let ballots_message = ProtocolMessage::<C>::ballots(
        &pm,
        DATE,
        cfg_hash,
        pk_hash,
        mixing_trustees.clone(),
        &ballots,
    );
    Transport::<C>::post(&manager_tx, vec![ballots_message]).await?;

    // --- phase 2: mixing + threshold decryption ---
    info!("Mixing and decrypting");
    drive(&mut sessions).await?;

    // --- read the plaintexts off b4 and compare ---
    let final_messages = Transport::<C>::fetch(&manager_tx).await?;
    let pt_body = final_messages
        .iter()
        .find(|m| m.message_type == MessageType::Plaintexts)
        .and_then(|m| m.body.as_ref())
        .ok_or_else(|| anyhow!("protocol did not produce plaintexts"))?;
    let plaintexts = Plaintexts::<C, W>::deser(pt_body)
        .map_err(|e| anyhow!("failed to deserialize plaintexts: {:?}", e))?;

    let expected: HashSet<[C::Element; W]> = plaintexts_in.into_iter().collect();
    let actual: HashSet<[C::Element; W]> = plaintexts.0.into_iter().collect();
    assert!(
        expected == actual,
        "decrypted plaintexts do not match the encrypted inputs"
    );

    let time = now.elapsed().as_millis() as f64 / 1000.0;
    info!("***************************************************************");
    info!("* Completed in {}s (board {})", time, board);
    info!("* Trustees = {} (threshold {})", n_trustees, n_threshold);
    info!("* Ciphertexts = {} (width = {})", ciphertexts, W);
    info!("***************************************************************");

    Ok(())
}

/// Drive the sessions to a protocol fixpoint over HTTP using the update-first
/// cycle (§6). Each round advances every session once (update → step → post);
/// a round that produces nothing is the fixpoint. Sequential (HTTP latency
/// dominates; the parallel-step path is exercised by the in-memory M1 harness).
async fn drive<C: Context>(sessions: &mut [HttpSession<C>]) -> Result<()> {
    for _ in 0..MAX_ROUNDS {
        let mut produced_any = false;
        for session in sessions.iter_mut() {
            if session.advance().await? {
                produced_any = true;
            }
        }
        if !produced_any {
            return Ok(());
        }
    }
    Err(anyhow!(
        "protocol did not reach a fixpoint within {} rounds",
        MAX_ROUNDS
    ))
}
