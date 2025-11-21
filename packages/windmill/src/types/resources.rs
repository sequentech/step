// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::anyhow;
use electoral_log::assign_value;
use immudb_rs::{sql_value::Value, Client, NamedParam, Row, SqlValue};
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};

#[derive(Serialize, Deserialize, Debug)]
pub struct Aggregate {
    pub count: i64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TotalAggregate {
    pub aggregate: Aggregate,
}

// Enumeration for the valid order directions
#[derive(Debug, Deserialize, EnumString, Display, Clone)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum OrderDirection {
    Asc,
    Desc,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DataList<T> {
    pub items: Vec<T>,
    pub total: TotalAggregate,
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
