// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! A trustee session: one [`Trustee`] bound to one [`BoardClient`],
//! driving the **update-first** cycle (§6 of `crates/braid/v0.6_spec.md`).
//!
//! A `Session` is the reusable per-trustee unit. The cycle is: the board client
//! updates from b4 → the (pure) trustee `step`s over the resulting view → the
//! board client posts the produced messages → repeat. A produced message takes
//! no local effect until it loops back on a later `update` (§6), so the trustee
//! never advances on its own unconfirmed output.
//!
//! The phases are exposed separately ([`update`](Session::update),
//! [`step`](Session::step), [`post`](Session::post)) so a multi-trustee harness
//! can, e.g., run the CPU-bound `step`s in parallel while keeping the async
//! transport phases sequential; [`advance`](Session::advance) bundles one full
//! cycle for the single-session case.

use anyhow::Result;

use cryptography::context::Context;

use crate::messages::wire::ProtocolMessage;

use crate::board::persistence::Persistence;
use crate::board::transport::Transport;
use crate::board::BoardClient;
use crate::trustee::Trustee;

/// One trustee driving one board.
pub struct Session<C: Context, T: Transport<C>, P: Persistence> {
    pub trustee: Trustee<C>,
    pub client: BoardClient<C, T, P>,
}

impl<C: Context, T: Transport<C>, P: Persistence> Session<C, T, P> {
    pub fn new(trustee: Trustee<C>, client: BoardClient<C, T, P>) -> Self {
        Self { trustee, client }
    }

    /// Phase 1: pull the latest board state from b4 into the board client.
    pub async fn update(&mut self) -> Result<()> {
        self.client.update().await
    }

    /// Phase 2: run inference over the current view and produce this trustee's
    /// next messages. Pure and side-effect-free — nothing is posted or stored.
    pub fn step(&self) -> Result<Vec<ProtocolMessage<C>>> {
        self.trustee.step(self.client.view())
    }

    /// Phase 3: post the produced messages to b4 (no local effect until they loop
    /// back on the next `update`).
    pub async fn post(&mut self, messages: Vec<ProtocolMessage<C>>) -> Result<()> {
        self.client.post(messages).await
    }

    /// One full update-first cycle; returns whether this trustee produced
    /// anything.
    pub async fn advance(&mut self) -> Result<bool> {
        self.update().await?;
        let produced = self.step()?;
        let produced_any = !produced.is_empty();
        self.post(produced).await?;
        Ok(produced_any)
    }
}
