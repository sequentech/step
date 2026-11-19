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

/// What a list's total says about its matches: a list stops counting past `LOW_SQL_LIMIT`.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TotalBound {
    /// The total is the number of matches.
    #[default]
    Exact,
    /// There are more matches than the total.
    MoreThan,
}

impl TotalBound {
    /// The total and its bound for a count that stopped after `count_limit + 1` matches.
    pub fn of_count(counted: i64, count_limit: i64) -> (i64, TotalBound) {
        if counted > count_limit {
            (count_limit, TotalBound::MoreThan)
        } else {
            (counted, TotalBound::Exact)
        }
    }
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

#[cfg(test)]
mod total_bound_tests {
    use super::*;

    #[test]
    fn a_count_up_to_the_limit_is_exact() {
        assert_eq!(TotalBound::of_count(0, 50), (0, TotalBound::Exact));
        assert_eq!(TotalBound::of_count(50, 50), (50, TotalBound::Exact));
    }

    #[test]
    fn a_count_past_the_limit_is_more_than_the_limit() {
        assert_eq!(TotalBound::of_count(51, 50), (50, TotalBound::MoreThan));
    }

    #[test]
    fn the_bound_is_sent_as_a_graphql_enum_value() {
        assert_eq!(
            serde_json::to_value(TotalBound::Exact).unwrap(),
            serde_json::json!("EXACT")
        );
        assert_eq!(
            serde_json::to_value(TotalBound::MoreThan).unwrap(),
            serde_json::json!("MORE_THAN")
        );
    }
}
