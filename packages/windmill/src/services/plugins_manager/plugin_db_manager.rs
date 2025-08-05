use crate::postgres::area::get_area_by_id;
// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::election::{get_election_by_id as get_election_by_id_postgres};
use crate::postgres::election_event::get_election_event_by_election_area as get_election_event_by_election_area_postgres;
use crate::postgres::results_event::get_results_event_by_id;
use crate::postgres::tally_session::get_tally_session_by_id;
use crate::postgres::tally_session_execution::get_last_tally_session_execution;
use crate::services::database::{get_hasura_pool, get_keycloak_pool};
use deadpool_postgres::{GenericClient, Object, Transaction};
use serde_json::{Value, Map};
use tokio_postgres::types::ToSql;
use tokio_postgres::Row;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;
use sequent_core::plugins_wit::lib::transactions_manager_bindings::plugins_manager::transactions_manager::{
    transaction::{Host as TransactionHost},
    postgres_queries::{Host as PostgresHost}
};
use uuid::Uuid;
#[ouroboros::self_referencing]
pub struct PluginDbManager {
    client: Option<Object>,

    /// a transaction borrowing from that client
    #[borrows(mut client)]
    #[not_covariant]
    txn: Option<Transaction<'this>>,
}

impl PluginDbManager {
    pub fn init() -> Self {
        PluginDbManager::try_new(None, |_client_ref| Ok(None) as Result<_, String>)
            .expect("Failed to create TransactionComponent")
    }
}

pub struct PluginTransactionsManager {
    hasura_manager: Arc<Mutex<PluginDbManager>>,
    keycloak_manager: Arc<Mutex<PluginDbManager>>,
}

impl PluginTransactionsManager {
    pub fn new(
        hasura_manager: Arc<Mutex<PluginDbManager>>,
        keycloak_manager: Arc<Mutex<PluginDbManager>>,
    ) -> Self {
        Self {
            hasura_manager,
            keycloak_manager,
        }
    }
}

pub fn parse_any_valid_uuid(s: &str) -> Option<Uuid> {
    Uuid::parse_str(s).ok()
}

fn parsed_transactions_query_results(
    results: Vec<Row>,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut rows_as_json_values: Vec<Value> = Vec::new();

    for row in results {
        let mut row_map = Map::new();

        for (i, column) in row.columns().iter().enumerate() {
            let column_name = column.name();
            let value: Value = match column.type_().name() {
                "int2" | "int4" | "int8" => row
                    .get::<usize, Option<i64>>(i)
                    .map_or(Value::Null, |val| val.into()),
                "float4" | "float8" => row
                    .get::<usize, Option<f64>>(i)
                    .map_or(Value::Null, |val| val.into()),
                "bool" => row
                    .get::<usize, Option<bool>>(i)
                    .map_or(Value::Null, |val| val.into()),
                "text" | "varchar" | "char" | "name" | "bpchar" => row
                    .get::<usize, Option<String>>(i)
                    .map_or(Value::Null, |val| val.into()),
                "json" | "jsonb" => row
                    .get::<usize, Option<Value>>(i)
                    .map_or(Value::Null, |val| val),
                "uuid" => row
                    .get::<usize, Option<Uuid>>(i)
                    .map_or(Value::Null, |val| val.to_string().into()),
                _ => Value::Null,
            };
            row_map.insert(column_name.to_string(), value);
        }
        rows_as_json_values.push(Value::Object(row_map));
    }

    // Serialize the vector of JSON objects (which represents a JSON array) to a String
    let json_string = serde_json::to_string(&rows_as_json_values)
        .map_err(|e| format!("Failed to serialize query results to JSON: {}", e))?;

    Ok(json_string)
}

//Implementing the Host trait for PluginTransactionsManager to handle database transactions
impl TransactionHost for PluginTransactionsManager {
    async fn create_hasura_transaction(&mut self) -> Result<(), String> {
        let mut manager = self.hasura_manager.lock().await;

        println!("Creating Hasura transaction");
        let hasura_client = get_hasura_pool()
            .await
            .get()
            .await
            .map_err(|e| format!("Failed to get hasura client: {}", e))?;

        let new_self = PluginDbManager::try_new_async_send(Some(hasura_client), |client_ref| {
            Box::pin(async move {
                match client_ref {
                    Some(client) => {
                        let txn: Transaction<'_> = client
                            .transaction()
                            .await
                            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
                        Ok::<Option<Transaction<'_>>, Box<dyn std::error::Error + Send + Sync>>(
                            Some(txn),
                        )
                    }
                    None => Ok(None),
                }
            })
                as Pin<
                    Box<
                        dyn Future<
                                Output = Result<
                                    Option<Transaction<'_>>,
                                    Box<dyn std::error::Error + Send + Sync>,
                                >,
                            > + Send,
                    >,
                >
        })
        .await
        .map_err(|e| format!("{}", e))?;

        *manager = new_self;
        Ok(())
    }

    async fn create_keycloak_transaction(&mut self) -> Result<(), String> {
        let mut manager = self.keycloak_manager.lock().await;

        let keycloak_client = get_keycloak_pool()
            .await
            .get()
            .await
            .map_err(|e| format!("Failed to get keycloak client: {}", e))?;

        let new_self = PluginDbManager::try_new_async_send(Some(keycloak_client), |client_ref| {
            Box::pin(async move {
                match client_ref {
                    Some(client) => {
                        let txn: Transaction<'_> = client
                            .transaction()
                            .await
                            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
                        Ok::<Option<Transaction<'_>>, Box<dyn std::error::Error + Send + Sync>>(
                            Some(txn),
                        )
                    }
                    None => Ok(None),
                }
            })
                as Pin<
                    Box<
                        dyn Future<
                                Output = Result<
                                    Option<Transaction<'_>>,
                                    Box<dyn std::error::Error + Send + Sync>,
                                >,
                            > + Send,
                    >,
                >
        })
        .await
        .map_err(|e| format!("{}", e))?;

        *manager = new_self;
        Ok(())
    }

    async fn execute_hasura_query(
        &mut self,
        sql: String,
        params: Vec<String>,
    ) -> Result<String, String> {
        let mut manager = self.hasura_manager.lock().await;

        let parsed_params: Vec<Box<dyn ToSql + Send + Sync>> = params
            .iter()
            .map(|p_ref| {
                let param_str: &str = p_ref.as_ref();

                if let Some(uuid) = parse_any_valid_uuid(param_str) {
                    Box::new(uuid) as Box<dyn ToSql + Send + Sync>
                } else {
                    Box::new(param_str.to_string()) as Box<dyn ToSql + Send + Sync>
                }
            })
            .collect();

        let hasura_transaction: &Transaction<'_> = manager
            .with_txn(|opt| opt.as_ref())
            .ok_or("No transaction")?;

        let query_params: Vec<&(dyn ToSql + Sync)> = parsed_params
            .iter()
            .map(|param| {
                let ref_p = param.as_ref();
                ref_p as &(dyn ToSql + Sync)
            })
            .collect();

        let results: Vec<Row> = hasura_transaction
            .query(&sql, query_params.as_slice())
            .await
            .map_err(|e| format!("Hasura query failed: {}", e))?;

        let json_string = parsed_transactions_query_results(results)
            .map_err(|e| format!("Failed to parse query results: {}", e))?;

        Ok(json_string)
    }

    async fn execute_keycloak_query(
        &mut self,
        sql: String,
        params: Vec<String>,
    ) -> Result<String, String> {
        let mut manager = self.keycloak_manager.lock().await;

        let parsed_params: Vec<Box<dyn ToSql + Send + Sync>> = params
            .iter()
            .map(|p_ref| {
                let param_str: &str = p_ref.as_ref();

                if let Some(uuid) = parse_any_valid_uuid(param_str) {
                    Box::new(uuid) as Box<dyn ToSql + Send + Sync>
                } else {
                    Box::new(param_str.to_string()) as Box<dyn ToSql + Send + Sync>
                }
            })
            .collect();

        let keycloak_transaction: &Transaction<'_> = manager
            .with_txn(|opt| opt.as_ref())
            .ok_or("No transaction")?;

        let query_params: Vec<&(dyn ToSql + Sync)> = parsed_params
            .iter()
            .map(|param| {
                let ref_p = param.as_ref();
                ref_p as &(dyn ToSql + Sync)
            })
            .collect();

        let results: Vec<Row> = keycloak_transaction
            .query(&sql, query_params.as_slice())
            .await
            .map_err(|e| format!("Keycloak query failed: {}", e))?;

        let json_string = parsed_transactions_query_results(results)
            .map_err(|e| format!("Failed to parse query results: {}", e))?;

        Ok(json_string)
    }

    async fn commit_hasura_transaction(&mut self) -> Result<(), String> {
        let mut manager = self.hasura_manager.lock().await;
        let hasura_transaction: Transaction<'_> = manager
            .with_txn_mut(|opt| opt.take())
            .ok_or("No transaction")?;
        hasura_transaction
            .commit()
            .await
            .map_err(|e| format!("Hasura commit failed: {}", e))?;
        Ok(())
    }

    async fn commit_keycloak_transaction(&mut self) -> Result<(), String> {
        let mut manager = self.keycloak_manager.lock().await;
        let keycloak_transaction: Transaction<'_> = manager
            .with_txn_mut(|opt| opt.take())
            .ok_or("No transaction")?;
        keycloak_transaction
            .commit()
            .await
            .map_err(|e| format!("Keycloak commit failed: {}", e))?;
        Ok(())
    }
}

impl PostgresHost for PluginTransactionsManager {
    async fn get_election_event_by_election_area(
        &mut self,
        tenant_id: String,
        election_id: String,
        area_id: String,
    ) -> Result<String, String> {
        println!(
            "Getting election event by election area: tenant_id={}, election_id={}, area_id={}",
            tenant_id, election_id, area_id
        );
        let mut manager = self.hasura_manager.lock().await;
        let hasura_transaction: &Transaction<'_> = manager
            .with_txn(|opt| opt.as_ref())
            .ok_or("No transaction")?;
        let res = get_election_event_by_election_area_postgres(
            hasura_transaction,
            &tenant_id,
            &election_id,
            &area_id,
        )
        .await
        .map_err(|e| format!("Failed to get election event by election area: {}", e))?;
        let str = serde_json::to_string(&res)
            .map_err(|e| format!("Failed to serialize election event: {}", e))?;
        Ok(str)
    }
    async fn get_election_by_id(
        &mut self,
        tenant_id: String,
        election_event_id: String,
        election_id: String,
    ) -> Result<Option<String>, String> {
        println!(
            "Getting election by id: tenant_id={}, election_event_id={}, election_id={}",
            tenant_id, election_event_id, election_id
        );
        let mut manager = self.hasura_manager.lock().await;
        let hasura_transaction: &Transaction<'_> = manager
            .with_txn(|opt| opt.as_ref())
            .ok_or("No transaction")?;
        let res = get_election_by_id_postgres(
            hasura_transaction,
            &tenant_id,
            &election_event_id,
            &election_id,
        )
        .await
        .map_err(|e| format!("Failed to get election by id: {}", e))?;
        if let Some(election) = res {
            let str = serde_json::to_string(&election)
                .map_err(|e| format!("Failed to serialize election: {}", e))?;
            Ok(Some(str))
        } else {
            Ok(None)
        }
    }
    async fn get_tally_session_by_id(
        &mut self,
        tenant_id: String,
        election_event_id: String,
        tally_session_id: String,
    ) -> Result<String, String> {
        let mut manager = self.hasura_manager.lock().await;
        let hasura_transaction: &Transaction<'_> = manager
            .with_txn(|opt| opt.as_ref())
            .ok_or("No transaction")?;
        let res = get_tally_session_by_id(
            hasura_transaction,
            &tenant_id,
            &election_event_id,
            &tally_session_id,
        )
        .await
        .map_err(|e| format!("Failed to get tally session by id: {}", e))?;
        let str = serde_json::to_string(&res)
            .map_err(|e| format!("Failed to serialize tally session: {}", e))?;
        Ok(str)
    }
    async fn get_document(
        &mut self,
        tenant_id: String,
        election_event_id: Option<String>,
        document_id: String,
    ) -> Result<String, String> {
        Ok("".to_string())
    }

    async fn get_area_by_id(
        &mut self,
        tenant_id: String,
        area_id: String,
    ) -> Result<Option<String>, String> {
        let mut manager = self.hasura_manager.lock().await;
        let hasura_transaction: &Transaction<'_> = manager
            .with_txn(|opt| opt.as_ref())
            .ok_or("No transaction")?;
        let res = get_area_by_id(hasura_transaction, &tenant_id, &area_id)
            .await
            .map_err(|e| format!("Failed to get area by id: {}", e))?;
        if let Some(area) = res {
            let str = serde_json::to_string(&area)
                .map_err(|e| format!("Failed to serialize area: {}", e))?;
            Ok(Some(str))
        } else {
            Ok(None)
        }
    }

    async fn get_last_tally_session_execution(
        &mut self,
        tenant_id: String,
        election_event_id: String,
        tally_session_id: String,
    ) -> Result<Option<String>, String> {
        let mut manager = self.hasura_manager.lock().await;
        let hasura_transaction: &Transaction<'_> = manager
            .with_txn(|opt| opt.as_ref())
            .ok_or("No transaction")?;
        let res = get_last_tally_session_execution(
            hasura_transaction,
            &tenant_id,
            &election_event_id,
            &tally_session_id,
        )
        .await
        .map_err(|e| format!("Failed to get last tally session execution: {}", e))?;
        if let Some(execution) = res {
            let str = serde_json::to_string(&execution)
                .map_err(|e| format!("Failed to serialize last tally session execution: {}", e))?;
            Ok(Some(str))
        } else {
            Ok(None)
        }
    }

    async fn get_results_event_by_id(
        &mut self,
        tenant_id: String,
        election_event_id: String,
        results_event_id: String,
    ) -> Result<String, String> {
        let mut manager = self.hasura_manager.lock().await;
        let hasura_transaction: &Transaction<'_> = manager
            .with_txn(|opt| opt.as_ref())
            .ok_or("No transaction")?;
        let res = get_results_event_by_id(
            hasura_transaction,
            &tenant_id,
            &election_event_id,
            &results_event_id,
        )
        .await
        .map_err(|e| format!("Failed to get results event by id: {}", e))?;
        let str = serde_json::to_string(&res)
            .map_err(|e| format!("Failed to serialize results event: {}", e))?;
        Ok(str)
    }
}
