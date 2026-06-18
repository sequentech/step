// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{Context, Result};
use deadpool_postgres::{Config, Pool, Runtime};
use std::fs;
use std::path::PathBuf;
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt};
use testcontainers_modules::postgres::Postgres;
use tokio::sync::OnceCell;

const DEVCONTAINER_BOOTSTRAP_SQL: &str =
    include_str!("../../../../.devcontainer/postgresql/init.sql");

static SHARED_POSTGRES: OnceCell<SharedPostgres> = OnceCell::const_new();

pub async fn shared_pool() -> Result<Pool> {
    Ok(shared_postgres().await?.pool.clone())
}

struct SharedPostgres {
    _container: ContainerAsync<Postgres>,
    pool: Pool,
}

async fn shared_postgres() -> Result<&'static SharedPostgres> {
    SHARED_POSTGRES.get_or_try_init(init_shared_postgres).await
}

async fn init_shared_postgres() -> Result<SharedPostgres> {
    let mut image =
        Postgres::default().with_init_sql(DEVCONTAINER_BOOTSTRAP_SQL.as_bytes().to_vec());

    for migration_sql in migration_sql_scripts()? {
        image = image.with_init_sql(migration_sql);
    }

    let container = image.with_tag("18-bookworm").start().await?;
    let pool = create_pool(&container).await?;

    Ok(SharedPostgres {
        _container: container,
        pool,
    })
}

async fn create_pool(postgres: &ContainerAsync<Postgres>) -> Result<Pool> {
    let host = postgres.get_host().await?;
    let port = postgres.get_host_port_ipv4(5432).await?;

    let mut config = Config::new();
    config.host = Some(host.to_string());
    config.port = Some(port);
    config.user = Some("postgres".to_string());
    config.password = Some("postgres".to_string());
    config.dbname = Some("postgres".to_string());

    config
        .create_pool(Some(Runtime::Tokio1), tokio_postgres::NoTls)
        .context("failed to create test Postgres pool")
}

fn migration_sql_scripts() -> Result<Vec<Vec<u8>>> {
    let mut migration_dirs: Vec<PathBuf> = fs::read_dir(migrations_root())?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    migration_dirs.sort();

    migration_dirs
        .into_iter()
        .map(|dir| {
            let migration_path = dir.join("up.sql");
            fs::read(&migration_path).with_context(|| {
                format!(
                    "failed to read migration script {}",
                    migration_path.display()
                )
            })
        })
        .collect()
}

fn migrations_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../hasura/migrations/backend-db")
}
