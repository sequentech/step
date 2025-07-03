// SPDX-FileCopyrightText: 2024 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::database::{get_hasura_pool, get_keycloak_pool};
use crate::services::plugins_manager::plugin_db_manager::PluginDbManager;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::{Object, Transaction};
use immudb_rs::client;
use ouroboros::self_referencing;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_postgres::NoTls;
use wasmtime::component::{Component, Func, Instance, Linker, ResourceTable, Val};
use wasmtime::{Engine, Store, StoreContextMut};
use wasmtime_wasi::p2::{add_to_linker_sync, IoView, WasiCtx, WasiCtxBuilder, WasiView};

use std::future::Future;
#[derive(Debug)]
pub enum HookValue {
    S32(i32),
    U32(u32),
    String(String),
    Bool(bool),
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
        }
    }

    pub fn from_val(val: Val) -> Result<Self, anyhow::Error> {
        Ok(match val {
            Val::S32(v) => HookValue::S32(v),
            Val::U32(v) => HookValue::U32(v),
            Val::String(s) => HookValue::String(s),
            Val::Bool(b) => HookValue::Bool(b),
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
}

pub struct PluginStore {
    pub wasi: WasiCtx,
    pub resource_table: ResourceTable,
    pub hasura_client: Option<Arc<Mutex<PluginDbManager>>>,
    pub keycloak_client: Option<Arc<Mutex<PluginDbManager>>>,
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
    pub manifest: serde_json::Value,
}

impl Plugin {
    pub async fn from_wasm_bytes(
        engine: &Engine,
        linker: &mut Linker<PluginStore>,
        wasm_bytes: Vec<u8>,
    ) -> Result<Self> {
        let component = Component::from_binary(engine, &wasm_bytes)?;
        let wasi = WasiCtxBuilder::new().inherit_stdio().build();
        add_to_linker_sync(linker)?;

        let hasura_client = Arc::new(Mutex::new(PluginDbManager::init()));
        let keycloak_client = Arc::new(Mutex::new(PluginDbManager::init()));

        let plugin_store = PluginStore {
            resource_table: ResourceTable::new(),
            wasi: wasi,
            hasura_client: Some(hasura_client),
            keycloak_client: Some(keycloak_client),
        };

        let mut store = Store::new(engine, plugin_store);
        let instance = linker.instantiate_async(&mut store, &component).await?;

        let func_index = component
            .get_export_index(None, "get-manifest")
            .with_context(|| "get-manifest export not found")?;
        let func = instance
            .get_func(&mut store, &func_index)
            .with_context(|| "get-manifest function not found")?;
        let mut results = [Val::String("".into())];
        func.call_async(&mut store, &[], &mut results).await?;
        let manifest_str = match &results[0] {
            Val::String(s) => s.clone(),
            _ => return Err(anyhow!("get-manifest did not return a string")),
        };
        let manifest_json: serde_json::Value = serde_json::from_str(&manifest_str)?;
        let plugin_name = manifest_json["plugin_name"].as_str().unwrap().to_string();

        linker.root().func_wrap_async(
            "execute-hasura-query",
            |mut ctx: StoreContextMut<'_, PluginStore>,
             (sql,): (String,)|
             -> Box<dyn Future<Output = Result<(), wasmtime::Error>> + Send> {
                Box::new(async move {
                    match &ctx.data().hasura_client {
                        Some(client) => {
                            let mut db_manager = client.lock().await;
                            let _ = db_manager
                                .create_hasura_transaction()
                                .await
                                .map_err(|e| wasmtime::Error::msg(e.to_string()))?;
                            Ok(())
                        }
                        None => {
                            return Err(wasmtime::Error::msg("Hasura transaction not initialized"));
                        }
                    }
                })
            },
        )?;

        linker.root().func_wrap_async(
            "execute-hasura-query",
            |mut ctx: StoreContextMut<'_, PluginStore>,
             (sql,): (String,)|
             -> Box<dyn Future<Output = Result<(String,), wasmtime::Error>> + Send> {
                Box::new(async move {
                    match &ctx.data().hasura_client {
                        Some(client) => {
                            let mut db_manager = client.lock().await;
                            let res = db_manager
                                .exec(&sql)
                                .await
                                .map_err(|e| wasmtime::Error::msg(e.to_string()))?;
                            Ok((res,))
                        }
                        None => {
                            return Err(wasmtime::Error::msg("Hasura transaction not initialized"));
                        }
                    }
                })
            },
        )?;

        Ok(Self {
            name: plugin_name,
            component,
            instance: Arc::new(Mutex::new((store, instance))),
            manifest: serde_json::from_str(&manifest_str)?,
        })
    }

    pub async fn call_hook_dynamic(
        &self,
        hook: &str,
        args: Vec<HookValue>,
        expected_result_types: Vec<HookValue>,
    ) -> Result<Vec<HookValue>> {
        let (ref mut store, ref instance) = *self.instance.lock().await;

        let func_index = self
            .component
            .get_export_index(None, hook)
            .context(format!("Export '{}' not found in component", hook))?;

        let func = instance
            .get_func(&mut *store, &func_index)
            .context(format!("Function '{}' not found in instance", hook))?;

        let wasm_args: Vec<_> = args.into_iter().map(|arg| arg.to_val()).collect();
        let mut result_placeholders: Vec<_> = expected_result_types
            .iter()
            .map(|expected| expected.to_val())
            .collect();

        func.call_async(store, &wasm_args, &mut result_placeholders)
            .await
            .context(format!("Failed to call hook '{}'", hook))?;

        result_placeholders
            .into_iter()
            .map(HookValue::from_val)
            .collect::<Result<Vec<_>>>()
    }
}
