// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! A private database with every Hasura backend migration applied, created
//! once per test binary on the PostgreSQL server that `HASURA_DB__*` names.
//! The same database holds the few Keycloak tables Harvest's user queries
//! read, in the public schema, where a Keycloak database keeps them.
//!
//! Route handlers commit their own transactions, so tests do not roll back:
//! each one works in a tenant of its own. Pools are cheap and tied to the
//! runtime that opens their connections, so every test takes a fresh one.

use deadpool_postgres::{Pool, Runtime};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::OnceCell;
use tokio_postgres::NoTls;
use windmill::services::database::PgConfig;

const PREFIX: &str = "harvest_schema_";
static DATABASE: OnceCell<String> = OnceCell::const_new();

/// The extensions and function `.devcontainer/postgresql/init.sh` creates.
const SERVER_SETUP: &str = r#"
CREATE EXTENSION IF NOT EXISTS pgcrypto;
CREATE EXTENSION IF NOT EXISTS unaccent;
CREATE OR REPLACE FUNCTION normalize_text(input_text TEXT)
RETURNS TEXT AS $$
BEGIN
RETURN lower(regexp_replace(unaccent(btrim(input_text)), '[-\s]+', '', 'g'));
END;
$$ LANGUAGE plpgsql IMMUTABLE;
"#;

/// The columns of Keycloak's own tables that Windmill's queries use.
const KEYCLOAK_TABLES: &str = r#"
CREATE TABLE realm (
    id varchar(36) PRIMARY KEY,
    name varchar(255) UNIQUE
);
CREATE TABLE user_entity (
    id varchar(36) PRIMARY KEY,
    email varchar(255),
    email_verified boolean NOT NULL DEFAULT false,
    enabled boolean NOT NULL DEFAULT false,
    first_name varchar(255),
    last_name varchar(255),
    realm_id varchar(255),
    username varchar(255),
    created_timestamp bigint,
    service_account_client_link varchar(255)
);
CREATE TABLE user_attribute (
    id varchar(36) PRIMARY KEY DEFAULT gen_random_uuid()::text,
    name varchar(255) NOT NULL,
    value varchar(255),
    user_id varchar(36) NOT NULL REFERENCES user_entity (id)
);
CREATE TABLE keycloak_group (
    id varchar(36) PRIMARY KEY,
    name varchar(255),
    realm_id varchar(36)
);
CREATE TABLE user_group_membership (
    group_id varchar(36) NOT NULL,
    user_id varchar(36) NOT NULL REFERENCES user_entity (id),
    PRIMARY KEY (group_id, user_id)
);
"#;

fn pool_for(database: &str) -> Pool {
    let mut config = PgConfig::from_env()
        .expect("HASURA_DB__* must name the test PostgreSQL server")
        .hasura_db;
    config.dbname = Some(database.to_owned());
    config
        .create_pool(Some(Runtime::Tokio1), NoTls)
        .expect("test database pool")
}

/// Migration directories in version order; each holds an `up.sql`.
fn migrations() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../hasura/migrations/backend-db");
    let mut directories: Vec<(u64, PathBuf)> = std::fs::read_dir(&root)
        .expect("backend-db migrations")
        .map(|entry| entry.expect("migration entry").path())
        .filter(|path| path.join("up.sql").is_file())
        .map(|path| {
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default()
                .to_owned();
            let version = name
                .split('_')
                .next()
                .and_then(|version| version.parse().ok())
                .unwrap_or_else(|| {
                    panic!("migration without a numeric version: {name}")
                });
            (version, path)
        })
        .collect();
    directories.sort();
    directories.into_iter().map(|(_, path)| path).collect()
}

async fn create() -> String {
    let server = PgConfig::from_env()
        .expect("HASURA_DB__* must name the test PostgreSQL server")
        .hasura_db
        .dbname
        .expect("HASURA_DB__DBNAME");
    let maintenance = pool_for(&server);
    let admin = maintenance.get().await.expect("maintenance connection");
    // Earlier runs leave their databases behind. Drop those nobody uses;
    // a database another test binary is using right now refuses to drop.
    for row in admin
        .query(
            "SELECT datname FROM pg_database WHERE starts_with(datname, $1)",
            &[&PREFIX],
        )
        .await
        .expect("list fixture databases")
    {
        let stale: String = row.get(0);
        let _ = admin
            .batch_execute(&format!("DROP DATABASE IF EXISTS \"{stale}\""))
            .await;
    }
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after 1970")
        .as_nanos();
    let database = format!("{PREFIX}{}_{nanos}", std::process::id());
    admin
        .batch_execute(&format!("CREATE DATABASE \"{database}\""))
        .await
        .expect("create fixture database");

    anchor(&database).await;
    let pool = pool_for(&database);
    let mut client = pool.get().await.expect("fixture connection");
    client
        .batch_execute(SERVER_SETUP)
        .await
        .expect("fixture extensions");
    for migration in migrations() {
        let sql = std::fs::read_to_string(migration.join("up.sql"))
            .expect("read up.sql");
        let transaction =
            client.transaction().await.expect("migration transaction");
        transaction
            .batch_execute(&sql)
            .await
            .unwrap_or_else(|error| {
                panic!("{}: {error:?}", migration.display())
            });
        transaction.commit().await.expect("commit migration");
    }
    client
        .batch_execute(KEYCLOAK_TABLES)
        .await
        .expect("Keycloak tables");
    database
}

/// Keeps one connection open until the process exits, on a thread of its own,
/// so another binary's cleanup above cannot drop the database between tests.
async fn anchor(database: &str) {
    let (connected, ready) = tokio::sync::oneshot::channel();
    let database = database.to_owned();
    std::thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("anchor runtime")
            .block_on(async move {
                let pool = pool_for(&database);
                let _connection = pool.get().await.expect("anchor connection");
                let _ = connected.send(());
                std::future::pending::<()>().await
            })
    });
    ready.await.expect("anchor connected");
}

/// A new pool on the migrated database.
pub async fn pool() -> Pool {
    pool_for(DATABASE.get_or_init(create).await)
}
