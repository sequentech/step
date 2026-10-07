// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use strum_macros::{Display, EnumString};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ElectoralLogMessage {
    pub id: i64,
    pub created: i64,
    pub sender_pk: String,
    pub statement_timestamp: i64,
    pub statement_kind: String,
    pub message: Vec<u8>,
    pub version: String,
    pub user_id: Option<String>,
    pub username: Option<String>,
    pub election_id: Option<String>,
    pub area_id: Option<String>,
    pub ballot_id: Option<String>,
}

/// Delivery identity is independent of signed bytes: two identical events can
/// be distinct deliveries, while retrying a delivery must be idempotent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LogEntry {
    pub delivery_id: String,
    pub message: ElectoralLogMessage,
}

#[derive(Debug, Clone, Copy, Display, EnumString, PartialEq, Eq, Ord, PartialOrd)]
#[strum(serialize_all = "snake_case")]
pub enum TextColumn {
    StatementKind,
    UserId,
    BallotId,
    Username,
    SenderPk,
    ElectionId,
    AreaId,
    Version,
}

pub type ElectoralLogVarCharColumn = TextColumn;

#[derive(Display, Debug, Clone, Copy)]
pub enum SqlCompOperators {
    #[strum(to_string = "=")]
    Equal,
    #[strum(to_string = "!=")]
    NotEqual,
    #[strum(to_string = ">")]
    GreaterThan,
    #[strum(to_string = "<")]
    LessThan,
    #[strum(to_string = ">=")]
    GreaterThanOrEqual,
    #[strum(to_string = "<=")]
    LessThanOrEqual,
    #[strum(to_string = "LIKE")]
    Like,
}

pub type WhereClauseBTreeMap = BTreeMap<TextColumn, (SqlCompOperators, String)>;

#[derive(Debug, Clone, Copy, Display)]
#[strum(serialize_all = "snake_case")]
pub enum NumberColumn {
    Id,
    Created,
    StatementTimestamp,
}

#[derive(Debug, Clone, Copy, Display)]
pub enum NumberComparison {
    #[strum(to_string = "=")]
    Equal,
    #[strum(to_string = ">")]
    GreaterThan,
    #[strum(to_string = ">=")]
    GreaterThanOrEqual,
    #[strum(to_string = "<")]
    LessThan,
    #[strum(to_string = "<=")]
    LessThanOrEqual,
}

#[derive(Debug, Clone)]
pub enum Filter {
    Text(TextColumn, SqlCompOperators, String),
    Number(NumberColumn, NumberComparison, i64),
}

#[derive(Debug, Clone, Copy, Display, EnumString, PartialEq, Eq)]
#[strum(serialize_all = "snake_case")]
pub enum OrderColumn {
    Id,
    Created,
    StatementTimestamp,
    StatementKind,
    Message,
    UserId,
    Username,
    BallotId,
    SenderPk,
    ElectionId,
    AreaId,
    Version,
}

#[derive(Debug, Clone, Copy, Display, EnumString)]
#[strum(serialize_all = "lowercase", ascii_case_insensitive)]
pub enum SortDirection {
    Asc,
    Desc,
}

/// Preserve the existing election/area visibility policy, including general
/// entries whose election and area are both absent. Some(empty) restricts the
/// result to general entries; None means no additional visibility restriction.
#[derive(Debug, Clone)]
pub struct LogVisibility {
    pub election_id: Option<String>,
    pub area_ids: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct LogQuery {
    pub filters: Vec<Filter>,
    pub visibility: Option<LogVisibility>,
    pub only_with_user: bool,
    pub order: Vec<(OrderColumn, SortDirection)>,
    pub limit: i64,
    pub offset: i64,
}

impl Default for LogQuery {
    fn default() -> Self {
        Self {
            filters: vec![],
            visibility: None,
            only_with_user: false,
            order: vec![(OrderColumn::Id, SortDirection::Desc)],
            limit: 900,
            offset: 0,
        }
    }
}

impl LogQuery {
    pub fn validate(&self) -> Result<()> {
        ensure!(self.limit >= 0, "Electoral-log limit must be nonnegative");
        ensure!(self.offset >= 0, "Electoral-log offset must be nonnegative");
        Ok(())
    }

    pub fn with_text_filters(mut self, filters: Option<WhereClauseBTreeMap>) -> Self {
        if let Some(filters) = filters {
            self.filters.extend(
                filters
                    .into_iter()
                    .map(|(column, (op, value))| Filter::Text(column, op, value)),
            );
        }
        self
    }
}
