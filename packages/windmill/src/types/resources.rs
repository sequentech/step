// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Database query result types and pagination helpers.
use anyhow::anyhow;
use electoral_log::assign_value;
use immudb_rs::{sql_value::Value, Client, NamedParam, Row, SqlValue};
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};

/// Single-column `COUNT(*)` (or equivalent) extracted from an SQL row.
#[derive(Serialize, Deserialize, Debug)]
pub struct Aggregate {
    /// Number of matching rows for the aggregate expression.
    pub count: i64,
}

/// Wrapper GraphQL uses for `aggregate { count }` style totals.
#[derive(Serialize, Deserialize, Debug)]
pub struct TotalAggregate {
    /// Nested aggregate payload.
    pub aggregate: Aggregate,
}

/// Enumeration for the valid order directions
#[derive(Debug, Deserialize, EnumString, Display, Clone)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum OrderDirection {
    /// Ascending key order.
    Asc,
    /// Descending key order.
    Desc,
}

/// Page of rows plus total count metadata from a list query.
#[derive(Serialize, Deserialize, Debug)]
pub struct DataList<T> {
    /// Rows returned for the current page.
    pub items: Vec<T>,
    /// Total number of rows matching the filter (not only this page).
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
