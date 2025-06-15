// // SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
// //
// // SPDX-License-Identifier: AGPL-3.0-only
use crate::services::plugins::plugin_manager::PluginManager;
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct AddRequest {
    pub a: i64,
    pub b: i64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AddResponse {
    pub result: i64,
}

#[async_trait]
pub trait PluginHooks {
    async fn add_hook(&self, a: i64, b: i64) -> Result<i64, anyhow::Error>;
}

#[async_trait::async_trait]
impl PluginHooks for PluginManager {
    async fn add_hook(&self, a: i64, b: i64) -> Result<i64> {
        let payload = AddRequest { a, b };
        let result: AddResponse = self.call_hook("add_hook", &payload).await?;
        Ok(result.result)
    }
}
