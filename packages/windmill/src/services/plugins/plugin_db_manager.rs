// // SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
// //
// // SPDX-License-Identifier: AGPL-3.0-only
use crate::services::database::get_hasura_pool;
use anyhow::{anyhow, Context, Error, Result};
use deadpool_postgres::{Client as DbClient, Transaction};
use extism::{
    CurrentPlugin, Function, Manifest, Plugin as ExtismPlugin, PluginBuilder, UserData, Val,
    ValType, Wasm,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Serialize, Deserialize, Debug)]
pub struct DbRequest {
    pub sql: String,
    pub params: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DbResponse {
    pub rows_affected: u64,
    pub result: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TxRequest {}

#[derive(Serialize, Deserialize, Debug)]
pub struct TxResponse {
    pub tx_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TxExecRequest {
    pub tx_id: String,
    pub statement: DbRequest,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TxExecResponse {
    pub response: DbResponse,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TxCommitRequest {
    pub tx_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TxRollbackRequest {
    pub tx_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TxCommitResponse {
    pub success: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TxRollbackResponse {
    pub success: bool,
}
static TX_STORE: once_cell::sync::Lazy<
    Mutex<HashMap<String, Arc<Mutex<Option<Transaction<'static>>>>>>,
> = once_cell::sync::Lazy::new(|| Mutex::new(HashMap::new()));
pub struct PluginDbManager;

impl PluginDbManager {
    pub fn register_db_host_functions(
        builder: PluginBuilder,
        manifest_val: Value,
    ) -> PluginBuilder {
        // Read host_functions list from manifest
        let mut funcs = Vec::new();
        if let Some(list) = manifest_val
            .get("host_functions")
            .and_then(|v| v.as_array())
        {
            for name in list.iter().filter_map(|v| v.as_str()) {
                // For each host function, push a tuple with module, function name, params, results, handler, user_data
                match name {
                    "start_hasura_transaction" => funcs.push((
                        "start_hasura_transaction".to_string(),
                        vec![ValType::I32],               // TxRequest pointer
                        vec![ValType::I32, ValType::I32], // TxResponse pointer (ptr, len)
                        start_hasura_transaction,
                        UserData::new(()),
                    )),
                    _ => {}
                }
            }
        }
        // Register all functions at once
        let functions = funcs
            .into_iter()
            .map(|(name, params, results, handler, ud)| {
                Function::new(name, params, results, ud, handler)
            });
        builder.with_functions(functions)
    }
}

fn start_hasura_transaction(
    plugin: &mut CurrentPlugin,
    params: &[Val],
    results: &mut [Val],
    _ud: UserData<()>,
) -> Result<()> {
    let tx_id = tokio::runtime::Handle::current().block_on(async {
        // Move the client into the transaction and store both in a struct
        let mut hasura_db_client: DbClient = get_hasura_pool()
            .await
            .get()
            .await
            .map_err(|e| anyhow!("Error getting hasura client {}", e))?;
        // Box the client to extend its lifetime
        let boxed_client: Box<DbClient> = Box::new(hasura_db_client);
        // SAFETY: We are extending the lifetime to 'static, but we must ensure the transaction is dropped before program exit.
        let hasura_transaction: Transaction<'static> = unsafe {
            std::mem::transmute::<Transaction<'_>, Transaction<'static>>(
                Box::leak(boxed_client)
                    .transaction()
                    .await
                    .map_err(|e| anyhow!("Error starting hasura transaction {}", e))?,
            )
        };
        let hasura_transaction_arc = Arc::new(Mutex::new(Some(hasura_transaction)));
        let transaction_id = uuid::Uuid::new_v4().to_string();
        TX_STORE
            .lock()
            .await
            .insert(transaction_id.clone(), hasura_transaction_arc);
        Ok::<String, anyhow::Error>(transaction_id)
    })?;

    // Serialize TxResponse { tx_id }
    let resp = TxResponse { tx_id };
    let out = serde_json::to_vec(&resp).map_err(|e| anyhow!(e.to_string()))?;
    // Write back into guest memory
    let out_ptr: extism::convert::MemoryHandle = plugin.memory_alloc(out.len() as u64)?;
    let mut ptr_val = Val::I64(out_ptr.offset() as i64);
    plugin.memory_set_val(&mut ptr_val, &out)?;

    // Return ptr and len to the guest
    results[0] = Val::I32(out_ptr.offset as i32);
    results[1] = Val::I32(out.len() as i32);
    Ok(())
}
