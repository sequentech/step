// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Route handlers behind Rocket's local client, with the dependencies they
//! reach through managed state replaced: pools on the migrated test
//! database, and in-memory task ledger, task queue and document storage.

use crate::adapters::memory::database::FixedDatabasePools;
use crate::adapters::memory::documents::MemoryDocumentStorage;
use crate::adapters::memory::electoral_log::MemoryElectoralLogs;
use crate::adapters::memory::identity::LocalIdentityAdmin;
use crate::adapters::memory::task_ledger::MemoryTaskLedger;
use crate::adapters::memory::task_queue::MemoryTaskQueue;
use crate::adapters::memory::vault::MemoryVault;
use crate::services::dependencies::HarvestServices;
use crate::test_claims::Claims;
use deadpool_postgres::{Pool, Runtime};
use rocket::http::{ContentType, Header, Status};
use rocket::local::asynchronous::{Client, LocalResponse};
use serde_json::Value;
use std::sync::Arc;
use tokio_postgres::NoTls;

#[path = "schema.rs"]
mod schema;

#[path = "rows.rs"]
pub mod rows;

pub struct Services {
    pub hasura: Arc<Pool>,
    pub keycloak: Arc<Pool>,
    pub documents: Arc<MemoryDocumentStorage>,
    pub electoral_log: Arc<MemoryElectoralLogs>,
    /// The local Keycloak stand-in admin clients talk to, if any.
    pub keycloak_url: Option<String>,
    pub ledger: Arc<MemoryTaskLedger>,
    pub tasks: MemoryTaskQueue,
    pub vault: Arc<MemoryVault>,
}

impl Services {
    /// Both pools on the migrated test database.
    pub async fn on_test_database() -> Self {
        let pool = Arc::new(schema::pool().await);
        Self {
            hasura: pool.clone(),
            keycloak: pool,
            documents: Default::default(),
            electoral_log: Default::default(),
            keycloak_url: None,
            ledger: Default::default(),
            tasks: Default::default(),
            vault: Default::default(),
        }
    }

    /// Pools whose connections are refused.
    pub fn without_database() -> Self {
        let pool = Arc::new(unreachable_pool());
        Self {
            hasura: pool.clone(),
            keycloak: pool,
            documents: Default::default(),
            electoral_log: Default::default(),
            keycloak_url: None,
            ledger: Default::default(),
            tasks: Default::default(),
            vault: Default::default(),
        }
    }

    pub fn with_ledger(mut self, ledger: MemoryTaskLedger) -> Self {
        self.ledger = Arc::new(ledger);
        self
    }

    pub fn with_tasks(mut self, tasks: MemoryTaskQueue) -> Self {
        self.tasks = tasks;
        self
    }

    pub fn with_electoral_log(mut self, log: MemoryElectoralLogs) -> Self {
        self.electoral_log = Arc::new(log);
        self
    }

    pub fn with_vault(mut self, vault: MemoryVault) -> Self {
        self.vault = Arc::new(vault);
        self
    }

    pub fn with_keycloak(mut self, url: &str) -> Self {
        self.keycloak_url = Some(url.to_string());
        self
    }

    pub async fn client(&self) -> Client {
        let services = HarvestServices {
            databases: Arc::new(FixedDatabasePools {
                hasura: self.hasura.clone(),
                keycloak: self.keycloak.clone(),
            }),
            documents: self.documents.clone(),
            electoral_log: self.electoral_log.clone(),
            identity: Arc::new(LocalIdentityAdmin {
                url: self.keycloak_url.clone(),
            }),
            ledger: self.ledger.clone(),
            tasks: Arc::new(self.tasks.clone()),
            vault: self.vault.clone(),
        };
        Client::tracked(crate::build_application_with(services).configure(
            rocket::Config {
                log_level: rocket::config::LogLevel::Off,
                ..Default::default()
            },
        ))
        .await
        .expect("the routes mount with test services")
    }
}

/// Nothing listens on port 1, so every connection attempt is refused.
fn unreachable_pool() -> Pool {
    deadpool_postgres::Config {
        host: Some("127.0.0.1".into()),
        port: Some(1),
        user: Some("test".into()),
        dbname: Some("test".into()),
        ..Default::default()
    }
    .create_pool(Some(Runtime::Tokio1), NoTls)
    .expect("pool configuration")
}

pub fn bearer(claims: &Claims) -> Header<'static> {
    Header::new("Authorization", claims.bearer())
}

/// POST a JSON body as the given caller.
pub async fn post<'c>(
    client: &'c Client,
    path: &'static str,
    claims: &Claims,
    body: &Value,
) -> LocalResponse<'c> {
    client
        .post(path)
        .header(ContentType::JSON)
        .header(bearer(claims))
        .body(body.to_string())
        .dispatch()
        .await
}

/// A handler's `(Status, String)` error: its status and plain-text body.
pub async fn text(response: LocalResponse<'_>) -> (Status, String) {
    let (status, content_type, body) = read(response).await;
    assert_eq!(content_type, Some(ContentType::Plain), "{status}: {body}");
    (status, body)
}

/// A JSON answer: its status and body.
pub async fn json(response: LocalResponse<'_>) -> (Status, Value) {
    let (status, content_type, body) = read(response).await;
    assert_eq!(content_type, Some(ContentType::JSON), "{status}: {body}");
    (status, serde_json::from_str(&body).expect("a JSON body"))
}

async fn read(
    response: LocalResponse<'_>,
) -> (Status, Option<ContentType>, String) {
    let status = response.status();
    let content_type = response.content_type();
    (
        status,
        content_type,
        response.into_string().await.unwrap_or_default(),
    )
}
