// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! WebAssembly bindings and TypeScript type mirrors for frontend packages.

/// Template and communication TypeScript extern types.
pub mod templates;
/// Hasura-related types exposed to JavaScript.
pub mod wasm_hasura_types;
/// Plaintext interpretation screen state TypeScript extern types.
pub mod wasm_interpret_plaintext;
/// Keycloak user/role TypeScript extern types.
pub mod wasm_keycloak;
/// Permission TypeScript extern types.
pub mod wasm_permissions;
/// Decoded vote and plaintext error TypeScript extern types.
pub mod wasm_plaintext;

/// Area tree helpers for the voting portal WASM test build.
#[cfg(feature = "wasmtest")]
pub mod areas;

/// Ballot encryption, hashing, and voting-screen WASM exports.
#[cfg(feature = "wasmtest")]
pub mod wasm;
