// SPDX-FileCopyrightText: 2024 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::plugins_manager::plugin::HookValue;
use crate::services::plugins_manager::plugin_manager::PluginManager;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;

#[async_trait]
pub trait PluginHooks {
    async fn add(&self, a: i32, b: i32) -> Result<i32>;
}

#[async_trait]
impl PluginHooks for PluginManager {
    async fn add(&self, a: i32, b: i32) -> Result<i32> {
        let results = self
            .call_hook_dynamic("add", vec![a.into(), b.into()], vec![HookValue::S32(0)])
            .await?;

        Ok(results[0].as_i32().context("Expected i32 result")?)
    }
}
