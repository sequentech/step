// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::plugins_manager::{
    plugin_db_manager::{PluginDbManager, PluginTransactionsManager},
    plugin_documents_manager::PluginDocumentsManager,
};
use anyhow::{anyhow, Context, Result};
use core::{
    option::Option::None,
    result::Result::{Err, Ok},
};
use sequent_core::{
    plugins::{get_plugin_shared_dir, Plugins},
    plugins_wit::lib::{
        authorization_bindings::plugins_manager::jwt::authorization::{
            add_to_linker as add_auth_to_linker, Host as HostAuth,
        },
        documents_bindings::plugins_manager::documents_manager::documents::add_to_linker as add_documents_to_linker,
        plugin_bindings::{plugins_manager::common::types::Manifest, Plugin as PluginInterface},
        transactions_manager_bindings::plugins_manager::transactions_manager::{
            postgres_queries::add_to_linker as add_postgres_queries_to_linker,
            transaction::add_to_linker as add_transaction_linker,
            vault::add_to_linker as add_vault_to_linker,
        },
    },
    services::{authorization::authorize, jwt::JwtClaims},
    types::permissions::Permissions,
};
use serde_json::Value;
use std::sync::Arc;
use std::{path::PathBuf, str::FromStr};
use tokio::sync::Mutex;
use wasmtime::component::{Component, Func, Instance, Linker, ResourceTable, Val};
use wasmtime::{Engine, Store};
use wasmtime_wasi::p2::{add_to_linker_async, IoView, WasiCtx, WasiCtxBuilder, WasiView};

/// Represents a value that can be passed to or returned from a plugin hook.
#[derive(Debug, Clone)]
pub enum HookValue {
    S32(i32),
    U32(u32),
    String(String),
    Bool(bool),
    Result(core::result::Result<Option<Box<HookValue>>, Option<Box<HookValue>>>),
    Option(Option<Box<HookValue>>),
}

impl From<i32> for HookValue {
    fn from(value: i32) -> Self {
        HookValue::S32(value)
    }
}

impl From<u32> for HookValue {
    fn from(value: u32) -> Self {
        HookValue::U32(value)
    }
}

impl From<&str> for HookValue {
    fn from(value: &str) -> Self {
        HookValue::String(value.to_string())
    }
}

impl HookValue {
    pub fn to_val(&self) -> Val {
        match self {
            HookValue::S32(v) => Val::S32(*v),
            HookValue::U32(v) => Val::U32(*v),
            HookValue::String(v) => Val::String(v.clone()),
            HookValue::Bool(v) => Val::Bool(*v),
            HookValue::Result(Ok(opt)) => {
                Val::Result(Ok(opt.as_ref().map(|v| Box::new(v.to_val()))))
            }
            HookValue::Result(Err(opt)) => {
                Val::Result(Err(opt.as_ref().map(|v| Box::new(v.to_val()))))
            }
            HookValue::Option(Some(opt)) => Val::Option(Some(Box::new(opt.to_val()))),
            HookValue::Option(None) => Val::Option(None),
        }
    }

    pub fn from_val(val: Val) -> Result<Self, anyhow::Error> {
        Ok(match val {
            Val::S32(v) => HookValue::S32(v),
            Val::U32(v) => HookValue::U32(v),
            Val::String(s) => HookValue::String(s),
            Val::Bool(b) => HookValue::Bool(b),
            Val::Result(Ok(Some(v))) => {
                HookValue::Result(Ok(Some(Box::new(HookValue::from_val(*v)?))))
            }
            Val::Result(Ok(None)) => HookValue::Result(Ok(None)),
            Val::Result(Err(Some(err_box))) => {
                let err_val = *err_box;
                HookValue::Result(Err(Some(Box::new(HookValue::from_val(err_val)?))))
            }
            Val::Option(Some(opt_box)) => {
                let opt_val = *opt_box;
                HookValue::Option(Some(Box::new(HookValue::from_val(opt_val)?)))
            }
            Val::Option(None) => HookValue::Option(None),
            _ => return Err(anyhow::anyhow!("Unsupported Val type: {:?}", val)),
        })
    }

    pub fn as_i32(&self) -> Option<i32> {
        match self {
            HookValue::S32(v) => Some(*v),
            HookValue::U32(v) => Some(*v as i32),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            HookValue::String(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_results_json(&self) -> Result<Value> {
        match self {
            HookValue::Result(Ok(Some(boxed_value))) => match &**boxed_value {
                HookValue::String(value) => {
                    let json_value: Value = serde_json::from_str(value)
                        .map_err(|e| anyhow!("Failed to parse string as JSON: {}", e))?;
                    Ok(json_value)
                }
                _ => Err(anyhow!("Unexpected boxed hook value type")),
            },
            HookValue::Result(Ok(None)) => Err(anyhow!("No value returned from plugin hook")),
            HookValue::Result(Err(Some(e))) => match &**e {
                HookValue::String(e) => Err(anyhow!("Plugin hook error: {}", e)),
                _ => Err(anyhow!("Error executing plugin hook")),
            },
            _ => Err(anyhow!("Unexpected hook value type")),
        }
    }
}

pub struct PluginServices {
    pub transactions: PluginTransactionsManager,
    pub documents: PluginDocumentsManager,
}

impl PluginServices {
    pub fn new(transactions: PluginTransactionsManager, documents: PluginDocumentsManager) -> Self {
        PluginServices {
            transactions,
            documents,
        }
    }
}
pub struct PluginStore {
    pub wasi: WasiCtx,
    pub resource_table: ResourceTable,
    pub plugin_auth: PluginAuth,
    pub services: PluginServices,
}

impl WasiView for PluginStore {
    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.wasi
    }
}

impl IoView for PluginStore {
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.resource_table
    }
}

#[derive(Clone)]
pub struct Plugin {
    pub name: String,
    pub component: Component,
    pub instance: Arc<Mutex<(Store<PluginStore>, Instance)>>,
    pub manifest: Manifest,
}

impl Plugin {
    /// Initializes a plugin from WASM bytes, setting up the necessary environment and returning a Plugin instance.
    /// Read Manifest from the plugin's common interface.
    pub async fn init_plugin_from_wasm_bytes(
        engine: &Engine,
        wasm_bytes: Vec<u8>,
        wasm_file_name: String,
        files_temp_dir: &PathBuf,
    ) -> Result<Option<Self>> {
        let mut linker = Linker::<PluginStore>::new(&engine);

        println!("Loading plugin from file: {}", wasm_file_name);
        let component = Component::from_binary(&engine, &wasm_bytes)?;
        //  {
        //     Ok(c) => c,
        //     Err(e) => {
        //         println!("Failed to load component from file {}: {}", wasm_file_name, e);
        //         return Ok(None);
        //     }
        // };]

        println!(
            "Successfully loaded component for plugin: {}",
            wasm_file_name
        );
        // let host_temp_path = files_temp_dir.as_path().to_owned();

        let mut builder = WasiCtxBuilder::new();
        let host_temp_path = files_temp_dir.as_path();

        std::fs::create_dir_all(&host_temp_path).map_err(|e| {
            anyhow!(
                "Failed to create host temporary directory for plugin {}: {}",
                wasm_file_name,
                e
            )
        })?;

        println!(
            "Preopening temp directory for plugin: {}",
            host_temp_path.display()
        );

        let plugin_name = match Plugins::from_str(&wasm_file_name) {
            Ok(name) => name,
            Err(e) => {
                println!(
                    "Failed to parse plugin name from file {}: {}",
                    wasm_file_name, e
                );
                return Ok(None);
            }
        };
        let plugin_temp_dir = get_plugin_shared_dir(&plugin_name);
        builder
            .preopened_dir(
                host_temp_path.clone(),
                plugin_temp_dir,
                wasmtime_wasi::DirPerms::all(),
                wasmtime_wasi::FilePerms::all(),
            )
            .map_err(|e| {
                println!(
                    "Failed to preopen directory for plugin {}: {}",
                    wasm_file_name, e
                );
                e
            })?;

        println!("Building WASI context for plugin: {}", wasm_file_name);
        let mut wasi: WasiCtx = builder.inherit_stdio().build();
        add_to_linker_async(&mut linker)?;

        let hasura_manager = Arc::new(Mutex::new(PluginDbManager::init()));
        let keycloak_manager = Arc::new(Mutex::new(PluginDbManager::init()));

        let transactions_manager =
            PluginTransactionsManager::new(hasura_manager.clone(), keycloak_manager.clone());
        let documents_manager = PluginDocumentsManager::new(host_temp_path.clone().to_path_buf());
        let plugin_services = PluginServices::new(transactions_manager, documents_manager);

        let plugin_store = PluginStore {
            resource_table: ResourceTable::new(),
            wasi: wasi,
            plugin_auth: PluginAuth::new(),
            services: plugin_services,
        };

        let mut store = Store::new(engine, plugin_store);

        add_transaction_linker(&mut linker, |s: &mut PluginStore| &mut s.services)?;

        add_postgres_queries_to_linker(&mut linker, |s: &mut PluginStore| &mut s.services)?;

        add_auth_to_linker(&mut linker, |store: &mut PluginStore| {
            &mut store.plugin_auth
        })?;

        add_documents_to_linker(&mut linker, |store: &mut PluginStore| &mut store.services)?;

        add_vault_to_linker(&mut linker, |s: &mut PluginStore| &mut s.services)?;

        let instance = linker.instantiate_async(&mut store, &component).await?;

        let plugin_common_instance = match PluginInterface::instantiate_async(
            &mut store, &component, &linker,
        )
        .await
        {
            Ok(instance) => instance,
            Err(e) => {
                println!(
                "Component {} does not seem to implement the common plugin interface or failed to instantiate it: {}",
                wasm_file_name, e
            );
                return Ok(None);
            }
        };

        let plugin_manifest = plugin_common_instance
            .plugins_manager_common_plugin_common()
            .call_get_manifest(&mut store)
            .await
            .ok()
            .context("Failed to get plugin manifest")?;

        Ok(Some(Self {
            name: plugin_manifest.plugin_name.clone(),
            component,
            instance: Arc::new(Mutex::new((store, instance))),
            manifest: plugin_manifest.clone(),
        }))
    }

    // Calls a hook dynamically with the provided arguments and expected result values.
    // args: Vec<HookValue> - The arguments to pass to the hook.
    // expected_result: Vec<HookValue> - The expected result values from the hook (call_async will fill these).
    // Returns a Result containing a vector of HookValue results from the hook call.
    pub async fn call_hook(
        &self,
        hook: &str,
        args: Vec<HookValue>,
        expected_result: Vec<HookValue>,
    ) -> Result<Vec<HookValue>> {
        let mut guard = self.instance.lock().await;
        let (store, instance) = &mut *guard;

        let func_index = self
            .component
            .get_export_index(None, hook)
            .context(anyhow!("Export {hook} not found in component"))?;

        let func: Func = instance
            .get_func(&mut *store, &func_index)
            .context(anyhow!("Function {hook} not found in instance"))?;

        let wasm_args: Vec<_> = args.into_iter().map(|arg| arg.to_val()).collect();
        let mut results: Vec<Val> = expected_result
            .iter()
            .map(|expected| expected.to_val()) // These are placeholders, their *type* is important.
            .collect();

        func.call_async(&mut *store, &wasm_args, results.as_mut_slice())
            .await
            .map_err(|e| anyhow!("Failed to call hook {hook}: {e}"))?;

        func.post_return_async(&mut *store).await?;

        results
            .into_iter()
            .map(HookValue::from_val)
            .collect::<Result<Vec<_>>>()
    }
}

pub struct PluginAuth;

impl PluginAuth {
    pub fn new() -> Self {
        PluginAuth
    }
}

impl HostAuth for PluginAuth {
    async fn authorize(
        &mut self,
        claims: String,
        allow_super_admin_auth: bool,
        tenant_id_opt: Option<String>,
        permissions: Vec<String>,
    ) -> Result<(), String> {
        let claims: JwtClaims =
            serde_json::from_str(&claims).map_err(|e| format!("Failed to parse claims: {e}"))?;

        let parsed_permissions: Vec<Permissions> = permissions
            .into_iter()
            .filter_map(|p_str| match Permissions::from_str(&p_str) {
                Ok(perm_enum) => Some(perm_enum),
                Err(e) => {
                    println!(
                        "Warning: Failed to parse permission string '{}': {}",
                        p_str, e
                    );
                    None
                }
            })
            .collect();

        match authorize(
            &claims,
            allow_super_admin_auth,
            tenant_id_opt,
            parsed_permissions,
        ) {
            Ok(_) => Ok(()),
            Err((_status, message)) => Err(format!("Authorization failed: {}", message)),
        }
    }
}
