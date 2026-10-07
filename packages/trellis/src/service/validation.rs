use anyhow::Result;
use deadpool_postgres::Object as PooledConnection;

/// Validation result for a single source configuration
#[derive(Debug, Clone)]
pub struct SourceValidation {
    /// Name of the source table being validated
    pub source_table: String,
    /// Name of the log this source belongs to
    pub log_name: String,
    /// Whether the source table exists in the database
    pub table_exists: bool,
    /// Validation result for the ID column (if present)
    pub id_column_valid: Option<ColumnValidation>,
    /// Validation result for the hash column (if present)
    pub hash_column_valid: Option<ColumnValidation>,
    /// Validation result for the timestamp column (if present)
    pub timestamp_column_valid: Option<ColumnValidation>,
}

/// Validation result for a single column
#[derive(Debug, Clone)]
pub struct ColumnValidation {
    /// Name of the column being validated
    pub column_name: String,
    /// Whether the column exists in the table
    pub exists: bool,
    /// The actual `PostgreSQL` data type of the column (if it exists)
    pub data_type: Option<String>,
    /// The expected `PostgreSQL` data type for this column
    pub expected_type: String,
    /// Whether the actual type matches the expected type
    pub type_matches: bool,
    /// Whether the column has a uniqueness constraint (PRIMARY KEY or UNIQUE) (only checked for id columns)
    pub unique: Option<bool>,
}

/// Validation result for a single log
#[derive(Debug, Clone)]
pub struct LogValidation {
    /// Name of the log being validated
    pub log_name: String,
    /// Whether the log is enabled in the configuration
    pub enabled: bool,
    /// Validation results for all sources belonging to this log
    pub sources: Vec<SourceValidation>,
}

impl SourceValidation {
    /// Checks if this source configuration is valid
    ///
    /// A source is valid if:
    /// - The table exists
    /// - The ID column exists, has the correct type, and is unique
    /// - The hash column exists and has the correct type
    /// - The timestamp column (if required) exists and has the correct type
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.table_exists
            && self
                .id_column_valid
                .as_ref()
                .is_some_and(|v| v.type_matches && v.unique == Some(true))
            && self
                .hash_column_valid
                .as_ref()
                .is_some_and(|v| v.type_matches)
            && self
                .timestamp_column_valid
                .as_ref()
                .is_none_or(|v| v.type_matches) // Optional, so true if None
    }

    /// Returns a list of human-readable validation error messages
    ///
    /// If the source is valid, returns an empty vector.
    #[must_use]
    pub fn errors(&self) -> Vec<String> {
        let mut errors = Vec::new();

        if !self.table_exists {
            errors.push(format!("Table '{}' does not exist", self.source_table));
            return errors; // If table doesn't exist, other checks are meaningless
        }

        if let Some(ref col) = self.id_column_valid {
            if !col.exists {
                errors.push(format!("ID column '{}' does not exist", col.column_name));
            } else if !col.type_matches {
                errors.push(format!(
                    "ID column '{}' has type '{}' but expected '{}'",
                    col.column_name,
                    col.data_type.as_ref().unwrap_or(&"unknown".to_string()),
                    col.expected_type
                ));
            } else if matches!(col.unique, Some(false)) {
                errors.push(format!(
                    "ID column '{}' MUST have a uniqueness constraint (PRIMARY KEY or UNIQUE)",
                    col.column_name
                ));
            }
        }

        if let Some(ref col) = self.hash_column_valid {
            if !col.exists {
                errors.push(format!("Hash column '{}' does not exist", col.column_name));
            } else if !col.type_matches {
                errors.push(format!(
                    "Hash column '{}' has type '{}' but expected '{}'",
                    col.column_name,
                    col.data_type.as_ref().unwrap_or(&"unknown".to_string()),
                    col.expected_type
                ));
            }
        }

        if let Some(ref col) = self.timestamp_column_valid {
            if !col.exists {
                errors.push(format!(
                    "Timestamp column '{}' does not exist",
                    col.column_name
                ));
            } else if !col.type_matches {
                errors.push(format!(
                    "Timestamp column '{}' has type '{}' but expected '{}'",
                    col.column_name,
                    col.data_type.as_ref().unwrap_or(&"unknown".to_string()),
                    col.expected_type
                ));
            }
        }

        errors
    }
}

impl LogValidation {
    /// Checks if this log configuration is valid
    ///
    /// A log is valid if it has at least one source and all sources are valid.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        !self.sources.is_empty() && self.sources.iter().all(SourceValidation::is_valid)
    }

    /// Returns the number of valid sources in this log
    #[must_use]
    pub fn valid_source_count(&self) -> usize {
        self.sources.iter().filter(|s| s.is_valid()).count()
    }

    /// Returns the number of invalid sources in this log
    #[must_use]
    pub fn invalid_source_count(&self) -> usize {
        self.sources.iter().filter(|s| !s.is_valid()).count()
    }
}

/// Validates all logs and their source configurations
///
/// # Errors
///
/// Returns an error if any database queries fail while loading logs or validating sources.
pub async fn validate_all_logs(conn: &PooledConnection) -> Result<Vec<LogValidation>> {
    // Get all logs
    let log_rows = conn
        .query(
            "SELECT log_name, enabled FROM verification_logs ORDER BY log_name",
            &[],
        )
        .await?;

    let mut log_validations = Vec::new();

    for log_row in log_rows {
        let log_name: String = log_row.get(0);
        let enabled: bool = log_row.get(1);

        // Get all sources for this log
        let source_rows = conn
            .query(
                "SELECT source_table, hash_column, id_column, timestamp_column 
                 FROM verification_sources 
                 WHERE log_name = $1 
                 ORDER BY source_table",
                &[&log_name],
            )
            .await?;

        let mut source_validations = Vec::new();

        for source_row in source_rows {
            let source_table: String = source_row.get(0);
            let hash_column: String = source_row.get(1);
            let id_column: String = source_row.get(2);
            let timestamp_column: Option<String> = source_row.get(3);

            let validation = validate_source(
                conn,
                &log_name,
                &source_table,
                &id_column,
                &hash_column,
                timestamp_column.as_deref(),
            )
            .await?;

            source_validations.push(validation);
        }

        log_validations.push(LogValidation {
            log_name,
            enabled,
            sources: source_validations,
        });
    }

    Ok(log_validations)
}

/// Validates a single source configuration
pub(crate) async fn validate_source(
    conn: &PooledConnection,
    log_name: &str,
    source_table: &str,
    id_column: &str,
    hash_column: &str,
    timestamp_column: Option<&str>,
) -> Result<SourceValidation> {
    // Check if table exists
    let table_exists: bool = conn
        .query_one(
            "SELECT EXISTS (
                SELECT FROM information_schema.tables 
                WHERE table_schema = 'public' AND table_name = $1
            )",
            &[&source_table],
        )
        .await?
        .get(0);

    if !table_exists {
        return Ok(SourceValidation {
            source_table: source_table.to_string(),
            log_name: log_name.to_string(),
            table_exists: false,
            id_column_valid: None,
            hash_column_valid: None,
            timestamp_column_valid: None,
        });
    }

    // Validate columns
    let id_col_validation = validate_column_with_constraints(
        conn,
        source_table,
        id_column,
        &["bigint", "integer", "smallint"],
        true, // Check uniqueness on id column
    )
    .await?;
    let hash_col_validation = validate_column_with_constraints(
        conn,
        source_table,
        hash_column,
        &["bytea"],
        false, // No uniqueness check for hash column
    )
    .await?;
    let timestamp_col_validation = if let Some(ts_col) = timestamp_column {
        Some(
            validate_column_with_constraints(
                conn,
                source_table,
                ts_col,
                &["timestamp without time zone", "timestamp with time zone"],
                false, // No uniqueness check for timestamp column
            )
            .await?,
        )
    } else {
        None
    };

    Ok(SourceValidation {
        source_table: source_table.to_string(),
        log_name: log_name.to_string(),
        table_exists: true,
        id_column_valid: Some(id_col_validation),
        hash_column_valid: Some(hash_col_validation),
        timestamp_column_valid: timestamp_col_validation,
    })
}

/// Validates a single column exists and has acceptable type
///
/// If `check_unique` is true, verifies the column has a uniqueness constraint.
async fn validate_column_with_constraints(
    conn: &PooledConnection,
    table_name: &str,
    column_name: &str,
    expected_types: &[&str],
    check_unique: bool,
) -> Result<ColumnValidation> {
    // Query column information
    let row_opt = conn
        .query_opt(
            "SELECT data_type, udt_name 
             FROM information_schema.columns 
             WHERE table_schema = 'public' 
               AND table_name = $1 
               AND column_name = $2",
            &[&table_name, &column_name],
        )
        .await?;

    if let Some(row) = row_opt {
        let data_type: String = row.get(0);
        let udt_name: String = row.get(1);

        // Use udt_name for more precise type matching (handles domains, custom types)
        let actual_type = if data_type == "USER-DEFINED" {
            udt_name.clone()
        } else {
            data_type.clone()
        };

        let type_matches = expected_types
            .iter()
            .any(|&expected| actual_type.eq_ignore_ascii_case(expected));

        // Check for uniqueness constraint if requested
        let unique = if check_unique {
            Some(check_column_unique(conn, table_name, column_name).await?)
        } else {
            None
        };

        Ok(ColumnValidation {
            column_name: column_name.to_string(),
            exists: true,
            data_type: Some(actual_type.clone()),
            expected_type: expected_types.join(" or "),
            type_matches,
            unique,
        })
    } else {
        Ok(ColumnValidation {
            column_name: column_name.to_string(),
            exists: false,
            data_type: None,
            expected_type: expected_types.join(" or "),
            type_matches: false,
            unique: None,
        })
    }
}

/// Checks if a column has a uniqueness constraint (PRIMARY KEY or UNIQUE)
///
/// Returns true if the column has a PRIMARY KEY or UNIQUE constraint on it alone
async fn check_column_unique(
    conn: &PooledConnection,
    table_name: &str,
    column_name: &str,
) -> Result<bool> {
    // Query pg_constraint to check for PRIMARY KEY or UNIQUE constraints
    let unique: bool = conn
        .query_one(
            "SELECT EXISTS (
                SELECT 1
                FROM pg_constraint con
                JOIN pg_class rel ON rel.oid = con.conrelid
                JOIN pg_namespace nsp ON nsp.oid = rel.relnamespace
                WHERE nsp.nspname = 'public'
                  AND rel.relname = $1
                  AND con.contype IN ('p', 'u')
                  AND con.conkey = ARRAY[(SELECT attnum FROM pg_attribute WHERE attrelid = rel.oid AND attname = $2)]
            )",
            &[&table_name, &column_name],
        )
        .await?
        .get(0);

    Ok(unique)
}

/// Pretty-prints validation results
#[allow(clippy::print_stdout)]
#[allow(clippy::arithmetic_side_effects)]
pub fn print_validation_report(validations: &[LogValidation]) {
    println!("\n╔════════════════════════════════════════════════════════════════╗");
    println!("║           DATABASE CONFIGURATION VALIDATION REPORT            ║");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    if validations.is_empty() {
        println!("⚠️  No logs configured in verification_logs table\n");
        return;
    }

    let mut total_logs = 0;
    let mut valid_logs = 0;
    let mut total_sources = 0;
    let mut valid_sources = 0;

    for log_validation in validations {
        total_logs += 1;
        total_sources += log_validation.sources.len();
        valid_sources += log_validation.valid_source_count();

        let status_icon = if log_validation.is_valid() {
            valid_logs += 1;
            "✅"
        } else {
            "❌"
        };

        let enabled_text = if log_validation.enabled {
            "ENABLED"
        } else {
            "DISABLED"
        };

        println!(
            "{} Log: '{}' [{}]",
            status_icon, log_validation.log_name, enabled_text
        );

        if log_validation.sources.is_empty() {
            println!("   ⚠️  No sources configured");
        } else {
            println!(
                "   Sources: {} total, {} valid, {} invalid",
                log_validation.sources.len(),
                log_validation.valid_source_count(),
                log_validation.invalid_source_count()
            );

            for source in &log_validation.sources {
                let source_status = if source.is_valid() { "✅" } else { "❌" };
                println!("   {} Source: '{}'", source_status, source.source_table);

                if !source.is_valid() {
                    for error in source.errors() {
                        println!("      ⚠️  {error}");
                    }
                }
            }
        }

        println!();
    }

    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║                            SUMMARY                             ║");
    println!("╚════════════════════════════════════════════════════════════════╝");
    println!("Total logs: {total_logs}");
    println!("Valid logs: {valid_logs} / {total_logs}");
    println!("Total sources: {total_sources}");
    println!("Valid sources: {valid_sources} / {total_sources}");
    println!();

    if valid_logs == total_logs && valid_sources == total_sources {
        println!("✅ All configurations are valid!\n");
    } else {
        println!("❌ Some configurations have errors. Please review above.\n");
    }
}
