// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

pub mod templates;
pub mod wasm_hasura_types;
pub mod wasm_interpret_plaintext;
pub mod wasm_keycloak;
pub mod wasm_permissions;
pub mod wasm_plaintext;

#[cfg(feature = "wasmtest")]
pub mod areas;

#[cfg(feature = "wasmtest")]
pub mod wasm;
