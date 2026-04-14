// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! This module contains the WebAssembly bindings for the Sequent Core library.
//! It includes TypeScript interfaces and enums that are used to
//! interact with the library from JavaScript/TypeScript code.

/// `wasm_bindgen` for template related types and interfaces.
pub mod templates;
/// `wasm_bindgen` for hasura related types and interfaces.
pub mod wasm_hasura_types;
/// `wasm_bindgen` for interpret plaintext related types and interfaces.
pub mod wasm_interpret_plaintext;
/// `wasm_bindgen` for Keycloak related types and interfaces.
pub mod wasm_keycloak;
/// `wasm_bindgen` for permission related types and interfaces.
pub mod wasm_permissions;
/// `wasm_bindgen` for plaintext related types and interfaces.
pub mod wasm_plaintext;

#[cfg(feature = "wasmtest")]
/// `WebAssembly` bindings exported to frontend packages for accessing area information.
pub mod areas;

#[cfg(feature = "wasmtest")]
#[allow(clippy::module_inception)]
/// `WebAssembly` bindings exported to frontend packages.
pub mod wasm;
