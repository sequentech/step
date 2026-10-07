// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Read-only views of an electoral-log database for administrators: a board's records
//! and an election event's ballot box, page by page in key order, and SQL queries in a
//! read-only transaction.

use super::ballot_box::canonical_uuid;
use super::postgres::PostgresStore;
use crate::messages::message::Message;
use anyhow::{anyhow, ensure, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::{Duration, Instant};
use strand::serialization::StrandDeserialize;
use strum_macros::{Display, EnumString};
use tokio_postgres::types::ToSql;

/// A table the console browses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, EnumString, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum ConsoleTable {
    /// The board's records, `electoral_log_messages`.
    Records,
    /// The event's accepted ballots, without their content.
    Ballots,
    /// The event's voters and how many times they voted.
    Voters,
    /// The event's ballots waiting for the sequencer.
    Queue,
}

/// Filters of a page. Each applies to the tables that have its column, and the
/// others ignore it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsoleFilters {
    /// Records.
    pub statement_kind: Option<String>,
    /// Records, ballots, voters and the queue.
    pub election_id: Option<String>,
    /// Ballots, voters and the queue.
    pub area_id: Option<String>,
    /// The record's user or the ballot's voter.
    pub user_id: Option<String>,
    /// Records and ballots.
    pub ballot_id: Option<String>,
    /// Ballots: `valid`, `pending` or `rejected`.
    pub status: Option<String>,
    /// Seconds since the epoch: a record's creation or a ballot's acceptance.
    pub created_after: Option<i64>,
    pub created_before: Option<i64>,
}

/// Which page to read.
#[derive(Debug, Clone)]
pub struct PageRequest<'a> {
    pub table: ConsoleTable,
    /// The event's board, for records.
    pub board: &'a str,
    pub election_event_id: &'a str,
    pub filters: &'a ConsoleFilters,
    /// Newest first, or oldest first.
    pub order: PageOrder,
    /// Key of the last row of the previous page.
    pub after: Option<&'a str>,
    pub limit: i64,
}

/// The order of a page's rows, by the table's key.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, Display, EnumString, Serialize, Deserialize,
)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum PageOrder {
    #[default]
    NewestFirst,
    OldestFirst,
}

/// Rows of a table, as arrays in the order of `columns`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Rows {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Value>>,
}

/// A page of a table.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Page {
    #[serde(flatten)]
    pub rows: Rows,
    /// Key of the page's last row, to read the next page; none at the end.
    pub next: Option<String>,
    /// Rows of the table for the board or event, ignoring the filters: exact for
    /// records, an estimate from the planner's statistics otherwise.
    pub estimated_rows: i64,
    /// Columns that hold personal data.
    pub personal_columns: Vec<String>,
}

/// The most rows a page has.
pub const MAX_PAGE_ROWS: i64 = 200;

/// Columns of a record, of a ballot or of a voter that hold personal data.
pub const PERSONAL_COLUMNS: [&str; 3] = ["username", "voter_ip", "voter_country"];

/// What personal data shows as when the reader may not see it.
pub const HIDDEN_VALUE: &str = "hidden";

/// Whether a page or a record shows voters' personal data: usernames, IP addresses and
/// countries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum PersonalData {
    Shown,
    Hidden,
}

impl Page {
    /// Replace the values of the personal columns with [`HIDDEN_VALUE`].
    pub fn hide_personal_data(&mut self) {
        let indexes: Vec<usize> = self
            .rows
            .columns
            .iter()
            .enumerate()
            .filter(|(_, column)| self.personal_columns.contains(column))
            .map(|(index, _)| index)
            .collect();
        for row in &mut self.rows.rows {
            for index in &indexes {
                if let Some(value) = row.get_mut(*index).filter(|value| !value.is_null()) {
                    *value = Value::from(HIDDEN_VALUE);
                }
            }
        }
    }
}

struct TableQuery {
    /// Selected columns, in order.
    select: &'static str,
    from: &'static str,
    /// Condition that scopes the table to the board or event, with `$1`.
    scope: &'static str,
    /// Columns of the key, in order, and the SQL type each has in the cursor.
    key: &'static [(&'static str, &'static str)],
}

/// Separates the key's columns in a cursor.
const CURSOR_SEPARATOR: char = '/';

fn table_query(table: ConsoleTable) -> TableQuery {
    match table {
        ConsoleTable::Records => TableQuery {
            select: "m.id AS position, to_timestamp(m.created) AS created, \
                     m.statement_kind AS kind, m.user_id, m.username, m.election_id, \
                     m.area_id, m.ballot_id, m.delivery_id, m.sender_pk, m.version",
            from: "electoral_log_messages m",
            scope: "m.board_name = $1",
            key: &[("m.id", "bigint")],
        },
        ConsoleTable::Ballots => TableQuery {
            select: "b.seq, b.id, b.ballot_id, b.election_id, b.area_id, b.voter_id, \
                     b.status, b.voting_channel, b.format, octet_length(b.content) AS content_bytes, \
                     b.accepted_at, b.username, b.voter_ip, b.voter_country",
            from: "ballot_box_ballot b",
            scope: "b.election_event_id = $1::text::uuid",
            key: &[("b.seq", "bigint")],
        },
        ConsoleTable::Voters => TableQuery {
            select: "v.election_id, v.voter_id, v.area_id, v.votes, v.last_ballot_id, v.updated_at",
            from: "ballot_box_voter v",
            scope: "v.election_event_id = $1::text::uuid",
            key: &[("v.election_id", "uuid"), ("v.voter_id", "text")],
        },
        ConsoleTable::Queue => TableQuery {
            select: "p.seq, b.ballot_id, b.election_id, b.area_id, b.accepted_at",
            from: "ballot_box_pending p JOIN ballot_box_ballot b \
                   ON b.election_event_id = p.election_event_id AND b.seq = p.seq",
            scope: "p.election_event_id = $1::text::uuid",
            key: &[("p.seq", "bigint")],
        },
    }
}

/// Conditions of the filters that apply to a table, numbered from `$first`.
fn filter_conditions(
    table: ConsoleTable,
    filters: &ConsoleFilters,
    first: usize,
) -> (Vec<String>, Vec<Box<dyn ToSql + Sync + Send>>) {
    let mut conditions = Vec::new();
    let mut params: Vec<Box<dyn ToSql + Sync + Send>> = Vec::new();
    let mut add = |condition: &str, value: Box<dyn ToSql + Sync + Send>| {
        let n = first + params.len();
        conditions.push(condition.replace("$N", &format!("${n}")));
        params.push(value);
    };
    let text = |value: &Option<String>| value.clone().filter(|value| !value.is_empty());
    match table {
        ConsoleTable::Records => {
            if let Some(kind) = text(&filters.statement_kind) {
                add("m.statement_kind = $N", Box::new(kind));
            }
            if let Some(election) = text(&filters.election_id) {
                add("m.election_id = $N", Box::new(election));
            }
            if let Some(user) = text(&filters.user_id) {
                add("m.user_id = $N", Box::new(user));
            }
            if let Some(ballot) = text(&filters.ballot_id) {
                add("m.ballot_id = $N", Box::new(ballot));
            }
            if let Some(after) = filters.created_after {
                add("m.created >= $N", Box::new(after));
            }
            if let Some(before) = filters.created_before {
                add("m.created < $N", Box::new(before));
            }
        }
        ConsoleTable::Ballots => {
            if let Some(election) = text(&filters.election_id) {
                add("b.election_id = $N::text::uuid", Box::new(election));
            }
            if let Some(area) = text(&filters.area_id) {
                add("b.area_id = $N::text::uuid", Box::new(area));
            }
            if let Some(voter) = text(&filters.user_id) {
                add("b.voter_id = $N", Box::new(voter));
            }
            if let Some(ballot) = text(&filters.ballot_id) {
                add("b.ballot_id = $N", Box::new(ballot));
            }
            if let Some(status) = text(&filters.status) {
                add("b.status = $N", Box::new(status));
            }
            if let Some(after) = filters.created_after {
                add("b.accepted_at >= to_timestamp($N::bigint)", Box::new(after));
            }
            if let Some(before) = filters.created_before {
                add("b.accepted_at < to_timestamp($N::bigint)", Box::new(before));
            }
        }
        ConsoleTable::Voters => {
            if let Some(election) = text(&filters.election_id) {
                add("v.election_id = $N::text::uuid", Box::new(election));
            }
            if let Some(area) = text(&filters.area_id) {
                add("v.area_id = $N::text::uuid", Box::new(area));
            }
            if let Some(voter) = text(&filters.user_id) {
                add("v.voter_id = $N", Box::new(voter));
            }
        }
        ConsoleTable::Queue => {
            if let Some(election) = text(&filters.election_id) {
                add("b.election_id = $N::text::uuid", Box::new(election));
            }
            if let Some(area) = text(&filters.area_id) {
                add("b.area_id = $N::text::uuid", Box::new(area));
            }
        }
    }
    (conditions, params)
}

/// Rows of `row_to_json` texts, ordered as `columns`.
fn json_rows(columns: &[String], texts: Vec<String>) -> Result<Vec<Vec<Value>>> {
    texts
        .into_iter()
        .map(|text| {
            let object: serde_json::Map<String, Value> =
                serde_json::from_str(&text).context("A row is not a JSON object")?;
            Ok(columns
                .iter()
                .map(|column| object.get(column).cloned().unwrap_or(Value::Null))
                .collect())
        })
        .collect()
}

fn column_names(select: &str) -> Vec<String> {
    select
        .split(", ")
        .map(|expression| {
            let expression = expression.trim();
            let name = expression
                .rsplit_once(" AS ")
                .map(|(_, alias)| alias)
                .unwrap_or(expression);
            name.rsplit('.').next().unwrap_or(name).trim().to_string()
        })
        .collect()
}

impl PostgresStore {
    /// A page of a table of a board or election event.
    pub async fn console_page(&self, request: &PageRequest<'_>) -> Result<Page> {
        let query = table_query(request.table);
        let scope_value = match request.table {
            ConsoleTable::Records => request.board.to_string(),
            _ => canonical_uuid(request.election_event_id)?,
        };
        let (mut conditions, filter_params) = filter_conditions(request.table, request.filters, 2);
        let mut params: Vec<Box<dyn ToSql + Sync + Send>> = vec![Box::new(scope_value.clone())];
        params.extend(filter_params);
        let (direction, comparison) = match request.order {
            PageOrder::NewestFirst => ("DESC", "<"),
            PageOrder::OldestFirst => ("ASC", ">"),
        };
        let key_columns: Vec<&str> = query.key.iter().map(|(column, _)| *column).collect();
        if let Some(after) = request.after {
            let parts: Vec<&str> = after.splitn(query.key.len(), CURSOR_SEPARATOR).collect();
            ensure!(
                parts.len() == query.key.len(),
                "Invalid page cursor {after:?}"
            );
            let mut placeholders = Vec::new();
            for ((_, sql_type), part) in query.key.iter().zip(parts) {
                params.push(Box::new(part.to_string()));
                placeholders.push(format!("${}::text::{sql_type}", params.len()));
            }
            conditions.push(format!(
                "({}) {comparison} ({})",
                key_columns.join(", "),
                placeholders.join(", ")
            ));
        }
        let limit = request.limit.clamp(1, MAX_PAGE_ROWS);
        let conditions = conditions
            .iter()
            .map(|condition| format!(" AND {condition}"))
            .collect::<String>();
        let order = key_columns
            .iter()
            .map(|column| format!("{column} {direction}"))
            .collect::<Vec<_>>()
            .join(", ");
        let cursor = key_columns
            .iter()
            .map(|column| format!("{column}::text"))
            .collect::<Vec<_>>()
            .join(&format!(" || '{CURSOR_SEPARATOR}' || "));
        let sql = format!(
            "SELECT row_to_json(r)::text, r.page_cursor FROM ( \
                 SELECT {}, {cursor} AS page_cursor FROM {} WHERE {}{conditions} \
                 ORDER BY {order} LIMIT {limit} \
             ) r",
            query.select, query.from, query.scope
        );
        let refs: Vec<&(dyn ToSql + Sync)> = params
            .iter()
            .map(|param| param.as_ref() as &(dyn ToSql + Sync))
            .collect();
        let client = self.client().await?;
        let found = client
            .query(&sql, &refs)
            .await
            .with_context(|| format!("Error reading the {} table", request.table))?;
        let columns = column_names(query.select);
        let next = if found.len() as i64 == limit {
            found
                .last()
                .map(|row| row.try_get::<_, String>(1))
                .transpose()?
        } else {
            None
        };
        let texts = found
            .iter()
            .map(|row| row.try_get::<_, String>(0))
            .collect::<Result<Vec<_>, _>>()?;
        let rows = json_rows(&columns, texts)?;
        let estimated_rows = self.estimated_rows(request.table, &scope_value).await?;
        let personal_columns = columns
            .iter()
            .filter(|column| PERSONAL_COLUMNS.contains(&column.as_str()))
            .cloned()
            .collect();
        Ok(Page {
            rows: Rows { columns, rows },
            next,
            estimated_rows,
            personal_columns,
        })
    }

    /// Rows of a table for a board or event: exact for records, from the planner's
    /// statistics for the ballot box's partitions once they have them, counted
    /// otherwise.
    async fn estimated_rows(&self, table: ConsoleTable, scope: &str) -> Result<i64> {
        let client = self.client().await?;
        let partition = |prefix: &str| format!("{prefix}_{}", scope.replace('-', ""));
        let (estimate, count) = match table {
            ConsoleTable::Records => {
                let row = client
                    .query_opt("SELECT size FROM trellis_logs WHERE name = $1", &[&scope])
                    .await?;
                return Ok(row.map(|row| row.get::<_, i64>(0)).unwrap_or(0));
            }
            ConsoleTable::Ballots => (
                partition("ballot_box_ballot"),
                "SELECT count(*) FROM ballot_box_ballot WHERE election_event_id = $1::text::uuid",
            ),
            ConsoleTable::Voters => (
                partition("ballot_box_voter"),
                "SELECT count(*) FROM ballot_box_voter WHERE election_event_id = $1::text::uuid",
            ),
            ConsoleTable::Queue => {
                return Ok(client
                    .query_one(
                        "SELECT count(*) FROM ballot_box_pending \
                         WHERE election_event_id = $1::text::uuid",
                        &[&scope],
                    )
                    .await?
                    .get(0));
            }
        };
        let estimate: Option<f32> = client
            .query_one(
                "SELECT (SELECT reltuples FROM pg_class WHERE oid = to_regclass($1))",
                &[&estimate],
            )
            .await?
            .get(0);
        match estimate {
            Some(estimate) if estimate >= 0.0 => Ok(estimate as i64),
            _ => Ok(client.query_one(count, &[&scope]).await?.get(0)),
        }
    }

    /// A record of a board, with its delivery ID.
    pub async fn console_record(&self, board: &str, id: i64) -> Result<Option<ConsoleRecord>> {
        let row = self
            .client()
            .await?
            .query_opt(
                "SELECT id, to_json(to_timestamp(created)) #>> '{}', statement_kind, user_id, \
                        username, election_id, area_id, ballot_id, delivery_id, sender_pk, \
                        version, message \
                 FROM electoral_log_messages WHERE board_name = $1 AND id = $2",
                &[&board, &id],
            )
            .await?;
        row.map(|row| {
            Ok(ConsoleRecord {
                id: row.try_get(0)?,
                created: row.try_get(1)?,
                statement_kind: row.try_get(2)?,
                user_id: row.try_get(3)?,
                username: row.try_get(4)?,
                election_id: row.try_get(5)?,
                area_id: row.try_get(6)?,
                ballot_id: row.try_get(7)?,
                delivery_id: row.try_get(8)?,
                sender_pk: row.try_get(9)?,
                version: row.try_get(10)?,
                message: row.try_get(11)?,
            })
        })
        .transpose()
    }

    /// Let `role` read every table of the database, those created later included.
    /// Run by the database's owner.
    pub async fn grant_read(&self, role: &str) -> Result<()> {
        ensure!(
            !role.is_empty()
                && role
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
                && !role.starts_with(|c: char| c.is_ascii_digit()),
            "Invalid PostgreSQL role name {role:?}"
        );
        self.client()
            .await?
            .batch_execute(&format!(
                "GRANT USAGE ON SCHEMA public TO {role};
                 GRANT SELECT ON ALL TABLES IN SCHEMA public TO {role};
                 ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT SELECT ON TABLES TO {role};"
            ))
            .await
            .with_context(|| format!("Error letting {role} read the database"))
    }
}

/// A record of a board, as the console shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsoleRecord {
    pub id: i64,
    /// When the log accepted it, as `2026-10-05T20:00:00+00:00`.
    pub created: String,
    pub statement_kind: String,
    pub user_id: Option<String>,
    pub username: Option<String>,
    pub election_id: Option<String>,
    pub area_id: Option<String>,
    pub ballot_id: Option<String>,
    pub delivery_id: String,
    pub sender_pk: String,
    pub version: String,
    /// The signed message, serialized.
    pub message: Vec<u8>,
}

impl ConsoleRecord {
    /// The record as JSON, with its message decoded. The message's artifact shows as
    /// its size in bytes, and hashes and other byte arrays of at least
    /// [`HEX_MIN_BYTES`] bytes as hexadecimal. With [`PersonalData::Hidden`], usernames
    /// and the IP addresses and countries of cast votes, anywhere in the record, show
    /// as [`HIDDEN_VALUE`].
    pub fn to_json(&self, personal_data: PersonalData) -> Result<Value> {
        let message = Message::strand_deserialize(&self.message)
            .map_err(|error| anyhow!("Error decoding the record's message: {error:?}"))?;
        let mut message = serde_json::to_value(&message)?;
        if let Some(object) = message.as_object_mut() {
            if let Some(artifact) = object.remove("artifact") {
                let bytes = artifact
                    .as_array()
                    .map_or(Value::Null, |bytes| Value::from(bytes.len()));
                object.insert("artifact_bytes".into(), bytes);
            }
        }
        bytes_as_hex(&mut message);
        let mut record = serde_json::json!({
            "position": self.id,
            "created": self.created,
            "kind": self.statement_kind,
            "user_id": self.user_id,
            "username": self.username,
            "election_id": self.election_id,
            "area_id": self.area_id,
            "ballot_id": self.ballot_id,
            "delivery_id": self.delivery_id,
            "sender_pk": self.sender_pk,
            "version": self.version,
            "message": message,
        });
        if personal_data == PersonalData::Hidden {
            hide_personal_values(&mut record);
        }
        Ok(record)
    }
}

/// The shortest array of numbers from 0 to 255 that a decoded record shows as
/// hexadecimal: hashes are 64 bytes.
pub const HEX_MIN_BYTES: usize = 16;

/// Show byte arrays, as serde writes hashes, as hexadecimal strings.
fn bytes_as_hex(value: &mut Value) {
    match value {
        Value::Array(values) => {
            let bytes: Option<Vec<u8>> = values
                .iter()
                .map(|value| value.as_u64().and_then(|byte| u8::try_from(byte).ok()))
                .collect();
            match bytes {
                Some(bytes) if bytes.len() >= HEX_MIN_BYTES => {
                    *value = Value::from(hex::encode(bytes));
                }
                _ => values.iter_mut().for_each(bytes_as_hex),
            }
        }
        Value::Object(object) => object.values_mut().for_each(bytes_as_hex),
        _ => {}
    }
}

/// Hide usernames anywhere in a JSON value, and the IP addresses and countries of cast
/// votes, which read `ip: …` and `country: …`.
fn hide_personal_values(value: &mut Value) {
    match value {
        Value::Object(object) => {
            for (key, value) in object.iter_mut() {
                if key == "username" && !value.is_null() {
                    *value = Value::from(HIDDEN_VALUE);
                } else {
                    hide_personal_values(value);
                }
            }
        }
        Value::Array(values) => values.iter_mut().for_each(hide_personal_values),
        Value::String(text) if text.starts_with("ip: ") || text.starts_with("country: ") => {
            *text = HIDDEN_VALUE.to_string();
        }
        _ => {}
    }
}

/// The result of a query.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct QueryResult {
    #[serde(flatten)]
    pub rows: Rows,
    /// Whether the query returned more rows than the limit.
    pub truncated: bool,
    pub elapsed_ms: u64,
}

/// Run one SQL query in a read-only transaction, with a time limit, and return at
/// most `max_rows` of its rows. Only a query that can be a subquery runs: a `SELECT`,
/// `VALUES`, `TABLE` or `WITH` without data-modifying statements. Use a connection of
/// its own, of a role that can only read, and close it afterwards, so that nothing the
/// query sets outlives it.
pub async fn run_read_only_query(
    client: &mut tokio_postgres::Client,
    sql: &str,
    max_rows: usize,
    timeout: Duration,
) -> Result<QueryResult> {
    let sql = sql
        .trim()
        .trim_end_matches(|c: char| c == ';' || c.is_whitespace());
    ensure!(!sql.is_empty(), "The query is empty");
    let started = Instant::now();
    let transaction = client.build_transaction().read_only(true).start().await?;
    transaction
        .batch_execute(&format!(
            "SET LOCAL statement_timeout = {}",
            timeout.as_millis()
        ))
        .await?;
    let statement = transaction
        .prepare(sql)
        .await
        .map_err(|error| anyhow!(query_error(&error)))?;
    let columns: Vec<String> = statement
        .columns()
        .iter()
        .map(|column| column.name().to_string())
        .collect();
    ensure!(
        !columns.is_empty(),
        "Only queries that return rows run: SELECT, VALUES, TABLE or WITH"
    );
    let found = transaction
        .query(
            &format!(
                "SELECT row_to_json(q)::text FROM ({sql}\n) q LIMIT {}",
                max_rows + 1
            ),
            &[],
        )
        .await
        .map_err(|error| anyhow!(query_error(&error)))?;
    transaction.rollback().await?;
    let truncated = found.len() > max_rows;
    let texts = found
        .iter()
        .take(max_rows)
        .map(|row| row.try_get::<_, String>(0))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(QueryResult {
        rows: Rows {
            rows: json_rows(&columns, texts)?,
            columns,
        },
        truncated,
        elapsed_ms: started.elapsed().as_millis() as u64,
    })
}

/// The server's message of a failed query, which the person who wrote it can act on.
fn query_error(error: &tokio_postgres::Error) -> String {
    match error.as_db_error() {
        Some(db) => match db.position() {
            Some(tokio_postgres::error::ErrorPosition::Original(position)) => {
                format!("{} (at character {position})", db.message())
            }
            _ => db.message().to_string(),
        },
        None => error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::message::SigningData;
    use crate::messages::newtypes::{
        CastVoteHash, ElectionIdString, EventIdString, PseudonymHash, VoterCountryString,
        VoterIpString,
    };
    use strand::signature::StrandSignatureSk;

    fn cast_vote_record() -> ConsoleRecord {
        let sk = StrandSignatureSk::generate().unwrap();
        let signing_data = SigningData::new(sk.clone(), "voting", sk);
        let message = Message::cast_vote_message(
            EventIdString("event".into()),
            ElectionIdString(Some("election".into())),
            PseudonymHash::new([1; 64]),
            CastVoteHash::new([2; 64]),
            &signing_data,
            VoterIpString("ip: 192.0.2.1".into()),
            VoterCountryString("country: ES".into()),
            Some("voter-id".into()),
            Some("voter@example.com".into()),
            "area".into(),
        )
        .unwrap();
        ConsoleRecord {
            id: 7,
            created: "2026-10-05T20:00:00+00:00".into(),
            statement_kind: "CastVote".into(),
            user_id: Some("voter-id".into()),
            username: Some("voter@example.com".into()),
            election_id: Some("election".into()),
            area_id: Some("area".into()),
            ballot_id: None,
            delivery_id: "delivery".into(),
            sender_pk: "pk".into(),
            version: "1".into(),
            message: borsh::to_vec(&message).unwrap(),
        }
    }

    fn strings(value: &Value, found: &mut Vec<String>) {
        match value {
            Value::String(text) => found.push(text.clone()),
            Value::Array(values) => values.iter().for_each(|value| strings(value, found)),
            Value::Object(object) => object.values().for_each(|value| strings(value, found)),
            _ => {}
        }
    }

    #[test]
    fn records_decode_their_message_and_hide_personal_data() {
        let record = cast_vote_record();
        let shown = record.to_json(PersonalData::Shown).unwrap();
        assert_eq!(shown["position"], 7);
        assert_eq!(shown["kind"], "CastVote");
        assert_eq!(shown["message"]["username"], "voter@example.com");
        let body = &shown["message"]["statement"]["body"]["CastVote"];
        assert_eq!(body[1], "01".repeat(64));
        assert_eq!(body[2], "02".repeat(64));
        assert!(shown["message"].get("artifact").is_none());
        let mut found = Vec::new();
        strings(&shown, &mut found);
        assert!(found.contains(&"ip: 192.0.2.1".to_string()));
        assert!(found.contains(&"country: ES".to_string()));

        let hidden = record.to_json(PersonalData::Hidden).unwrap();
        assert_eq!(hidden["username"], HIDDEN_VALUE);
        assert_eq!(hidden["message"]["username"], HIDDEN_VALUE);
        assert_eq!(hidden["user_id"], "voter-id");
        let mut found = Vec::new();
        strings(&hidden, &mut found);
        assert!(!found.iter().any(|text| text.starts_with("ip: ")
            || text.starts_with("country: ")
            || text.contains("192.0.2.1")
            || text.contains('@')));
        assert_eq!(
            hidden["message"]["statement"]["head"],
            shown["message"]["statement"]["head"]
        );
    }

    #[test]
    fn only_long_byte_arrays_become_hex() {
        let mut value = serde_json::json!({
            "hash": vec![255; HEX_MIN_BYTES],
            "short": [1, 2, 3],
            "numbers": vec![256; HEX_MIN_BYTES],
            "nested": [vec![0; HEX_MIN_BYTES], "text"],
        });
        bytes_as_hex(&mut value);
        assert_eq!(value["hash"], "ff".repeat(HEX_MIN_BYTES));
        assert_eq!(value["short"], serde_json::json!([1, 2, 3]));
        assert_eq!(
            value["numbers"],
            serde_json::json!(vec![256; HEX_MIN_BYTES])
        );
        assert_eq!(value["nested"][0], "00".repeat(HEX_MIN_BYTES));
    }

    #[test]
    fn records_that_do_not_decode_are_errors() {
        let record = ConsoleRecord {
            message: vec![1, 2, 3],
            ..cast_vote_record()
        };
        assert!(record.to_json(PersonalData::Shown).is_err());
    }

    #[test]
    fn pages_hide_only_the_values_of_personal_columns() {
        let mut page = Page {
            rows: Rows {
                columns: vec!["seq".into(), "username".into(), "voter_ip".into()],
                rows: vec![
                    vec![Value::from(1), Value::from("a@example.com"), Value::Null],
                    vec![Value::from(2), Value::Null, Value::from("192.0.2.1")],
                ],
            },
            next: None,
            estimated_rows: 2,
            personal_columns: vec!["username".into(), "voter_ip".into()],
        };
        page.hide_personal_data();
        assert_eq!(
            page.rows.rows,
            [
                vec![Value::from(1), Value::from(HIDDEN_VALUE), Value::Null],
                vec![Value::from(2), Value::Null, Value::from(HIDDEN_VALUE)],
            ]
        );
        assert_eq!(PersonalData::Hidden.to_string(), "hidden");
    }

    #[test]
    fn columns_are_named_by_alias_or_column() {
        assert_eq!(
            column_names("m.id AS position, to_timestamp(m.created) AS created, m.user_id"),
            ["position", "created", "user_id"]
        );
        for table in [
            ConsoleTable::Records,
            ConsoleTable::Ballots,
            ConsoleTable::Voters,
            ConsoleTable::Queue,
        ] {
            assert!(!column_names(table_query(table).select).is_empty());
        }
    }

    #[test]
    fn filters_apply_only_to_tables_with_their_column() {
        let filters = ConsoleFilters {
            statement_kind: Some("CastVote".into()),
            area_id: Some("a".into()),
            status: Some(String::new()),
            created_after: Some(10),
            ..Default::default()
        };
        let (records, params) = filter_conditions(ConsoleTable::Records, &filters, 2);
        assert_eq!(records, ["m.statement_kind = $2", "m.created >= $3"]);
        assert_eq!(params.len(), 2);
        let (ballots, _) = filter_conditions(ConsoleTable::Ballots, &filters, 2);
        assert_eq!(
            ballots,
            [
                "b.area_id = $2::text::uuid",
                "b.accepted_at >= to_timestamp($3::bigint)"
            ]
        );
        let (voters, _) = filter_conditions(ConsoleTable::Voters, &filters, 5);
        assert_eq!(voters, ["v.area_id = $5::text::uuid"]);
    }

    #[test]
    fn rows_follow_the_columns_order() {
        let columns = vec!["b".to_string(), "a".to_string(), "c".to_string()];
        let rows = json_rows(&columns, vec![r#"{"a":1,"b":"x"}"#.to_string()]).unwrap();
        assert_eq!(rows, [vec![Value::from("x"), Value::from(1), Value::Null]]);
    }

    #[test]
    fn tables_and_orders_have_stable_names() {
        assert_eq!(ConsoleTable::Records.to_string(), "records");
        assert_eq!(
            "queue".parse::<ConsoleTable>().unwrap(),
            ConsoleTable::Queue
        );
        assert_eq!(PageOrder::default().to_string(), "newest-first");
    }
}
