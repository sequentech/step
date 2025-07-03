// SPDX-FileCopyrightText: 2024 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::database::{get_hasura_pool, get_keycloak_pool};
use crate::services::plugins_manager::plugin::PluginStore;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::{Object, Transaction};
use immudb_rs::client;
use ouroboros::self_referencing;
use std::future::Future;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_postgres::NoTls;
use wasmtime::component::{Component, Func, Instance, Linker, ResourceTable, Val};
use wasmtime::{Engine, Store, StoreContextMut};
use wasmtime_wasi::p2::{
    add_to_linker_async, add_to_linker_sync, IoView, WasiCtx, WasiCtxBuilder, WasiView,
};

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
        PluginDbManager::try_new(None, |_client_ref| Ok(None) as Result<_>)
            .expect("Failed to create TransactionComponent")
    }

    pub async fn create_hasura_transaction(&mut self) -> Result<()> {
        let client = get_hasura_pool()
            .await
            .get()
            .await
            .map_err(|e| anyhow!("Failed to get client: {}", e))?;

        let new_self = PluginDbManager::try_new_async_send(Some(client), |client_ref| {
            Box::pin(async move {
                if let Some(c) = client_ref {
                    let txn = c
                        .transaction()
                        .await
                        .map_err(|e| anyhow!("Failed to begin transaction: {}", e))?;
                    Ok::<Option<Transaction<'_>>, anyhow::Error>(Some(txn))
                } else {
                    Err(anyhow!("Client is None"))
                }
            })
        })
        .await?;

        *self = new_self;
        Ok(())
    }

    /// Execute a query on the live transaction.
    pub async fn exec(&mut self, sql: &str) -> Result<String> {
        // `with_txn` gives you a `&Transaction<'this>`
        let txn: &Transaction<'_> = self.with_txn(|opt| opt.as_ref()).unwrap();
        let rows = txn.query(sql, &[]).await?;
        Ok(format!("query {} rows", rows.len()))
    }

    /// Commit the transaction and consume the component.
    pub async fn commit(&mut self) -> Result<()> {
        let txn = self.with_txn_mut(|opt| opt.take()).unwrap();
        txn.commit().await?;
        Ok(())
    }
}
