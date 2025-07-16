// SPDX-FileCopyrightText: 2025 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::plugins_manager::plugin::HookValue;
use crate::services::plugins_manager::plugin_manager::PluginManager;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;

// This module defines the hooks implementation for the plugin system.
// Each plugin hook is a method that can be called by the plugin manager to interact with plugins.
#[async_trait]
pub trait PluginHooks {
    //Add plugins hooks here
}

#[async_trait]
impl PluginHooks for PluginManager {
    //Implement the PluginHooks trait for PluginManager
}
