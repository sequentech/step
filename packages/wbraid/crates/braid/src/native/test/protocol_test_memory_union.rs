// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! In-memory board-union test (§8.2): run the DKG once, then run one or more
//! **tallies** over it, each on its own child board unioned with the shared DKG
//! parent.
//!
//! This is the mixnet setting the union exists for: a single DKG backs many
//! tallies (this is what the old `batch` field did — §4.4/§8.2). Each tally is a
//! separate child board + separate [`BoardClient`]/session, seeing only
//! `dkg ∪ its-own-child`; siblings are isolated so their (same-`cfg_hash`)
//! predicates never false-collide. The DKG parent is shared and its integrity is
//! carried across the union by **seeding** each tally with the trustee's own
//! committed DKG digests ([`BoardClient::committed`]) — never a b4 re-fetch (§8.2).
//!
//! The trustee is a pure component, so the SAME [`Trustee`] instances drive
//! the DKG phase and every tally phase; only the board client changes.

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

use crate::board::persistence::{NoOpPersistence, Persistence};
use crate::board::transport::{MemoryBoard, MemoryTransport, Transport};
use crate::board::BoardClient;
use crate::messages::predicate::Predicate;
use crate::trustee::Trustee;

/// Wire `date` for every message the harness posts (§3.1).
const DATE: Timestamp = 0;

/// Safety cap on driver rounds.
const MAX_ROUNDS: usize = 200;

/// Entry point. `tallies` is the number of child tallies to run over the one DKG
/// (this is the union's reason for existing — the old `batch` count, §8.2); it
/// must be at least 1.
pub fn run<C: Context>(ciphertexts: u32, tallies: usize, ciphertext_width: usize) {
    assert!(tallies >= 1, "at least one tally");
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("failed to build tokio runtime");
    crate::dispatch_ciphertext_width!(ciphertext_width, {
        runtime
            .block_on(run_with_width::<C, W>(ciphertexts, tallies))
            .unwrap()
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

    // --- phase 1: DKG on the shared parent board ---
    let parent = MemoryBoard::<C>::new();
    parent.push(cfg_message);

    let mut dkg_clients: Vec<BoardClient<C, MemoryTransport<C>, NoOpPersistence>> =
        Vec::with_capacity(n_trustees);
    for _ in 0..n_trustees {
        dkg_clients.push(
            BoardClient::connect(MemoryTransport::new(parent.clone()), NoOpPersistence).await?,
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

    // The joint public key produced by the DKG (read once off the parent board).
    let dkg_messages = parent.snapshot();
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

    // --- phase 2: one or more tallies, each over its own child board unioned with
    //     the shared DKG parent ---
    for tally in 0..tallies {
        // A fresh child board and a fresh ciphertext set per tally.
        let child = MemoryBoard::<C>::new();
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
        child.push(ballots_message);

        // One union client per trustee: child (tally) ∪ parent (DKG), seeded with
        // that trustee's own DKG digests.
        let mut tally_clients: Vec<BoardClient<C, MemoryTransport<C>, NoOpPersistence>> =
            Vec::with_capacity(n_trustees);
        for seed in &seeds {
            tally_clients.push(
                BoardClient::connect_union(
                    MemoryTransport::new(child.clone()),
                    MemoryTransport::new(parent.clone()),
                    NoOpPersistence,
                    seed.clone(),
                )
                .await?,
            );
        }

        info!("Mixing and decrypting tally {}", tally);
        drive(&trustees, &mut tally_clients).await?;

        let final_messages = child.snapshot();
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
        info!("tally {} verified", tally);
    }

    let time = now.elapsed().as_millis() as f64 / 1000.0;
    info!("***************************************************************");
    info!("* Completed {} tallies in {}s", tallies, time);
    info!("* Trustees = {} (threshold {})", n_trustees, n_threshold);
    info!("* Ciphertexts = {} per tally (width = {})", ciphertexts, W);
    info!("***************************************************************");

    Ok(())
}

/// Drive `trustees` over their board `clients` to a protocol fixpoint using the
/// update-first cycle (§6). The trustees are shared (pure `step`); only the
/// clients are advanced. Each round: update every client, `step` every trustee in
/// parallel over its client view (CPU-bound crypto), then post. A round that
/// produces nothing is the fixpoint. Trustee `i` is paired with client `i`.
async fn drive<C, T, P>(
    trustees: &[Trustee<C>],
    clients: &mut [BoardClient<C, T, P>],
) -> Result<()>
where
    C: Context,
    // `Transport`/`Persistence` are `?Send` (Option B); the parallel step below
    // shares `&BoardClient` across rayon threads, so the concrete transport and
    // persistence must be `Sync` here (everything C-derived already is, via
    // `Context: Send + Sync`).
    T: Transport<C> + Sync,
    P: Persistence + Sync,
{
    for _ in 0..MAX_ROUNDS {
        for client in clients.iter_mut() {
            client.update().await?;
        }

        let produced: Vec<Vec<ProtocolMessage<C>>> = trustees
            .par_iter()
            .zip(clients.par_iter())
            .map(|(trustee, client)| trustee.step(client.view()))
            .collect::<Result<_>>()?;

        let mut produced_any = false;
        for (client, messages) in clients.iter_mut().zip(produced) {
            if !messages.is_empty() {
                produced_any = true;
                client.post(messages).await?;
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
