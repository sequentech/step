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

use anyhow::Result;

use cryptography::context::Context;

use b4::messages::artifact::Configuration;
use b4::messages::wire::WireMessage;

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
        Ok(Self {
            store,
            transport,
            persistence,
        })
    }

    /// Update-first (§6): fetch from b4, verify each message, persist its digest,
    /// and only then admit it to the in-memory store. Idempotent — the store is a
    /// set keyed by predicate — so a full re-fetch each call is safe (§12).
    pub async fn update(&mut self) -> Result<()> {
        let messages = self.transport.fetch().await?;
        for message in &messages {
            let (predicate, body) = verify(message, self.store.configuration())?;
            self.persistence.persist(&predicate).await?;
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
