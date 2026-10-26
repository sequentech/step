// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The users import a configuration package runs in-process for its
//! administrator accounts, on the worker's own pools, while the package
//! application holds its claim's connection. The airgap rehearsal ran workers
//! whose Hasura pool deadpool had sized from a half-CPU quota: two connections.
//! The import kept its connection after committing and its maintenance asked
//! for another, so it waited forever.
//!
//! This binary points the process-wide pools at the fixture database (as
//! postgres_task_adapters_global_pool.rs does), gives the Keycloak side the
//! tables and columns the import writes, serves the uploaded file from a
//! loopback S3 stub, and runs `import_users::execute` on a two-connection pool
//! with the claim held. Everything is synthetic and local.

// The import is one deeply nested future.
#![recursion_limit = "256"]

#[path = "support/s3_stub.rs"]
mod s3_stub;
#[path = "support/schema.rs"]
mod schema;

use deadpool_postgres::{Pool, PoolError, Timeouts};
use serde_json::json;
use std::ffi::OsString;
use std::future::Future;
use std::sync::LazyLock;
use std::time::Duration;
use tokio::runtime::Runtime;
use tokio::sync::OnceCell;
use windmill::postgres::tasks_execution::get_task_by_id;
use windmill::services::database::{get_hasura_pool, get_keycloak_pool};
use windmill::tasks::import_users::{execute, ImportUsersBody};

static FIXTURE: OnceCell<Pool> = OnceCell::const_new();

/// One runtime: the process-wide pools outlive any one test, and a pooled
/// connection is driven by the runtime that opened it.
static RUNTIME: LazyLock<Runtime> = LazyLock::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
});

fn run<F: Future>(test: F) -> F::Output {
    RUNTIME.block_on(test)
}

async fn current_database(pool: &Pool) -> String {
    pool.get()
        .await
        .unwrap()
        .query_one("SELECT current_database()", &[])
        .await
        .unwrap()
        .get(0)
}

fn pool_settings() -> Vec<(String, OsString)> {
    std::env::vars_os()
        .filter_map(|(key, value)| Some((key.into_string().ok()?, value)))
        .filter(|(key, _)| key.starts_with("HASURA_DB__") || key.starts_with("KEYCLOAK_DB__"))
        .collect()
}

/// Both process-wide pools on the fixture database.
async fn fixture() -> &'static Pool {
    FIXTURE
        .get_or_init(|| async {
            let pool = schema::pool().await;
            let database = current_database(&pool).await;
            let saved = pool_settings();
            for (key, _) in &saved {
                std::env::remove_var(key);
            }
            for (key, value) in &saved {
                if let Some(setting) = key.strip_prefix("HASURA_DB__") {
                    std::env::set_var(key, value);
                    std::env::set_var(format!("KEYCLOAK_DB__{setting}"), value);
                }
            }
            std::env::set_var("HASURA_DB__DBNAME", &database);
            std::env::set_var("KEYCLOAK_DB__DBNAME", &database);
            // The worker's Hasura pool as the rehearsal had it: two connections,
            // capped where deadpool caps it, when the pool is built (a later
            // resize leaves the semaphore's original permits).
            std::env::set_var("HASURA_DB__POOL__MAX_SIZE", "2");
            let (hasura, keycloak) = (get_hasura_pool().await, get_keycloak_pool().await);
            for (key, _) in pool_settings() {
                std::env::remove_var(key);
            }
            for (key, value) in &saved {
                std::env::set_var(key, value);
            }
            assert_eq!(current_database(&hasura).await, database);
            assert_eq!(current_database(&keycloak).await, database);
            // The Keycloak tables and columns the users import writes, with
            // Keycloak's own types (its schema is Liquibase's, not ours).
            pool.get()
                .await
                .unwrap()
                .batch_execute(
                    "CREATE TABLE IF NOT EXISTS realm (id varchar(36) PRIMARY KEY, name varchar(255) UNIQUE);
                     CREATE TABLE IF NOT EXISTS keycloak_group (id varchar(36) PRIMARY KEY,
                         name varchar(255), realm_id varchar(36), parent_group varchar(36));
                     CREATE TABLE IF NOT EXISTS user_entity (id varchar(36) PRIMARY KEY,
                         email varchar(255), email_constraint varchar(255), email_verified boolean,
                         enabled boolean, federation_link varchar(255), first_name varchar(255),
                         last_name varchar(255), realm_id varchar(255), username varchar(255),
                         created_timestamp bigint, service_account_client_link varchar(255),
                         not_before integer);
                     CREATE TABLE IF NOT EXISTS user_attribute (id varchar(36) PRIMARY KEY,
                         user_id varchar(36), name varchar(255), value varchar(255));
                     CREATE TABLE IF NOT EXISTS user_group_membership (group_id varchar(36),
                         user_id varchar(36), membership_type varchar(255));
                     CREATE TABLE IF NOT EXISTS credential (id varchar(36) PRIMARY KEY,
                         type varchar(255), user_id varchar(36), created_date bigint,
                         user_label varchar(255), secret_data text, credential_data text,
                         priority integer);",
                )
                .await
                .unwrap();
            pool
        })
        .await
}

const ACCOUNTS: &[u8] = b"username,email,first_name,last_name\n\
synthetic-operator,operator@example.invalid,Synthetic,Operator\n";

#[test]
fn the_admin_accounts_import_finishes_on_a_two_connection_pool_with_the_claim_held() {
    run(async {
        let pool = fixture().await;
        let (endpoint, asked) = s3_stub::serve(ACCOUNTS).await;
        // The worker's S3 settings, at the stub; fake credentials.
        std::env::set_var("AWS_REGION", "us-east-1");
        std::env::set_var("AWS_S3_PRIVATE_URI", &endpoint);
        std::env::set_var("AWS_S3_BUCKET", "synthetic-documents");
        std::env::set_var("AWS_S3_ACCESS_KEY", "synthetic");
        std::env::set_var("AWS_S3_ACCESS_SECRET", "synthetic");

        let (tenant, document, task, realm) = (
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
        );
        let setup = pool.get().await.unwrap();
        setup
            .execute(
                "INSERT INTO sequent_backend.tenant (id, slug, is_active) VALUES ($1, $2, true)",
                &[&tenant, &format!("accounts-{tenant}")],
            )
            .await
            .unwrap();
        setup
            .execute(
                "INSERT INTO sequent_backend.document (id, tenant_id, name) VALUES ($1, $2, 'admin_users.csv')",
                &[&document, &tenant],
            )
            .await
            .unwrap();
        setup
            .execute(
                "INSERT INTO sequent_backend.tasks_execution (id, tenant_id, name, type, execution_status, executed_by_user)
                 VALUES ($1, $2, 'Import users', 'IMPORT_USERS', 'IN_PROGRESS', 'synthetic-operator')",
                &[&task, &tenant],
            )
            .await
            .unwrap();
        setup
            .execute(
                "INSERT INTO realm (id, name) VALUES ($1, $2)",
                &[&realm.to_string(), &format!("tenant-{tenant}")],
            )
            .await
            .unwrap();
        drop(setup);
        let task_execution = get_task_by_id(&task.to_string()).await.unwrap();
        let body: ImportUsersBody = serde_json::from_value(json!({
            "tenant_id": tenant.to_string(),
            "document_id": document.to_string(),
            "election_event_id": null,
            "is_admin": true,
            "sha256": null,
        }))
        .unwrap();

        // The package application's claim holds one of the pool's two
        // connections throughout.
        let hasura = get_hasura_pool().await;
        let claim = hasura.get().await.unwrap();
        // The cap is real: with the claim and one more held, a third is refused
        // at once rather than created.
        let second = hasura.get().await.unwrap();
        let third = hasura
            .timeout_get(&Timeouts {
                wait: Some(Duration::ZERO),
                ..Default::default()
            })
            .await;
        assert!(
            matches!(third, Err(PoolError::Timeout(_))),
            "the Hasura pool is not capped at two connections"
        );
        drop(second);
        let finished =
            tokio::time::timeout(Duration::from_secs(60), execute(body, task_execution)).await;
        drop(claim);
        finished
            .expect("the import waited on the pool: claim, importer and maintenance")
            .expect("the import");

        let client = pool.get().await.unwrap();
        let status: String = client
            .query_one(
                "SELECT execution_status FROM sequent_backend.tasks_execution WHERE id = $1",
                &[&task],
            )
            .await
            .unwrap()
            .get(0);
        assert_eq!(status, "SUCCESS");
        let user: (String, String, Option<String>) = {
            let row = client
                .query_one(
                    "SELECT id, username, email FROM user_entity WHERE realm_id = $1",
                    &[&realm.to_string()],
                )
                .await
                .unwrap();
            (row.get(0), row.get(1), row.get(2))
        };
        assert_eq!(user.1, "synthetic-operator");
        assert_eq!(user.2.as_deref(), Some("operator@example.invalid"));
        let tenant_attribute: String = client
            .query_one(
                "SELECT value FROM user_attribute WHERE user_id = $1 AND name = 'tenant-id'",
                &[&user.0],
            )
            .await
            .unwrap()
            .get(0);
        assert_eq!(tenant_attribute, tenant.to_string());
        let asked = asked.lock().unwrap().clone();
        assert!(
            asked
                .iter()
                .any(|line| line.starts_with("GET ") && line.contains(&document.to_string())),
            "the file was read from the stub: {asked:?}"
        );
    })
}
