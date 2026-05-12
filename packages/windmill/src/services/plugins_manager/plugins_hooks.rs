// SPDX-FileCopyrightText: 2025 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
//! Convenience trait that exposes strongly-typed plugin hooks on [`PluginManager`].
use crate::services::plugins_manager::plugin::HookValue;
use crate::services::plugins_manager::plugin_manager::PluginManager;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::Value;

/// This module defines the hooks implementation for the plugin system.
/// Each plugin hook is a method that can be called by the plugin manager to interact with plugins.
#[async_trait]
pub trait PluginHooks {
    /// Calls the `create-transmission-package` hook and returns its string result.
    async fn create_transmission_package(&self, input: Value) -> Result<String>;
}

#[async_trait]
impl PluginHooks for PluginManager {
    /// Calls the corresponding plugin hook and unwraps the first plugin result as a string.
    ///
    /// # Errors
    ///
    /// Returns an error if the hook call fails or returns an unexpected shape.
    async fn create_transmission_package(&self, input: Value) -> Result<String> {
        let res: Vec<Vec<HookValue>> = self
            .call_hook(
                "create-transmission-package",
                vec![HookValue::String(input.to_string())],
                vec![HookValue::Result(core::result::Result::Ok(None))],
            )
            .await
            .map_err(|e| anyhow!("Failed to call plugin hook: {e}"))?;

        let result = res
            .first()
            .ok_or_else(|| anyhow!("Plugin hook returned no results"))?;
        if let Some(result_hook_value) = result.first() {
            match result_hook_value {
                HookValue::Result(Ok(Some(boxed_value))) => match &**boxed_value {
                    HookValue::String(value) => Ok(value.clone()),
                    _ => Err(anyhow!("Unexpected boxed hook value type")),
                },
                HookValue::Result(Ok(None)) => Err(anyhow!("No value returned from plugin hook")),
                HookValue::Result(Err(Some(e))) => match &**e {
                    HookValue::String(e) => Err(anyhow!("Plugin hook error: {e}")),
                    _ => Err(anyhow!("Error executing plugin hook",)),
                },
                _ => Err(anyhow!("Unexpected hook value type")),
            }
        } else {
            Err(anyhow!("No hook value returned"))
        }
    }
}
