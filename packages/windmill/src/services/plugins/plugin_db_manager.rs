// // SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
// //
// // SPDX-License-Identifier: AGPL-3.0-only
use crate::services::database::get_hasura_pool;
use anyhow::{anyhow, Context, Error, Result};
use deadpool_postgres::{Client as DbClient, Transaction};
// use extism::convert::MemoryHandle;
// use extism::{
//     host_fn, CurrentPlugin, Function, Manifest, Plugin as ExtismPlugin, PluginBuilder, UserData, Val, ValType, Wasm
// };
use extism::*;
use sequent_core::types::hasura;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

// #[derive(Serialize, Deserialize, Debug)]
// pub struct DbRequest {
//     pub statement: String,
//     pub params: serde_json::Value,
// }

// #[derive(Serialize, Deserialize, Debug)]
// pub struct DbResponse {
//     pub rows_affected: u64,
//     pub result: serde_json::Value,
// }
// #[derive(Serialize, Deserialize, Debug)]
// pub struct TxResponse {
//     pub transaction_id: String,
// }

// #[derive(Serialize, Deserialize, Debug)]
// pub struct TxExecRequest {
//     pub transaction_id: String,
//     pub query: DbRequest,
// }

// #[derive(Serialize, Deserialize, Debug)]
// pub struct TxExecResponse {
//     pub response: DbResponse,
// }

// #[derive(Serialize, Deserialize, Debug)]
// pub struct TxCommitRequest {
//     pub transaction_id: String,
// }

// #[derive(Serialize, Deserialize, Debug)]
// pub struct TxRollbackRequest {
//     pub transaction_id: String,
// }

// #[derive(Serialize, Deserialize, Debug)]
// pub struct TxCommitResponse {
//     pub success: bool,
// }

// #[derive(Serialize, Deserialize, Debug)]
// pub struct TxRollbackResponse {
//     pub success: bool,
// }

struct TransactionUserData {
    // client: &'a mut DbClient,
    transaction: Arc<Mutex<Option<Transaction<'static>>>>,
}

static TRANSACTION_STORE: once_cell::sync::Lazy<
    tokio::sync::Mutex<HashMap<String, Arc<tokio::sync::Mutex<Option<Transaction<'static>>>>>>,
> = once_cell::sync::Lazy::new(|| tokio::sync::Mutex::new(HashMap::new()));

pub struct PluginDbManager;

impl PluginDbManager {
    pub fn register_db_host_functions(
        builder: PluginBuilder,
        manifest_val: Value,
    ) -> PluginBuilder {
        let plugin_user_data = UserData::new(TransactionUserData {
            transaction: Arc::new(Mutex::new(None)),
        });
        let mut funcs: Vec<Function> = Vec::new();
        if let Some(list) = manifest_val
            .get("host_functions")
            .and_then(|v| v.as_array())
        {
            for name in list.iter().filter_map(|v| v.as_str()) {
                match name {
                    "start_hasura_transaction" => funcs.push(Function::new(
                        "start_hasura_transaction".to_string(),
                        vec![],
                        vec![ValType::I64], // TxResponse pointer
                        plugin_user_data.clone(),
                        start_hasura_transaction,
                    )),
                    "execute_transaction_query" => funcs.push(Function::new(
                        "execute_transaction_query".to_string(),
                        vec![ValType::I64], // TxExecRequest pointer
                        vec![ValType::I64], // TxExecResponse pointer
                        plugin_user_data.clone(),
                        execute_transaction_query,
                    )),
                    _ => {}
                }
            }
        }
        builder.with_functions(funcs.into_iter())
    }
}

// another alternative to start_hasura_transaction, using UserData.
host_fn!(start_transaction(user_data: TransactionUserData) {
    let transaction_context_data = user_data.get()?;
    let mut tx_data = transaction_context_data.lock().unwrap();

    let _ = tokio::runtime::Handle::current().block_on(async {
        let mut db_client = get_hasura_pool()
            .await
            .get()
            .await
            .map_err(|e| anyhow!("Error getting hasura client {}", e))?;
        let transaction = db_client
            .transaction()
            .await
            .map_err(|e| anyhow!("Error starting hasura transaction {}", e))?;
        transaction.execute("INSERT INTO kv_store (key, value) VALUES ($1, $2)", &[&key, &value])
            .await
            .map_err(|e| anyhow!("Error executing insert: {}", e))?;
        let transaction_arc = Arc::new(Mutex::new(Some(transaction)));
        tx_data.transaction = Arc::clone(&transaction_arc);
        Ok::<(), anyhow::Error>(())
    })?;
    Ok(())
}
);

fn start_hasura_transaction(
    plugin: &mut CurrentPlugin,
    _params: &[Val],
    results: &mut [Val],
    ud: UserData<()>,
) -> Result<()> {
    let transaction_id = tokio::runtime::Handle::current().block_on(async {
        // Move the client into the transaction and store both in a struct
        let mut hasura_db_client: DbClient = get_hasura_pool()
            .await
            .get()
            .await
            .map_err(|e| anyhow!("Error getting hasura client {}", e))?;
        let hasura_transaction = hasura_db_client
            .transaction()
            .await
            .map_err(|e| anyhow!("Error starting hasura transaction {}", e))?;
        let hasura_transaction_arc = Arc::new(Mutex::new(Some(hasura_transaction)));
        let transaction_id = uuid::Uuid::new_v4().to_string();
        TRANSACTION_STORE
            .lock()
            .await
            .insert(transaction_id.clone(), hasura_transaction_arc);
        Ok::<String, anyhow::Error>(transaction_id)
    })?;

    let resp = TxResponse { transaction_id };
    let out = serde_json::to_vec(&resp).map_err(|e| anyhow!(e.to_string()))?;
    // Write back into memory
    let out_ptr: extism::convert::MemoryHandle = plugin.memory_alloc(out.len() as u64)?;
    let mut ptr_val = Val::I64(out_ptr.offset() as i64);
    plugin.memory_set_val(&mut ptr_val, &out)?;

    // Return ptr and len
    results[0] = Val::I64(out_ptr.offset as i64);
    results[1] = Val::I64(out.len() as i64);
    Ok(())
}

// fn execute_transaction_query(
//     plugin: &mut CurrentPlugin,
//     params: &[Val],
//     results: &mut [Val],
//     ud: UserData<TransactionUserData>,
// ) -> Result<()> {
//     let offset: Val = params[0].clone();
//     let data: Vec<u8> = plugin.memory_get_val(&offset)?;

//     let user_data = ud.get()?;
//     let req: TxExecRequest = serde_json::from_slice(&data).map_err(|e| anyhow!("{e}"))?;

//     let mut query_params: Vec<Box<dyn tokio_postgres::types::ToSql + Sync>> = Vec::new();
//     if let Some(arr) = req.query.params.as_array() {
//         for v in arr {
//             let boxed: Box<dyn tokio_postgres::types::ToSql + Sync> = match v {
//                 Value::Null => Box::new(None::<i32>),
//                 Value::Bool(b) => Box::new(*b),
//                 Value::Number(n) => {
//                     if let Some(i) = n.as_i64() {
//                         Box::new(i)
//                     } else if let Some(f) = n.as_f64() {
//                         Box::new(f)
//                     } else {
//                         Box::new(0)
//                     }
//                 }
//                 Value::String(s) => {
//                     // Try UUID first
//                     if let Ok(u) = uuid::Uuid::parse_str(&s) {
//                         Box::new(u)
//                     } else {
//                         Box::new(s.clone())
//                     }
//                 }
//                 _ => return Err(anyhow!("Nested arrays/objects not supported")),
//             };
//             query_params.push(boxed);
//         }
//     }
//     // Create slice of &ToSql for execute
//     let query_params_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> =
//         query_params.iter().map(|b| &**b).collect();

//     let rows: Vec<tokio_postgres::Row> = tokio::runtime::Handle::current().block_on(async {
//         match user_data.lock() {
//             Ok(lock) => {
//                 let tx: Arc<Mutex<Option<Transaction<'_>>>> = lock.transaction.clone();
//                 let mut tx = tx.lock().await;
//                 let a = tx
//                     .as_mut()
//                     .ok_or_else(|| anyhow!("No active transaction"))?;
//                 let res = a
//                     .query(&req.query.statement, &query_params_refs)
//                     .await
//                     .map_err(|e| anyhow!(e))?;
//                 Ok(res)
//             }
//             Err(e) => Err(anyhow!("Error locking user data: {}", e)),
//         }
//     })?;

//     let mut res_rows = Vec::new();
//     for row in rows {
//         let mut obj = serde_json::Map::new();
//         for col in row.columns() {
//             let name = col.name();
//             let val: serde_json::Value = row.try_get(name).map_err(|e| anyhow!({ "{e}" }))?;
//             obj.insert(name.to_string(), val);
//         }
//         res_rows.push(serde_json::Value::Object(obj));
//     }
//     let json_result = serde_json::Value::Array(res_rows);

//     let db_resp = DbResponse {
//         rows_affected: json_result.as_array().unwrap().len() as u64,
//         result: json_result,
//     };
//     let resp = TxExecResponse { response: db_resp };
//     let out = serde_json::to_vec(&resp).map_err(|e| anyhow!({ "{e}" }))?;

//     let out_ptr: extism::convert::MemoryHandle = plugin.memory_alloc(out.len() as u64)?;
//     let mut ptr_val = Val::I64(out_ptr.offset() as i64);
//     plugin.memory_set_val(&mut ptr_val, &out)?;

//     // Return the pointer + length
//     results[0] = Val::I64(out_ptr.offset() as i64);
//     results[1] = Val::I64(out.len() as i64);
//     Ok(())
// }
