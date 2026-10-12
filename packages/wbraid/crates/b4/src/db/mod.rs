// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
//! b4 storage: a dumb, board-agnostic blob store (§8 of `crates/braid/v0.6_spec.md`).
//!
//! b4 stores each message as **opaque bytes** — it does NOT interpret contents.
//! There is no parent/child lineage (the union is a client concern, §8.2), no
//! slot `UNIQUE` and no protocol metadata (`sender_pk`/`statement_kind`/... are
//! gone; the slot lives only in datalog `collides()`, §5). Boards are independent;
//! messages are ordered per board by the autoincrement `id`. The `version` string
//! is retained for the exact-match boundary check (§10.1); the `inline_data`/
//! `s3_key` split is a pure transport detail (§8.1).
pub mod common;
#[cfg(feature = "postgres")]
mod postgres;
#[cfg(feature = "sqlite")]
mod sqlite;
mod utils;

pub use common::{open_from_env, Board, BoardDb};
