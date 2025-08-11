// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::plugins_manager::plugin::{HookValue, Plugin};
use anyhow::{anyhow, Context, Result};
use dashmap::DashMap;
use futures::future;
use once_cell::sync::OnceCell;
use sequent_core::plugins_wit::lib::plugin_bindings::plugins_manager::common::types::{
    Manifest, PluginRoute,
};
use sequent_core::services::s3::{get_files_names_bytes_from_s3, get_public_bucket};
use serde_json::Value;
use std::env;
use std::sync::Arc;
use tempfile::tempdir;
use wasmtime::{Config, Engine};

pub struct PluginManager {
    pub plugins: DashMap<String, Arc<Plugin>>,
    pub hooks: DashMap<String, Vec<String>>, // (hook, list of plugin names)
    pub routes: DashMap<String, (String, String)>, // (path, (handler, plugin_name)) - Routes remain 1:1
    pub tasks: DashMap<String, Vec<String>>,       // (task, list of plugin names)
    pub engine: Engine,
}

impl PluginManager {
    /// Creates a new PluginManager instance with an async-enabled Wasmtime engine and empty plugin registries.
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

    /// Loads all plugin WASM files from the S3 bucket, initializes them, and registers their hooks, routes, and tasks.
    pub async fn load_plugins(&self) -> Result<()> {
        let bucket: String = get_public_bucket().context("failed to get public S3 bucket")?;
        let wasms_files: Vec<(String, Vec<u8>)> =
            get_files_names_bytes_from_s3(bucket, "plugins/".to_string()).await?;

        let temp_dir = env::temp_dir();

        for (file_name, wasm) in wasms_files {
            let file_name = file_name.trim_end_matches(".wasm").to_string();
            let plugin_tmpdir = temp_dir.join(&file_name);
            let plugin_tmpdir_path = plugin_tmpdir.to_path_buf();

            let plugin: Option<Plugin> = Plugin::init_plugin_from_wasm_bytes(
                &self.engine,
                wasm,
                file_name,
                &plugin_tmpdir_path,
            )
            .await?;
            if plugin.is_none() {
                continue;
            }
            let plugin = plugin.unwrap();
            let plugin_name = plugin.name.clone();
            let plugin_manifest: Manifest = plugin.manifest.clone();

            let plugin_hooks = &plugin_manifest.hooks;
            let plugin_routes: &Vec<PluginRoute> = &plugin_manifest.routes;
            let plugin_tasks: &Vec<String> = &plugin_manifest.tasks;

            for hook in plugin_hooks {
                self.hooks
                    .entry(hook.clone())
                    .or_default()
                    .push(plugin_name.clone());
            }

            for route in plugin_routes {
                let path = route.path.clone();
                if path.starts_with(format!("/{}", plugin_name).as_str()) {
                    let handler = route.handler.clone();
                    self.routes
                        .insert(path, (handler.clone(), plugin_name.clone()));
                    if route.process_as_task {
                        self.tasks
                            .entry(handler.clone())
                            .or_default()
                            .push(plugin_name.clone());
                    }
                }
            }
            for task in plugin_tasks {
                self.tasks
                    .entry(task.clone())
                    .or_default()
                    .push(plugin_name.clone());
            }

            self.plugins.insert(plugin_name, Arc::new(plugin));
        }

        Ok(())
    }

    /// Calls a hook by name on all plugins that registered for it, passing arguments and expected result values.
    /// Returns a vector of results from each plugin.
    pub async fn call_hook(
        &self,
        hook: &str,
        args: Vec<HookValue>,
        expected_result: Vec<HookValue>,
    ) -> Result<Vec<Vec<HookValue>>> {
        let plugin_names = self
            .hooks
            .get(hook)
            .context(anyhow!("Hook {hook} not registered by any plugin"))?;

        let mut tasks = Vec::new();

        for plugin_name in plugin_names.iter() {
            let plugin = self
                .plugins
                .get(plugin_name)
                .context(anyhow!("Plugin {plugin_name} not found for hook {hook}"))?;

            let args_clone = args.clone();
            let expected_result_clone = expected_result.clone();
            let hook_clone = hook.to_string();

            let plugin_arc_clone = std::sync::Arc::new(plugin.clone());

            let plugin_name_clone = plugin_name.clone();

            let task = tokio::spawn(async move {
                let results = plugin_arc_clone
                    .call_hook(&hook_clone, args_clone, expected_result_clone)
                    .await
                    .map_err(|e| {
                        anyhow!(
                            "Failed to call hook {hook_clone} in plugin {plugin_name_clone}: {e}",
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
                Err(join_error) => Err(anyhow!("Hook task panicked or failed: {join_error}")),
            })
            .collect();

        let mut all_results: Vec<Vec<HookValue>> = Vec::new();
        for result in results_from_tasks {
            all_results.push(result?);
        }

        Ok(all_results)
    }

    /// Calls a registered route handler by path, passing a JSON string as input, and returns the JSON value.
    pub async fn call_route(&self, path: &str, input_json: String) -> Result<Value> {
        if let Some(route_entry) = self.routes.get(path) {
            let (handler, plugin_name) = route_entry.value();

            let plugin = self
                .plugins
                .get(plugin_name)
                .context(anyhow!("Plugin {plugin_name} not found for route"))?;

            // Call the route handler, routes should always receive string of json response
            // Return value should be a Result<String,String>.
            let results: Vec<HookValue> = plugin
                .value()
                .call_hook(
                    handler,
                    vec![HookValue::String(input_json)],
                    vec![HookValue::Result(core::result::Result::Ok(None))],
                )
                .await
                .map_err(|e| anyhow!("Failed to call hook {handler}: {e}"))?;
            if let Some(result_hook_value) = results.get(0) {
                let results_json = result_hook_value
                    .as_results_json()
                    .map_err(|e| anyhow!("Failed to convert hook return value to JSON: {e}"))?;
                Ok(results_json)
            } else {
                Err(anyhow!("Route {path} handler did not return a string"))
            }
        } else {
            return Err(anyhow!("Route {path} not found"));
        }
    }

    /// Executes a registered task by name, passing a JSON string as input, on all plugins that registered for the task.
    pub async fn execute_task(&self, task: &str, input_json: String) -> Result<()> {
        let plugin_names = self
            .tasks
            .get(task)
            .context(anyhow!("Task {task} not registered by any plugin"))?;

        for plugin_name in plugin_names.value() {
            let plugin = self
                .plugins
                .get(plugin_name)
                .context(anyhow!("Plugin {plugin_name} not found for task {task}"))?;

            let _ = plugin
                .value()
                .call_hook(
                    task,
                    vec![HookValue::String(input_json.clone())],
                    vec![HookValue::Result(core::result::Result::Ok(None))],
                )
                .await
                .map_err(|e| anyhow!("Failed to call task {task} in plugin {plugin_name}: {e}"))?;
        }

        Ok(())
    }

    pub fn get_route_task_handler(&self, path: &str) -> Option<String> {
        if let Some(route_entry) = self.routes.get(path) {
            let (handler, _plugin_name) = route_entry.value();
            if self.tasks.get(&handler.clone()).is_some() {
                return Some(handler.clone());
            }
        }
        None
    }
}

static PLUGIN_MANAGER: OnceCell<PluginManager> = OnceCell::new();

/// Returns a reference to the global PluginManager singleton, initializing it if necessary.
pub async fn get_plugin_manager() -> Result<&'static PluginManager> {
    let plugin_manager = match PLUGIN_MANAGER.get() {
        Some(manager) => manager,
        _ => {
            let _ = init_plugin_manager()
                .await
                .map_err(|e| anyhow!("Failed to initialize PluginManager: {e}"));
            PLUGIN_MANAGER
                .get()
                .expect("PluginManager should be initialized")
        }
    };
    Ok(plugin_manager)
}

/// Initializes the global PluginManager singleton and loads all plugins from S3.
pub async fn init_plugin_manager() -> Result<()> {
    if PLUGIN_MANAGER.get().is_some() {
        return Ok(());
    }
    let plugin_manager = PluginManager::new()?;
    plugin_manager
        .load_plugins()
        .await
        .map_err(|e| anyhow!("Failed to load plugins: {e}"))?;

    PLUGIN_MANAGER
        .set(plugin_manager)
        .map_err(|_| anyhow!("PluginManager already initialized"))
}

pub use super::plugins_hooks::PluginHooks;
