// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The unified board client (§6 of `crates/braid/v0.6_spec.md`).
//!
//! One component with the three §6 responsibilities: (a) the in-memory
//! [`MessageStore`], (b) predicate [`persistence`] (anti-rewrite), and (c) b4
//! interaction via a [`transport`]. The orchestration is written once here; the
//! persistence and transport backends are swappable (in-memory / SQLite / HTTP+S3
//! / IndexedDB) so the same logic serves M1 → M3.
//!
//! The client enforces the **update-first / loop-back** rule (§6): a message a
//! trustee produces has no local effect until it is fetched back, verified,
//! digest-persisted, and only then admitted to the store. The trustee reads the
//! store through the [`MessageStore`] returned by [`BoardClient::view`].

pub mod persistence;
pub mod transport;

// The live HTTP+S3 transport is native-only (reqwest); the in-memory transport
// (in `transport`) is platform-agnostic.
#[cfg(feature = "native")]
pub mod http_transport;

use anyhow::{bail, Result};

use cryptography::context::Context;

use b4::messages::artifact::Configuration;
use b4::messages::wire::WireMessage;

use crate::messages::predicate::Predicate;
use crate::messages::store::MessageStore;
use crate::messages::verify::verify;
use persistence::Persistence;
use transport::Transport;

/// The unified board client: owns the in-memory store, a persistence backend, and
/// a transport to b4. A constructed client always holds a `Configuration` (§9.8).
pub struct BoardClient<C: Context, T: Transport<C>, P: Persistence> {
    store: MessageStore<C>,
    transport: T,
    persistence: P,
    /// The durable set of predicates b4 has committed to (§6.2). Loaded on
    /// [`connect`](Self::connect) and grown by [`update`](Self::update); its sole
    /// purpose is the boundary anti-rewrite check (§6.3).
    committed: Vec<Predicate>,
}

impl<C: Context, T: Transport<C>, P: Persistence> BoardClient<C, T, P> {
    /// Construct against a board: fetch + accept its `Configuration` (§9.8
    /// post-condition — no Configuration ⇒ construction fails).
    ///
    /// Restart (loading persisted anti-rewrite predicates and re-checking them on
    /// re-fetch) is an M2 concern; `NoOpPersistence::load` returns empty.
    pub async fn connect(transport: T, persistence: P) -> Result<Self> {
        let configuration_message = transport.fetch_configuration().await?;
        let store = MessageStore::from_configuration_message(&configuration_message)?;
        let committed = persistence.load().await?;
        Ok(Self {
            store,
            transport,
            persistence,
            committed,
        })
    }

    /// Update-first (§6): fetch from b4, verify each message, persist its digest,
    /// and only then admit it to the in-memory store. Idempotent — the store is a
    /// set keyed by predicate — so a full re-fetch each call is safe (§12).
    pub async fn update(&mut self) -> Result<()> {
        let messages = self.transport.fetch().await?;
        for message in &messages {
            let (predicate, body) = verify(message, self.store.configuration())?;
            // Anti-rewrite boundary check (§6.3): a freshly fetched predicate must
            // never collide with one b4 already committed to. Signatures were just
            // re-verified by `verify`; this is the additional, durable layer that
            // forbids b4 from filling a slot with a different body across restarts.
            if let Some(prior) = self.committed.iter().find(|p| p.collides(&predicate)) {
                bail!(
                    "anti-rewrite violation: fetched predicate {:?} collides with committed {:?}",
                    predicate,
                    prior
                );
            }
            self.persistence.persist(&predicate).await?;
            if !self.committed.contains(&predicate) {
                self.committed.push(predicate.clone());
            }
            self.store.insert(predicate, body)?;
        }
        Ok(())
    }

    /// Post messages to b4. Per the loop-back rule they take no local effect here;
    /// they become visible only after a subsequent [`update`](Self::update).
    pub async fn post(&mut self, messages: Vec<WireMessage<C>>) -> Result<()> {
        self.transport.post(messages).await
    }

    /// The accepted board `Configuration`.
    pub fn configuration(&self) -> &Configuration<C> {
        self.store.configuration()
    }

    /// The read view the trustee's `step` consumes.
    pub fn view(&self) -> &MessageStore<C> {
        &self.store
    }
}

#[cfg(all(test, feature = "native"))]
mod tests {
    use std::marker::PhantomData;

    use anyhow::Result;

    use cryptography::context::{Context, RistrettoCtx};
    use cryptography::cryptosystem::elgamal::KeyPair;
    use cryptography::utils::signatures::SignatureScheme;

    use b4::messages::artifact::Configuration;
    use b4::messages::newtypes::ConfigurationHash;
    use b4::messages::protocol_manager::ProtocolManager;
    use b4::messages::wire::WireMessage;

    use crate::board::persistence::SqlitePersistence;
    use crate::board::transport::{MemoryBoard, MemoryTransport};
    use crate::board::BoardClient;
    use crate::runtime::SessionTrustee;

    const DATE: b4::messages::newtypes::Timestamp = 0;

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
            PhantomData,
        )
        .with_share_encryption_keys(share_enc_keys);
        let cfg_hash = ConfigurationHash::from_configuration(&cfg)?;
        let cfg_message = WireMessage::<C>::configuration(&pm, DATE, &cfg);

        let board = MemoryBoard::<C>::new();
        board.push(cfg_message);

        // --- first run: post a Shares, update (persists its predicate) ---
        let mut first_sk = signing_keys.into_iter();
        let trustee = {
            let transport = MemoryTransport::new(board.clone());
            let client =
                BoardClient::connect(transport, SqlitePersistence::open(&db_path)?).await?;
            let trustee = SessionTrustee::<C>::new(
                "1".to_string(),
                first_sk.next().unwrap(),
                KeyPair::<C>::generate(),
                client.configuration(),
            )?;
            let mut client = client;
            let shares = WireMessage::<C>::shares(&trustee, DATE, cfg_hash, &vec![1u8, 2, 3]);
            client.post(vec![shares]).await?;
            client.update().await?;
            trustee
            // client dropped here (and with it the in-memory committed set)
        };

        // --- b4 is asked to rewrite the slot: a colliding Shares from the same
        //     trustee with a different body appears on the board ---
        let colliding = WireMessage::<C>::shares(&trustee, DATE, cfg_hash, &vec![4u8, 5, 6]);
        board.push(colliding);

        // --- restart: reopen persistence, reconnect, update must halt ---
        let persistence = SqlitePersistence::open(&db_path)?;
        let mut client =
            BoardClient::connect(MemoryTransport::new(board.clone()), persistence).await?;
        let result = client.update().await;

        let _ = std::fs::remove_file(&db_path);
        assert!(
            result.is_err(),
            "reloaded committed predicate must block the rewrite"
        );
        Ok(())
    }
}
