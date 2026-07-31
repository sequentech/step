// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The complete protocol message stack (v0.6).
//!
//! This module holds every layer of the protocol's message model:
//!
//! - [`newtypes`]: content-addressed hash wrappers and index types.
//! - [`artifact`]: the `Configuration` artifact and related domain types.
//! - [`wire`]: the [`ProtocolMessage`](wire::ProtocolMessage) structure, per-type
//!   heads, signing, and the `statement_bytes` byte layout.
//! - [`sender`]: the sender/signer identity types.
//! - [`protocol_manager`]: protocol-manager identity and key management.
//! - [`predicate`]: the typed, content-addressed statements (the datalog EDB)
//!   and the slot/collision logic (§4.2, §5.1).
//!
//! The message *store* and *verification* (`ProtocolMessage` → `Predicate`,
//! §3.4) are board-client operations, not message vocabulary, and live in
//! [`crate::board`] (`store`, `verify`) instead.

pub mod newtypes;
pub mod artifact;
pub mod sender;
pub mod protocol_manager;
pub mod wire;

pub mod predicate;
