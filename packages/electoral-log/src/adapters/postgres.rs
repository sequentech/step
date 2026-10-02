// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::{
    domain::*,
    ports::ElectoralLogStore,
    proofs::{leaf_hash, Journal, RecordProof},
};
use anyhow::{ensure, Context, Result};
use async_trait::async_trait;
use deadpool_postgres::{Manager, Pool};
use openssl::ssl::{SslConnector, SslMethod, SslVerifyMode};
use postgres_openssl::MakeTlsConnector;
use std::{env, time::Duration};
use tokio_postgres::{types::ToSql, Config, Row};

const COLUMNS: &str = "id, created, sender_pk, statement_timestamp, statement_kind, message, version, user_id, username, election_id, area_id, ballot_id";

#[derive(Clone)]
pub struct PostgresStore {
    pool: Pool,
}

impl PostgresStore {
    pub fn new(config: Config) -> Result<Self> {
        let tls = MakeTlsConnector::new(SslConnector::builder(SslMethod::tls())?.build());
        Self::with_tls(config, tls)
    }

    fn with_tls(config: Config, tls: MakeTlsConnector) -> Result<Self> {
        let pool = Pool::builder(Manager::new(config, tls))
            .max_size(8)
            .build()?;
        Ok(Self { pool })
    }

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
        config.dbname(&required("ELECTORAL_LOG_PG_DATABASE")?);
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
        Self::with_tls(config, MakeTlsConnector::new(tls.build()))
    }

    pub fn journal(&self) -> Journal {
        Journal::new(self.pool.clone())
    }

    pub async fn record_proof(
        &self,
        journal: &Journal,
        board: &str,
        id: i64,
    ) -> Result<Option<RecordProof>> {
        let row = self.pool.get().await?.query_opt(
            &format!("SELECT {COLUMNS}, delivery_id FROM electoral_log_messages WHERE board_name=$1 AND id=$2"),
            &[&board, &id]).await?.context("Electoral-log record does not exist")?;
        let delivery_id = row.try_get("delivery_id")?;
        let entry = LogEntry {
            delivery_id,
            message: decode(row)?,
        };
        let Some(inclusion) = journal.inclusion(board, id).await? else {
            return Ok(None);
        };
        Ok(Some(RecordProof { entry, inclusion }))
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

fn decode(row: Row) -> Result<ElectoralLogMessage> {
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

fn predicate(board: &str, query: &LogQuery) -> (String, Parameters) {
    let mut params = Parameters::default();
    let scope = params.push(board.to_owned());
    let mut clauses = vec![format!("board_name = {scope}")];
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
        tx.execute(
            "INSERT INTO trellis_logs (name) VALUES ($1) ON CONFLICT DO NOTHING",
            &[&board],
        )
        .await?;
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
        ensure!(
            tx.query_opt(
                "SELECT board_name FROM electoral_log_boards WHERE board_name = $1 FOR UPDATE",
                &[&board]
            )
            .await?
            .is_some(),
            "Electoral-log board does not exist"
        );
        let insert = tx.prepare("INSERT INTO electoral_log_messages (board_name, delivery_id, created, sender_pk, statement_timestamp, statement_kind, message, version, user_id, username, election_id, area_id, ballot_id) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13) ON CONFLICT (board_name, delivery_id) DO NOTHING RETURNING id").await?;
        for entry in entries {
            let mut entry = entry?;
            ensure!(
                !entry.delivery_id.is_empty(),
                "Electoral-log delivery ID must not be empty"
            );
            let m = &entry.message;
            let inserted = tx
                .query_opt(
                    &insert,
                    &[
                        &board,
                        &entry.delivery_id,
                        &m.created,
                        &m.sender_pk,
                        &m.statement_timestamp,
                        &m.statement_kind,
                        &m.message,
                        &m.version,
                        &m.user_id,
                        &m.username,
                        &m.election_id,
                        &m.area_id,
                        &m.ballot_id,
                    ],
                )
                .await?;
            if let Some(row) = inserted {
                entry.message.id = row.try_get(0)?;
                Journal::append(&tx, board, entry.message.id, &leaf_hash(board, &entry)?).await?;
            }
        }
        tx.commit().await?;
        Ok(())
    }

    async fn query(&self, board: &str, query: &LogQuery) -> Result<Vec<ElectoralLogMessage>> {
        query.validate()?;
        let (where_clause, mut params) = predicate(board, query);
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

    async fn count(&self, board: &str, query: &LogQuery) -> Result<i64> {
        let (where_clause, params) = predicate(board, query);
        let sql = format!("SELECT COUNT(*) FROM electoral_log_messages WHERE {where_clause}");
        Ok(self
            .pool
            .get()
            .await?
            .query_one(&sql, &params.refs())
            .await?
            .try_get(0)?)
    }
}
