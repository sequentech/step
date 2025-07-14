// SPDX-FileCopyrightText: 2024 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::plugins_manager::plugin::HookValue;
use crate::services::plugins_manager::plugin_manager::PluginManager;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;

#[async_trait]
pub trait PluginHooks {
    async fn add(&self, a: u32, b: u32) -> Result<String>;
}

#[async_trait]
impl PluginHooks for PluginManager {
    async fn add(&self, a: u32, b: u32) -> Result<String> {
        let res: Vec<Vec<HookValue>> = self
            .call_hook_dynamic(
                "add",
                vec![HookValue::U32(a), HookValue::U32(b)],
                vec![HookValue::String("".to_string())],
            )
            .await
            .map_err(|e| anyhow!("Failed to call hook 'add': {}", e))?;

        let result: Option<&str> = res[0][0].as_str();
        let result = result.ok_or_else(|| anyhow!("Hook 'add' did not return a string"))?;
        Ok(result.to_string())
    }
}
