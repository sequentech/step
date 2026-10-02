// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Voter messaging shared by harvest and windmill: provider adapters for
//! email, SMS, WhatsApp, Viber and Messenger, the decisions about delivery
//! attempts, sending rates, Messenger link references and the verification
//! of provider webhooks.
//!
//! Persistence stays with the callers: this crate decides and talks to
//! providers, and never stores message bodies or codes.

pub mod attempts;
pub mod destination;
pub mod link;
pub mod providers;
pub mod rate_limit;
pub mod sender;
pub mod webhooks;

#[cfg(test)]
mod test_server;
