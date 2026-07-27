// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Predicate persistence trait + no-op backend (§6.2 of `crates/braid/v0.6_spec.md`).
//!
//! Persisted predicates are the b4-sourced (looped-back) ones; their sole job is
//! **anti-rewrite** — they pin, irreversibly, which `H(body)` b4 has committed to
//! for each slot (§6.3). Persistence is NOT what prevents re-execution (that is
//! update-first + loop-back), and it is NOT a defense against b4 dropping a
//! message (that is availability).
//!
//! The trait is async (and `?Send`, spec Option B) so platform backends fit the
//! same shape regardless of their `Send`-ness. M1 uses [`NoOpPersistence`]:
//! nothing is persisted, and a restart therefore relies entirely on re-fetching
//! from b4 (§6.3). The concrete durable backends live in
//! `crate::native::persistence` (SQLite, M2) and `crate::wasm::persistence`
//! (IndexedDB, M3).

use anyhow::Result;
use async_trait::async_trait;

use crate::messages::predicate::Predicate;

/// One persistence backend, two media (§6.2): SQLite (native, M2) / IndexedDB
/// (wasm, M3), with [`NoOpPersistence`] for M1. `?Send` (Option B) so the wasm
/// backend's `!Send` futures fit; native parallelism bounds the concrete type
/// with `+ Sync` at the rayon call site, not the trait.
#[async_trait(?Send)]
pub trait Persistence {
    /// Load the persisted predicate set on restart (§6.3). NoOp returns empty.
    async fn load(&self) -> Result<Vec<Predicate>>;
    /// Persist a predicate digest before its body is admitted to memory (§6.2).
    async fn persist(&mut self, predicate: &Predicate) -> Result<()>;
}

/// No-op persistence (M1): nothing is persisted; restart loads nothing.
pub struct NoOpPersistence;

#[async_trait(?Send)]
impl Persistence for NoOpPersistence {
    async fn load(&self) -> Result<Vec<Predicate>> {
        Ok(Vec::new())
    }
    async fn persist(&mut self, _predicate: &Predicate) -> Result<()> {
        Ok(())
    }
}
