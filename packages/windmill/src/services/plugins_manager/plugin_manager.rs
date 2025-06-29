// SPDX-FileCopyrightText: 2024 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::plugins_manager::plugin::{HookParam, Plugin, PluginCtx};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use dashmap::DashMap;
use sequent_core::services::s3::{get_files_bytes_from_s3, get_public_bucket};
use std::sync::Arc;
use wasmtime::component::{Linker, Val};
use wasmtime::{Config, Engine};

pub struct PluginManager {
    pub plugins: DashMap<String, Arc<Plugin>>,
    pub hook_index: DashMap<String, String>,
    pub engine: Engine,
    pub linker: Linker<PluginCtx>,
}

impl PluginManager {
    pub fn new() -> Result<Self> {
        let mut config = Config::new();
        config.wasm_component_model(true);
        let engine = Engine::new(&config)?;
        let linker = Linker::new(&engine);

        Ok(Self {
            plugins: DashMap::new(),
            hook_index: DashMap::new(),
            engine,
            linker,
        })
    }

    pub async fn load_plugins(&self) -> Result<()> {
        let bucket = get_public_bucket().context("failed to get public S3 bucket")?;
        let blobs = get_files_bytes_from_s3(bucket, "plugins/".into()).await?;

        for wasm in blobs {
            let plugin =
                Plugin::from_wasm_bytes(&self.engine, &mut self.linker.clone(), wasm).await?;
            let plugin_name = plugin.name.clone();

            self.hook_index
                .insert("add".to_string(), plugin_name.clone()); // hardcoded for demo
            self.plugins.insert(plugin_name, Arc::new(plugin));
        }

        Ok(())
    }

    pub async fn call_hook_dynamic(
        &self,
        hook: &str,
        params: Vec<HookParam>,
        result_count: usize,
    ) -> Result<Vec<Val>> {
        let plugin_name = self.hook_index.get(hook).context("hook not registered")?;
        let plugin = self
            .plugins
            .get(plugin_name.value())
            .context("plugin not found")?;

        plugin.call_hook_dynamic(hook, params, result_count).await
    }
}

#[async_trait]
pub trait PluginHooks {
    async fn add(&self, a: i32, b: i32) -> Result<i32>;
}

#[async_trait]
impl PluginHooks for PluginManager {
    async fn add(&self, a: i32, b: i32) -> Result<i32> {
        let results = self
            .call_hook_dynamic("add", vec![a.into(), b.into()], 1)
            .await?;

        match results.get(0) {
            Some(Val::S32(sum)) => Ok(*sum),
            Some(Val::U32(sum)) => Ok(*sum as i32),
            _ => Err(anyhow!("Unexpected return type from 'add'")),
        }
    }
}
