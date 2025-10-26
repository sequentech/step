// SPDX-FileCopyrightText: 2025 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::plugins_manager::plugin::HookValue;
use crate::services::plugins_manager::plugin_manager::PluginManager;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::Value;

// This module defines the hooks implementation for the plugin system.
// Each plugin hook is a method that can be called by the plugin manager to interact with plugins.
#[async_trait]
pub trait PluginHooks {
    //Add plugins hooks here
    async fn create_transmission_package(&self, input: Value) -> Result<()>;
}

#[async_trait]
impl PluginHooks for PluginManager {
    //Implement the PluginHooks trait for PluginManager
    async fn create_transmission_package(&self, input: Value) -> Result<()> {
        let res: Vec<Vec<HookValue>> = self
            .call_hook(
                "create-transmission-package",
                vec![HookValue::String(input.to_string())],
                vec![HookValue::Result(core::result::Result::Ok(None))],
            )
            .await
            .map_err(|e| anyhow!("Failed to call plugin hook: {}", e))?;

        let result = &res[0];
        if let Some(result_hook_value) = result.get(0) {
            match result_hook_value {
                HookValue::Result(Ok(out)) => Ok(()),
                HookValue::Result(Err(Some(e))) => match &**e {
                    HookValue::String(e) => Err(anyhow!("Plugin hook error: {}", e)),
                    _ => Err(anyhow!("Error executing plugin hook",)),
                },
                _ => Err(anyhow!("Unexpected hook value type")),
            }
        } else {
            Err(anyhow!("No hook value returned"))
        }
    }
}
