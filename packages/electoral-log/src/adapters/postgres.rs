// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::{
    domain::*,
    ports::ElectoralLogStore,
    proofs::{leaf_hash, Checkpoint, Journal, JournalError, LogIdentity, RecordProof, Uuid},
};
use anyhow::{ensure, Context, Result};
use async_trait::async_trait;
use deadpool_postgres::{Manager, Object, Pool};
use openssl::ssl::{SslConnector, SslMethod, SslVerifyMode};
use postgres_openssl::MakeTlsConnector;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    env,
    time::{Duration, Instant},
};
use tokio_postgres::{types::ToSql, Config, Row};
use trellis::journal::{audit_tree, create_log, TreeAudit, INSERT_CHUNK};

/// Prefix of the advisory lock key that serializes audits of a board.
pub const AUDIT_LOCK_NAMESPACE: &str = "electoral-log-audit:";
/// Longest wait for another audit of the same board to finish.
const AUDIT_LOCK_WAIT: Duration = Duration::from_secs(10 * 60);
/// Pause between attempts to start an audit while another one runs.
const AUDIT_LOCK_RETRY: Duration = Duration::from_secs(2);

pub(crate) const COLUMNS: &str = "id, created, sender_pk, statement_timestamp, statement_kind, message, version, user_id, username, election_id, area_id, ballot_id";

/// Role that may only read the election events' databases, for the super-admin
/// tenant's console queries.
pub const READER_USER_ENV: &str = "ELECTORAL_LOG_PG_READER_USER";
pub const READER_PASSWORD_ENV: &str = "ELECTORAL_LOG_PG_READER_PASSWORD";
/// Role of the environment's backups, which reads every election event's database.
pub const BACKUP_USER_ENV: &str = "ELECTORAL_LOG_PG_BACKUP_USER";
/// Connections of a store's pool.
const POOL_SIZE: usize = 8;

/// Connection settings of the electoral-log database.
#[derive(Clone)]
pub struct PostgresConnection {
    config: Config,
    tls: MakeTlsConnector,
    database: String,
}

impl PostgresConnection {
    /// Read the `ELECTORAL_LOG_PG_*` variables.
    pub fn from_env() -> Result<Self> {
        let required = |name| env::var(name).with_context(|| format!("{name} must be set"));
        let mut config = Config::new();
        config.host(&required("ELECTORAL_LOG_PG_HOST")?);
        config.port(
            required("ELECTORAL_LOG_PG_PORT")?
                .parse()
                .context("Invalid electoral-log PostgreSQL port")?,
        );
        config.user(&required("ELECTORAL_LOG_PG_USER")?);
        config.password(required("ELECTORAL_LOG_PG_PASSWORD")?);
        let database = required("ELECTORAL_LOG_PG_DATABASE")?;
        config.dbname(&database);
        let mode = env::var("ELECTORAL_LOG_PG_SSLMODE").unwrap_or_else(|_| "require".into());
        let mut tls = SslConnector::builder(SslMethod::tls())?;
        match mode.as_str() {
            "disable" => {
                config.ssl_mode(tokio_postgres::config::SslMode::Disable);
            }
            "require" => {
                config.ssl_mode(tokio_postgres::config::SslMode::Require);
                tls.set_verify(SslVerifyMode::NONE);
            }
            "verify-full" => {
                config.ssl_mode(tokio_postgres::config::SslMode::Require);
                if let Some(ca) = env::var("ELECTORAL_LOG_PG_SSLROOTCERT")
                    .ok()
                    .filter(|s| !s.is_empty())
                {
                    tls.set_ca_file(ca)
                        .context("Unable to load electoral-log PostgreSQL CA")?;
                }
            }
            _ => anyhow::bail!("ELECTORAL_LOG_PG_SSLMODE must be disable, require or verify-full"),
        }
        config.connect_timeout(Duration::from_secs(10));
        Ok(Self {
            config,
            tls: MakeTlsConnector::new(tls.build()),
            database,
        })
    }

    /// The database of `ELECTORAL_LOG_PG_DATABASE`.
    pub fn database(&self) -> &str {
        &self.database
    }

    /// A store of the database, with its own connection pool.
    pub fn store(&self) -> Result<PostgresStore> {
        self.store_of(&self.database, POOL_SIZE)
    }

    /// A store of a database of the server, with at most `pool_size` connections, as
    /// load tests use.
    pub fn store_of(&self, database: &str, pool_size: usize) -> Result<PostgresStore> {
        let mut config = self.config.clone();
        config.dbname(database);
        PostgresStore::with_tls(config, self.tls.clone(), pool_size)
    }

    /// The role of `ELECTORAL_LOG_PG_READER_USER`, if set.
    pub fn reader() -> Option<String> {
        Self::role(READER_USER_ENV)
    }

    /// The roles that read every election event's database: those of
    /// `ELECTORAL_LOG_PG_READER_USER` and `ELECTORAL_LOG_PG_BACKUP_USER`, if set.
    pub fn read_roles() -> Vec<String> {
        [READER_USER_ENV, BACKUP_USER_ENV]
            .into_iter()
            .filter_map(Self::role)
            .collect()
    }

    fn role(variable: &str) -> Option<String> {
        env::var(variable)
            .ok()
            .map(|role| role.trim().to_string())
            .filter(|role| !role.is_empty())
    }

    /// A connection of its own to the database as the reader role, which needs
    /// `ELECTORAL_LOG_PG_READER_USER` and `ELECTORAL_LOG_PG_READER_PASSWORD`.
    pub async fn reader_client(&self) -> Result<tokio_postgres::Client> {
        self.reader_client_of(&self.database).await
    }

    /// A connection of its own to a database of the server as the reader role.
    pub async fn reader_client_of(&self, database: &str) -> Result<tokio_postgres::Client> {
        let reader = Self::reader().with_context(|| format!("{READER_USER_ENV} is not set"))?;
        let password = env::var(READER_PASSWORD_ENV)
            .with_context(|| format!("{READER_PASSWORD_ENV} must be set"))?;
        let mut config = self.config.clone();
        config.dbname(database).user(&reader).password(password);
        self.connect(config, database).await
    }

    /// A connection of its own to a database of the server as the application role,
    /// closed when the client is dropped.
    pub async fn client_of(&self, database: &str) -> Result<tokio_postgres::Client> {
        let mut config = self.config.clone();
        config.dbname(database);
        self.connect(config, database).await
    }

    async fn connect(&self, config: Config, database: &str) -> Result<tokio_postgres::Client> {
        let (client, connection) = config
            .connect(self.tls.clone())
            .await
            .with_context(|| format!("Error connecting to {database}"))?;
        tokio::spawn(async move {
            if let Err(error) = connection.await {
                tracing::error!("Electoral-log connection failed: {error}");
            }
        });
        Ok(client)
    }
}

#[derive(Clone)]
pub struct PostgresStore {
    pool: Pool,
}

impl PostgresStore {
    pub fn new(config: Config) -> Result<Self> {
        let tls = MakeTlsConnector::new(SslConnector::builder(SslMethod::tls())?.build());
        Self::with_tls(config, tls, POOL_SIZE)
    }

    fn with_tls(config: Config, tls: MakeTlsConnector, pool_size: usize) -> Result<Self> {
        let pool = Pool::builder(Manager::new(config, tls))
            .max_size(pool_size)
            .build()?;
        Ok(Self { pool })
    }

    /// The store of the database of `ELECTORAL_LOG_PG_DATABASE`.
    pub fn from_env() -> Result<Self> {
        PostgresConnection::from_env()?.store()
    }

    /// A pooled connection, for queries outside the store's API.
    pub async fn client(&self) -> Result<Object> {
        Ok(self.pool.get().await?)
    }

    /// Close the pool: its idle connections now, the others when returned.
    pub fn close(&self) {
        self.pool.close();
    }

    /// Close the pool's connections that were not used for `idle`.
    pub fn close_idle(&self, idle: Duration) {
        self.pool.retain(|_, metrics| metrics.last_used() < idle);
    }

    pub fn journal(&self) -> Journal {
        Journal::new(self.pool.clone())
    }

    /// Load a stored record with Trellis evidence of its membership in its log, which
    /// is `board` or, in an election event's database, any log of the database when
    /// `board` is `None`.
    ///
    /// With a trusted checkpoint, the evidence verifies against it: the proof is at the
    /// trusted checkpoint when that already contains the record, and otherwise at the
    /// current checkpoint together with a proof that it extends the trusted one.
    pub async fn record_proof(
        &self,
        journal: &Journal,
        scope: LogScope<'_>,
        id: i64,
        trusted: Option<&Checkpoint>,
    ) -> Result<RecordProof, JournalError> {
        let (condition, board) = match scope {
            LogScope::Board(board) => ("board_name = $2", Some(board)),
            LogScope::Database => ("$2::text IS NULL", None),
        };
        let row = self
            .pool
            .get()
            .await?
            .query_opt(
                &format!(
                    "SELECT {COLUMNS}, delivery_id, board_name FROM electoral_log_messages \
                     WHERE id = $1 AND {condition}"
                ),
                &[&id, &board],
            )
            .await?
            .ok_or_else(|| JournalError::NotFound(format!("Electoral-log record {id}")))?;
        let delivery_id = row.try_get("delivery_id")?;
        let board: String = row.try_get("board_name")?;
        let entry = LogEntry {
            delivery_id,
            message: decode(row)?,
        };
        let evidence =
            journal
                .evidence(&board, id, trusted)
                .await
                .map_err(|error| match error {
                    // The record exists, so a missing leaf is an integrity fault, not a bad request.
                    JournalError::NotFound(_) => {
                        JournalError::Corrupt(format!("electoral-log record {id} has no leaf"))
                    }
                    error => error,
                })?;
        Ok(RecordProof::new(entry, evidence))
    }

    /// Compare a board's stored records with its Trellis journal and stored tree, and
    /// check each given (for example, published) checkpoint against the roots
    /// recomputed from the leaves.
    ///
    /// Reads one consistent snapshot, recomputes every record commitment and every
    /// stored subtree, so its cost grows with the board. Run it as a background task or
    /// from the CLI, not per request. Audits of a board run one at a time: a second one
    /// waits, without holding a connection or a snapshot, until the first finishes.
    pub async fn audit(&self, board: &str, published: &[Checkpoint]) -> Result<AuditReport> {
        let deadline = Instant::now() + AUDIT_LOCK_WAIT;
        loop {
            let mut client = self.pool.get().await?;
            if let Some(report) = audit_snapshot(&mut client, board, published).await? {
                return Ok(report);
            }
            drop(client);
            ensure!(
                Instant::now() < deadline,
                "Another audit of this board is still running"
            );
            tokio::time::sleep(AUDIT_LOCK_RETRY).await;
        }
    }

    /// Run with the database owner during provisioning, not on each request.
    pub async fn initialize(&self) -> Result<()> {
        self.pool
            .get()
            .await?
            .batch_execute(include_str!("../../schema.sql"))
            .await?;
        Ok(())
    }
}

/// Audit a board in one snapshot taken once its audit lock is held, or return `None`
/// without reading it when another audit holds the lock.
async fn audit_snapshot(
    client: &mut Object,
    board: &str,
    published: &[Checkpoint],
) -> Result<Option<AuditReport>> {
    let tx = client
        .build_transaction()
        .isolation_level(tokio_postgres::IsolationLevel::RepeatableRead)
        .read_only(true)
        .start()
        .await?;
    let free: bool = tx
        .query_one(
            "SELECT pg_try_advisory_xact_lock(hashtextextended($1, 0))",
            &[&format!("{AUDIT_LOCK_NAMESPACE}{board}")],
        )
        .await?
        .try_get(0)?;
    if !free {
        tx.rollback().await?;
        return Ok(None);
    }
    let row = tx
        .query_opt(
            "SELECT l.id, l.uid, l.size, l.root, \
             (SELECT count(*) FROM trellis_leaves e WHERE e.log_id = l.id), \
             (SELECT coalesce(max(e.leaf_index) + 1, 0) FROM trellis_leaves e WHERE e.log_id = l.id), \
             (SELECT count(*) FROM electoral_log_messages m WHERE m.board_name = l.name) \
             FROM trellis_logs l WHERE l.name = $1",
            &[&board],
        )
        .await?
        .context("Electoral-log board does not exist")?;
    let log_id: i64 = row.try_get(0)?;
    let log = LogIdentity {
        name: board.to_owned(),
        uid: row.try_get(1)?,
    };
    let mut report = AuditReport {
        board: board.to_owned(),
        log_uid: log.uid,
        committed_size: row.try_get(2)?,
        root: hex::encode(row.try_get::<_, Vec<u8>>(3)?),
        leaves: row.try_get(4)?,
        leaf_index_end: row.try_get(5)?,
        messages: row.try_get(6)?,
        ..AuditReport::default()
    };
    report.leaves_out_of_order = tx
        .query_one(
            "SELECT count(*) FROM (SELECT source_id < lag(source_id) OVER (ORDER BY leaf_index) AS reordered \
             FROM trellis_leaves WHERE log_id = $1) leaves WHERE reordered",
            &[&log_id],
        )
        .await?
        .try_get(0)?;
    report.messages_without_leaf = tx
        .query(
            "SELECT m.id FROM electoral_log_messages m WHERE m.board_name = $1 \
             AND NOT EXISTS (SELECT 1 FROM trellis_leaves e WHERE e.log_id = $2 AND e.source_id = m.id) \
             ORDER BY m.id LIMIT $3",
            &[&board, &log_id, &AUDIT_SAMPLE],
        )
        .await?
        .iter()
        .map(|row| row.try_get(0))
        .collect::<Result<_, _>>()?;
    report.leaves_without_message = tx
        .query(
            "SELECT e.source_id FROM trellis_leaves e WHERE e.log_id = $2 \
             AND NOT EXISTS (SELECT 1 FROM electoral_log_messages m WHERE m.board_name = $1 AND m.id = e.source_id) \
             ORDER BY e.leaf_index LIMIT $3",
            &[&board, &log_id, &AUDIT_SAMPLE],
        )
        .await?
        .iter()
        .map(|row| row.try_get(0))
        .collect::<Result<_, _>>()?;
    let page = tx.prepare(&audit_page_sql()).await?;
    let mut cursor = i64::MIN;
    loop {
        let rows = tx.query(&page, &[&board, &log_id, &cursor]).await?;
        let Some(last) = rows.last() else { break };
        cursor = last.try_get("id")?;
        for row in rows {
            let stored: Vec<u8> = row.try_get("hash")?;
            let entry = LogEntry {
                delivery_id: row.try_get("delivery_id")?,
                message: decode(row)?,
            };
            report.checked_hashes += 1;
            if leaf_hash(&log, &entry)? != stored {
                report.hash_mismatch_count += 1;
                if report.hash_mismatches.len() < AUDIT_SAMPLE as usize {
                    report.hash_mismatches.push(entry.message.id);
                }
            }
        }
    }
    let snapshot: &tokio_postgres::Transaction<'_> = &tx;
    let sizes: Vec<u64> = published.iter().map(|c| c.tree_size).collect();
    report.tree = audit_tree(snapshot, board, &sizes).await?;
    for checkpoint in published {
        report.published_checked += 1;
        let reason = if checkpoint.log_name != board {
            Some("it names another board".to_string())
        } else if checkpoint.log_uid != log.uid {
            Some("it belongs to another log".to_string())
        } else {
            match report.tree.roots.get(&checkpoint.tree_size) {
                None => Some(format!(
                    "the log's leaves end at position {}",
                    report.tree.leaves
                )),
                Some(root) if *root != checkpoint.root => {
                    Some("the log's leaves have a different root at that size".to_string())
                }
                Some(_) => None,
            }
        };
        if let Some(reason) = reason {
            report.published_mismatches.push(PublishedMismatch {
                tree_size: checkpoint.tree_size,
                log_uid: checkpoint.log_uid,
                reason,
            });
        }
    }
    tx.commit().await?;
    Ok(Some(report))
}

/// Store records in one statement, in order, and append the new ones to the journal,
/// then empty `pending`. A delivery that is already stored, or that appears earlier in
/// `pending`, is not stored again.
pub(crate) async fn insert_records(
    tx: &deadpool_postgres::Transaction<'_>,
    log: &LogIdentity,
    pending: &mut Vec<LogEntry>,
) -> Result<()> {
    let board = log.name.as_str();
    let mut seen = HashSet::with_capacity(pending.len());
    pending.retain(|entry| seen.insert(entry.delivery_id.clone()));
    if pending.is_empty() {
        return Ok(());
    }
    let text = |field: fn(&ElectoralLogMessage) -> &str| -> Vec<&str> {
        pending.iter().map(|entry| field(&entry.message)).collect()
    };
    let optional = |field: fn(&ElectoralLogMessage) -> &Option<String>| -> Vec<Option<&str>> {
        pending
            .iter()
            .map(|entry| field(&entry.message).as_deref())
            .collect()
    };
    let number = |field: fn(&ElectoralLogMessage) -> i64| -> Vec<i64> {
        pending.iter().map(|entry| field(&entry.message)).collect()
    };
    let deliveries: Vec<&str> = pending.iter().map(|e| e.delivery_id.as_str()).collect();
    let messages: Vec<&[u8]> = pending
        .iter()
        .map(|e| e.message.message.as_slice())
        .collect();
    let rows = tx
        .query(
            "INSERT INTO electoral_log_messages (board_name, delivery_id, created, sender_pk, \
             statement_timestamp, statement_kind, message, version, user_id, username, \
             election_id, area_id, ballot_id) \
             SELECT $1, delivery_id, created, sender_pk, statement_timestamp, statement_kind, \
             message, version, user_id, username, election_id, area_id, ballot_id \
             FROM UNNEST($2::TEXT[], $3::BIGINT[], $4::TEXT[], $5::BIGINT[], $6::TEXT[], \
             $7::BYTEA[], $8::TEXT[], $9::TEXT[], $10::TEXT[], $11::TEXT[], $12::TEXT[], \
             $13::TEXT[]) WITH ORDINALITY AS r(delivery_id, created, sender_pk, \
             statement_timestamp, statement_kind, message, version, user_id, username, \
             election_id, area_id, ballot_id, position) \
             ORDER BY position \
             ON CONFLICT (board_name, delivery_id) DO NOTHING RETURNING id, delivery_id",
            &[
                &board,
                &deliveries,
                &number(|m| m.created),
                &text(|m| &m.sender_pk),
                &number(|m| m.statement_timestamp),
                &text(|m| &m.statement_kind),
                &messages,
                &text(|m| &m.version),
                &optional(|m| &m.user_id),
                &optional(|m| &m.username),
                &optional(|m| &m.election_id),
                &optional(|m| &m.area_id),
                &optional(|m| &m.ballot_id),
            ],
        )
        .await?;
    let positions: HashMap<&str, usize> = deliveries
        .iter()
        .enumerate()
        .map(|(index, delivery)| (*delivery, index))
        .collect();
    let mut stored = Vec::with_capacity(rows.len());
    for row in rows {
        let delivery: &str = row.try_get(1)?;
        let index = *positions
            .get(delivery)
            .context("Stored an electoral-log delivery that was not appended")?;
        stored.push((row.try_get::<_, i64>(0)?, index));
    }
    // The journal lists records in ID order.
    stored.sort_unstable();
    let mut leaves = Vec::with_capacity(stored.len());
    for (id, index) in stored {
        leaves.push((id, leaf_hash(log, &pending[index])?));
    }
    Journal::append_batch(tx, board, &leaves).await?;
    pending.clear();
    Ok(())
}

/// Records of a board after the cursor `$3`, with their leaves, in ID order.
///
/// The planner does not carry `m.id > $3` over to the leaves, so the page bounds both
/// sides; otherwise every page rescans the leaves from the start of the log.
fn audit_page_sql() -> String {
    format!(
        "SELECT {COLUMNS}, delivery_id, e.hash FROM electoral_log_messages m \
         JOIN trellis_leaves e ON e.log_id = $2 AND e.source_id = m.id \
         WHERE m.board_name = $1 AND m.id > $3 AND e.source_id > $3 \
         ORDER BY m.id LIMIT {AUDIT_PAGE}"
    )
}

pub(crate) fn decode(row: Row) -> Result<ElectoralLogMessage> {
    Ok(ElectoralLogMessage {
        id: row.try_get("id")?,
        created: row.try_get("created")?,
        sender_pk: row.try_get("sender_pk")?,
        statement_timestamp: row.try_get("statement_timestamp")?,
        statement_kind: row.try_get("statement_kind")?,
        message: row.try_get("message")?,
        version: row.try_get("version")?,
        user_id: row.try_get("user_id")?,
        username: row.try_get("username")?,
        election_id: row.try_get("election_id")?,
        area_id: row.try_get("area_id")?,
        ballot_id: row.try_get("ballot_id")?,
    })
}

/// Maximum IDs listed per audit finding.
const AUDIT_SAMPLE: i64 = 100;
/// Records compared per audit query.
const AUDIT_PAGE: i64 = 1000;

/// Result of an audit run, as recorded for operators.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    strum_macros::Display,
    strum_macros::EnumString,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum AuditOutcome {
    /// The board is consistent.
    Clean,
    /// The audit ran and found integrity problems.
    Findings,
    /// The audit could not run to completion.
    Error,
}

/// An audit result as recorded in task annotations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditAnnotations {
    pub outcome: AuditOutcome,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub board: Option<String>,
    /// Size of the log in the audited snapshot.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tree_size: Option<i64>,
    /// Hex-encoded stored root in the audited snapshot.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root: Option<String>,
    /// Number of findings.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub findings: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub published_checkpoints: Option<usize>,
}

/// Comparison of a board's stored records with its Trellis journal.
#[derive(Debug, Default, Clone, Serialize)]
pub struct AuditReport {
    pub board: String,
    /// Identity of the Trellis log audited.
    pub log_uid: Uuid,
    /// `trellis_logs.size`: leaves allocated by committed appends.
    pub committed_size: i64,
    /// Hex-encoded stored root at the audited size.
    pub root: String,
    /// Persisted leaves.
    pub leaves: i64,
    /// One past the highest persisted leaf index.
    pub leaf_index_end: i64,
    /// Stored records on the board.
    pub messages: i64,
    /// Records with no leaf (first IDs only).
    pub messages_without_leaf: Vec<i64>,
    /// Leaves whose record is missing (first source IDs only).
    pub leaves_without_message: Vec<i64>,
    /// Leaves whose record ID is lower than the previous leaf's. Appends add records
    /// in ID order, so any such leaf was moved.
    pub leaves_out_of_order: i64,
    /// Records whose recomputed commitment was compared with their leaf.
    pub checked_hashes: u64,
    /// Records whose recomputed commitment differs from their leaf.
    pub hash_mismatch_count: u64,
    /// IDs of mismatched records (first IDs only).
    pub hash_mismatches: Vec<i64>,
    /// Stored tree compared with the tree recomputed from the leaves.
    pub tree: TreeAudit,
    /// Supplied checkpoints that were checked against the history.
    pub published_checked: u64,
    /// Supplied checkpoints whose root differs from the leaves' root at their size.
    pub published_mismatches: Vec<PublishedMismatch>,
}

/// A supplied checkpoint that is not part of the board's history.
#[derive(Debug, Clone, Serialize)]
pub struct PublishedMismatch {
    pub log_uid: Uuid,
    pub tree_size: u64,
    pub reason: String,
}

impl AuditReport {
    /// True when every record has exactly one matching leaf, the journal is dense, the
    /// stored tree matches its leaves and every supplied checkpoint is in the history.
    pub fn is_clean(&self) -> bool {
        self.committed_size == self.leaves
            && self.leaf_index_end == self.leaves
            && self.messages == self.leaves
            && self.messages_without_leaf.is_empty()
            && self.leaves_without_message.is_empty()
            && self.leaves_out_of_order == 0
            && self.hash_mismatch_count == 0
            && i64::try_from(self.checked_hashes).ok() == Some(self.messages)
            && self.tree.is_clean()
            && self.published_mismatches.is_empty()
    }

    /// One line per finding, for task logs and operators. Empty when clean.
    pub fn findings(&self) -> Vec<String> {
        let mut findings = Vec::new();
        if self.committed_size != self.leaves || self.leaf_index_end != self.leaves {
            findings.push(format!(
                "Journal records {} committed leaves but stores {} leaves ending at position {}",
                self.committed_size, self.leaves, self.leaf_index_end
            ));
        }
        if self.messages != self.leaves {
            findings.push(format!(
                "Board stores {} records but {} leaves",
                self.messages, self.leaves
            ));
        }
        if !self.messages_without_leaf.is_empty() {
            findings.push(format!(
                "Records without a leaf: {:?}",
                self.messages_without_leaf
            ));
        }
        if !self.leaves_without_message.is_empty() {
            findings.push(format!(
                "Leaves without a record: {:?}",
                self.leaves_without_message
            ));
        }
        if self.leaves_out_of_order > 0 {
            findings.push(format!(
                "{} leaves are not in record order",
                self.leaves_out_of_order
            ));
        }
        if self.hash_mismatch_count > 0 {
            findings.push(format!(
                "{} records do not match their leaf; first: {:?}",
                self.hash_mismatch_count, self.hash_mismatches
            ));
        }
        let tree = &self.tree;
        if tree.unbuilt {
            findings.push("The log has no stored tree and must be rebuilt".into());
        }
        if !tree.dense {
            findings.push(format!("No leaf at position {}", tree.leaves));
        }
        if tree.node_mismatches > 0 {
            findings.push(format!(
                "{} stored subtrees differ from their leaves; first (level, index): {:?}",
                tree.node_mismatches, tree.first_node_mismatches
            ));
        }
        if tree.dense && tree.stored_nodes != tree.expected_nodes() {
            findings.push(format!(
                "{} stored subtrees, expected {}",
                tree.stored_nodes,
                tree.expected_nodes()
            ));
        }
        if tree.dense && !tree.root_matches {
            findings.push("Stored root differs from the leaves".into());
        }
        for mismatch in &self.published_mismatches {
            findings.push(format!(
                "Published checkpoint at size {} (log {}) does not match the log: {}",
                mismatch.tree_size, mismatch.log_uid, mismatch.reason
            ));
        }
        findings
    }
}

#[derive(Default)]
struct Parameters(Vec<Box<dyn ToSql + Sync + Send>>);
impl Parameters {
    fn push(&mut self, value: impl ToSql + Sync + Send + 'static) -> String {
        self.0.push(Box::new(value));
        format!("${}", self.0.len())
    }
    fn refs(&self) -> Vec<&(dyn ToSql + Sync)> {
        self.0
            .iter()
            .map(|v| v.as_ref() as &(dyn ToSql + Sync))
            .collect()
    }
}

/// Which logs of a database a read covers.
#[derive(Debug, Clone, Copy)]
pub enum LogScope<'a> {
    /// The records of one board.
    Board(&'a str),
    /// Every record of the database. An election event's database holds its board and
    /// the sealed logs that board continues, so these are the event's records.
    Database,
}

fn predicate(scope: LogScope<'_>, query: &LogQuery) -> (String, Parameters) {
    let mut params = Parameters::default();
    let mut clauses = vec![match scope {
        LogScope::Board(board) => format!("board_name = {}", params.push(board.to_owned())),
        LogScope::Database => "TRUE".to_string(),
    }];
    for filter in &query.filters {
        clauses.push(match filter {
            Filter::Text(column, op, value) => {
                format!("{column} {op} {}", params.push(value.clone()))
            }
            Filter::Number(column, op, value) => format!("{column} {op} {}", params.push(*value)),
        });
    }
    if let Some(visibility) = &query.visibility {
        let mut allowed = vec![
            "((election_id IS NULL OR election_id = '') AND (area_id IS NULL OR area_id = ''))"
                .to_string(),
        ];
        if let Some(election) = visibility.election_id.as_ref().filter(|id| !id.is_empty()) {
            allowed.push(format!("election_id = {}", params.push(election.clone())));
        }
        let areas: Vec<_> = visibility
            .area_ids
            .iter()
            .filter(|id| !id.is_empty())
            .cloned()
            .collect();
        if !areas.is_empty() {
            allowed.push(format!("area_id = ANY({})", params.push(areas)));
        }
        clauses.push(format!("({})", allowed.join(" OR ")));
    }
    if query.only_with_user {
        clauses.push("user_id IS NOT NULL AND user_id <> ''".into());
    }
    (clauses.join(" AND "), params)
}

#[async_trait]
impl ElectoralLogStore for PostgresStore {
    async fn create_board(&self, board: &str) -> Result<()> {
        ensure!(!board.is_empty(), "Electoral-log board must not be empty");
        let mut conn = self.pool.get().await?;
        let tx = conn.transaction().await?;
        create_log(&*tx, board, None).await?;
        tx.execute(
            "INSERT INTO electoral_log_boards (board_name) VALUES ($1) ON CONFLICT DO NOTHING",
            &[&board],
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn delete_board(&self, board: &str) -> Result<()> {
        self.pool
            .get()
            .await?
            .execute("DELETE FROM trellis_logs WHERE name = $1", &[&board])
            .await?;
        Ok(())
    }

    async fn has_board(&self, board: &str) -> Result<bool> {
        Ok(self
            .pool
            .get()
            .await?
            .query_opt(
                "SELECT 1 FROM electoral_log_boards WHERE board_name = $1",
                &[&board],
            )
            .await?
            .is_some())
    }

    async fn append(
        &self,
        board: &str,
        entries: &mut (dyn Iterator<Item = Result<LogEntry>> + Send),
    ) -> Result<()> {
        let mut client = self.pool.get().await?;
        let tx = client.transaction().await?;
        // Serialize appends within a board before allocating IDs. Otherwise a
        // later transaction can commit first and make a cursor skip earlier IDs.
        let log = tx
            .query_opt(
                "SELECT l.uid, l.sealed_at IS NULL FROM electoral_log_boards b \
                 JOIN trellis_logs l ON l.name = b.board_name \
                 WHERE b.board_name = $1 FOR UPDATE OF b",
                &[&board],
            )
            .await?
            .context("Electoral-log board does not exist")?;
        ensure!(
            log.try_get::<_, bool>(1)?,
            "Electoral-log board {board} is sealed"
        );
        let log = LogIdentity {
            name: board.to_owned(),
            uid: log.try_get(0)?,
        };
        let mut pending = Vec::with_capacity(entries.size_hint().0.min(INSERT_CHUNK));
        for entry in entries {
            let entry = entry?;
            ensure!(
                !entry.delivery_id.is_empty(),
                "Electoral-log delivery ID must not be empty"
            );
            pending.push(entry);
            if pending.len() >= INSERT_CHUNK {
                insert_records(&tx, &log, &mut pending).await?;
            }
        }
        insert_records(&tx, &log, &mut pending).await?;
        tx.commit().await?;
        Ok(())
    }

    async fn query(&self, board: &str, query: &LogQuery) -> Result<Vec<ElectoralLogMessage>> {
        self.query_scope(LogScope::Board(board), query).await
    }

    async fn count(&self, board: &str, query: &LogQuery) -> Result<i64> {
        self.count_scope(LogScope::Board(board), query).await
    }
}

impl PostgresStore {
    /// Records of `scope` that match `query`.
    pub async fn query_scope(
        &self,
        scope: LogScope<'_>,
        query: &LogQuery,
    ) -> Result<Vec<ElectoralLogMessage>> {
        query.validate()?;
        let (where_clause, mut params) = predicate(scope, query);
        let mut order: Vec<_> = query
            .order
            .iter()
            .map(|(col, dir)| format!("{col} {dir}"))
            .collect();
        if !query.order.iter().any(|(col, _)| *col == OrderColumn::Id) {
            order.push("id DESC".into());
        }
        let limit = params.push(query.limit);
        let offset = params.push(query.offset);
        let sql = format!("SELECT {COLUMNS} FROM electoral_log_messages WHERE {where_clause} ORDER BY {} LIMIT {limit} OFFSET {offset}", order.join(", "));
        self.pool
            .get()
            .await?
            .query(&sql, &params.refs())
            .await?
            .into_iter()
            .map(decode)
            .collect()
    }

    /// Number of records of `scope` that match `query`.
    pub async fn count_scope(&self, scope: LogScope<'_>, query: &LogQuery) -> Result<i64> {
        let client = self.pool.get().await?;
        if query.filters.is_empty() && query.visibility.is_none() && !query.only_with_user {
            // Each record commits with exactly one leaf, so the journals' sizes count the
            // records without scanning them.
            let board = match scope {
                LogScope::Board(board) => Some(board),
                LogScope::Database => None,
            };
            return Ok(client
                .query_one(
                    "SELECT coalesce(sum(l.size), 0)::bigint FROM trellis_logs l \
                     JOIN electoral_log_boards b ON b.board_name = l.name \
                     WHERE $1::text IS NULL OR l.name = $1",
                    &[&board],
                )
                .await?
                .try_get(0)?);
        }
        let (where_clause, params) = predicate(scope, query);
        let sql = format!("SELECT COUNT(*) FROM electoral_log_messages WHERE {where_clause}");
        Ok(client.query_one(&sql, &params.refs()).await?.try_get(0)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_outcomes_use_the_same_names_in_json_and_text() {
        for (outcome, name) in [
            (AuditOutcome::Clean, "clean"),
            (AuditOutcome::Findings, "findings"),
            (AuditOutcome::Error, "error"),
        ] {
            assert_eq!(outcome.to_string(), name);
            assert_eq!(serde_json::to_value(outcome).unwrap(), name);
            assert_eq!(name.parse::<AuditOutcome>().unwrap(), outcome);
        }
    }

    #[test]
    fn audit_annotations_round_trip_and_omit_absent_fields() {
        let error = AuditAnnotations {
            outcome: AuditOutcome::Error,
            board: None,
            tree_size: None,
            root: None,
            findings: None,
            published_checkpoints: None,
        };
        assert_eq!(
            serde_json::to_value(&error).unwrap(),
            serde_json::json!({"outcome": "error"})
        );
        let findings = AuditAnnotations {
            outcome: AuditOutcome::Findings,
            board: Some("board".into()),
            tree_size: Some(9),
            root: Some("00".repeat(32)),
            findings: Some(3),
            published_checkpoints: Some(2),
        };
        let value = serde_json::to_value(&findings).unwrap();
        assert_eq!(value["outcome"], "findings");
        assert_eq!(value["findings"], 3);
        assert_eq!(
            serde_json::from_value::<AuditAnnotations>(value).unwrap(),
            findings
        );
    }

    #[test]
    fn every_inconsistency_is_reported() {
        let clean = AuditReport {
            board: "board".into(),
            committed_size: 3,
            leaves: 3,
            leaf_index_end: 3,
            messages: 3,
            checked_hashes: 3,
            tree: TreeAudit {
                committed_size: 3,
                leaves: 3,
                dense: true,
                stored_nodes: 1,
                root_matches: true,
                ..TreeAudit::default()
            },
            ..AuditReport::default()
        };
        assert!(clean.is_clean());
        assert!(clean.findings().is_empty());
        let broken = [
            AuditReport {
                leaves_out_of_order: 1,
                ..clean.clone()
            },
            AuditReport {
                hash_mismatch_count: 1,
                hash_mismatches: vec![2],
                ..clean.clone()
            },
            AuditReport {
                messages_without_leaf: vec![4],
                ..clean.clone()
            },
            AuditReport {
                published_mismatches: vec![PublishedMismatch {
                    log_uid: Uuid::from_u128(1),
                    tree_size: 2,
                    reason: "it names another board".into(),
                }],
                ..clean.clone()
            },
            AuditReport {
                tree: TreeAudit {
                    root_matches: false,
                    ..clean.tree.clone()
                },
                ..clean.clone()
            },
            AuditReport {
                tree: TreeAudit {
                    stored_nodes: 2,
                    ..clean.tree.clone()
                },
                ..clean.clone()
            },
            AuditReport {
                tree: TreeAudit {
                    dense: false,
                    leaves: 1,
                    ..clean.tree.clone()
                },
                ..clean.clone()
            },
        ];
        let unbuilt = AuditReport {
            tree: TreeAudit {
                unbuilt: true,
                ..clean.tree.clone()
            },
            ..clean.clone()
        };
        assert!(unbuilt
            .findings()
            .iter()
            .any(|finding| finding.contains("must be rebuilt")));
        for report in broken.into_iter().chain([unbuilt]) {
            assert!(!report.is_clean(), "{report:?}");
            assert!(!report.findings().is_empty(), "{report:?}");
        }
    }

    /// The JSON that `EXPLAIN (FORMAT JSON)` returns.
    struct Plan(serde_json::Value);

    impl<'a> tokio_postgres::types::FromSql<'a> for Plan {
        fn from_sql(
            _: &tokio_postgres::types::Type,
            raw: &'a [u8],
        ) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
            Ok(Self(serde_json::from_slice(raw)?))
        }

        fn accepts(ty: &tokio_postgres::types::Type) -> bool {
            *ty == tokio_postgres::types::Type::JSON
        }
    }

    /// Rows a plan reads from a table, including those its filters discard, over all
    /// loops.
    fn rows_read(plan: &serde_json::Value, table: &str) -> f64 {
        let own = if plan["Relation Name"] == table {
            let value = |key: &str| plan[key].as_f64().unwrap_or_default();
            (value("Actual Rows")
                + value("Rows Removed by Filter")
                + value("Rows Removed by Index Recheck"))
                * value("Actual Loops")
        } else {
            0.0
        };
        own + plan["Plans"].as_array().map_or(0.0, |children| {
            children.iter().map(|child| rows_read(child, table)).sum()
        })
    }

    #[tokio::test]
    #[ignore = "requires ELECTORAL_LOG_TEST_DATABASE_URL"]
    async fn audit_pages_read_only_the_leaves_after_their_cursor() -> Result<()> {
        let store = PostgresStore::new(std::env::var("ELECTORAL_LOG_TEST_DATABASE_URL")?.parse()?)?;
        store.initialize().await?;
        let board = format!("audit-page-{}", uuid::Uuid::new_v4());
        store.create_board(&board).await?;
        let records = 20 * AUDIT_PAGE;
        let mut entries = (0..records).map(|n| {
            Ok(LogEntry {
                delivery_id: n.to_string(),
                message: ElectoralLogMessage {
                    id: 0,
                    created: n,
                    sender_pk: "sender".into(),
                    statement_timestamp: n,
                    statement_kind: "CastVote".into(),
                    message: vec![1, 2, 3],
                    version: "1".into(),
                    user_id: None,
                    username: None,
                    election_id: None,
                    area_id: None,
                    ballot_id: None,
                },
            })
        });
        store.append(&board, &mut entries).await?;
        let client = store.pool.get().await?;
        client
            .batch_execute("ANALYZE electoral_log_messages; ANALYZE trellis_leaves")
            .await?;
        let log_id: i64 = client
            .query_one("SELECT id FROM trellis_logs WHERE name = $1", &[&board])
            .await?
            .try_get(0)?;
        let cursor: i64 = client
            .query_one(
                "SELECT id FROM electoral_log_messages WHERE board_name = $1 ORDER BY id OFFSET $2 LIMIT 1",
                &[&board, &(records - AUDIT_PAGE)],
            )
            .await?
            .try_get(0)?;
        let Plan(plan) = client
            .query_one(
                &format!("EXPLAIN (ANALYZE, FORMAT JSON) {}", audit_page_sql()),
                &[&board, &log_id, &cursor],
            )
            .await?
            .try_get(0)?;
        store.delete_board(&board).await?;
        let read = rows_read(&plan[0]["Plan"], "trellis_leaves");
        assert!(
            read <= (2 * AUDIT_PAGE) as f64,
            "One audit page read {read} leaves"
        );
        Ok(())
    }
}
