// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
//! One electoral-log database per election event.
//!
//! An election event's board, the sealed logs it continues and its ballot box live in
//! a database of their own, named after the event: `<ELECTORAL_LOG_PG_DATABASE>_<event
//! ID as 32 hexadecimal digits>`. The database of `ELECTORAL_LOG_PG_DATABASE` holds the
//! catalog of these databases, `electoral_log_events`, and marks the events whose
//! ballot boxes have work for background tasks.
//!
//! A database is copied, backed up, restored and dropped with its event, and
//! everything a checkpoint commits to is in it.

use super::ballot_box::canonical_uuid;
use super::postgres::{LogScope, PostgresConnection, PostgresStore};
use crate::{
    domain::{ElectoralLogMessage, LogEntry, LogQuery},
    ports::ElectoralLogStore,
};
use anyhow::{anyhow, ensure, Context, Result};
use async_trait::async_trait;
use std::{
    collections::HashMap,
    env,
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime},
};
use tokio::sync::OnceCell;

/// Connections of each election event's pool.
pub const EVENT_POOL_SIZE_ENV: &str = "ELECTORAL_LOG_PG_EVENT_POOL_SIZE";
pub const DEFAULT_EVENT_POOL_SIZE: usize = 4;
/// Election event databases a process keeps pools of. Opening another one drops the
/// pool used least recently.
pub const OPEN_EVENTS_ENV: &str = "ELECTORAL_LOG_PG_OPEN_EVENTS";
pub const DEFAULT_OPEN_EVENTS: usize = 16;
/// Connections of the catalog's pool.
const CATALOG_POOL_SIZE: usize = 4;
/// Longest wait for another process's creation of an event database.
const CREATE_LOCK_TIMEOUT: &str = "120s";
/// PostgreSQL's limit on identifiers.
pub const MAX_DATABASE_NAME_BYTES: usize = 63;
/// Separates the base database's name from the event in an event database's name.
const DATABASE_NAME_SEPARATOR: &str = "_";
/// Text before the event ID at the end of an election event's board name.
const BOARD_EVENT_MARKER: &str = "event";
/// Length of an election event ID without hyphens.
const EVENT_HEX_LEN: usize = 32;
/// Connections idle for longer are closed.
const IDLE_CONNECTION: Duration = Duration::from_secs(60);
/// How often idle connections are looked for.
const IDLE_SWEEP: Duration = Duration::from_secs(30);
/// A process marks an event's ballot activity at most this often.
pub const ACTIVITY_MARK_INTERVAL: Duration = Duration::from_secs(10);
/// A mark is cleared only when older than this, which covers marks a process skipped
/// within `ACTIVITY_MARK_INTERVAL` and votes accepted while the event was checked.
pub const ACTIVITY_CLEAR_MARGIN: Duration = Duration::from_secs(60);
/// Session advisory lock that serializes the creation of event databases.
const CREATE_DATABASE_LOCK: i64 = 7_307_648_119_525_449_475;
/// Catalog of the event databases, applied to the base database.
pub const CATALOG_SCHEMA: &str = include_str!("../../catalog.sql");

/// How to mark an event's ballot activity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityMark {
    /// Unless this process marked the event in the last `ACTIVITY_MARK_INTERVAL`, as
    /// accepting a vote does.
    Throttled,
    /// Always, as before waiting for the sequencer.
    Immediate,
}

/// An election event with a database of its own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventDatabase {
    pub election_event_id: String,
    pub tenant_id: String,
    pub database_name: String,
}

/// An event marked as having ballots that may wait for the sequencer or for review.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BallotActivity {
    pub election_event_id: String,
    pub tenant_id: String,
    pub marked_at: SystemTime,
    /// The server's time when the mark was read, before the event was checked.
    pub listed_at: SystemTime,
}

/// Pools of the event databases a process uses.
struct OpenEvents {
    stores: HashMap<String, (PostgresStore, Instant)>,
    swept: Instant,
}

struct Inner {
    connection: PostgresConnection,
    catalog: PostgresStore,
    catalog_ready: OnceCell<()>,
    pool_size: usize,
    capacity: usize,
    open: Mutex<OpenEvents>,
    marked: Mutex<HashMap<String, Instant>>,
}

/// The election events' databases on the electoral-log server, with a pool per event
/// in use. Cloning shares the pools.
#[derive(Clone)]
pub struct EventDatabases {
    inner: Arc<Inner>,
}

/// Read a positive count from `name`, or `default` when unset.
fn positive_env(name: &str, default: usize) -> Result<usize> {
    match env::var(name) {
        Ok(value) if !value.trim().is_empty() => {
            let count: usize = value
                .trim()
                .parse()
                .with_context(|| format!("{name} must be a positive integer"))?;
            ensure!(count > 0, "{name} must be a positive integer");
            Ok(count)
        }
        _ => Ok(default),
    }
}

/// Quote a name generated here as a PostgreSQL identifier.
fn quote_identifier(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

/// The election event whose board this is: its name ends with `event` and the event
/// ID without hyphens.
pub fn event_of_board(board: &str) -> Result<String> {
    let invalid = || anyhow!("{board:?} is not an election event's board");
    let split = board
        .len()
        .checked_sub(EVENT_HEX_LEN)
        .filter(|split| board.is_char_boundary(*split))
        .ok_or_else(invalid)?;
    let (prefix, hex) = board.split_at(split);
    ensure!(prefix.ends_with(BOARD_EVENT_MARKER), invalid());
    canonical_uuid(hex).map_err(|_| invalid())
}

/// Whether `database` is named as an election event's database of base database
/// `base`.
fn is_event_database_of(base: &str, database: &str) -> bool {
    database
        .strip_prefix(base)
        .and_then(|rest| rest.strip_prefix(DATABASE_NAME_SEPARATOR))
        .is_some_and(|hex| {
            hex.len() == EVENT_HEX_LEN
                && hex
                    .chars()
                    .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
        })
}

impl EventDatabases {
    /// Read the `ELECTORAL_LOG_PG_*` variables.
    pub fn from_env() -> Result<Self> {
        Self::new(
            PostgresConnection::from_env()?,
            positive_env(EVENT_POOL_SIZE_ENV, DEFAULT_EVENT_POOL_SIZE)?,
            positive_env(OPEN_EVENTS_ENV, DEFAULT_OPEN_EVENTS)?,
        )
    }

    /// Event databases on the server of `connection`, whose database is the base: it
    /// names the event databases and holds their catalog.
    pub fn new(connection: PostgresConnection, pool_size: usize, capacity: usize) -> Result<Self> {
        let base = connection.database();
        ensure!(
            !base.is_empty()
                && base
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'),
            "ELECTORAL_LOG_PG_DATABASE must hold only lowercase letters, digits and \
             underscores, as event databases are named after it"
        );
        ensure!(
            base.len() + DATABASE_NAME_SEPARATOR.len() + EVENT_HEX_LEN <= MAX_DATABASE_NAME_BYTES,
            "ELECTORAL_LOG_PG_DATABASE is too long to name event databases after it: at most \
             {} bytes",
            MAX_DATABASE_NAME_BYTES - DATABASE_NAME_SEPARATOR.len() - EVENT_HEX_LEN
        );
        let catalog = connection.store_of(base, CATALOG_POOL_SIZE)?;
        Ok(Self {
            inner: Arc::new(Inner {
                connection,
                catalog,
                catalog_ready: OnceCell::new(),
                pool_size,
                capacity,
                open: Mutex::new(OpenEvents {
                    stores: HashMap::new(),
                    swept: Instant::now(),
                }),
                marked: Mutex::new(HashMap::new()),
            }),
        })
    }

    /// Connection settings of the server.
    pub fn connection(&self) -> &PostgresConnection {
        &self.inner.connection
    }

    /// The store of the base database, which holds the catalog.
    pub fn catalog(&self) -> &PostgresStore {
        &self.inner.catalog
    }

    /// Name of an election event's database.
    pub fn database_name(&self, election_event_id: &str) -> Result<String> {
        let event = canonical_uuid(election_event_id)?.replace('-', "");
        Ok(format!(
            "{}{DATABASE_NAME_SEPARATOR}{event}",
            self.inner.connection.database()
        ))
    }

    /// Whether a database of the server is named as an election event's database.
    pub fn is_event_database(&self, database: &str) -> bool {
        is_event_database_of(self.inner.connection.database(), database)
    }

    /// Create the catalog in the base database. Idempotent; run by provisioning and
    /// before the first event database a process creates.
    pub async fn initialize(&self) -> Result<()> {
        self.inner
            .catalog_ready
            .get_or_try_init(|| async {
                self.inner
                    .catalog
                    .client()
                    .await?
                    .batch_execute(CATALOG_SCHEMA)
                    .await
                    .context("Error creating the electoral-log catalog")
            })
            .await?;
        Ok(())
    }

    /// The store of a registered event's database, refusing an event of another
    /// tenant.
    fn registered_store(&self, entry: &EventDatabase, tenant: &str) -> Result<PostgresStore> {
        ensure!(
            entry.tenant_id == tenant,
            "Election event {} is registered to tenant {}, not {tenant}",
            entry.election_event_id,
            entry.tenant_id
        );
        self.open(&entry.election_event_id, &entry.database_name)
    }

    /// Create an election event's database, its schema and its catalog entry, and let
    /// the roles of `ELECTORAL_LOG_PG_READER_USER` and `ELECTORAL_LOG_PG_BACKUP_USER`
    /// read it. Needs a role that may create databases.
    ///
    /// An event already registered to the tenant is returned as it is, without running
    /// any DDL on its database; one registered to another tenant is refused.
    /// `apply_schema` upgrades an existing event's database.
    pub async fn create_event(
        &self,
        tenant_id: &str,
        election_event_id: &str,
    ) -> Result<PostgresStore> {
        self.initialize().await?;
        let event = canonical_uuid(election_event_id)?;
        let tenant = canonical_uuid(tenant_id)?;
        if let Some(entry) = self.event(&event).await? {
            return self.registered_store(&entry, &tenant);
        }
        let client = self.locked_client().await?;
        if let Some(entry) = self.event(&event).await? {
            return self.registered_store(&entry, &tenant);
        }
        let database = self.database_name(&event)?;
        let quoted = quote_identifier(&database);
        let exists = client
            .query_opt("SELECT 1 FROM pg_database WHERE datname = $1", &[&database])
            .await?
            .is_some();
        if !exists {
            client
                .batch_execute(&format!("CREATE DATABASE {quoted}"))
                .await
                .with_context(|| format!("Error creating database {database}"))?;
        }
        // Also after a creation that stopped before registering the event.
        client
            .batch_execute(&format!(
                "REVOKE CONNECT, TEMPORARY ON DATABASE {quoted} FROM PUBLIC"
            ))
            .await?;
        let store = self.open(&event, &database)?;
        self.apply_schema_to(&client, &store, &database).await?;
        client
            .execute(
                "INSERT INTO electoral_log_events (election_event_id, tenant_id, database_name) \
                 VALUES ($1::text::uuid, $2::text::uuid, $3)",
                &[&event, &tenant, &database],
            )
            .await
            .with_context(|| format!("Error registering the database of election event {event}"))?;
        Self::unlock(&client).await?;
        Ok(store)
    }

    /// A connection of its own to the base database holding the lock that makes the
    /// creations and drops of event databases take turns. The lock ends with the
    /// connection, also when a call is dropped half way.
    async fn locked_client(&self) -> Result<tokio_postgres::Client> {
        let client = self
            .inner
            .connection
            .client_of(self.inner.connection.database())
            .await?;
        client
            .batch_execute(&format!("SET lock_timeout = '{CREATE_LOCK_TIMEOUT}'"))
            .await?;
        client
            .execute("SELECT pg_advisory_lock($1)", &[&CREATE_DATABASE_LOCK])
            .await
            .context("Error waiting for another creation or drop of an event database")?;
        Ok(client)
    }

    async fn unlock(client: &tokio_postgres::Client) -> Result<()> {
        client
            .execute("SELECT pg_advisory_unlock($1)", &[&CREATE_DATABASE_LOCK])
            .await?;
        Ok(())
    }

    /// Apply the schema to a registered event's database again, and let the reader
    /// and backup roles read it, as an upgrade does. Takes locks that wait for the
    /// event's appends and audits.
    pub async fn apply_schema(&self, election_event_id: &str) -> Result<()> {
        let event = canonical_uuid(election_event_id)?;
        let entry = self
            .event(&event)
            .await?
            .with_context(|| format!("Election event {event} has no electoral-log database"))?;
        let client = self
            .inner
            .connection
            .client_of(self.inner.connection.database())
            .await?;
        let store = self.open(&event, &entry.database_name)?;
        self.apply_schema_to(&client, &store, &entry.database_name)
            .await
    }

    async fn apply_schema_to(
        &self,
        base: &tokio_postgres::Client,
        store: &PostgresStore,
        database: &str,
    ) -> Result<()> {
        let readers = PostgresConnection::read_roles();
        for reader in &readers {
            base.batch_execute(&format!(
                "GRANT CONNECT ON DATABASE {} TO {}",
                quote_identifier(database),
                quote_identifier(reader)
            ))
            .await
            .with_context(|| format!("Error letting {reader} connect to {database}"))?;
        }
        store.initialize().await?;
        for reader in &readers {
            store.grant_read(reader).await?;
        }
        Ok(())
    }

    /// Drop an election event's database and its catalog entry, closing every
    /// connection to it, those of other processes included. Idempotent. Refuses an
    /// event registered to another tenant.
    pub async fn drop_event(&self, tenant_id: &str, election_event_id: &str) -> Result<()> {
        self.initialize().await?;
        let event = canonical_uuid(election_event_id)?;
        let tenant = canonical_uuid(tenant_id)?;
        let client = self.locked_client().await?;
        if let Some(entry) = self.event(&event).await? {
            ensure!(
                entry.tenant_id == tenant,
                "Election event {event} is registered to tenant {}, not {tenant}",
                entry.tenant_id
            );
        }
        let database = self.database_name(&event)?;
        if let Some((store, _)) = self.lock_open().stores.remove(&event) {
            store.close();
        }
        self.lock_marked().remove(&event);
        client
            .batch_execute(&format!(
                "DROP DATABASE IF EXISTS {} WITH (FORCE)",
                quote_identifier(&database)
            ))
            .await
            .with_context(|| format!("Error dropping database {database}"))?;
        client
            .execute(
                "DELETE FROM electoral_log_events WHERE election_event_id = $1::text::uuid",
                &[&event],
            )
            .await?;
        Self::unlock(&client).await
    }

    /// The catalog entry of an election event.
    pub async fn event(&self, election_event_id: &str) -> Result<Option<EventDatabase>> {
        let event = canonical_uuid(election_event_id)?;
        self.initialize().await?;
        let row = self
            .inner
            .catalog
            .client()
            .await?
            .query_opt(
                "SELECT election_event_id::text, tenant_id::text, database_name \
                 FROM electoral_log_events WHERE election_event_id = $1::text::uuid",
                &[&event],
            )
            .await?;
        row.map(|row| {
            Ok(EventDatabase {
                election_event_id: row.try_get(0)?,
                tenant_id: row.try_get(1)?,
                database_name: row.try_get(2)?,
            })
        })
        .transpose()
    }

    /// Every election event with a database, by event ID.
    pub async fn events(&self) -> Result<Vec<EventDatabase>> {
        self.initialize().await?;
        let rows = self
            .inner
            .catalog
            .client()
            .await?
            .query(
                "SELECT election_event_id::text, tenant_id::text, database_name \
                 FROM electoral_log_events ORDER BY election_event_id",
                &[],
            )
            .await?;
        rows.into_iter()
            .map(|row| {
                Ok(EventDatabase {
                    election_event_id: row.try_get(0)?,
                    tenant_id: row.try_get(1)?,
                    database_name: row.try_get(2)?,
                })
            })
            .collect()
    }

    /// Whether an election event has a database. An event whose pool this process
    /// keeps has one, unless another process dropped it since.
    pub async fn has_event(&self, election_event_id: &str) -> Result<bool> {
        let event = canonical_uuid(election_event_id)?;
        if self.cached(&event).is_some() {
            return Ok(true);
        }
        Ok(self.event(&event).await?.is_some())
    }

    /// The store of an election event's database.
    pub async fn store(&self, election_event_id: &str) -> Result<PostgresStore> {
        let event = canonical_uuid(election_event_id)?;
        if let Some(store) = self.cached(&event) {
            return Ok(store);
        }
        let entry = self
            .event(&event)
            .await?
            .with_context(|| format!("Election event {event} has no electoral-log database"))?;
        self.open(&event, &entry.database_name)
    }

    /// The store of the election event whose board this is.
    pub async fn board_store(&self, board: &str) -> Result<PostgresStore> {
        self.store(&event_of_board(board)?).await
    }

    /// The store of the election event whose board this is, when the board is in it.
    /// A board name of another tenant with the same event ID reads nothing.
    async fn existing_board_store(&self, board: &str) -> Result<Option<PostgresStore>> {
        let event = event_of_board(board)?;
        if !self.has_event(&event).await? {
            return Ok(None);
        }
        let store = self.store(&event).await?;
        Ok(store.has_board(board).await?.then_some(store))
    }

    /// A connection of its own to an election event's database as the reader role.
    pub async fn reader_client(&self, election_event_id: &str) -> Result<tokio_postgres::Client> {
        let event = canonical_uuid(election_event_id)?;
        let entry = self
            .event(&event)
            .await?
            .with_context(|| format!("Election event {event} has no electoral-log database"))?;
        self.inner
            .connection
            .reader_client_of(&entry.database_name)
            .await
    }

    /// Mark that an event's ballot box may have ballots waiting for the sequencer or
    /// for review. Mark before accepting a vote, so that no accepted vote goes
    /// unmarked.
    pub async fn mark_ballot_activity(
        &self,
        election_event_id: &str,
        mark: ActivityMark,
    ) -> Result<()> {
        let event = canonical_uuid(election_event_id)?;
        if mark == ActivityMark::Throttled {
            let marked = self.lock_marked().get(&event).copied();
            if marked.is_some_and(|at| at.elapsed() < ACTIVITY_MARK_INTERVAL) {
                return Ok(());
            }
        }
        let started = Instant::now();
        self.inner
            .catalog
            .client()
            .await?
            .execute(
                "UPDATE electoral_log_events SET ballots_accepted_at = now() \
                 WHERE election_event_id = $1::text::uuid",
                &[&event],
            )
            .await
            .context("Error marking the ballot activity of an election event")?;
        self.lock_marked().insert(event, started);
        Ok(())
    }

    /// Events marked as having ballot activity.
    pub async fn events_with_ballot_activity(&self) -> Result<Vec<BallotActivity>> {
        self.initialize().await?;
        let rows = self
            .inner
            .catalog
            .client()
            .await?
            .query(
                "SELECT election_event_id::text, tenant_id::text, ballots_accepted_at, \
                 now() AS listed_at FROM electoral_log_events \
                 WHERE ballots_accepted_at IS NOT NULL ORDER BY ballots_accepted_at",
                &[],
            )
            .await?;
        rows.into_iter()
            .map(|row| {
                Ok(BallotActivity {
                    election_event_id: row.try_get(0)?,
                    tenant_id: row.try_get(1)?,
                    marked_at: row.try_get(2)?,
                    listed_at: row.try_get(3)?,
                })
            })
            .collect()
    }

    /// Clear an event's mark, read as `marked_at` at `listed_at`, after finding no
    /// ballot waiting for the sequencer or for review. Nothing changes when the event
    /// was marked again since, or when the mark was younger than
    /// `ACTIVITY_CLEAR_MARGIN` when it was read: a vote accepted under it may have
    /// been stored after the check. Returns whether the mark was cleared.
    pub async fn clear_ballot_activity(&self, activity: &BallotActivity) -> Result<bool> {
        let margin = f64::from(u32::try_from(ACTIVITY_CLEAR_MARGIN.as_secs())?);
        let cleared = self
            .inner
            .catalog
            .client()
            .await?
            .execute(
                "UPDATE electoral_log_events SET ballots_accepted_at = NULL \
                 WHERE election_event_id = $1::text::uuid AND ballots_accepted_at = $2 \
                 AND ballots_accepted_at < $3::timestamptz - make_interval(secs => $4)",
                &[
                    &activity.election_event_id,
                    &activity.marked_at,
                    &activity.listed_at,
                    &margin,
                ],
            )
            .await?;
        Ok(cleared > 0)
    }

    fn lock_open(&self) -> std::sync::MutexGuard<'_, OpenEvents> {
        // A panic while holding the lock leaves the map usable.
        self.inner
            .open
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn lock_marked(&self) -> std::sync::MutexGuard<'_, HashMap<String, Instant>> {
        self.inner
            .marked
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// A cached store, also closing idle connections of every cached store now and
    /// then.
    fn cached(&self, event: &str) -> Option<PostgresStore> {
        let mut open = self.lock_open();
        if open.swept.elapsed() >= IDLE_SWEEP {
            open.swept = Instant::now();
            for (store, _) in open.stores.values() {
                store.close_idle(IDLE_CONNECTION);
            }
        }
        let (store, used) = open.stores.get_mut(event)?;
        *used = Instant::now();
        Some(store.clone())
    }

    /// Open a pool to an event's database, or return the one open, dropping the pool
    /// used least recently when too many are open. Requests holding a dropped pool's
    /// store finish with it.
    fn open(&self, event: &str, database: &str) -> Result<PostgresStore> {
        let mut open = self.lock_open();
        if let Some((store, used)) = open.stores.get_mut(event) {
            *used = Instant::now();
            return Ok(store.clone());
        }
        while open.stores.len() >= self.inner.capacity {
            let Some(oldest) = open
                .stores
                .iter()
                .min_by_key(|(_, (_, used))| *used)
                .map(|(event, _)| event.clone())
            else {
                break;
            };
            open.stores.remove(&oldest);
        }
        let store = self
            .inner
            .connection
            .store_of(database, self.inner.pool_size)?;
        open.stores
            .insert(event.to_string(), (store.clone(), Instant::now()));
        Ok(store)
    }
}

/// Boards route to their election event's database. Reads cover every record of the
/// database, the event's board and the sealed logs it continues, when the board is in
/// it; reads of a board that is not read nothing.
#[async_trait]
impl ElectoralLogStore for EventDatabases {
    async fn create_board(&self, board: &str) -> Result<()> {
        self.board_store(board).await?.create_board(board).await
    }

    async fn delete_board(&self, board: &str) -> Result<()> {
        match self.existing_board_store(board).await? {
            Some(store) => store.delete_board(board).await,
            None => Ok(()),
        }
    }

    async fn has_board(&self, board: &str) -> Result<bool> {
        Ok(self.existing_board_store(board).await?.is_some())
    }

    async fn append(
        &self,
        board: &str,
        entries: &mut (dyn Iterator<Item = Result<LogEntry>> + Send),
    ) -> Result<()> {
        self.board_store(board).await?.append(board, entries).await
    }

    async fn query(&self, board: &str, query: &LogQuery) -> Result<Vec<ElectoralLogMessage>> {
        match self.existing_board_store(board).await? {
            Some(store) => store.query_scope(LogScope::Database, query).await,
            None => Ok(vec![]),
        }
    }

    async fn count(&self, board: &str, query: &LogQuery) -> Result<i64> {
        match self.existing_board_store(board).await? {
            Some(store) => store.count_scope(LogScope::Database, query).await,
            None => Ok(0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boards_name_their_election_event() {
        assert_eq!(
            event_of_board("devtenant90505c8a23a94cdf2event0d1b2c3d4e5f40718293a4b5c6d7e8f9")
                .unwrap(),
            "0d1b2c3d-4e5f-4071-8293-a4b5c6d7e8f9"
        );
    }

    #[test]
    fn boards_without_an_event_are_refused() {
        for board in [
            "",
            "event",
            "tenantabcelection0d1b2c3d4e5f40718293a4b5c6d7e8f9",
            "tenantabcevent0d1b2c3d4e5f40718293a4b5c6d7e8fz",
            "tenantabcevent0d1b2c3d4e5f40718293a4b5c6d7e8f",
            "tenantabcévent0d1b2c3d4e5f40718293a4b5c6d7e8f9",
        ] {
            assert!(event_of_board(board).is_err(), "{board:?}");
        }
    }

    #[test]
    fn event_databases_are_told_apart_by_name() {
        let base = "electoral_log";
        assert!(is_event_database_of(
            base,
            "electoral_log_0d1b2c3d4e5f40718293a4b5c6d7e8f9"
        ));
        for other in [
            "electoral_log",
            "electoral_log_shared",
            "electoral_logdev90505c8a23a94cdfa",
            "electoral_log_0d1b2c3d4e5f40718293a4b5c6d7e8f",
            "electoral_log_0D1B2C3D4E5F40718293A4B5C6D7E8F9",
            "other_0d1b2c3d4e5f40718293a4b5c6d7e8f9",
        ] {
            assert!(!is_event_database_of(base, other), "{other}");
        }
    }

    #[test]
    fn identifiers_are_quoted() {
        assert_eq!(quote_identifier("a_b"), "\"a_b\"");
        assert_eq!(quote_identifier("a\"b"), "\"a\"\"b\"");
    }
}
