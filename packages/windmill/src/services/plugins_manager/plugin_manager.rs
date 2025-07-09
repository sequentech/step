// SPDX-FileCopyrightText: 2024 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::plugins_manager::plugin::{HookValue, Plugin, PluginStore};
use anyhow::{anyhow, Context, Result};
use dashmap::DashMap;
use once_cell::sync::OnceCell;
use sequent_core::services::s3::{get_files_bytes_from_s3, get_public_bucket};
use std::sync::Arc;
use wasmtime::component::Linker;
use wasmtime::{Config, Engine};

pub struct PluginManager {
    pub plugins: DashMap<String, Arc<Plugin>>,
    pub hooks: DashMap<String, String>,
    pub engine: Engine,
    pub linker: Linker<PluginStore>,
}

impl PluginManager {
    pub fn new() -> Result<Self> {
        let mut config = Config::new();
        // config.wasm_component_model(true);
        config.async_support(true);
        let engine = Engine::new(&config)?;
        let linker = Linker::<PluginStore>::new(&engine);

        Ok(Self {
            plugins: DashMap::new(),
            hooks: DashMap::new(),
            engine,
            linker,
        })
    }

    pub async fn load_plugins(&self) -> Result<()> {
        let bucket = get_public_bucket().context("failed to get public S3 bucket")?;
        let blobs = get_files_bytes_from_s3(bucket, "plugins/".to_string()).await?;

        for wasm in blobs {
            let plugin =
                Plugin::from_wasm_bytes(&self.engine, &mut self.linker.clone(), wasm).await?;
            let manifest_json: serde_json::Value = plugin.manifest.clone();
            let hooks = manifest_json["hooks"]
                .as_array()
                .unwrap()
                .iter()
                .map(|h| h.as_str().unwrap().to_string())
                .collect::<Vec<String>>();
            for hook in hooks {
                self.hooks.insert(hook, plugin.name.clone());
            }
            self.plugins.insert(plugin.name.clone(), Arc::new(plugin));
        }

        Ok(())
    }

    pub async fn call_hook_dynamic(
        &self,
        hook: &str,
        args: Vec<HookValue>,
        expected_result_types: Vec<HookValue>,
    ) -> Result<Vec<HookValue>> {
        let plugin_name = self
            .hooks
            .get(hook)
            .context(format!("Hook '{:?}' not registered", hook))?;

        let plugin = self
            .plugins
            .get(plugin_name.value())
            .context(format!("Plugin '{:?}' not found", plugin_name))?;

        plugin
            .value()
            .call_hook_dynamic(hook, args, expected_result_types)
            .await
    }
}

static EXT_PLUGIN_MANAGER: OnceCell<PluginManager> = OnceCell::new();

pub async fn get_plugin_manager() -> Result<&'static PluginManager> {
    let plugin_manager = match EXT_PLUGIN_MANAGER.get() {
        Some(manager) => manager,
        None => {
            let _ = init_plugin_manager()
                .await
                .expect("Failed to initialize PluginManager");
            EXT_PLUGIN_MANAGER
                .get()
                .expect("PluginManager should be initialized now")
        }
    };
    Ok(plugin_manager)
}

pub async fn init_plugin_manager() -> Result<()> {
    let plugin_manager = PluginManager::new()?;
    plugin_manager
        .load_plugins()
        .await
        .context("Failed to load plugins")?;

    EXT_PLUGIN_MANAGER
        .set(plugin_manager)
        .map_err(|_| anyhow!("PluginManager already initialized"))
}

pub use super::plugins_hooks::PluginHooks;
