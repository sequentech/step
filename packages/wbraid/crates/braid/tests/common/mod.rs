// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Fixtures shared by the model-checking harnesses (`model_check.rs`,
//! `model_check_symbolic.rs`). Each harness compiles this module into its own
//! test crate (`mod common;`).

use anyhow::Result;
use async_trait::async_trait;

use braid::board::persistence::Persistence;
use braid::board::transport::StagedRef;
use braid::messages::predicate::Predicate;

/// In-memory persistence whose contents can be **seeded and read back**.
///
/// Durable in the sense that matters for a model-checking harness: the store
/// lives outside the board client, so a client can be dropped and a new one
/// connected over the same contents — a restart, without a filesystem. That is
/// what lets a checker keep each trustee's durable state in its own state
/// value and rehydrate a real `BoardClient` per transition.
///
/// The handle is cheap to clone and shares one store, so a harness keeps a
/// clone to snapshot after driving a cycle. Test-only: the production backends
/// are SQLite (M2) and IndexedDB (M3), with `NoOpPersistence` for M1.
#[derive(Clone, Default)]
pub struct MemoryPersistence {
    inner: std::rc::Rc<std::cell::RefCell<MemoryPersistenceInner>>,
}

#[derive(Clone, Default)]
struct MemoryPersistenceInner {
    predicates: Vec<Predicate>,
    own_posts: Vec<(Predicate, StagedRef)>,
}

impl MemoryPersistence {
    /// Seeded with previously persisted contents (a restart).
    pub fn restored(predicates: Vec<Predicate>, own_posts: Vec<(Predicate, StagedRef)>) -> Self {
        Self {
            inner: std::rc::Rc::new(std::cell::RefCell::new(MemoryPersistenceInner {
                predicates,
                own_posts,
            })),
        }
    }

    /// The current contents: `(committed predicates, own-post record)`.
    pub fn snapshot(&self) -> (Vec<Predicate>, Vec<(Predicate, StagedRef)>) {
        let inner = self.inner.borrow();
        (inner.predicates.clone(), inner.own_posts.clone())
    }
}

#[async_trait(?Send)]
impl Persistence for MemoryPersistence {
    async fn load(&self) -> Result<Vec<Predicate>> {
        Ok(self.inner.borrow().predicates.clone())
    }

    async fn persist(&mut self, predicate: &Predicate) -> Result<()> {
        let mut inner = self.inner.borrow_mut();
        if !inner.predicates.contains(predicate) {
            inner.predicates.push(predicate.clone());
        }
        Ok(())
    }

    async fn load_own_posts(&self) -> Result<Vec<(Predicate, StagedRef)>> {
        Ok(self.inner.borrow().own_posts.clone())
    }

    async fn persist_own_post(&mut self, predicate: &Predicate, staged: &StagedRef) -> Result<()> {
        let mut inner = self.inner.borrow_mut();
        if !inner.own_posts.iter().any(|(p, _)| p == predicate) {
            inner.own_posts.push((predicate.clone(), staged.clone()));
        }
        Ok(())
    }
}
