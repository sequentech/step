// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! WASM plugin runtime: plugin loading, hook dispatch, and plugin-managed transactions.

pub mod plugin;
pub mod plugin_db_manager;
pub mod plugin_manager;
pub mod plugins_hooks;
