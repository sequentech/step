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
use cryptography::utils::serialization::Serializable;
use cryptography::utils::signatures::SignatureScheme;

use crate::board::transport::{MemoryBoard, MemoryTransport};
use crate::board::BoardClient;
use crate::messages::artifact::Configuration;
use crate::messages::newtypes::{ConfigurationHash, Timestamp};
use crate::messages::wire::ProtocolMessage;
use crate::native::persistence::SqlitePersistence;
use crate::protocol_manager::ProtocolManager;
use crate::trustee::Trustee;

const DATE: Timestamp = 0;

/// Restart + anti-rewrite completeness gate (§6.2/§6.3): a predicate persisted
/// before a restart is reloaded into the committed set; if b4 no longer serves
/// it back (because it's serving a different body for that slot instead), the
/// completeness gate in `update()` blocks before `step` ever runs.
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
        let trustee = Trustee::<C>::new(
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

    // --- b4 now serves a DIFFERENT board on reconnect: same Configuration, but a
    //     colliding Shares (different body) in place of the one it served before
    //     the restart — never the original alongside it, or the datalog's own
    //     collides() rule would catch it directly with no persistence involved ---
    let rewritten_board = MemoryBoard::<C>::new();
    rewritten_board.push(ProtocolMessage::<C>::configuration(&pm, DATE, &cfg));
    let colliding = ProtocolMessage::<C>::shares(&trustee, DATE, cfg_hash, &vec![4u8, 5, 6]);
    rewritten_board.push(colliding);

    // --- restart: reopen persistence, reconnect, update must halt ---
    let persistence = SqlitePersistence::open(&db_path)?;
    let mut client =
        BoardClient::connect(MemoryTransport::new(rewritten_board.clone()), persistence).await?;
    let result = client.update().await;

    let _ = std::fs::remove_file(&db_path);
    assert!(
        result.is_err(),
        "reloaded committed predicate must block the rewrite"
    );
    Ok(())
}

/// The own-post record is durable (§6.4): a client that reconnects to the same
/// persistence reloads what it has already staged, so compute-once survives a
/// restart. This is the case that matters most, since a crash is a prime reason
/// an acknowledgement was never observed — and the state it leaves behind is
/// exactly the dangerous one: the message is on the board, but this trustee's
/// own view shows its slot unfilled, so the datalog re-enables the action.
#[tokio::test]
async fn own_post_record_survives_restart() -> Result<()> {
    run_own_post_restart::<RistrettoCtx>().await
}

async fn run_own_post_restart<C: Context>() -> Result<()> {
    let db_path =
        std::env::temp_dir().join(format!("braid_own_post_{}.sqlite", std::process::id()));
    let _ = std::fs::remove_file(&db_path);

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
    let board = MemoryBoard::<C>::new();
    board.push(ProtocolMessage::<C>::configuration(&pm, DATE, &cfg));

    let sk = signing_keys.into_iter().next().unwrap();
    let trustee = Trustee::<C>::new("1".to_string(), sk, KeyPair::<C>::generate(), &cfg)?;
    let first = ProtocolMessage::<C>::shares(&trustee, DATE, cfg_hash, &vec![1u8, 2, 3]);
    let first_bytes = first.ser();

    // First run: stage and record a sharing. b4 holds it but never serves it back.
    {
        let mut client = BoardClient::connect(
            MemoryTransport::new(board.clone()),
            SqlitePersistence::open(&db_path)?,
        )
        .await?;
        client.post(vec![first]).await?;
        assert_eq!(client.own_posts().len(), 1, "the slot is recorded");
    }

    // Restart against the same database.
    let mut client = BoardClient::connect(
        MemoryTransport::new(board.clone()),
        SqlitePersistence::open(&db_path)?,
    )
    .await?;
    assert_eq!(
        client.own_posts().len(),
        1,
        "the own-post record must be reloaded on restart"
    );

    // The datalog would re-enable ComputeShares here (the slot reads unfilled in
    // this client's store), so the action layer produces a fresh artifact. It
    // must not reach the board.
    let recomputed = ProtocolMessage::<C>::shares(&trustee, DATE, cfg_hash, &vec![7u8, 8, 9]);
    client.post(vec![recomputed]).await?;

    let snapshot = board.snapshot();
    assert_eq!(
        snapshot.len(),
        2,
        "after a restart the recomputed artifact must still be suppressed"
    );
    assert_eq!(
        snapshot[1].ser(),
        first_bytes,
        "the board still holds the originally recorded message"
    );

    let _ = std::fs::remove_file(&db_path);
    Ok(())
}
