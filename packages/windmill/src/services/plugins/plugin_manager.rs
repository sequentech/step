// // SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
// //
// // SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Context, Result};
use dashmap::DashMap;
use extism::{Manifest, Plugin as ExtismPlugin, PluginBuilder, Wasm};
use once_cell::sync::OnceCell;
use sequent_core::services::s3::{get_files_bytes_from_s3, get_public_bucket};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;

type SharedPlugin = Arc<Mutex<ExtismPlugin>>;

#[derive(Clone)]
pub struct Plugin {
    name: String,
    hooks: Vec<String>,
    routes: DashMap<String, String>,
    inner: SharedPlugin,
}

impl Plugin {
    pub fn from_wasm_bytes(wasm_bytes: Vec<u8>) -> Result<Self> {
        let manifest = Manifest::new([Wasm::data(wasm_bytes.clone())]);
        let mut plugin = PluginBuilder::new(manifest).build()?;

        let raw: Vec<u8> = plugin.call("get_manifest", b"[]".as_ref())?;
        let manifest_json: Value = serde_json::from_slice(&raw)?;

        let name = manifest_json["name"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();
        let hooks = manifest_json["hooks"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|h| h["name"].as_str().map(String::from))
            .collect();

        let mut routes = DashMap::new();
        if let Some(route_arr) = manifest_json["routes"].as_array() {
            for r in route_arr {
                if let (Some(path), Some(handler)) = (r["path"].as_str(), r["handler"].as_str()) {
                    routes.insert(path.to_string(), handler.to_string());
                }
            }
        }

        Ok(Self {
            name,
            hooks,
            routes,
            inner: Arc::new(Mutex::new(plugin)),
        })
    }

    pub async fn call_raw(&self, hook: &str, input_json: &str) -> Result<Vec<u8>> {
        let mut locked = self.inner.lock().await;
        locked
            .call(hook, input_json.as_bytes())
            .context("Plugin call failed")
    }

    pub fn get_handler_for_route(&self, path: &str) -> Option<String> {
        self.routes.get(path).map(|s| s.clone())
    }
}

pub struct PluginManager {
    plugins: DashMap<String, Plugin>,
    hook_index: DashMap<String, String>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            plugins: DashMap::new(),
            hook_index: DashMap::new(),
        }
    }

    pub async fn load_plugins(&self) -> Result<()> {
        let s3_bucket = get_public_bucket().context("error getting public bucket")?;
        let wasm_blobs = get_files_bytes_from_s3(s3_bucket, "plugins/".into()).await?;

        for wasm in wasm_blobs {
            let plugin = Plugin::from_wasm_bytes(wasm)?;
            let plugin_name = plugin.name.clone();

            for hook in plugin.hooks.iter() {
                self.hook_index.insert(hook.clone(), plugin_name.clone());
            }

            self.plugins.insert(plugin_name, plugin);
        }

        Ok(())
    }

    pub async fn call_route(&self, path: &str, input_json: &str) -> Result<Value> {
        for plugin in self.plugins.iter() {
            if let Some(handler) = plugin.get_handler_for_route(path) {
                let bytes = plugin.call_raw(&handler, input_json).await?;
                let output = serde_json::from_slice::<Value>(&bytes)?;
                return Ok(output);
            }
        }

        Err(anyhow!("Route not found: {}", path))
    }

    pub async fn call_hook<TIn, TOut>(&self, hook: &str, input: &TIn) -> Result<TOut>
    where
        TIn: Serialize + Sync,
        TOut: for<'de> Deserialize<'de>,
    {
        let input_json = serde_json::to_string(input)?;
        let plugin = self
            .hook_index
            .get(hook)
            .and_then(|plugin_name| self.plugins.get(plugin_name.value()))
            .context("Plugin for hook not found")?;

        let bytes = plugin.call_raw(hook, &input_json).await?;
        let output = serde_json::from_slice::<TOut>(&bytes)?;
        Ok(output)
    }
}

static EXT_PLUGIN_MANAGER: OnceCell<PluginManager> = OnceCell::new();

pub fn get_plugin_manager() -> &'static PluginManager {
    EXT_PLUGIN_MANAGER.get_or_init(|| PluginManager::new())
}

pub async fn init_plugin_manager() -> Result<()> {
    let plugin_manager = PluginManager::new();
    plugin_manager
        .load_plugins()
        .await
        .context("Failed to load plugins")?;

    EXT_PLUGIN_MANAGER
        .set(plugin_manager)
        .map_err(|_| anyhow!("PluginManager already initialized"))
}

pub use super::plugins_hooks::PluginHooks;
