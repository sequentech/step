// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The trustee's verified-message layer (v0.6).
//!
//! This is the braid-side counterpart to `b4::messages`. b4 owns the wire
//! *format* — the [`WireMessage`](b4::messages::wire::WireMessage) structure,
//! the per-type heads, signing, and the `statement_bytes` byte layout — plus the
//! availability-only bulletin-board server. braid owns the *interpretation*: it
//! checks signatures against the configuration and reconstructs the datalog
//! [`Predicate`](predicate::Predicate)s that drive the protocol.
//!
//! - [`predicate`]: the typed, content-addressed statements (the datalog EDB)
//!   and the slot/collision logic (§4.2, §5.1).
//! - [`verify`]: the trust boundary — `WireMessage` -> `Predicate` (§3.4).
//! - [`store`]: the in-memory message store, the pure core of the board client
//!   and the source of the datalog EDB (§6.1).

pub mod predicate;
pub mod store;
pub mod verify;
