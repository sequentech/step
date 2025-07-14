// SPDX-FileCopyrightText: 2024 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::plugins_manager::plugin::{HookValue, Plugin};
use anyhow::{anyhow, Context, Result};
use dashmap::DashMap;
use futures::future;
use once_cell::sync::OnceCell;
use sequent_core::services::s3::{get_files_bytes_from_s3, get_public_bucket};
use std::sync::Arc;
use wasmtime::{Config, Engine};

pub struct PluginManager {
    pub plugins: DashMap<String, Arc<Plugin>>,
    pub hooks: DashMap<String, Vec<String>>, // (hook, list of plugin names)
    pub routes: DashMap<String, (String, String)>, // (path, (handler, plugin_name)) - Routes remain 1:1
    pub tasks: DashMap<String, Vec<String>>,       // (task, list of plugin names)
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
            tasks: DashMap::new(),
            engine,
        })
    }

    pub async fn load_plugins(&self) -> Result<()> {
        let bucket: String = get_public_bucket().context("failed to get public S3 bucket")?;
        let wasms_files: Vec<Vec<u8>> =
            get_files_bytes_from_s3(bucket, "plugins/".to_string()).await?;

        for wasm in wasms_files {
            let plugin = Plugin::init_plugin_from_wasm_bytes(&self.engine, wasm).await?;
            let plugin_name = plugin.name.clone();
            let plugin_manifest = &plugin.manifest;

            let parse_string_array = |key: &str| -> Result<Vec<String>> {
                plugin_manifest[key]
                    .as_array()
                    .ok_or_else(|| {
                        anyhow!(
                            "'{}' field not found or not an array in plugin manifest",
                            key
                        )
                    })?
                    .iter()
                    .map(|v| {
                        v.as_str()
                            .map(|s| s.to_string())
                            .ok_or_else(|| anyhow!("Value in '{}' array is not a string", key))
                    })
                    .collect::<Result<Vec<String>>>()
            };

            let hooks = parse_string_array("hooks").context(format!(
                "Failed to parse 'hooks' for plugin '{}'",
                plugin_name
            ))?;
            for hook in hooks {
                self.hooks
                    .entry(hook)
                    .or_default()
                    .push(plugin_name.clone());
            }

            let routes_array = plugin_manifest["routes"]
                .as_array()
                .ok_or_else(|| {
                    anyhow!("'routes' field not found or not an array in plugin manifest")
                })
                .context(format!(
                    "Failed to parse 'routes' array for plugin '{}'",
                    plugin_name
                ))?;

            for route_value in routes_array {
                let path = route_value["path"]
                    .as_str()
                    .map(|s| s.to_string())
                    .ok_or_else(|| anyhow!("Route 'path' is not a string"))
                    .context(format!(
                        "Failed to get 'path' for a route in plugin '{}'",
                        plugin_name
                    ))?;
                let handler = route_value["handler"]
                    .as_str()
                    .map(|s| s.to_string())
                    .ok_or_else(|| anyhow!("Route 'handler' is not a string"))
                    .context(format!(
                        "Failed to get 'handler' for a route in plugin '{}'",
                        plugin_name
                    ))?;
                self.routes.insert(path, (handler, plugin_name.clone()));
            }

            let tasks = parse_string_array("tasks").context(format!(
                "Failed to parse 'tasks' for plugin '{}'",
                plugin_name
            ))?;
            for task in tasks {
                self.tasks
                    .entry(task)
                    .or_default()
                    .push(plugin_name.clone());
            }

            self.plugins.insert(plugin_name, Arc::new(plugin));
        }

        Ok(())
    }

    pub async fn call_hook_dynamic(
        &self,
        hook: &str,
        args: Vec<HookValue>,
        expected_result_types: Vec<HookValue>,
    ) -> Result<Vec<Vec<HookValue>>> {
        let plugin_names = self
            .hooks
            .get(hook)
            .context(format!("Hook '{:?}' not registered by any plugin", hook))?;

        let mut tasks = Vec::new();

        for plugin_name in plugin_names.iter() {
            let plugin = self.plugins.get(plugin_name).context(format!(
                "Plugin '{:?}' not found for hook '{}'",
                plugin_name, hook
            ))?;

            let args_clone = args.clone();
            let expected_result_types_clone = expected_result_types.clone();
            let hook_clone = hook.to_string(); // Clone the hook string

            let plugin_arc_clone = std::sync::Arc::new(plugin.clone());

            let plugin_name_clone = plugin_name.clone();

            let task = tokio::spawn(async move {
                let results = plugin_arc_clone
                    .call_hook_dynamic(&hook_clone, args_clone, expected_result_types_clone)
                    .await
                    .map_err(|e| {
                        anyhow!(
                            "Failed to call hook '{}' in plugin '{}': {:?}",
                            hook_clone,
                            plugin_name_clone,
                            e
                        )
                    })?;
                Ok(results)
            });
            tasks.push(task);
        }

        let results_from_tasks: Vec<Result<Vec<HookValue>>> = future::join_all(tasks)
            .await
            .into_iter()
            .map(|join_result| match join_result {
                Ok(inner_result) => inner_result,
                Err(join_error) => Err(anyhow!("Hook task panicked or failed: {}", join_error)),
            })
            .collect();

        let mut all_results: Vec<Vec<HookValue>> = Vec::new();
        for result in results_from_tasks {
            all_results.push(result?);
        }

        Ok(all_results)
    }

    pub async fn call_route(&self, path: &str, input_json: String) -> Result<String> {
        if let Some(route_entry) = self.routes.get(path) {
            let (handler, plugin_name) = route_entry.value();

            let plugin = self
                .plugins
                .get(plugin_name)
                .context(format!("Plugin '{}' not found for route", plugin_name))?;

            // Call the route handler, routes should always receive and return a string of json response
            let results: Vec<HookValue> = plugin
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

    pub async fn execute_task(&self, task: &str, input_json: String) -> Result<()> {
        let plugin_names = self
            .tasks
            .get(task)
            .context(format!("Task '{:?}' not registered by any plugin", task))?;

        for plugin_name in plugin_names.value() {
            let plugin = self.plugins.get(plugin_name).context(format!(
                "Plugin '{:?}' not found for task '{}'",
                plugin_name, task
            ))?;

            let _ = plugin
                .value()
                .call_hook_dynamic(task, vec![HookValue::String(input_json.clone())], vec![])
                .await
                .map_err(|e| {
                    anyhow!(
                        "Failed to call task '{}' in plugin '{}': {:?}",
                        task,
                        plugin_name,
                        e
                    )
                })?;
        }

        Ok(())
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
    if PLUGIN_MANAGER.get().is_some() {
        return Ok(());
    }
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

// How to call hook:
/*
use windmill::services::plugins_manager::plugin_manager::{self,PluginHooks,PluginManager};
....
        let plugin_manager = plugin_manager::get_plugin_manager()
            .await
            .map_err(|e| (Status::InternalServerError, e.to_string()))?;

        /*  ---- when  add have its own implementation in plugin_hooks.rs  ---- */
        let res = plugin_manager.add(2,4).await.map_err(|e| (Status::InternalServerError, e.to_string()))?;
        println!("Result from plugin manager add: {}", res);
....
*/
