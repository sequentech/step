// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use strum_macros::Display;

#[derive(Debug, Clone, Copy, Display, PartialEq, Eq, Ord, PartialOrd)]
#[strum(serialize_all = "snake_case")]
pub enum ElectoralLogVarCharColumn {
    StatementKind,
    UserId,
    BallotId,
    Username,
    SenderPk,
    ElectionId,
    AreaId,
    Version,
}

/// Supported comparisons for indexed application audit metadata.
#[derive(Display, Debug, Clone)]
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

pub type WhereClauseBTreeMap = BTreeMap<ElectoralLogVarCharColumn, (SqlCompOperators, String)>;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, sqlx::FromRow)]
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
