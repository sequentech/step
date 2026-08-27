// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! HTTP board-union test (§8.2): run the DKG once on a live b4 board, then run one
//! or more tallies, each on its own child board unioned with the shared DKG parent,
//! all over real HTTP + S3.
//!
//! This is the HTTP analogue of `protocol_test_memory_union` and, unlike the M1/M2
//! happy-path tests, it wires [`SqlitePersistence`] end-to-end: the DKG session
//! persists its predicates, and each tally is **seeded** with that session's
//! committed digests (§8.2) into its own SQLite store. So it exercises both the
//! SQLite persistence path and the union seed against a real board.
//!
//! `#[ignore]`d (see `tests/test_protocol.rs`): it requires a running b4 at
//! [`HTTP_URL`] backed by S3/LocalStack.

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
use cryptography::utils::serialization::VDeserializable;
use cryptography::utils::signatures::SignatureScheme;

use crate::messages::artifact::{Ballots, Configuration, DkgPublicKey, Plaintexts};
use crate::messages::newtypes::{
    hash_bytes, ConfigurationHash, PublicKeyHash, Timestamp, TrusteeIndex, MAX_TRUSTEES,
};
use crate::protocol_manager::ProtocolManager;
use crate::messages::wire::{MessageType, ProtocolMessage};

use crate::board::persistence::Persistence;
use crate::board::transport::Transport;
use crate::board::BoardClient;
use crate::messages::predicate::Predicate;
use crate::native::http_transport::HttpTransport;
use crate::native::persistence::SqlitePersistence;
use crate::trustee::Trustee;

/// b4 server endpoint the test drives against (must be running, with S3).
const HTTP_URL: &str = "http://127.0.0.1:3000";

/// Wire `date` for every message the harness posts (§3.1).
const DATE: Timestamp = 0;

/// Safety cap on driver rounds.
const MAX_ROUNDS: usize = 200;

/// Entry point. `tallies` is the number of child tallies to run over the one DKG.
pub async fn run<C: Context>(ciphertexts: u32, tallies: usize, ciphertext_width: usize) {
    assert!(tallies >= 1, "at least one tally");
    crate::dispatch_ciphertext_width!(ciphertext_width, {
        run_with_width::<C, W>(ciphertexts, tallies).await.unwrap()
    });
}

async fn run_with_width<C: Context, const W: usize>(
    ciphertexts: u32,
    tallies: usize,
) -> Result<()> {
    // --- committee, threshold, and the mixing/decrypting subset ---
    let mut setup_rng = rand::rng();
    let n_trustees = setup_rng.random_range(2..=MAX_TRUSTEES);
    let n_threshold = setup_rng.random_range(2..=n_trustees);
    let all: Vec<TrusteeIndex> = (1..=n_trustees).collect();
    let mixing_trustees: Vec<TrusteeIndex> = all
        .choose_multiple(&mut setup_rng, n_threshold)
        .cloned()
        .collect();

    // Fresh board names per run so re-runs never collide on b4's persistent store.
    let run_id: u64 = setup_rng.random();
    let dkg_board = format!("uniondkg_{}", run_id);

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

    // The trustees are pure components, built once and reused across the DKG and
    // every tally phase (only the board client changes, §8.2).
    let mut trustees: Vec<Trustee<C>> = Vec::with_capacity(n_trustees);
    for (i, (signing_key, keypair)) in signing_keys.into_iter().zip(share_keypairs).enumerate() {
        trustees.push(Trustee::new(
            (i + 1).to_string(),
            signing_key,
            keypair,
            &cfg,
        )?);
    }

    // --- phase 1: DKG on the shared parent board (with SQLite persistence) ---
    info!("Creating DKG board {} on b4", dkg_board);
    HttpTransport::create_board(HTTP_URL, &dkg_board).await?;
    let dkg_manager_tx = HttpTransport::new(HTTP_URL, &dkg_board);
    Transport::<C>::publish(&dkg_manager_tx, &cfg_message).await?;

    let mut dkg_clients: Vec<BoardClient<C, HttpTransport, SqlitePersistence>> =
        Vec::with_capacity(n_trustees);
    for i in 0..n_trustees {
        let path = temp_db(&format!("dkg_{}_{}", run_id, i));
        dkg_clients.push(
            BoardClient::connect(
                HttpTransport::new(HTTP_URL, &dkg_board),
                SqlitePersistence::open(&path)?,
            )
            .await?,
        );
    }
    info!(
        "Running DKG for {} trustees (threshold {})",
        n_trustees, n_threshold
    );
    drive(&trustees, &mut dkg_clients).await?;

    // Each trustee's own committed DKG digests are the anti-rewrite seed handed to
    // its tallies (§8.2).
    let seeds: Vec<Vec<Predicate>> = dkg_clients.iter().map(|c| c.committed().to_vec()).collect();

    // The joint public key produced by the DKG.
    let dkg_messages = Transport::<C>::fetch(&dkg_manager_tx).await?;
    let pk_body = dkg_messages
        .iter()
        .find(|m| m.message_type == MessageType::PublicKey)
        .and_then(|m| m.body.as_ref())
        .ok_or_else(|| anyhow!("DKG did not produce a public key"))?;
    let dkg_pk = DkgPublicKey::<C>::deser(pk_body)
        .map_err(|e| anyhow!("failed to deserialize public key: {:?}", e))?;
    let pk_hash = PublicKeyHash(hash_bytes(pk_body));
    let pk = PublicKey::<C>::new(dkg_pk.pk.clone());

    // The ballot encryption context is tally-agnostic (execution + key scoped),
    // so one Naor-Yung key serves every tally of this DKG.
    use cryptography::cryptosystem::naoryung;
    let ctx_enc = crate::trustee::ballot_encryption_context::<C>(cfg.id, &dkg_pk.pk);
    let ny_pk = naoryung::PublicKey::augment(&pk, &ctx_enc)
        .map_err(|e| anyhow!("failed to derive the ballot auxiliary key: {:?}", e))?;

    // --- phase 2: one or more tallies, each a child board unioned with the DKG ---
    for tally in 0..tallies {
        let tally_board = format!("uniontally_{}_{}", run_id, tally);
        info!("Creating tally board {} on b4", tally_board);
        HttpTransport::create_board(HTTP_URL, &tally_board).await?;

        // Manager encrypts a fresh ciphertext set and posts the ballots.
        let mut enc_rng = C::get_rng();
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
            // Distinct tally-execution identifier per sibling tally (§8.2).
            1 + tally as u128,
            &ballots,
        );
        let tally_manager_tx = HttpTransport::new(HTTP_URL, &tally_board);
        Transport::<C>::publish(&tally_manager_tx, &ballots_message).await?;

        // One union client per trustee: child (tally) ∪ parent (DKG), seeded with
        // that trustee's own DKG digests, into a fresh SQLite store.
        let mut tally_clients: Vec<BoardClient<C, HttpTransport, SqlitePersistence>> =
            Vec::with_capacity(n_trustees);
        for (i, seed) in seeds.iter().enumerate() {
            let path = temp_db(&format!("tally_{}_{}_{}", run_id, tally, i));
            tally_clients.push(
                BoardClient::connect_union(
                    HttpTransport::new(HTTP_URL, &tally_board),
                    HttpTransport::new(HTTP_URL, &dkg_board),
                    SqlitePersistence::open(&path)?,
                    seed.clone(),
                )
                .await?,
            );
        }

        info!("Mixing and decrypting tally {}", tally);
        drive(&trustees, &mut tally_clients).await?;

        let final_messages = Transport::<C>::fetch(&tally_manager_tx).await?;
        let pt_body = final_messages
            .iter()
            .find(|m| m.message_type == MessageType::Plaintexts)
            .and_then(|m| m.body.as_ref())
            .ok_or_else(|| anyhow!("tally {} did not produce plaintexts", tally))?;
        let plaintexts = Plaintexts::<C, W>::deser(pt_body)
            .map_err(|e| anyhow!("failed to deserialize plaintexts: {:?}", e))?;

        let expected: HashSet<[C::Element; W]> = plaintexts_in.into_iter().collect();
        let actual: HashSet<[C::Element; W]> = plaintexts.0.into_iter().collect();
        assert!(
            expected == actual,
            "tally {} plaintexts do not match the encrypted inputs",
            tally
        );
        info!("tally {} verified (board {})", tally, tally_board);
    }

    let time = now.elapsed().as_millis() as f64 / 1000.0;
    info!("***************************************************************");
    info!(
        "* Completed {} tallies in {}s (DKG board {})",
        tallies, time, dkg_board
    );
    info!("* Trustees = {} (threshold {})", n_trustees, n_threshold);
    info!("* Ciphertexts = {} per tally (width = {})", ciphertexts, W);
    info!("***************************************************************");

    Ok(())
}

/// A fresh temp SQLite path for a trustee's predicate store.
fn temp_db(tag: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!("braid_union_{}.sqlite", tag));
    let _ = std::fs::remove_file(&path);
    path
}

/// Drive `trustees` over their board `clients` to a fixpoint via the update-first
/// cycle (§6), sequentially (HTTP latency dominates). Trustee `i` is paired with
/// client `i`.
async fn drive<C, T, P>(
    trustees: &[Trustee<C>],
    clients: &mut [BoardClient<C, T, P>],
) -> Result<()>
where
    C: Context,
    T: Transport<C>,
    P: Persistence,
{
    for _ in 0..MAX_ROUNDS {
        let mut produced_any = false;
        for (trustee, client) in trustees.iter().zip(clients.iter_mut()) {
            client.update().await?;
            let produced = trustee.step(client.view())?;
            if !produced.is_empty() {
                produced_any = true;
                client.post(produced).await?;
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
