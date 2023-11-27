// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
#[cfg(feature = "wasm")]
pub mod wasm_ballot;

#[cfg(feature = "wasm")]
pub mod wasm_plaintext;

#[cfg(feature = "wasm")]
pub mod wasm_interpret_plaintext;

#[cfg(feature = "wasm")]
pub mod wasm_permissions;

#[cfg(feature = "wasm")]
pub mod wasm_keycloak;

#[cfg(feature = "wasm")]
pub mod wasm_hasura_types;

#[cfg(feature = "wasmtest")]
pub mod test;
