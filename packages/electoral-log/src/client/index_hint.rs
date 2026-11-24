// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! immudb scans the primary key for `ORDER BY id` and filters every row on its own, even when
//! a secondary index covers the query. These helpers find the board's indexes and name the one
//! a query should scan with `USE INDEX ON (...)`.

use anyhow::{anyhow, Result};
use immudb_rs::{sql_value::Value, Client};
use tracing::warn;

/// Columns whose values belong to a handful of rows: an index range on them is small enough to
/// sort in memory.
const SELECTIVE_COLUMNS: [&str; 2] = ["user_id", "ballot_id"];

/// The primary key column: a query without `ORDER BY` reads rows in its order.
pub const PRIMARY_KEY_COLUMN: &str = "id";

/// A secondary index of a table, as its columns in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableIndex {
    columns: Vec<String>,
}

/// How a query reads the rows of the index it scans.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexScan<'a> {
    /// Counts the rows: any index that narrows the range helps.
    Count,
    /// Reads rows sorted by this column.
    OrderedBy(&'a str),
    /// Reads rows sorted by several columns, which no index gives, so they are sorted after the
    /// scan.
    SortedByColumns,
}

impl<'a> IndexScan<'a> {
    /// The scan of a query sorted by `order_columns`, read in primary key order when there are
    /// none.
    pub fn sorted_by(order_columns: &[&'a str]) -> Self {
        match order_columns {
            [] => IndexScan::OrderedBy(PRIMARY_KEY_COLUMN),
            [column] => IndexScan::OrderedBy(column),
            _ => IndexScan::SortedByColumns,
        }
    }
}

impl TableIndex {
    pub fn new(columns: &[&str]) -> Self {
        TableIndex {
            columns: columns.iter().map(|column| column.to_string()).collect(),
        }
    }

    /// Parses an index name as immudb lists it, `table(column_a,column_b)`.
    pub fn from_name(table: &str, name: &str) -> Option<Self> {
        let columns = name
            .strip_prefix(table)?
            .strip_prefix('(')?
            .strip_suffix(')')?
            .split(',')
            .map(|column| column.trim().to_string())
            .collect::<Vec<String>>();
        if columns.iter().any(|column| column.is_empty()) {
            return None;
        }
        Some(TableIndex { columns })
    }

    pub fn use_index_clause(&self) -> String {
        format!("USE INDEX ON ({})", self.columns.join(", "))
    }
}

/// The secondary indexes of `table` on the client's current database.
pub async fn table_indexes(client: &mut Client, table: &str) -> Result<Vec<TableIndex>> {
    let sql = format!("SELECT * FROM INDEXES('{table}')");
    let response = client.sql_query(&sql, vec![]).await?;
    let mut indexes = vec![];
    // Each row is (table, name, unique, primary).
    for row in &response.get_ref().rows {
        let value = |position: usize| {
            row.values
                .get(position)
                .and_then(|value| value.value.as_ref())
        };
        let Some(Value::S(name)) = value(1) else {
            return Err(anyhow!("Index without a name in table {table}: {row:?}"));
        };
        if let Some(Value::B(true)) = value(3) {
            continue;
        }
        match TableIndex::from_name(table, name) {
            Some(index) => indexes.push(index),
            None => warn!("Skipping index {name} of table {table}: its columns can't be read"),
        }
    }
    Ok(indexes)
}

/// The index whose range matches the most of `equal_columns`, the columns the query fixes with
/// `=`, if scanning it beats scanning the primary key.
///
/// A query that reads rows in primary key order can stop at its limit scanning the primary key,
/// so an index only beats it when it gives that order too, or when its range is a voter's
/// handful of rows. Rows in any other order are all sorted after the scan, so the narrowest
/// range wins.
pub fn choose_index<'a>(
    indexes: &'a [TableIndex],
    equal_columns: &[&str],
    scan: IndexScan,
) -> Option<&'a TableIndex> {
    let mut chosen: Option<(&TableIndex, (bool, bool, usize))> = None;
    for index in indexes {
        let fixed = index
            .columns
            .iter()
            .take_while(|column| equal_columns.contains(&column.as_str()))
            .count();
        if fixed == 0 {
            continue;
        }
        let selective = index.columns[..fixed]
            .iter()
            .any(|column| SELECTIVE_COLUMNS.contains(&column.as_str()));
        let in_order = match scan {
            IndexScan::Count => true,
            IndexScan::OrderedBy(order_column) => {
                index.columns.get(fixed).map(String::as_str) == Some(order_column)
            }
            IndexScan::SortedByColumns => false,
        };
        let primary_key_order = scan == IndexScan::OrderedBy(PRIMARY_KEY_COLUMN);
        if primary_key_order && !selective && !in_order {
            continue;
        }
        let score = (selective, in_order, fixed);
        if chosen.map_or(true, |(_, best)| score > best) {
            chosen = Some((index, score));
        }
    }
    chosen.map(|(index, _)| index)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TABLE: &str = "electoral_log_messages";

    fn board_indexes() -> Vec<TableIndex> {
        vec![
            TableIndex::new(&[
                "statement_kind",
                "user_id",
                "ballot_id",
                "election_id",
                "id",
            ]),
            TableIndex::new(&[
                "statement_kind",
                "user_id",
                "ballot_id",
                "election_id",
                "statement_timestamp",
            ]),
            TableIndex::new(&["statement_kind", "election_id", "id"]),
            TableIndex::new(&["statement_kind", "election_id", "statement_timestamp"]),
            TableIndex::new(&["user_id", "election_id", "area_id", "id"]),
            TableIndex::new(&["election_id", "area_id", "id"]),
            TableIndex::new(&["area_id", "id"]),
        ]
    }

    fn chosen(equal_columns: &[&str], scan: IndexScan) -> Option<String> {
        let indexes = board_indexes();
        choose_index(&indexes, equal_columns, scan).map(TableIndex::use_index_clause)
    }

    #[test]
    fn parses_the_index_names_immudb_lists() {
        assert_eq!(
            TableIndex::from_name(
                TABLE,
                "electoral_log_messages(statement_kind,election_id,id)"
            ),
            Some(TableIndex::new(&["statement_kind", "election_id", "id"]))
        );
        assert_eq!(
            TableIndex::from_name(TABLE, "electoral_log_messages(id)"),
            Some(TableIndex::new(&["id"]))
        );
    }

    #[test]
    fn rejects_names_of_other_tables_or_without_columns() {
        for name in [
            "electoral_log_delivery_receipts(delivery_id)",
            "electoral_log_messages",
            "electoral_log_messages()",
            "electoral_log_messages(a,,b)",
            "electoral_log_messages(id",
        ] {
            assert_eq!(TableIndex::from_name(TABLE, name), None, "{name}");
        }
    }

    #[test]
    fn the_use_index_clause_lists_the_columns_in_order() {
        assert_eq!(
            TableIndex::new(&["statement_kind", "election_id", "id"]).use_index_clause(),
            "USE INDEX ON (statement_kind, election_id, id)"
        );
    }

    #[test]
    fn an_elections_cast_votes_are_read_newest_first_from_their_index() {
        assert_eq!(
            chosen(
                &["statement_kind", "election_id"],
                IndexScan::OrderedBy("id")
            ),
            Some("USE INDEX ON (statement_kind, election_id, id)".to_string())
        );
        assert_eq!(
            chosen(
                &["statement_kind", "election_id"],
                IndexScan::OrderedBy("statement_timestamp")
            ),
            Some("USE INDEX ON (statement_kind, election_id, statement_timestamp)".to_string())
        );
    }

    #[test]
    fn an_elections_cast_votes_are_counted_on_their_index() {
        assert_eq!(
            chosen(&["statement_kind", "election_id"], IndexScan::Count),
            Some("USE INDEX ON (statement_kind, election_id, id)".to_string())
        );
    }

    #[test]
    fn a_voters_cast_votes_are_read_from_the_range_of_that_voter() {
        assert_eq!(
            chosen(
                &["statement_kind", "user_id", "election_id"],
                IndexScan::OrderedBy("id")
            ),
            Some("USE INDEX ON (statement_kind, user_id, ballot_id, election_id, id)".to_string())
        );
    }

    #[test]
    fn a_voters_entries_are_read_from_the_range_of_that_voter() {
        for scan in [
            IndexScan::OrderedBy("id"),
            IndexScan::OrderedBy("created"),
            IndexScan::SortedByColumns,
            IndexScan::Count,
        ] {
            assert_eq!(
                chosen(&["user_id"], scan),
                Some("USE INDEX ON (user_id, election_id, area_id, id)".to_string()),
                "{scan:?}"
            );
        }
    }

    #[test]
    fn a_wide_range_out_of_id_order_is_left_to_the_primary_key() {
        assert_eq!(
            chosen(&["statement_kind"], IndexScan::OrderedBy("id")),
            None
        );
        assert_eq!(
            chosen(&["area_id"], IndexScan::OrderedBy("id")),
            Some("USE INDEX ON (area_id, id)".to_string())
        );
    }

    #[test]
    fn rows_sorted_after_the_scan_are_read_from_the_narrowest_range() {
        assert_eq!(
            chosen(
                &["statement_kind", "election_id"],
                IndexScan::OrderedBy("ballot_id")
            ),
            Some("USE INDEX ON (statement_kind, election_id, id)".to_string())
        );
        assert_eq!(
            chosen(
                &["statement_kind", "election_id"],
                IndexScan::SortedByColumns
            ),
            Some("USE INDEX ON (statement_kind, election_id, id)".to_string())
        );
    }

    #[test]
    fn a_wide_range_is_still_counted_on_its_index() {
        assert_eq!(
            chosen(&["statement_kind"], IndexScan::Count),
            Some("USE INDEX ON (statement_kind, user_id, ballot_id, election_id, id)".to_string())
        );
    }

    #[test]
    fn no_index_is_used_without_an_equal_column_leading_one() {
        assert_eq!(chosen(&[], IndexScan::OrderedBy("id")), None);
        assert_eq!(chosen(&[], IndexScan::Count), None);
        assert_eq!(chosen(&["ballot_id"], IndexScan::Count), None);
    }

    #[test]
    fn only_the_boards_own_indexes_are_chosen() {
        let old_board = vec![TableIndex::new(&["area_id", "id"])];
        assert_eq!(
            choose_index(
                &old_board,
                &["statement_kind", "election_id"],
                IndexScan::Count
            ),
            None
        );
        assert_eq!(choose_index(&[], &["user_id"], IndexScan::Count), None);
    }

    #[test]
    fn an_unsorted_query_is_read_in_primary_key_order() {
        assert_eq!(IndexScan::sorted_by(&[]), IndexScan::OrderedBy("id"));
    }

    #[test]
    fn a_query_sorted_by_one_column_is_read_in_its_order() {
        assert_eq!(
            IndexScan::sorted_by(&["statement_timestamp"]),
            IndexScan::OrderedBy("statement_timestamp")
        );
    }

    #[test]
    fn a_query_sorted_by_several_columns_is_sorted_after_the_scan() {
        assert_eq!(
            IndexScan::sorted_by(&["created", "id"]),
            IndexScan::SortedByColumns
        );
    }
}
