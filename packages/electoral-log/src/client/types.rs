// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::assign_value;
use crate::messages::statement::{StatementBody, StatementType};
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use immudb_rs::{sql_value::Value, Row};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::fmt::Debug;
use std::str::FromStr;
use strum_macros::{Display, EnumString};
use tracing::{error, info, instrument, warn};

#[derive(Debug, Clone, Display, PartialEq, Eq, Ord, PartialOrd, EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum ElectoralLogVarCharColumn {
    StatementKind,
    AreaId,
    ElectionId,
    UserIdKey,
    UserId,
    BallotId,
    Username,
    SenderPk,
    Version,
}

/// SQL comparison operators supported by immudb.
/// ILIKE is not supported.
#[derive(Display, Debug, Clone)]
pub enum SqlCompOperators {
    #[strum(to_string = "=")]
    Equal(String),
    #[strum(to_string = "!=")]
    NotEqual(String),
    #[strum(to_string = ">")]
    GreaterThan(String),
    #[strum(to_string = "<")]
    LessThan(String),
    #[strum(to_string = ">=")]
    GreaterThanOrEqual(String),
    #[strum(to_string = "<=")]
    LessThanOrEqual(String),
    #[strum(to_string = "LIKE")]
    Like(String),
    #[strum(to_string = "IN")]
    In(Vec<String>),
    #[strum(to_string = "NOT IN")]
    NotIn(Vec<String>),
}

pub type WhereClauseBTreeMap = BTreeMap<ElectoralLogVarCharColumn, SqlCompOperators>;

// Enumeration for the valid fields in the immudb table
#[derive(Debug, Deserialize, Hash, PartialEq, Eq, EnumString, Display, Clone)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum OrderField {
    Id,
    Created,
    StatementTimestamp,
    StatementKind,
    Message,
    UserId,
    Username,
    BallotId,
    SenderPk,
    LogType,
    EventType,
    Description,
    Version,
}

// Enumeration for the valid order directions
#[derive(Debug, Deserialize, EnumString, Display, Clone)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum OrderDirection {
    Asc,
    Desc,
}

#[derive(Deserialize, Debug, Default, Clone)]
pub struct GetElectoralLogBody {
    pub tenant_id: String,
    pub election_event_id: String,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub filter: Option<HashMap<OrderField, String>>,
    pub order_by: Option<HashMap<OrderField, OrderDirection>>,
    pub election_id: Option<String>,
    pub area_ids: Option<Vec<String>>,
    pub only_with_user: Option<bool>,
    pub statement_kind: Option<StatementType>,
}

impl GetElectoralLogBody {
    #[instrument(skip_all)]
    pub fn get_min_max_ts(&self) -> Result<(Option<i64>, Option<i64>)> {
        let mut min_ts: Option<i64> = None;
        let mut max_ts: Option<i64> = None;
        if let Some(filters_map) = &self.filter {
            for (field, value) in filters_map.iter() {
                match field {
                    OrderField::Created | OrderField::StatementTimestamp => {
                        let date_time_utc = DateTime::parse_from_rfc3339(&value)
                            .map_err(|err| anyhow!("{:?}", err))?;
                        let datetime = date_time_utc.with_timezone(&Utc);
                        let ts: i64 = datetime.timestamp();
                        let ts_end: i64 = ts + 60; // Search along that minute, the second is not specified by the front.
                        min_ts = Some(ts);
                        max_ts = Some(ts_end);
                    }
                    _ => {}
                }
            }
        }

        Ok((min_ts, max_ts))
    }

    #[instrument(skip_all)]
    pub fn as_where_clause_map(&self) -> Result<WhereClauseBTreeMap> {
        let mut cols_match_select = WhereClauseBTreeMap::new();
        if let Some(filters_map) = &self.filter {
            for (field, value) in filters_map.iter() {
                match field {
                    OrderField::Id => {} // Why would someone filter the electoral log by id?
                    OrderField::SenderPk | OrderField::Username | OrderField::BallotId | OrderField::StatementKind | OrderField::Version => { // sql VARCHAR type
                        let variant = ElectoralLogVarCharColumn::from_str(field.to_string().as_str()).map_err(|_| anyhow!("Field not found"))?; 
                        cols_match_select.insert(
                            variant,
                            SqlCompOperators::Like(value.clone()),
                        );
                    }
                    OrderField::UserId => {
                        // insert user_id_mod
                        cols_match_select.insert(
                            ElectoralLogVarCharColumn::UserIdKey,
                            SqlCompOperators::Equal(value.clone().chars().take(3).collect()),
                        );
                        let variant = ElectoralLogVarCharColumn::from_str(field.to_string().as_str()).map_err(|_| anyhow!("Field not found"))?; 
                        cols_match_select.insert(
                            variant,
                            SqlCompOperators::Like(value.clone()),
                        );
                    }
                    OrderField::StatementTimestamp | OrderField::Created => {} // handled by `get_min_max_ts`
                    OrderField::EventType | OrderField::LogType | OrderField::Description // these have no column but are inside of Message
                    | OrderField::Message => {} // Message column is sql BLOB type and it´s encrypted so we can't filter it without expensive operations
                }
            }
        }
        if let Some(election_id) = &self.election_id {
            if !election_id.is_empty() {
                cols_match_select.insert(
                    ElectoralLogVarCharColumn::ElectionId,
                    SqlCompOperators::Like(election_id.clone()),
                );
            }
        }

        if let Some(area_ids) = &self.area_ids {
            if !area_ids.is_empty() {
                // NOTE: `IN` values must be handled later in SQL building, here we just join them
                cols_match_select.insert(
                    ElectoralLogVarCharColumn::AreaId,
                    SqlCompOperators::In(area_ids.clone()), // TODO: NullOrIn
                );
            }
        }

        if let Some(statement_kind) = &self.statement_kind {
            cols_match_select.insert(
                ElectoralLogVarCharColumn::StatementKind,
                SqlCompOperators::Equal(statement_kind.to_string()),
            );
        }

        Ok(cols_match_select)
    }

    #[instrument]
    pub fn as_cast_vote_count_and_select_clauses(
        &self,
        election_id: &str,
        user_id: &str,
        ballot_id_filter: &str,
    ) -> (WhereClauseBTreeMap, WhereClauseBTreeMap) {
        let cols_match_count = BTreeMap::from([
            (
                ElectoralLogVarCharColumn::StatementKind,
                SqlCompOperators::Equal(StatementType::CastVote.to_string()),
            ),
            (
                ElectoralLogVarCharColumn::ElectionId,
                SqlCompOperators::Equal(election_id.to_string()),
            ),
        ]);
        let mut cols_match_select = cols_match_count.clone();
        // Restrict the SQL query to user_id and ballot_id in case of filtering
        if !ballot_id_filter.is_empty() {
            cols_match_select.insert(
                ElectoralLogVarCharColumn::UserIdKey,
                SqlCompOperators::Equal(user_id.clone().chars().take(3).collect()),
            );
            cols_match_select.insert(
                ElectoralLogVarCharColumn::UserId,
                SqlCompOperators::Equal(user_id.to_string()),
            );
            cols_match_select.insert(
                ElectoralLogVarCharColumn::BallotId,
                SqlCompOperators::Like(ballot_id_filter.to_string()),
            );
        }

        (cols_match_count, cols_match_select)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl TryFrom<&Row> for ElectoralLogMessage {
    type Error = anyhow::Error;

    fn try_from(row: &Row) -> Result<Self, Self::Error> {
        let mut id = 0;
        let mut created = 0;
        let mut sender_pk = String::from("");
        let mut statement_timestamp = 0;
        let mut statement_kind = String::from("");
        let mut message = vec![];
        let mut version = String::from("");
        let mut user_id: Option<String> = None;
        let mut username: Option<String> = None;
        let mut election_id: Option<String> = None;
        let mut area_id: Option<String> = None;
        let mut ballot_id: Option<String> = None;

        for (column, value) in row.columns.iter().zip(row.values.iter()) {
            // FIXME for some reason columns names appear with parentheses
            let dot = column
                .find('.')
                .ok_or(anyhow!("invalid column found '{}'", column.as_str()))?;
            let bare_column = &column[dot + 1..column.len() - 1];

            match bare_column {
                "id" => assign_value!(Value::N, value, id),
                "created" => assign_value!(Value::Ts, value, created),
                "sender_pk" => assign_value!(Value::S, value, sender_pk),
                "statement_timestamp" => {
                    assign_value!(Value::Ts, value, statement_timestamp)
                }
                "statement_kind" => assign_value!(Value::S, value, statement_kind),
                "message" => assign_value!(Value::Bs, value, message),
                "version" => assign_value!(Value::S, value, version),
                "user_id" => match value.value.as_ref() {
                    Some(Value::S(inner)) => user_id = Some(inner.clone()),
                    Some(Value::Null(_)) => user_id = None,
                    None => user_id = None,
                    _ => {
                        return Err(anyhow!(
                            "invalid column value for 'user_id': {:?}",
                            value.value.as_ref()
                        ))
                    }
                },
                "username" => match value.value.as_ref() {
                    Some(Value::S(inner)) => username = Some(inner.clone()),
                    Some(Value::Null(_)) => username = None,
                    None => username = None,
                    _ => {
                        return Err(anyhow!(
                            "invalid column value for 'username': {:?}",
                            value.value.as_ref()
                        ))
                    }
                },
                "election_id" => match value.value.as_ref() {
                    Some(Value::S(inner)) => election_id = Some(inner.clone()),
                    Some(Value::Null(_)) => election_id = None,
                    None => election_id = None,
                    _ => {
                        return Err(anyhow!(
                            "invalid column value for 'election_id': {:?}",
                            value.value.as_ref()
                        ))
                    }
                },
                "area_id" => match value.value.as_ref() {
                    Some(Value::S(inner)) => area_id = Some(inner.clone()),
                    Some(Value::Null(_)) => area_id = None,
                    None => area_id = None,
                    _ => {
                        return Err(anyhow!(
                            "invalid column value for 'area_id': {:?}",
                            value.value.as_ref()
                        ))
                    }
                },
                "ballot_id" => match value.value.as_ref() {
                    Some(Value::S(inner)) => ballot_id = Some(inner.clone()),
                    Some(Value::Null(_)) => ballot_id = None,
                    None => ballot_id = None,
                    _ => {
                        return Err(anyhow!(
                            "invalid column value for 'ballod_id': {:?}",
                            value.value.as_ref()
                        ))
                    }
                },
                _ => return Err(anyhow!("invalid column found '{}'", bare_column)),
            }
        }

        Ok(ElectoralLogMessage {
            id,
            created,
            sender_pk,
            statement_timestamp,
            statement_kind,
            message,
            version,
            user_id,
            username,
            election_id,
            area_id,
            ballot_id,
        })
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Aggregate {
    pub count: i64,
}

impl TryFrom<&Row> for Aggregate {
    type Error = anyhow::Error;

    fn try_from(row: &Row) -> Result<Self, Self::Error> {
        let mut count = 0;

        for (column, value) in row.columns.iter().zip(row.values.iter()) {
            match column.as_str() {
                _ => assign_value!(Value::N, value, count),
            }
        }
        Ok(Aggregate { count })
    }
}
