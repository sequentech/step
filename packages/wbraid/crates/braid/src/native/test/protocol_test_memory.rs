// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! In-memory M1 protocol test: drive the v0.6 [`SessionTrustee`] runtime through
//! a full DKG → encrypt → mix → threshold-decrypt round on a single shared board.
//!
//! The "board" is just an ordered `Vec<WireMessage>`; the driver relays each
//! trustee's freshly produced messages back to the others and iterates to a
//! fixpoint (§9). Unlike the legacy `protocol::*` harness there are no channels,
//! symmetric wrapping, or batches (§9.4): the manager posts a single `Ballots`
//! set directly naming the mixing subset, and every trustee's share is
//! ElGamal-encrypted to its configured share-encryption key.

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

use b4::messages::artifact::{Ballots, Configuration, DkgPublicKey, Plaintexts};
use b4::messages::newtypes::{
    ConfigurationHash, PublicKeyHash, Timestamp, TrusteeIndex, MAX_TRUSTEES,
};
use b4::messages::protocol_manager::ProtocolManager;
use b4::messages::wire::{MessageType, WireMessage};

use crate::runtime::SessionTrustee;

/// Wire `date` for every message the harness posts (§3.1); a fixed value is fine
/// (M1 does not verify timestamps).
const DATE: Timestamp = 0;

/// Safety cap on driver rounds; a healthy run converges in a handful of passes.
const MAX_ROUNDS: usize = 200;

/// Entry point (kept signature-compatible with the legacy harness). `batches` is
/// vestigial — M1 runs a single ballot set — and must be 1.
pub fn run<C: Context>(ciphertexts: u32, batches: usize, ciphertext_width: usize) {
    assert_eq!(batches, 1, "M1 in-memory harness runs a single ballot set");
    crate::dispatch_ciphertext_width!(ciphertext_width, {
        run_with_width::<C, W>(ciphertexts).unwrap()
    });
}

fn run_with_width<C: Context, const W: usize>(ciphertexts: u32) -> Result<()> {
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
        PhantomData,
    )
    .with_share_encryption_keys(share_enc_keys);
    let cfg_hash = ConfigurationHash::from_configuration(&cfg)?;
    let cfg_message = WireMessage::<C>::configuration(&pm, DATE, &cfg);

    // --- one SessionTrustee per configured trustee ---
    let mut trustees: Vec<SessionTrustee<C>> = Vec::with_capacity(n_trustees);
    for (i, (signing_key, keypair)) in signing_keys.into_iter().zip(share_keypairs).enumerate() {
        trustees.push(SessionTrustee::new(
            (i + 1).to_string(),
            signing_key,
            keypair,
            &cfg_message,
        )?);
    }

    // The board is the ordered log of every posted message; `cursors[i]` tracks
    // how far trustee `i` has already consumed.
    let mut board: Vec<WireMessage<C>> = Vec::new();
    let mut cursors = vec![0usize; n_trustees];

    // --- phase 1: DKG (shares + joint public key) ---
    info!(
        "Running DKG for {} trustees (threshold {})",
        n_trustees, n_threshold
    );
    drive(&mut trustees, &mut board, &mut cursors)?;

    let pk_body = board
        .iter()
        .find(|m| m.message_type == MessageType::PublicKey)
        .and_then(|m| m.body.as_ref())
        .ok_or_else(|| anyhow!("DKG did not produce a public key"))?
        .clone();
    let dkg_pk = DkgPublicKey::<C>::deser(&pk_body)
        .map_err(|e| anyhow!("failed to deserialize public key: {:?}", e))?;
    let pk_hash = PublicKeyHash(b4::hash_bytes(&pk_body));

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
    let ballots_message = WireMessage::<C>::ballots(
        &pm,
        DATE,
        cfg_hash,
        pk_hash,
        mixing_trustees.clone(),
        &ballots,
    );
    board.push(ballots_message);

    // --- phase 2: mixing + threshold decryption ---
    info!("Mixing and decrypting");
    drive(&mut trustees, &mut board, &mut cursors)?;

    let pt_body = board
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

/// Relay messages between trustees until no trustee produces anything new.
///
/// Each pass steps every trustee **in parallel** (rayon) over the same immutable
/// snapshot of the board — the messages it has not yet consumed — then appends
/// everything they produce so the next pass observes it. Reaching a pass that
/// produces nothing is the protocol fixpoint. Stepping in parallel (rather than
/// one trustee at a time) exercises the concurrent use of `SessionTrustee` /
/// `WireMessage` that the deployed mixnet relies on; it costs at most a few extra
/// passes (a trustee sees this pass's messages next pass instead of same-pass),
/// which the order-independent datalog fixpoint absorbs.
fn drive<C: Context>(
    trustees: &mut [SessionTrustee<C>],
    board: &mut Vec<WireMessage<C>>,
    cursors: &mut [usize],
) -> Result<()> {
    for _ in 0..MAX_ROUNDS {
        // Freeze the board frontier and each trustee's cursor for this pass so
        // every trustee reads a consistent, immutable view while stepping.
        let frontier = board.len();
        let starts: Vec<usize> = cursors.to_vec();

        let produced: Vec<Vec<WireMessage<C>>> = {
            let board_view: &[WireMessage<C>] = board;
            trustees
                .par_iter_mut()
                .enumerate()
                .map(|(idx, trustee)| trustee.step(&board_view[starts[idx]..frontier]))
                .collect::<Result<_>>()?
        };

        // Every trustee has now consumed up to the frozen frontier.
        for cursor in cursors.iter_mut() {
            *cursor = frontier;
        }

        let mut produced_any = false;
        for messages in produced {
            if !messages.is_empty() {
                produced_any = true;
                board.extend(messages);
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
