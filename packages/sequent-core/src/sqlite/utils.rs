// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{bail, Result};
use ordered_float::NotNan;
use rusqlite::Transaction;
use serde_json::{to_string, Value};
use std::collections::HashSet;

pub fn opt_json(opt: &Option<Value>) -> Option<String> {
    opt.as_ref().and_then(|v| to_string(v).ok())
}

pub fn opt_f64(opt: &Option<NotNan<f64>>) -> Option<f64> {
    opt.map(|n| n.into_inner())
}

pub fn ensure_blank_vote_columns(
    transaction: &Transaction<'_>,
    table: &str,
) -> Result<()> {
    if !matches!(table, "results_contest" | "results_area_contest") {
        bail!("Unsupported results table: {table}");
    }

    let mut columns: HashSet<String> = {
        let mut statement =
            transaction.prepare(&format!("PRAGMA table_info(\"{table}\")"))?;
        let rows = statement.query_map([], |row| row.get::<_, String>(1))?;
        let columns = rows.collect::<rusqlite::Result<_>>()?;
        columns
    };

    for (old_name, new_name) in [
        ("blank_votes", "total_blank_votes"),
        ("blank_votes_percent", "total_blank_votes_percent"),
    ] {
        if columns.contains(old_name) && !columns.contains(new_name) {
            transaction.execute_batch(&format!(
                "ALTER TABLE \"{table}\" RENAME COLUMN \"{old_name}\" TO \"{new_name}\";"
            ))?;
            columns.remove(old_name);
            columns.insert(new_name.to_string());
        }
    }

    for (column, data_type) in [
        ("explicit_blank_votes", "INTEGER"),
        ("implicit_blank_votes", "INTEGER"),
        ("explicit_blank_votes_percent", "REAL"),
        ("implicit_blank_votes_percent", "REAL"),
    ] {
        if !columns.contains(column) {
            transaction.execute_batch(&format!(
                "ALTER TABLE \"{table}\" ADD COLUMN \"{column}\" {data_type};"
            ))?;
            columns.insert(column.to_string());
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn upgrades_legacy_blank_vote_columns_without_losing_totals() {
        let mut connection = Connection::open_in_memory().unwrap();
        let transaction = connection.transaction().unwrap();

        for table in ["results_contest", "results_area_contest"] {
            transaction
                .execute_batch(&format!(
                    "CREATE TABLE \"{table}\" (
                        id TEXT PRIMARY KEY,
                        blank_votes INTEGER,
                        blank_votes_percent REAL
                    );
                    INSERT INTO \"{table}\" VALUES ('result', 7, 0.7);"
                ))
                .unwrap();

            ensure_blank_vote_columns(&transaction, table).unwrap();

            let values: (i64, f64) = transaction
                .query_row(
                    &format!(
                        "SELECT total_blank_votes, total_blank_votes_percent FROM \"{table}\""
                    ),
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .unwrap();
            assert_eq!(values, (7, 0.7));

            let columns: HashSet<String> = transaction
                .prepare(&format!("PRAGMA table_info(\"{table}\")"))
                .unwrap()
                .query_map([], |row| row.get::<_, String>(1))
                .unwrap()
                .collect::<rusqlite::Result<_>>()
                .unwrap();
            for expected in [
                "total_blank_votes",
                "explicit_blank_votes",
                "implicit_blank_votes",
                "total_blank_votes_percent",
                "explicit_blank_votes_percent",
                "implicit_blank_votes_percent",
            ] {
                assert!(
                    columns.contains(expected),
                    "missing {expected} in {table}"
                );
            }
        }
    }
}
