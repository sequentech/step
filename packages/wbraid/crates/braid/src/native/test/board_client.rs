// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Board-client integration tests that require a real (SQLite) persistence
//! backend: the restart / anti-rewrite boundary check (§6.2–§6.3).

#![cfg(test)]

use std::marker::PhantomData;

use anyhow::Result;
use cryptography::context::{Context, RistrettoCtx};
use cryptography::cryptosystem::elgamal::KeyPair;
use cryptography::utils::signatures::SignatureScheme;

use crate::board::transport::{MemoryBoard, MemoryTransport};
use crate::board::BoardClient;
use crate::messages::artifact::Configuration;
use crate::messages::newtypes::{ConfigurationHash, Timestamp};
use crate::messages::protocol_manager::ProtocolManager;
use crate::messages::wire::ProtocolMessage;
use crate::native::persistence::SqlitePersistence;
use crate::runtime::SessionTrustee;

const DATE: Timestamp = 0;

/// Restart + anti-rewrite (§6.2/§6.3): a predicate persisted before a restart
/// is reloaded into the committed set and forbids b4 from later filling the
/// same slot with a different body.
///
/// The `Shares` bodies are dummy bytes: `verify` only re-hashes the body into
/// the predicate and checks the signature, so distinct bodies yield distinct
/// (colliding) predicates without needing real DKG artifacts.
#[tokio::test]
async fn persisted_predicate_blocks_rewrite_across_restart() -> Result<()> {
    run_restart_anti_rewrite::<RistrettoCtx>().await
}

async fn run_restart_anti_rewrite<C: Context>() -> Result<()> {
    let db_path =
        std::env::temp_dir().join(format!("braid_anti_rewrite_{}.sqlite", std::process::id()));
    let _ = std::fs::remove_file(&db_path);

    // --- manager + two trustees + configuration ---
    let mut key_rng = C::get_rng();
    let pm = ProtocolManager::<C>::new(C::SignatureScheme::gen_signing_key(&mut key_rng));

    let mut signing_keys = Vec::new();
    let mut trustee_vks = Vec::new();
    let mut share_enc_keys = Vec::new();
    for _ in 0..2 {
        let sk = C::SignatureScheme::gen_signing_key(&mut key_rng);
        trustee_vks.push(C::SignatureScheme::verifying_key(&sk));
        signing_keys.push(sk);
        let keypair = KeyPair::<C>::generate();
        share_enc_keys.push(keypair.pkey.y.clone());
    }

    let cfg = Configuration::<C>::new(
        0,
        C::SignatureScheme::verifying_key(&pm.signing_key),
        trustee_vks,
        2,
        2,
        share_enc_keys,
        PhantomData,
    );
    let cfg_hash = ConfigurationHash::from_configuration(&cfg)?;
    let cfg_message = ProtocolMessage::<C>::configuration(&pm, DATE, &cfg);

    let board = MemoryBoard::<C>::new();
    board.push(cfg_message);

    // --- first run: post a Shares, update (persists its predicate) ---
    let mut first_sk = signing_keys.into_iter();
    let trustee = {
        let transport = MemoryTransport::new(board.clone());
        let client = BoardClient::connect(transport, SqlitePersistence::open(&db_path)?).await?;
        let trustee = SessionTrustee::<C>::new(
            "1".to_string(),
            first_sk.next().unwrap(),
            KeyPair::<C>::generate(),
            client.configuration(),
        )?;
        let mut client = client;
        let shares = ProtocolMessage::<C>::shares(&trustee, DATE, cfg_hash, &vec![1u8, 2, 3]);
        client.post(vec![shares]).await?;
        client.update().await?;
        trustee
        // client dropped here (and with it the in-memory committed set)
    };

    // --- b4 is asked to rewrite the slot: a colliding Shares from the same
    //     trustee with a different body appears on the board ---
    let colliding = ProtocolMessage::<C>::shares(&trustee, DATE, cfg_hash, &vec![4u8, 5, 6]);
    board.push(colliding);

    // --- restart: reopen persistence, reconnect, update must halt ---
    let persistence = SqlitePersistence::open(&db_path)?;
    let mut client = BoardClient::connect(MemoryTransport::new(board.clone()), persistence).await?;
    let result = client.update().await;

    let _ = std::fs::remove_file(&db_path);
    assert!(
        result.is_err(),
        "reloaded committed predicate must block the rewrite"
    );
    Ok(())
}
