// SPDX-FileCopyrightText: 2024 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::database::{get_hasura_pool, get_keycloak_pool};
use crate::services::plugins_manager::plugin::PluginStore;
use deadpool_postgres::{Object, Transaction};
use std::future::Future;
use std::pin::Pin;
use wasmtime::component::Linker;
wasmtime::component::bindgen!({
    path: "src/services/plugins_manager/wit/transaction.wit",
    world: "transactions-manager",
    async: true,
});

use docs::transactions_manager::transaction::Host;
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

// Implement the generated trait for TransactionHost

impl Host for PluginStore {
    async fn create_hasura_transaction(&mut self) -> Result<String, String> {
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
        .map_err(|e| format!("{e}"))?;

        *self.hasura_manager.lock().await = new_self;
        Ok("Hasura transaction created".to_string())
    }

    async fn create_keycloak_transaction(&mut self) -> Result<(), String> {
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
        .expect("Failed to create transaction component");

        *self.keycloak_manager.lock().await = new_self;
        Ok(())
    }

    async fn execute_hasura_query(&mut self, sql: String) -> Result<String, String> {
        let mut manager = self.hasura_manager.lock().await;
        let hasura_transaction: &Transaction<'_> = manager
            .with_txn(|opt| opt.as_ref())
            .ok_or("No transaction")?;
        hasura_transaction.execute(&sql, &[]).await;
        Ok("".to_string())
    }

    async fn execute_keycloak_query(&mut self, sql: String) -> Result<String, String> {
        let mut manager = self.keycloak_manager.lock().await;
        let keycloak_transaction: &Transaction<'_> = manager
            .with_txn(|opt| opt.as_ref())
            .ok_or("No transaction")?;
        keycloak_transaction.execute(&sql, &[]).await;
        Ok("".to_string())
    }

    async fn commit_hasura_transaction(&mut self) -> Result<(), String> {
        let mut manager = self.hasura_manager.lock().await;
        let hasura_transaction: Transaction<'_> = manager
            .with_txn_mut(|opt| opt.take())
            .ok_or("No transaction")?;
        hasura_transaction.commit().await;
        Ok(())
    }

    async fn commit_keycloak_transaction(&mut self) -> Result<(), String> {
        let mut manager = self.keycloak_manager.lock().await;
        let keycloak_transaction: Transaction<'_> = manager
            .with_txn_mut(|opt| opt.take())
            .ok_or("No transaction")?;
        keycloak_transaction.commit().await;
        Ok(())
    }
}
