// SPDX-FileCopyrightText: 2024 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{Context, Result};
use std::sync::Arc;
use tokio::sync::Mutex;
use wasmtime::component::{Component, Func, Instance, Linker, ResourceTable, Val};
use wasmtime::{Engine, Store};
use wasmtime_wasi::p2::{add_to_linker_sync, IoView, WasiCtx, WasiCtxBuilder, WasiView};

#[derive(Debug)]
pub enum HookParam {
    S32(i32),
    U32(u32),
    String(String),
    Bool(bool),
}

impl From<i32> for HookParam {
    fn from(value: i32) -> Self {
        HookParam::S32(value)
    }
}

impl From<u32> for HookParam {
    fn from(value: u32) -> Self {
        HookParam::U32(value)
    }
}

impl From<&str> for HookParam {
    fn from(value: &str) -> Self {
        HookParam::String(value.to_string())
    }
}

impl HookParam {
    pub fn to_val(&self) -> Val {
        match self {
            HookParam::S32(v) => Val::S32(*v),
            HookParam::U32(v) => Val::U32(*v),
            HookParam::String(v) => Val::String(v.clone()),
            HookParam::Bool(v) => Val::Bool(*v),
        }
    }
}

pub struct PluginCtx {
    pub wasi: WasiCtx,
    pub resource_table: ResourceTable,
}

impl WasiView for PluginCtx {
    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.wasi
    }
}

impl IoView for PluginCtx {
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.resource_table
    }
}

#[derive(Clone)]
pub struct Plugin {
    pub name: String,
    pub component: Component,
    pub instance: Arc<Mutex<(Store<PluginCtx>, Instance)>>,
}

impl Plugin {
    pub async fn from_wasm_bytes(
        engine: &Engine,
        linker: &mut Linker<PluginCtx>,
        wasm_bytes: Vec<u8>,
    ) -> Result<Self> {
        let component = Component::from_binary(engine, &wasm_bytes)?;
        let wasi = WasiCtxBuilder::new().inherit_stdio().build();
        add_to_linker_sync(linker)?;

        let store_ctx = PluginCtx {
            resource_table: ResourceTable::new(),
            wasi: wasi,
        };

        let mut store = Store::new(engine, store_ctx);
        let instance = linker.instantiate_async(&mut store, &component).await?;

        Ok(Self {
            name: "plugin".into(),
            component,
            instance: Arc::new(Mutex::new((store, instance))),
        })
    }

    pub async fn call_hook_dynamic(
        &self,
        hook: &str,
        params: Vec<HookParam>,
        result_count: usize,
    ) -> Result<Vec<Val>> {
        let (ref mut store, ref instance) = *self.instance.lock().await;

        let func_index = self
            .component
            .get_export_index(None, hook)
            .with_context(|| format!("hook export '{}' not found", hook))?;

        let func = instance
            .get_func(&mut *store, &func_index)
            .with_context(|| format!("hook function '{}' not found", hook))?;

        let wasm_params: Vec<Val> = params.into_iter().map(|p| p.to_val()).collect();
        let mut results = vec![Val::Bool(false); result_count]; // dummy initial values

        func.call_async(store, &wasm_params, &mut results)
            .await
            .context("hook call failed")?;

        Ok(results)
    }
}
