// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! # b4 — the untrusted bulletin board
//!
//! b4 is the **untrusted** bulletin board the braid mixnet trustees exchange
//! messages through (§8 of `crates/braid/v0.6_spec.md` — the authoritative
//! design doc; this crate carries no design doc of its own). b4 is trusted for
//! **availability only**: it stores and serves opaque bytes and never
//! interprets, parses, or enforces anything about their contents — no protocol
//! schema, no slot/uniqueness checks, no parent/child board lineage. All of
//! that safety logic lives client-side, in braid's `board` and `datalog`
//! modules. A dishonest b4 can therefore only withhold or serve stale data,
//! never forge a message or get away with silently rewriting history.
//!
//! ## Design
//!
//! - **Boards are independent.** Each is just a name; there is no lineage
//!   column (a tally-over-DKG union, where it exists, is composed client-side).
//! - **Messages are opaque, autoincrement-ordered blobs**, keyed by
//!   `(board_name, id)`, plus a `version` string for the exact-match schema
//!   boundary check — the only fields b4 itself interprets.
//! - **Small messages stay inline; large ones offload to S3** via a two-step
//!   upload flow (initiate → client uploads → confirm) — a pure transport
//!   detail with no consequence for verification, since fetching always
//!   returns the whole message.
//!
//! ## Module map
//!
//! | Module | Role |
//! |---|---|
//! | [`api_types`] | the HTTP request/response schema — shared with clients (e.g. braid's transport) |
//! | `db` (native) | SQLite persistence: boards + messages |
//! | `handlers` (native) | Axum route handlers |
//! | `s3` (native) | S3 client + presigned upload URLs |
//! | `state` (native) | shared Axum `AppState` (db pool, S3 client, bucket) |
//! | `main` (bin, native) | wires the above into the Axum router and starts the server |
//!
//! ## Feature flags
//!
//! - `native` (default) — the server implementation (Axum, sqlx/SQLite, S3).
//!   Disabling it leaves only [`api_types`], for non-native consumers (e.g. a
//!   wasm HTTP client) that need the wire schema without the server.

pub mod api_types;

// Native-only modules
#[cfg(feature = "native")]
pub mod db;
#[cfg(feature = "native")]
pub mod handlers;
#[cfg(feature = "native")]
pub mod s3;
#[cfg(feature = "native")]
pub mod state;
