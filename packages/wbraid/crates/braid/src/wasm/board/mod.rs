// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! WASM-specific board implementations

pub mod http;
pub mod storage_indexeddb;

pub use http::{WasmHttpBoard, WasmHttpBoardFactory, WasmHttpBoardParams};
pub use storage_indexeddb::IndexedDbStorage;

