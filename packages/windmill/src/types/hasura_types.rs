// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

#![allow(non_camel_case_types)]

//! Rust shapes for GraphQL types where Hasura exposes `PostgreSQL` scalars.

use serde_json::Value;

/// GraphQL / Hasura `uuid` column serialized as a string in JSON payloads.
pub type uuid = String;
/// GraphQL / Hasura `jsonb` column as arbitrary JSON ([`serde_json::Value`]).
pub type jsonb = Value;
/// RFC 3339 timestamp string from a `timestamptz` column.
pub type timestamptz = String;
/// Hex or base64-encoded binary from a `bytea` column.
pub type bytea = String;
/// Unbounded text column (`text`).
pub type text = String;
/// Bounded text column (`varchar`).
pub type varchar = String;
/// Numeric column exposed as a floating-point value in generated types.
pub type numeric = f64;
