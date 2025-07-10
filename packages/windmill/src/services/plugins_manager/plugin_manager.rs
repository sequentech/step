// SPDX-FileCopyrightText: 2024 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::plugins_manager::plugin::{HookValue, Plugin};
use anyhow::{anyhow, Context, Result};
use dashmap::DashMap;
use once_cell::sync::OnceCell;
use sequent_core::services::s3::{get_files_bytes_from_s3, get_public_bucket};
use std::sync::Arc;
use wasmtime::{Config, Engine};

pub struct PluginManager {
    pub plugins: DashMap<String, Arc<Plugin>>,
    pub hooks: DashMap<String, String>, // (hook, plugin name)
    pub routes: DashMap<String, (String, String)>, // (path, (handler, plugin_name))
    pub engine: Engine,
}

impl PluginManager {
    pub fn new() -> Result<Self> {
        let mut config = Config::new();
        config.async_support(true);
        let engine = Engine::new(&config)?;

        Ok(Self {
            plugins: DashMap::new(),
            hooks: DashMap::new(),
            routes: DashMap::new(),
            engine,
        })
    }

    pub async fn load_plugins(&self) -> Result<()> {
        let bucket: String = get_public_bucket().context("failed to get public S3 bucket")?;
        let wasms_files: Vec<Vec<u8>> =
            get_files_bytes_from_s3(bucket, "plugins/".to_string()).await?;

        for wasm in wasms_files {
            let plugin = Plugin::init_plugin_from_wasm_bytes(&self.engine, wasm).await?;
            let plugin_manifest: serde_json::Value = plugin.manifest.clone();
            let hooks = plugin_manifest["hooks"]
                .as_array()
                .unwrap()
                .iter()
                .map(|h| h.as_str().unwrap().to_string())
                .collect::<Vec<String>>();
            for hook in hooks {
                self.hooks.insert(hook, plugin.name.clone());
            }

            let routes = plugin_manifest["routes"]
                .as_array()
                .unwrap()
                .iter()
                .map(|r| {
                    (
                        r["path"].as_str().unwrap().to_string(),
                        r["handler"].as_str().unwrap().to_string(),
                    )
                })
                .collect::<Vec<(String, String)>>();
            for (path, handler) in routes {
                self.routes.insert(path, (handler, plugin.name.clone()));
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
        println!(
            "Calling hook '{}' in plugin '{}'",
            hook,
            plugin_name.value()
        );

        let plugin = self
            .plugins
            .get(plugin_name.value())
            .context(format!("Plugin '{:?}' not found", plugin_name))?;

        plugin
            .value()
            .call_hook_dynamic(hook, args, expected_result_types)
            .await
    }

    pub async fn call_route(&self, path: &str, input_json: String) -> Result<String> {
        if let Some(route_entry) = self.routes.get(path) {
            let (handler, plugin_name) = route_entry.value();

            let plugin = self
                .plugins
                .get(plugin_name)
                .context(format!("Plugin '{}' not found for route", plugin_name))?;

            let results = plugin
                .value()
                .call_hook_dynamic(
                    handler,
                    vec![HookValue::String(input_json)],
                    vec![HookValue::String("".to_string())],
                )
                .await
                .map_err(|e| anyhow!("Failed to call hook '{}': {:?}", handler, e))?;
            let cal_res = results[0].as_str();

            if let Some(res) = cal_res {
                Ok(res.to_string())
            } else {
                Err(anyhow!("Route handler did not return a string"))
            }
        } else {
            return Err(anyhow!("Route '{}' not found", path));
        }
    }
}

static PLUGIN_MANAGER: OnceCell<PluginManager> = OnceCell::new();

pub async fn get_plugin_manager() -> Result<&'static PluginManager> {
    let plugin_manager = match PLUGIN_MANAGER.get() {
        Some(manager) => manager,
        _ => {
            let _ = init_plugin_manager()
                .await
                .map_err(|e| anyhow!("Failed to initialize PluginManager: {:?}", e));
            PLUGIN_MANAGER
                .get()
                .expect("PluginManager should be initialized")
        }
    };
    Ok(plugin_manager)
}

pub async fn init_plugin_manager() -> Result<()> {
    let plugin_manager = PluginManager::new()?;
    plugin_manager
        .load_plugins()
        .await
        .map_err(|e| anyhow!("Failed to load plugins: {:?}", e))?;

    PLUGIN_MANAGER
        .set(plugin_manager)
        .map_err(|_| anyhow!("PluginManager already initialized"))
}

pub use super::plugins_hooks::PluginHooks;

//How to call hook:
/*

use windmill::services::plugins_manager::plugin_manager::{self,PluginHooks,PluginManager};
....
        let plugin_manager = plugin_manager::get_plugin_manager()
            .await
            .map_err(|e| (Status::InternalServerError, e.to_string()))?;
        let res = plugin_manager.add(2,4).await.map_err(|e| (Status::InternalServerError, e.to_string()))?;
        println!("Result from plugin manager add: {}", res);
....
*/
