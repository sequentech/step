// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use ordered_float::NotNan;
use rusqlite::Transaction;
use serde_json::{to_string, Value};

pub fn opt_json(opt: &Option<Value>) -> Option<String> {
    opt.as_ref().and_then(|v| to_string(v).ok())
}

pub fn opt_f64(opt: &Option<NotNan<f64>>) -> Option<f64> {
    opt.map(|n| n.into_inner())
}

/// Adds `column` to `table` if it doesn't already exist.
///
/// `CREATE TABLE IF NOT EXISTS` is a no-op against a table that already
/// exists (e.g. a `results.db` copied forward from a prior report), so
/// columns added to the schema after a table's first release need to be
/// migrated in explicitly. `table`/`column`/`column_def` must be trusted,
/// caller-controlled literals: they are interpolated directly into SQL.
pub fn ensure_column(
    sqlite_transaction: &Transaction<'_>,
    table: &str,
    column: &str,
    column_def: &str,
) -> Result<()> {
    let existing_columns = sqlite_transaction
        .prepare(&format!("PRAGMA table_info({table})"))?
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<rusqlite::Result<Vec<String>>>()?;

    let column_exists = existing_columns
        .iter()
        .any(|existing_column| existing_column == column);

    if !column_exists {
        sqlite_transaction.execute_batch(&format!(
            "ALTER TABLE {table} ADD COLUMN {column} {column_def};"
        ))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn is_a_no_op_when_the_column_already_exists() {
        let mut connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch("CREATE TABLE t (id INTEGER, value TEXT);")
            .unwrap();

        let transaction = connection.transaction().unwrap();
        ensure_column(&transaction, "t", "value", "TEXT").unwrap();
        transaction.commit().unwrap();

        let columns: Vec<String> = connection
            .prepare("PRAGMA table_info(t)")
            .unwrap()
            .query_map([], |row| row.get(1))
            .unwrap()
            .collect::<rusqlite::Result<Vec<String>>>()
            .unwrap();
        assert_eq!(columns, vec!["id".to_string(), "value".to_string()]);
    }

    #[test]
    fn errors_on_a_table_that_does_not_exist() {
        let mut connection = Connection::open_in_memory().unwrap();
        let transaction = connection.transaction().unwrap();

        let result =
            ensure_column(&transaction, "missing_table", "value", "TEXT");

        assert!(result.is_err());
    }
}
