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

use b4::messages::wire::{MessageType, WireMessage};

/// b4 transport: fetch the board's Configuration + protocol messages, post new
/// ones. Send + Sync so a board client can be driven from async tasks / used
/// across the harness's parallel step.
#[async_trait]
pub trait Transport<C: Context>: Send + Sync {
    /// The board's `Configuration` message (consumed once at construction, §9.8).
    async fn fetch_configuration(&self) -> Result<WireMessage<C>>;
    /// All protocol (non-`Configuration`) messages currently on the board.
    async fn fetch(&self) -> Result<Vec<WireMessage<C>>>;
    /// Post messages to the board.
    async fn post(&self, messages: Vec<WireMessage<C>>) -> Result<()>;
}

/// In-memory stand-in for b4 (M1 + tests): an ordered, shared message log. Every
/// trustee's [`MemoryTransport`] points at one shared `MemoryBoard`; the harness
/// seeds the `Configuration` and posts the manager's `Ballots` directly.
pub struct MemoryBoard<C: Context> {
    messages: Mutex<Vec<WireMessage<C>>>,
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
    pub fn push(&self, message: WireMessage<C>) {
        self.lock().push(message);
    }

    /// A copy of every message currently on the board (used by the harness to
    /// read off results).
    pub fn snapshot(&self) -> Vec<WireMessage<C>> {
        self.lock().clone()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Vec<WireMessage<C>>> {
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

#[async_trait]
impl<C: Context> Transport<C> for MemoryTransport<C> {
    async fn fetch_configuration(&self) -> Result<WireMessage<C>> {
        self.board
            .snapshot()
            .into_iter()
            .find(|m| m.message_type == MessageType::Configuration)
            .ok_or_else(|| anyhow!("board has no Configuration message"))
    }

    async fn fetch(&self) -> Result<Vec<WireMessage<C>>> {
        Ok(self
            .board
            .snapshot()
            .into_iter()
            .filter(|m| m.message_type != MessageType::Configuration)
            .collect())
    }

    async fn post(&self, messages: Vec<WireMessage<C>>) -> Result<()> {
        self.board.lock().extend(messages);
        Ok(())
    }
}
