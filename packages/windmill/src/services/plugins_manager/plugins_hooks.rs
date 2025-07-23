// SPDX-FileCopyrightText: 2025 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::plugins_manager::plugin::HookValue;
use crate::services::plugins_manager::plugin_manager::PluginManager;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use serde_json::Value;

// This module defines the hooks implementation for the plugin system.
// Each plugin hook is a method that can be called by the plugin manager to interact with plugins.
#[async_trait]
pub trait PluginHooks {
    //Add plugins hooks here
    async fn create_transmission_package(&self, input: Value) -> Result<String>;
}

#[async_trait]
impl PluginHooks for PluginManager {
    //Implement the PluginHooks trait for PluginManager
    async fn create_transmission_package(&self, input: Value) -> Result<String> {
        let res = self
            .call_hook(
                "create-transmission-package",
                vec![HookValue::String(input.to_string())],
                vec![HookValue::Result(core::result::Result::Ok(None))],
            )
            .await
            .map_err(|e| anyhow!("Failed to call plugin hook: {}", e))?;

        Ok("".to_string()) //TODO: return error if needed
    }
}
