// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! A private database with every Hasura backend migration applied, created
//! once per test binary on the PostgreSQL server that `HASURA_DB__*` names.
//! Tests work inside a transaction and roll it back, so they only see their
//! own rows. Include it with `#[path = "support/schema.rs"] mod schema;`.
//!
//! A connection is driven by the runtime that opened it, and `#[tokio::test]`
//! gives each test its own runtime, so every call to [`pool`] returns a new
//! pool whose connections end with the test.

use deadpool_postgres::{Pool, Runtime};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::OnceCell;
use tokio_postgres::NoTls;
use windmill::services::database::PgConfig;

const PREFIX: &str = "windmill_schema_";
static DATABASE: OnceCell<String> = OnceCell::const_new();

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
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../hasura/migrations/backend-db");
    let mut directories: Vec<(u64, PathBuf)> = std::fs::read_dir(&root)
        .expect("backend-db migrations")
        .map(|entry| entry.expect("migration entry").path())
        .filter(|path| path.join("up.sql").is_file())
        .map(|path| {
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default();
            let version = name
                .split('_')
                .next()
                .and_then(|version| version.parse().ok())
                .unwrap_or_else(|| panic!("migration without a numeric version: {name}"));
            (version, path)
        })
        .collect();
    directories.sort();
    directories.into_iter().map(|(_, path)| path).collect()
}

async fn create() -> String {
    let maintenance = pool_for(
        &PgConfig::from_env()
            .expect("HASURA_DB__* must name the test PostgreSQL server")
            .hasura_db
            .dbname
            .expect("HASURA_DB__DBNAME"),
    );
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
    // The extensions .devcontainer/postgresql/init.sh creates.
    client
        .batch_execute(
            "CREATE EXTENSION IF NOT EXISTS pgcrypto; CREATE EXTENSION IF NOT EXISTS unaccent;",
        )
        .await
        .expect("fixture extensions");
    for migration in migrations() {
        let sql = std::fs::read_to_string(migration.join("up.sql")).expect("read up.sql");
        let transaction = client.transaction().await.expect("migration transaction");
        transaction
            .batch_execute(&sql)
            .await
            .unwrap_or_else(|error| panic!("{}: {error:?}", migration.display()));
        transaction.commit().await.expect("commit migration");
    }
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

/// A new pool on the migrated database, for the calling test only.
pub async fn pool() -> Pool {
    pool_for(DATABASE.get_or_init(create).await)
}
