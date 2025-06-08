// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Context, Result};
use dashmap::DashMap;
use extism::{Manifest, Plugin, Wasm};
use once_cell::sync::OnceCell;
use sequent_core::services::s3::get_file_from_s3;
use std::sync::Arc;
use tokio::sync::Mutex;

type SharedPlugin = Arc<Mutex<extism::Plugin>>;

#[derive(Debug)]
pub struct PluginManager {
    cache: dashmap::DashMap<String, SharedPlugin>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            cache: DashMap::new(),
        }
    }

    pub async fn load(&self, plugin_name: &str) -> Result<SharedPlugin> {
        if let Some(p) = self.cache.get(plugin_name) {
            return Ok(p.clone());
        }

        let bytes = get_file_from_s3(
            "public".to_string(),
            format!("extensions/{}.wasm", plugin_name),
        )
        .await
        .context("error at gettring wasm file from s3")?;

        let wasm = Wasm::data(bytes);
        let plugin = Plugin::new(Manifest::new([wasm]), [], true)?;

        let shared = Arc::new(Mutex::new(plugin));
        self.cache.insert(plugin_name.to_string(), shared.clone());
        Ok(shared)
    }
}

static EXT_PLUGIN_MANAGER: OnceCell<PluginManager> = OnceCell::new();

pub fn get_plugin_manager() -> &'static PluginManager {
    EXT_PLUGIN_MANAGER.get().expect("Registry not initialised")
}

pub fn init_plugin_manager() -> Result<()> {
    EXT_PLUGIN_MANAGER
        .set(PluginManager::new())
        .map_err(|_| anyhow!("PluginManager already initialized"))
}

pub fn ensure_registry() -> &'static PluginManager {
    EXT_PLUGIN_MANAGER.get_or_init(|| PluginManager::new())
}
