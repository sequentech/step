// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Route handlers behind Rocket's local client, with the dependencies they
//! reach through managed state replaced: pools on the migrated test
//! database, a local Keycloak stand-in, and in-memory fakes of everything
//! else.

use crate::adapters::memory::cast_votes::ScriptedCastVotes;
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

/// Core's bounded HTTP peer, standing in for Keycloak.
#[path = "../../../sequent-core/tests/support/http.rs"]
#[allow(dead_code)]
pub mod http;

pub struct Services {
    pub cast_votes: Arc<ScriptedCastVotes>,
    pub hasura: Arc<Pool>,
    pub keycloak: Arc<Pool>,
    pub documents: Arc<MemoryDocumentStorage>,
    pub electoral_log: Arc<MemoryElectoralLogs>,
    pub identity: Arc<LocalIdentityAdmin>,
    pub ledger: Arc<MemoryTaskLedger>,
    pub tasks: MemoryTaskQueue,
    pub vault: Arc<MemoryVault>,
}

impl Services {
    /// Both pools on the migrated test database.
    pub async fn on_test_database() -> Self {
        Self::with_pool(Arc::new(schema::pool().await))
    }

    /// Pools whose connections are refused.
    pub fn without_database() -> Self {
        Self::with_pool(Arc::new(unreachable_pool()))
    }

    fn with_pool(pool: Arc<Pool>) -> Self {
        Self {
            cast_votes: Default::default(),
            hasura: pool.clone(),
            keycloak: pool,
            documents: Default::default(),
            electoral_log: Default::default(),
            identity: Default::default(),
            ledger: Default::default(),
            tasks: Default::default(),
            vault: Default::default(),
        }
    }

    pub fn with_cast_votes(mut self, cast_votes: ScriptedCastVotes) -> Self {
        self.cast_votes = Arc::new(cast_votes);
        self
    }

    pub fn with_electoral_log(mut self, log: MemoryElectoralLogs) -> Self {
        self.electoral_log = Arc::new(log);
        self
    }

    pub fn with_identity(mut self, identity: LocalIdentityAdmin) -> Self {
        self.identity = Arc::new(identity);
        self
    }

    pub fn with_ledger(mut self, ledger: MemoryTaskLedger) -> Self {
        self.ledger = Arc::new(ledger);
        self
    }

    pub fn with_tasks(mut self, tasks: MemoryTaskQueue) -> Self {
        self.tasks = tasks;
        self
    }

    pub fn with_vault(mut self, vault: MemoryVault) -> Self {
        self.vault = Arc::new(vault);
        self
    }

    pub async fn client(&self) -> Client {
        self.client_with_log_level(rocket::config::LogLevel::Off)
            .await
    }

    pub async fn client_with_log_level(
        &self,
        log_level: rocket::config::LogLevel,
    ) -> Client {
        let services = HarvestServices {
            cast_votes: self.cast_votes.clone(),
            databases: Arc::new(FixedDatabasePools {
                hasura: self.hasura.clone(),
                keycloak: self.keycloak.clone(),
            }),
            documents: self.documents.clone(),
            electoral_log: self.electoral_log.clone(),
            identity: self.identity.clone(),
            ledger: self.ledger.clone(),
            tasks: Arc::new(self.tasks.clone()),
            vault: self.vault.clone(),
        };
        Client::tracked(crate::build_application_with(services).configure(
            rocket::Config {
                log_level,
                cli_colors: false,
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
