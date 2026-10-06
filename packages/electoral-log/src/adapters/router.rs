// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Routes each board to the database that holds it.
//!
//! With the `shared` layout every board lives in the database named by
//! `ELECTORAL_LOG_PG_DATABASE`. With `per-tenant`, each tenant's boards live in a
//! database of their own, named after the shared one, the environment slug and the
//! tenant. Boards that already exist in the shared database stay there, so an
//! installation can switch layouts without moving its logs.

use super::postgres::{PostgresConnection, PostgresStore};
use crate::domain::{ElectoralLogMessage, LogEntry, LogQuery};
use crate::ports::ElectoralLogStore;
use anyhow::{ensure, Context, Result};
use async_trait::async_trait;
use std::collections::HashMap;
use std::env;
use strum_macros::{Display, EnumString};
use tokio::sync::Mutex;
use tokio_postgres::error::SqlState;

/// Environment variable with the database layout: `shared` or `per-tenant`.
pub const LAYOUT_ENV: &str = "ELECTORAL_LOG_PG_DATABASE_LAYOUT";
/// Role that creates tenant databases; it needs `CREATEDB` and membership in the
/// application role, which owns the databases.
pub const PROVISIONING_USER_ENV: &str = "ELECTORAL_LOG_PG_PROVISIONING_USER";
pub const PROVISIONING_PASSWORD_ENV: &str = "ELECTORAL_LOG_PG_PROVISIONING_PASSWORD";
/// Database the provisioning role connects to in order to create others.
pub const PROVISIONING_DATABASE_ENV: &str = "ELECTORAL_LOG_PG_PROVISIONING_DATABASE";
/// Role that may only read the electoral-log databases, for administrators' queries.
/// When set, provisioning lets it read each new tenant database.
pub const READER_USER_ENV: &str = "ELECTORAL_LOG_PG_READER_USER";
pub const READER_PASSWORD_ENV: &str = "ELECTORAL_LOG_PG_READER_PASSWORD";
/// Connections per tenant database and process.
pub const TENANT_POOL_SIZE_ENV: &str = "ELECTORAL_LOG_PG_TENANT_POOL_SIZE";
pub const DEFAULT_PROVISIONING_DATABASE: &str = "postgres";
pub const DEFAULT_TENANT_POOL_SIZE: usize = 4;
/// Connections to the shared database per process.
pub const SHARED_POOL_SIZE: usize = 8;
/// Board names carry this many characters of the tenant ID, without dashes.
const TENANT_PREFIX_CHARS: usize = 17;
const MAX_DATABASE_NAME_BYTES: usize = 63;

/// Where boards are stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, EnumString)]
#[strum(serialize_all = "kebab-case")]
pub enum DatabaseLayout {
    /// Every board in one database.
    Shared,
    /// Each tenant's new boards in a database of their own.
    PerTenant,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Location {
    Shared,
    Tenant(String),
}

struct Provisioning {
    user: String,
    password: String,
    database: String,
}

pub struct StoreRouter {
    connection: PostgresConnection,
    layout: DatabaseLayout,
    slug: String,
    shared: PostgresStore,
    tenant_pool_size: usize,
    provisioning: Option<Provisioning>,
    reader: Option<String>,
    stores: Mutex<HashMap<String, PostgresStore>>,
    boards: Mutex<HashMap<String, Location>>,
}

impl StoreRouter {
    /// Configure from the `ELECTORAL_LOG_PG_*` variables and, for the `per-tenant`
    /// layout, `ENV_SLUG`.
    pub fn from_env() -> Result<Self> {
        let connection = PostgresConnection::from_env()?;
        let layout = match non_empty(env::var(LAYOUT_ENV).ok()) {
            None => DatabaseLayout::Shared,
            Some(value) => value.parse().map_err(|_| {
                anyhow::anyhow!("{LAYOUT_ENV} must be shared or per-tenant, got {value:?}")
            })?,
        };
        let slug = match layout {
            DatabaseLayout::Shared => String::new(),
            DatabaseLayout::PerTenant => {
                env::var("ENV_SLUG").context("ENV_SLUG must be set for per-tenant databases")?
            }
        };
        let tenant_pool_size = match non_empty(env::var(TENANT_POOL_SIZE_ENV).ok()) {
            None => DEFAULT_TENANT_POOL_SIZE,
            Some(value) => value
                .parse::<usize>()
                .ok()
                .filter(|size| *size > 0)
                .with_context(|| {
                    format!("{TENANT_POOL_SIZE_ENV} must be a positive integer, got {value:?}")
                })?,
        };
        let provisioning = match (
            non_empty(env::var(PROVISIONING_USER_ENV).ok()),
            env::var(PROVISIONING_PASSWORD_ENV).ok(),
        ) {
            (Some(user), Some(password)) => Some(Provisioning {
                user,
                password,
                database: non_empty(env::var(PROVISIONING_DATABASE_ENV).ok())
                    .unwrap_or_else(|| DEFAULT_PROVISIONING_DATABASE.to_string()),
            }),
            (Some(_), None) => anyhow::bail!("{PROVISIONING_PASSWORD_ENV} must be set"),
            (None, _) => None,
        };
        let mut router = Self::new(connection, layout, slug, tenant_pool_size, provisioning)?;
        router.reader = non_empty(env::var(READER_USER_ENV).ok());
        Ok(router)
    }

    fn new(
        connection: PostgresConnection,
        layout: DatabaseLayout,
        slug: String,
        tenant_pool_size: usize,
        provisioning: Option<Provisioning>,
    ) -> Result<Self> {
        let shared = connection.store(connection.database(), SHARED_POOL_SIZE)?;
        Ok(Self {
            connection,
            layout,
            slug,
            shared,
            tenant_pool_size,
            provisioning,
            reader: None,
            stores: Mutex::new(HashMap::new()),
            boards: Mutex::new(HashMap::new()),
        })
    }

    pub fn layout(&self) -> DatabaseLayout {
        self.layout
    }

    /// The store of the shared database.
    pub fn shared(&self) -> PostgresStore {
        self.shared.clone()
    }

    /// The store that holds a board.
    pub async fn store_for(&self, board: &str) -> Result<PostgresStore> {
        match self.locate(board).await? {
            Location::Shared => Ok(self.shared.clone()),
            Location::Tenant(database) => self.tenant_store(&database, false).await,
        }
    }

    /// Create a tenant's database, with the schema, if the layout gives tenants their
    /// own and it does not exist yet. Idempotent.
    pub async fn provision_tenant(&self, tenant_id: &str) -> Result<()> {
        if self.layout == DatabaseLayout::Shared {
            return Ok(());
        }
        let database = tenant_database_name(
            self.connection.database(),
            &self.slug,
            &tenant_prefix(tenant_id),
        )?;
        self.tenant_store(&database, true).await.map(|_| ())
    }

    /// The connection settings of the server.
    pub fn connection(&self) -> &PostgresConnection {
        &self.connection
    }

    /// The role of `ELECTORAL_LOG_PG_READER_USER`, if set.
    pub fn reader(&self) -> Option<&str> {
        self.reader.as_deref()
    }

    /// The database that holds only a tenant's boards, which exists only with the
    /// `per-tenant` layout.
    pub fn tenant_database(&self, tenant_id: &str) -> Result<String> {
        ensure!(
            self.layout == DatabaseLayout::PerTenant,
            "Tenants have no database of their own with the shared layout"
        );
        tenant_database_name(
            self.connection.database(),
            &self.slug,
            &tenant_prefix(tenant_id),
        )
    }

    /// A connection of its own to a tenant's database as the reader role, which
    /// needs the `per-tenant` layout and `ELECTORAL_LOG_PG_READER_USER` and
    /// `ELECTORAL_LOG_PG_READER_PASSWORD`.
    pub async fn reader_client(&self, tenant_id: &str) -> Result<tokio_postgres::Client> {
        let reader = self
            .reader()
            .with_context(|| format!("{READER_USER_ENV} is not set"))?;
        let password = env::var(READER_PASSWORD_ENV)
            .with_context(|| format!("{READER_PASSWORD_ENV} must be set"))?;
        let database = self.tenant_database(tenant_id)?;
        self.connection
            .client(&database, reader, &password)
            .await
            .with_context(|| format!("Error connecting to {database} as {reader}"))
    }

    /// Apply the schema to a tenant database and let the reader role, if set, read it.
    pub async fn initialize(&self, store: &PostgresStore) -> Result<()> {
        store.initialize().await?;
        if let Some(reader) = &self.reader {
            store.grant_read(reader).await?;
        }
        Ok(())
    }

    /// The tenant databases of this environment that exist on the server.
    pub async fn tenant_databases(&self) -> Result<Vec<String>> {
        if self.layout == DatabaseLayout::Shared {
            return Ok(Vec::new());
        }
        let prefix = tenant_database_prefix(self.connection.database(), &self.slug);
        let rows = self
            .shared
            .client()
            .await?
            .query(
                "SELECT datname FROM pg_database WHERE starts_with(datname, $1) ORDER BY datname",
                &[&prefix],
            )
            .await
            .context("Error listing the tenant electoral-log databases")?;
        Ok(rows.into_iter().map(|row| row.get(0)).collect())
    }

    /// The store of a database, with the shared database's credentials.
    pub async fn database_store(&self, database: &str) -> Result<PostgresStore> {
        if database == self.connection.database() {
            return Ok(self.shared.clone());
        }
        self.tenant_store(database, false).await
    }

    async fn locate(&self, board: &str) -> Result<Location> {
        if self.layout == DatabaseLayout::Shared {
            return Ok(Location::Shared);
        }
        if let Some(location) = self.boards.lock().await.get(board) {
            return Ok(location.clone());
        }
        let location = match board_tenant_prefix(board, &self.slug) {
            None => Location::Shared,
            Some(tenant) => {
                if self.shared.has_board(board).await? {
                    Location::Shared
                } else {
                    Location::Tenant(tenant_database_name(
                        self.connection.database(),
                        &self.slug,
                        tenant,
                    )?)
                }
            }
        };
        self.boards
            .lock()
            .await
            .insert(board.to_string(), location.clone());
        Ok(location)
    }

    async fn tenant_store(&self, database: &str, provision: bool) -> Result<PostgresStore> {
        let mut stores = self.stores.lock().await;
        if let Some(store) = stores.get(database) {
            return Ok(store.clone());
        }
        if provision {
            self.create_database(database).await?;
        }
        let store = self.connection.store(database, self.tenant_pool_size)?;
        if provision {
            self.initialize(&store)
                .await
                .with_context(|| format!("Error applying the schema to {database}"))?;
        }
        stores.insert(database.to_string(), store.clone());
        Ok(store)
    }

    async fn create_database(&self, database: &str) -> Result<()> {
        let provisioning = self.provisioning.as_ref().with_context(|| {
            format!(
                "Electoral-log database {database} cannot be created: \
                 {PROVISIONING_USER_ENV} is not set"
            )
        })?;
        let client = self
            .connection
            .client(
                &provisioning.database,
                &provisioning.user,
                &provisioning.password,
            )
            .await
            .context("Error connecting with the electoral-log provisioning role")?;
        let exists = client
            .query_opt("SELECT 1 FROM pg_database WHERE datname = $1", &[&database])
            .await?
            .is_some();
        if exists {
            return Ok(());
        }
        let statement = format!(
            "CREATE DATABASE {} OWNER {}",
            quote_identifier(database),
            quote_identifier(self.connection.user())
        );
        match client.batch_execute(&statement).await {
            Ok(()) => Ok(()),
            Err(error) if error.code() == Some(&SqlState::DUPLICATE_DATABASE) => Ok(()),
            Err(error) => Err(error)
                .with_context(|| format!("Error creating electoral-log database {database}")),
        }
    }
}

#[async_trait]
impl ElectoralLogStore for StoreRouter {
    async fn create_board(&self, board: &str) -> Result<()> {
        let store = match self.locate(board).await? {
            Location::Shared => self.shared.clone(),
            Location::Tenant(database) => self.tenant_store(&database, true).await?,
        };
        store.create_board(board).await
    }

    async fn delete_board(&self, board: &str) -> Result<()> {
        self.store_for(board).await?.delete_board(board).await?;
        self.boards.lock().await.remove(board);
        Ok(())
    }

    async fn has_board(&self, board: &str) -> Result<bool> {
        self.store_for(board).await?.has_board(board).await
    }

    async fn append(
        &self,
        board: &str,
        entries: &mut (dyn Iterator<Item = Result<LogEntry>> + Send),
    ) -> Result<()> {
        self.store_for(board).await?.append(board, entries).await
    }

    async fn query(&self, board: &str, query: &LogQuery) -> Result<Vec<ElectoralLogMessage>> {
        self.store_for(board).await?.query(board, query).await
    }

    async fn count(&self, board: &str, query: &LogQuery) -> Result<i64> {
        self.store_for(board).await?.count(board, query).await
    }
}

fn non_empty(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// The part of a tenant ID that board names carry.
pub fn tenant_prefix(tenant_id: &str) -> String {
    tenant_id
        .chars()
        .filter(|c| *c != '-')
        .take(TENANT_PREFIX_CHARS)
        .collect::<String>()
        .to_ascii_lowercase()
}

/// The tenant part of an election event's board name, `<slug>tenant<17 characters of
/// the tenant ID>event<event ID>`, or `None` for any other board.
fn board_tenant_prefix<'a>(board: &'a str, slug: &str) -> Option<&'a str> {
    let rest = board.strip_prefix(slug)?.strip_prefix("tenant")?;
    let tenant = rest.get(..TENANT_PREFIX_CHARS)?;
    let rest = rest.get(TENANT_PREFIX_CHARS..)?;
    (rest.starts_with("event") && tenant.chars().all(|c| c.is_ascii_hexdigit())).then_some(tenant)
}

fn tenant_database_prefix(shared_database: &str, slug: &str) -> String {
    let slug: String = slug
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    format!("{shared_database}_{slug}").to_ascii_lowercase()
}

/// Name of a tenant's database.
pub fn tenant_database_name(
    shared_database: &str,
    slug: &str,
    tenant_prefix: &str,
) -> Result<String> {
    let name = format!(
        "{}{tenant_prefix}",
        tenant_database_prefix(shared_database, slug)
    )
    .to_ascii_lowercase();
    ensure!(
        name.len() <= MAX_DATABASE_NAME_BYTES
            && name
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'),
        "Invalid tenant electoral-log database name {name:?}: it must be at most \
         {MAX_DATABASE_NAME_BYTES} lowercase letters, digits and underscores"
    );
    Ok(name)
}

fn quote_identifier(identifier: &str) -> String {
    format!("\"{}\"", identifier.replace('"', "\"\""))
}

#[cfg(test)]
mod tests {
    use super::*;

    const TENANT: &str = "90505c8a-23a9-4cdf-a26b-4e19f6a097d5";

    #[test]
    fn event_boards_name_their_tenant() {
        let board = "devtenant90505c8a23a94cdfaeventfdd21db2dd68497490eb7f2750b2b5df";
        assert_eq!(board_tenant_prefix(board, "dev"), Some("90505c8a23a94cdfa"));
        assert_eq!(tenant_prefix(TENANT), "90505c8a23a94cdfa");
        assert_eq!(board_tenant_prefix(board, "prod"), None);
        assert_eq!(board_tenant_prefix("devtenant90505c8a", "dev"), None);
        assert_eq!(
            board_tenant_prefix("devtenant90505c8a23a94cdfaelection1", "dev"),
            None
        );
        assert_eq!(
            board_tenant_prefix("devtenantzz505c8a23a94cdfaevent1", "dev"),
            None
        );
        assert_eq!(board_tenant_prefix("loadtest", "dev"), None);
    }

    #[test]
    fn tenant_databases_are_named_after_the_shared_one_the_slug_and_the_tenant() {
        assert_eq!(
            tenant_database_name("electoral_log", "dev", &tenant_prefix(TENANT)).unwrap(),
            "electoral_log_dev90505c8a23a94cdfa"
        );
        assert_eq!(
            tenant_database_name("electoral_log", "acme-prod", "90505c8a23a94cdfa").unwrap(),
            "electoral_log_acme_prod90505c8a23a94cdfa"
        );
        assert!(tenant_database_name(&"x".repeat(50), "dev", "90505c8a23a94cdfa").is_err());
        assert!(tenant_database_name("Electoral-Log", "dev", "90505c8a23a94cdfa").is_err());
    }

    #[test]
    fn layouts_have_stable_names() {
        assert_eq!(
            "shared".parse::<DatabaseLayout>().unwrap(),
            DatabaseLayout::Shared
        );
        assert_eq!(
            "per-tenant".parse::<DatabaseLayout>().unwrap(),
            DatabaseLayout::PerTenant
        );
        assert!("tenant".parse::<DatabaseLayout>().is_err());
    }

    #[test]
    fn identifiers_are_quoted() {
        assert_eq!(quote_identifier("electoral_log"), "\"electoral_log\"");
        assert_eq!(quote_identifier("a\"b"), "\"a\"\"b\"");
    }
}
