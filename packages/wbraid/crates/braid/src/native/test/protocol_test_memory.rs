// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! In-memory M1 protocol test: drive the v0.6 runtime through a full DKG →
//! encrypt → mix → threshold-decrypt round.
//!
//! Each trustee runs as a [`Session`] (a functional [`Trustee`] over a
//! [`BoardClient`]); all board clients share one in-memory [`MemoryBoard`] that
//! stands in for b4. The driver runs the **update-first** cycle (§6): every board
//! client pulls the latest board, each trustee `step`s (in parallel — CPU-bound
//! crypto) over its view, and the produced messages are posted back. A trustee's
//! own output takes effect only once it loops back on the next update. There are
//! no channels, symmetric wrapping, or batches (§9.4): the manager posts a single
//! `Ballots` set directly, and each share is ElGamal-encrypted to its recipient's
//! configured share-encryption key.

use anyhow::{anyhow, Result};
use log::info;
use rand::seq::IndexedRandom;
use rand::Rng;
use rayon::prelude::*;
use std::collections::HashSet;
use std::marker::PhantomData;
use std::time::Instant;

use cryptography::context::Context;
use cryptography::cryptosystem::elgamal::{KeyPair, PublicKey};
use cryptography::traits::groups::CryptographicGroup;
use cryptography::utils::serialization::Deserializable;
use cryptography::utils::signatures::SignatureScheme;

use crate::messages::artifact::{Ballots, Configuration, DkgPublicKey, Plaintexts};
use crate::messages::newtypes::{
    hash_bytes, ConfigurationHash, PublicKeyHash, Timestamp, TrusteeIndex, MAX_TRUSTEES,
};
use crate::messages::wire::{MessageType, ProtocolMessage};
use crate::protocol_manager::ProtocolManager;

use crate::board::persistence::NoOpPersistence;
use crate::board::transport::{MemoryBoard, MemoryTransport};
use crate::board::BoardClient;
use crate::session::Session;
use crate::trustee::Trustee;

/// Wire `date` for every message the harness posts (§3.1); a fixed value is fine
/// (M1 does not verify timestamps).
const DATE: Timestamp = 0;

/// Safety cap on driver rounds; a healthy run converges in a handful of passes.
const MAX_ROUNDS: usize = 200;

/// A trustee session backed by the in-memory (mock-b4) transport and no
/// persistence — the M1 shape.
type MemorySession<C> = Session<C, MemoryTransport<C>, NoOpPersistence>;

/// Entry point (kept signature-compatible with the legacy harness). `batches` is
/// vestigial — M1 runs a single ballot set — and must be 1.
///
/// The session/board-client cycle is async; this sync `#[test]` entry point drives
/// it on a current-thread tokio runtime. The heavy crypto still runs on rayon's
/// pool (the parallel `step`), independent of the runtime.
pub fn run<C: Context>(ciphertexts: u32, batches: usize, ciphertext_width: usize) {
    assert_eq!(batches, 1, "M1 in-memory harness runs a single ballot set");
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("failed to build tokio runtime");
    crate::dispatch_ciphertext_width!(ciphertext_width, {
        runtime
            .block_on(run_with_width::<C, W>(ciphertexts))
            .unwrap()
    });
}

async fn run_with_width<C: Context, const W: usize>(ciphertexts: u32) -> Result<()> {
    // --- pick a random committee size and threshold/mixing subset ---
    let mut setup_rng = rand::rng();
    let n_trustees = setup_rng.random_range(2..=MAX_TRUSTEES);
    let n_threshold = setup_rng.random_range(2..=n_trustees);
    // 1-based trustee indices; the chosen subset is the mixing/decrypting set and
    // its order is the mixing order.
    let all: Vec<TrusteeIndex> = (1..=n_trustees).collect();
    let mixing_trustees: Vec<TrusteeIndex> = all
        .choose_multiple(&mut setup_rng, n_threshold)
        .cloned()
        .collect();

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
        share_enc_keys,
        PhantomData,
    );
    let cfg_hash = ConfigurationHash::from_configuration(&cfg)?;
    let cfg_message = ProtocolMessage::<C>::configuration(&pm, DATE, &cfg);

    // --- the shared in-memory board (mock b4), seeded with the Configuration ---
    let board = MemoryBoard::<C>::new();
    board.push(cfg_message);

    // --- one Session (trustee + board client) per configured trustee ---
    let mut sessions: Vec<MemorySession<C>> = Vec::with_capacity(n_trustees);
    for (i, (signing_key, keypair)) in signing_keys.into_iter().zip(share_keypairs).enumerate() {
        let transport = MemoryTransport::new(board.clone());
        let client = BoardClient::connect(transport, NoOpPersistence).await?;
        let trustee = Trustee::new(
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

    let dkg_messages = board.snapshot();
    let pk_body = dkg_messages
        .iter()
        .find(|m| m.message_type == MessageType::PublicKey)
        .and_then(|m| m.body.as_ref())
        .ok_or_else(|| anyhow!("DKG did not produce a public key"))?;
    let dkg_pk = DkgPublicKey::<C>::deser(pk_body)
        .map_err(|e| anyhow!("failed to deserialize public key: {:?}", e))?;
    let pk_hash = PublicKeyHash(hash_bytes(pk_body));

    // --- manager encrypts a batch of plaintexts and posts the ballots ---
    use cryptography::cryptosystem::naoryung;
    let pk = PublicKey::<C>::new(dkg_pk.pk.clone());
    let ctx_enc = crate::trustee::ballot_encryption_context::<C>(cfg.id, &dkg_pk.pk);
    let ny_pk = naoryung::PublicKey::augment(&pk, &ctx_enc)
        .map_err(|e| anyhow!("failed to derive the ballot auxiliary key: {:?}", e))?;
    let mut enc_rng = C::get_rng();
    info!("Encrypting {} ciphertexts (width {})", ciphertexts, W);
    let plaintexts_in: Vec<[C::Element; W]> = (0..ciphertexts)
        .map(|_| std::array::from_fn(|_| C::G::random_element(&mut enc_rng)))
        .collect();
    let encrypted: Vec<naoryung::Ciphertext<C, W>> = plaintexts_in
        .par_iter()
        .map(|p| ny_pk.encrypt(p, &ctx_enc))
        .collect::<Result<_, _>>()
        .map_err(|e| anyhow!("ballot encryption failed: {:?}", e))?;
    let ballots = Ballots::<C, W>::new(encrypted);
    let ballots_message = ProtocolMessage::<C>::ballots(
        &pm,
        DATE,
        cfg_hash,
        pk_hash,
        mixing_trustees.clone(),
        1, // tally_id: single tally in this harness
        &ballots,
    );
    board.push(ballots_message);

    // --- phase 2: mixing + threshold decryption ---
    info!("Mixing and decrypting");
    drive(&mut sessions).await?;

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
    assert!(
        expected == actual,
        "decrypted plaintexts do not match the encrypted inputs"
    );

    let time = now.elapsed().as_millis() as f64 / 1000.0;
    info!("***************************************************************");
    info!("* Completed in {}s", time);
    info!("* Trustees = {} (threshold {})", n_trustees, n_threshold);
    info!("* Ciphertexts = {} (width = {})", ciphertexts, W);
    info!("***************************************************************");

    Ok(())
}

/// Negative control: a ballot with an invalid well-formedness proof must halt
/// the tally at first-mix engagement (PROTOCOL.md §5.5) — no mix and no
/// plaintexts are produced.
///
/// With all-honest sessions the refusal surfaces in the mixer's `ComputeMix`
/// (no mix ever exists for the others to `SignMix`), but both actions fetch
/// ballots through the same verify-and-strip seam
/// (`Trustee::mix_input_ciphertexts`), so this exercises the one location
/// where a trustee accepts external ballots.
pub fn run_rejects_invalid_ballot<C: Context>() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("failed to build tokio runtime");
    runtime.block_on(run_invalid_ballot::<C>()).unwrap()
}

async fn run_invalid_ballot<C: Context>() -> Result<()> {
    use cryptography::cryptosystem::naoryung;
    const W: usize = 2;

    let n_trustees = 3;
    let n_threshold = 2;
    let mixing_trustees: Vec<TrusteeIndex> = vec![1, 2];

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
        share_enc_keys,
        PhantomData,
    );
    let cfg_hash = ConfigurationHash::from_configuration(&cfg)?;
    let cfg_message = ProtocolMessage::<C>::configuration(&pm, DATE, &cfg);

    let board = MemoryBoard::<C>::new();
    board.push(cfg_message);
    let mut sessions: Vec<MemorySession<C>> = Vec::with_capacity(n_trustees);
    for (i, (signing_key, keypair)) in signing_keys.into_iter().zip(share_keypairs).enumerate() {
        let transport = MemoryTransport::new(board.clone());
        let client = BoardClient::connect(transport, NoOpPersistence).await?;
        let trustee = Trustee::new(
            (i + 1).to_string(),
            signing_key,
            keypair,
            client.configuration(),
        )?;
        sessions.push(Session::new(trustee, client));
    }
    drive(&mut sessions).await?;

    let dkg_messages = board.snapshot();
    let pk_body = dkg_messages
        .iter()
        .find(|m| m.message_type == MessageType::PublicKey)
        .and_then(|m| m.body.as_ref())
        .ok_or_else(|| anyhow!("DKG did not produce a public key"))?;
    let dkg_pk = DkgPublicKey::<C>::deser(pk_body)
        .map_err(|e| anyhow!("failed to deserialize public key: {:?}", e))?;
    let pk_hash = PublicKeyHash(hash_bytes(pk_body));

    let pk = PublicKey::<C>::new(dkg_pk.pk.clone());
    let ctx_enc = crate::trustee::ballot_encryption_context::<C>(cfg.id, &dkg_pk.pk);
    let ny_pk = naoryung::PublicKey::augment(&pk, &ctx_enc)
        .map_err(|e| anyhow!("failed to derive the ballot auxiliary key: {:?}", e))?;
    let mut enc_rng = C::get_rng();
    let mut encrypted: Vec<naoryung::Ciphertext<C, W>> = (0..4)
        .map(|_| {
            let p: [C::Element; W] = std::array::from_fn(|_| C::G::random_element(&mut enc_rng));
            ny_pk.encrypt(&p, &ctx_enc)
        })
        .collect::<Result<_, _>>()
        .map_err(|e| anyhow!("ballot encryption failed: {:?}", e))?;

    // Swap the well-formedness proofs of two ballots: each ciphertext is
    // intact and each proof is valid — but for the other statement.
    let proof0 = encrypted[0].proof.clone();
    encrypted[0].proof = encrypted[1].proof.clone();
    encrypted[1].proof = proof0;

    let ballots = Ballots::<C, W>::new(encrypted);
    let ballots_message =
        ProtocolMessage::<C>::ballots(&pm, DATE, cfg_hash, pk_hash, mixing_trustees, 1, &ballots);
    board.push(ballots_message);

    let err = drive(&mut sessions)
        .await
        .expect_err("a tampered ballot must halt the tally");
    assert!(
        err.to_string().contains("failed Naor-Yung verification"),
        "unexpected error: {err}"
    );

    // The tally must have produced nothing: no mix, no plaintexts.
    let final_messages = board.snapshot();
    assert!(
        !final_messages
            .iter()
            .any(|m| matches!(m.message_type, MessageType::Mix | MessageType::Plaintexts)),
        "a tampered ballot must not produce mixes or plaintexts"
    );
    Ok(())
}

/// Drive the sessions to a protocol fixpoint using the update-first cycle (§6).
///
/// Each round has three phases: (1) every board client updates from the shared
/// board (async, sequential); (2) every trustee `step`s **in parallel** (rayon —
/// CPU-bound, over an immutable board view); (3) the produced messages are posted
/// back (async). A round that produces nothing is the fixpoint. Because it is
/// update-first, a trustee's own output only takes effect once it loops back on
/// the next round's update. The parallel `step` also exercises the concurrent use
/// of `Trustee`/`ProtocolMessage` that the deployed mixnet relies on.
async fn drive<C: Context>(sessions: &mut [MemorySession<C>]) -> Result<()> {
    for _ in 0..MAX_ROUNDS {
        for session in sessions.iter_mut() {
            session.update().await?;
        }

        let produced: Vec<Vec<ProtocolMessage<C>>> = sessions
            .par_iter()
            .map(|session| session.step())
            .collect::<Result<_>>()?;

        let mut produced_any = false;
        for (session, messages) in sessions.iter_mut().zip(produced) {
            if !messages.is_empty() {
                produced_any = true;
                session.post(messages).await?;
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
