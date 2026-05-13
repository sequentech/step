// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::anyhow;
use electoral_log::assign_value;
use immudb_rs::{sql_value::Value, Client, NamedParam, Row, SqlValue};
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};

/// Single numeric aggregate (for example a total count).
#[derive(Serialize, Deserialize, Debug)]
pub struct Aggregate {
    /// Aggregate value, typically a row count.
    pub count: i64,
}

/// Wrapper for nested aggregates in API list responses.
#[derive(Serialize, Deserialize, Debug)]
pub struct TotalAggregate {
    /// Root aggregate payload.
    pub aggregate: Aggregate,
}

/// Enumeration for the valid order directions
#[derive(Debug, Deserialize, EnumString, Display)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum OrderDirection {
    /// Ascending order.
    Asc,
    /// Descending order.
    Desc,
}

#[derive(Deserialize, Debug)]
/// Payload for sorting.
pub struct SortPayload {
    /// The field to sort by.
    pub field: String,
    /// The order to sort by.
    pub order: String,
}

/// Paginated list envelope with items and total aggregate metadata.
#[derive(Serialize, Deserialize, Debug)]
pub struct DataList<T> {
    /// Items for the current page or query.
    pub items: Vec<T>,
    /// Total counts or related aggregates for the full result set.
    pub total: TotalAggregate,
}

impl TryFrom<&Row> for Aggregate {
    type Error = anyhow::Error;

    fn try_from(row: &Row) -> Result<Self, Self::Error> {
        let mut count = 0;

        for (column, value) in row.columns.iter().zip(row.values.iter()) {
            assign_value!(Value::N, value, count);
        }
        Ok(Aggregate { count })
    }
}
