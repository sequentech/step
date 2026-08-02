// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Result};
use deadpool_postgres::Pool;

/// Escapes a SQL identifier (table name, column name, etc.) by wrapping
/// it in double quotes and escaping any internal double quotes by doubling
/// them. This prevents SQL injection when interpolating identifiers.
pub fn escape_sql_identifier(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

/// Escapes a string value for safe interpolation into a SQL literal
/// (i.e. inside single quotes). Use this for contexts where parameterized
/// queries ($1, $2) are not available, such as COPY TO STDOUT.
///
/// With PostgreSQL's standard_conforming_strings = on (default since 9.1),
/// the only character that needs escaping inside a single-quoted literal
/// is the single quote itself, which is escaped by doubling it.
pub fn escape_sql_literal(value: &str) -> String {
    value.replace('\'', "''")
}

/// Verifies that PostgreSQL has standard_conforming_strings = on, which is
/// required for escape_sql_literal to be a complete defense against SQL
/// injection. Returns an error if the setting is off.
pub async fn assert_standard_conforming_strings(pool: &Pool) -> Result<()> {
    let client = pool.get().await.map_err(|e| {
        anyhow!("Failed to get connection for standard_conforming_strings check: {e}")
    })?;
    let row = client
        .query_one("SHOW standard_conforming_strings", &[])
        .await
        .map_err(|e| anyhow!("Failed to query standard_conforming_strings: {e}"))?;
    let value: &str = row.get(0);
    if value != "on" {
        return Err(anyhow!(
            "PostgreSQL standard_conforming_strings is '{value}', expected 'on'. \
             This is required for SQL literal escaping to be safe."
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identifier_simple() {
        assert_eq!(
            escape_sql_identifier("temp_voters_123"),
            "\"temp_voters_123\""
        );
    }

    #[test]
    fn test_identifier_with_double_quotes() {
        assert_eq!(escape_sql_identifier("table\"name"), "\"table\"\"name\"");
    }

    #[test]
    fn test_no_quotes() {
        assert_eq!(escape_sql_literal("hello"), "hello");
    }

    #[test]
    fn test_single_quote() {
        assert_eq!(escape_sql_literal("d'Arquitectes"), "d''Arquitectes");
    }

    #[test]
    fn test_multiple_quotes() {
        assert_eq!(escape_sql_literal("it's a 'test'"), "it''s a ''test''");
    }

    #[test]
    fn test_empty_string() {
        assert_eq!(escape_sql_literal(""), "");
    }
}
