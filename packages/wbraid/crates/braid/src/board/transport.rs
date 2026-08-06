// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! b4 interaction (§6c of `crates/braid/v0.6_spec.md`): the swappable transport
//! behind the board client.
//!
//! The board client's orchestration (verify → persist → admit; post) is written
//! once against this trait; the concrete backend is in-memory (M1 + tests),
//! HTTP+S3 (M2), or browser fetch (M3). v0.6 does a **full re-fetch** each update
//! — no monotonic cursor (deferred, §12) — which is safe because the store is a
//! set keyed by predicate (idempotent).

use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Result};
use async_trait::async_trait;

use cryptography::context::Context;

use crate::messages::wire::{MessageType, ProtocolMessage};

/// b4 transport: fetch the board's Configuration + protocol messages, post new
/// ones.
///
/// `?Send` (spec Option B): the browser/wasm backends (M3) produce `!Send`
/// futures (web-sys `JsFuture`, `JsValue`), so the seam is deliberately
/// single-thread-friendly and carries no `Send`/`Sync` bound. Native
/// cross-trustee parallelism is a *harness* concern: the rayon step bounds the
/// concrete transport/persistence types with `+ Sync` where it needs them, not
/// the trait. The async I/O runs on a current-thread runtime.
#[async_trait(?Send)]
pub trait Transport<C: Context> {
    /// The board's `Configuration` message (consumed once at construction, §9.8).
    async fn fetch_configuration(&self) -> Result<ProtocolMessage<C>>;
    /// All protocol (non-`Configuration`) messages currently on the board.
    async fn fetch(&self) -> Result<Vec<ProtocolMessage<C>>>;
    /// **Stage** a message (§6.4): put the bytes where b4 will read them from,
    /// without making the message visible on the board, and return the handle
    /// needed to publish it later.
    ///
    /// This is the *persist-before-send* half of posting. Splitting it from
    /// [`commit`](Self::commit) is what lets the outgoing mailbox re-send a
    /// message it has already produced without re-uploading the body — and
    /// therefore without holding the body in local durable storage, which §6.2
    /// forbids.
    async fn stage(&self, message: &ProtocolMessage<C>) -> Result<StagedRef>;
    /// **Commit** a staged message: make it visible on the board.
    ///
    /// Safe to repeat with the same handle. A repeat may leave b4 holding two
    /// copies of identical bytes, which is protocol-identical and deduplicated on
    /// read (§8.5 Note 2).
    async fn commit(&self, staged: &StagedRef) -> Result<()>;

    /// Publish a message in one shot: `stage` then `commit`.
    ///
    /// For callers that keep no own-post record — the manager seeding a board
    /// with its `Configuration` or `Ballots`. A *trustee's* own artifacts must go
    /// through [`BoardClient::post`](crate::board::BoardClient::post) instead, so
    /// they get the mailbox discipline (§6.4).
    async fn publish(&self, message: &ProtocolMessage<C>) -> Result<()> {
        let staged = self.stage(message).await?;
        self.commit(&staged).await
    }
}

/// Opaque, persistable reference to a **staged** message (§6.4).
///
/// Whatever the transport needs to publish the message later without re-sending
/// the body: the b4 message id for HTTP/wasm (b4 reconstructs the S3 key from
/// it), a placeholder for the in-memory board. Deliberately small — it lives in
/// the durable own-post record, which must stay predicate-sized on every
/// platform (§6.2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StagedRef(pub String);

/// In-memory stand-in for b4 (M1 + tests): an ordered, shared message log. Every
/// trustee's [`MemoryTransport`] points at one shared `MemoryBoard`; the harness
/// seeds the `Configuration` and posts the manager's `Ballots` directly.
pub struct MemoryBoard<C: Context> {
    messages: Mutex<Vec<ProtocolMessage<C>>>,
}

impl<C: Context> MemoryBoard<C> {
    /// A fresh, empty shared board.
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            messages: Mutex::new(Vec::new()),
        })
    }

    /// Append a message directly (used by the harness to seed the Configuration
    /// and post the manager's Ballots).
    pub fn push(&self, message: ProtocolMessage<C>) {
        self.lock().push(message);
    }

    /// A copy of every message currently on the board (used by the harness to
    /// read off results).
    pub fn snapshot(&self) -> Vec<ProtocolMessage<C>> {
        self.lock().clone()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Vec<ProtocolMessage<C>>> {
        self.messages.lock().expect("MemoryBoard mutex poisoned")
    }
}

/// A handle onto a shared [`MemoryBoard`], implementing [`Transport`].
pub struct MemoryTransport<C: Context> {
    board: Arc<MemoryBoard<C>>,
}

impl<C: Context> MemoryTransport<C> {
    pub fn new(board: Arc<MemoryBoard<C>>) -> Self {
        Self { board }
    }
}

// Manual Clone to avoid an unwanted `C: Clone` bound (the handle just clones the
// Arc).
impl<C: Context> Clone for MemoryTransport<C> {
    fn clone(&self) -> Self {
        Self {
            board: Arc::clone(&self.board),
        }
    }
}

#[async_trait(?Send)]
impl<C: Context> Transport<C> for MemoryTransport<C> {
    async fn fetch_configuration(&self) -> Result<ProtocolMessage<C>> {
        self.board
            .snapshot()
            .into_iter()
            .find(|m| m.message_type == MessageType::Configuration)
            .ok_or_else(|| anyhow!("board has no Configuration message"))
    }

    async fn fetch(&self) -> Result<Vec<ProtocolMessage<C>>> {
        Ok(self
            .board
            .snapshot()
            .into_iter()
            .filter(|m| m.message_type != MessageType::Configuration)
            .collect())
    }

    /// The in-memory board has no staging area to separate from publication, so
    /// staging *is* the append and [`commit`](Self::commit) has nothing left to
    /// do. That collapses the two phases, which is harmless here: this backend
    /// models neither the S3/b4 split nor restarts (it is paired with
    /// `NoOpPersistence`). The externally visible behaviour the mailbox relies on
    /// is preserved — a staged message is published exactly once, and committing
    /// its handle again does not duplicate it.
    async fn stage(&self, message: &ProtocolMessage<C>) -> Result<StagedRef> {
        self.board.lock().push(message.clone());
        Ok(StagedRef("memory".to_string()))
    }

    async fn commit(&self, _staged: &StagedRef) -> Result<()> {
        Ok(())
    }
}
