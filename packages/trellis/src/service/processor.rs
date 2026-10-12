use anyhow::Result;
use deadpool_postgres::Object as PooledConnection;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::LeafHash;
use crate::service::metrics::LogMetrics;
use crate::service::metrics::u64_millis;
use crate::service::state::AppState;

// ============================================================================
// Log Name Validation and Table Naming
// ============================================================================

/// Validates that a log name contains only allowed characters.
///
/// Log names must match the pattern `[a-z0-9_]+` to ensure they can be safely
/// used as `PostgreSQL` table name suffixes without quoting or escaping.
///
/// # Returns
/// - `Ok(())` if the log name is valid
/// - `Err` with description if invalid
///
/// # Errors
/// Returns an error if the log name is empty, contains invalid characters,
/// exceeds 52 bytes, or starts with a digit.
pub fn validate_log_name(log_name: &str) -> Result<()> {
    if log_name.is_empty() {
        return Err(anyhow::anyhow!("Log name cannot be empty"));
    }

    anyhow::ensure!(log_name.len() <= 52, "Log name exceeds 52 bytes");

    if !log_name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    {
        return Err(anyhow::anyhow!(
            "Log name '{log_name}' contains invalid characters. Only lowercase letters, digits, and underscores are allowed."
        ));
    }

    // Cannot start with a digit (PostgreSQL identifier rule)
    if log_name.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        return Err(anyhow::anyhow!(
            "Log name '{log_name}' cannot start with a digit"
        ));
    }

    Ok(())
}

/// Returns the `PostgreSQL` table name for a log's merkle entries.
///
/// Each log has its own dedicated table named `merkle_log_{log_name}`.
/// This provides isolation between logs and enables potential parallelization.
///
/// # Panics
/// Panics if `log_name` is invalid. Callers should validate first with `validate_log_name`.
fn merkle_log_table_name(log_name: &str) -> String {
    debug_assert!(
        validate_log_name(log_name).is_ok(),
        "Invalid log name: {log_name}"
    );
    format!("merkle_log_{log_name}")
}

/// Creates the per-log merkle table if it doesn't exist.
///
/// Each log gets its own table with schema:
/// - `id BIGSERIAL PRIMARY KEY` - Sequential ID for this log (monotonic)
/// - `source_table TEXT NOT NULL` - Which source table this entry came from
/// - `source_id BIGINT NOT NULL` - The ID in the source table
/// - `leaf_hash BYTEA NOT NULL` - SHA256 hash of the entry
/// - `processed_at TIMESTAMPTZ` - When the entry was added
/// - `UNIQUE (source_table, source_id)` - Defense in depth against duplicates
///
/// The unique source identity is the durable receipt used by reconciliation.
/// Source IDs need not be allocated in commit order.
async fn ensure_merkle_log_table_exists(conn: &PooledConnection, log_name: &str) -> Result<()> {
    let table_name = merkle_log_table_name(log_name);

    // Use IF NOT EXISTS to make this idempotent
    // SET client_min_messages suppresses the NOTICE when table already exists
    let create_table = format!(
        r"
        SET client_min_messages = warning;
        CREATE TABLE IF NOT EXISTS {table_name} (
            id BIGSERIAL PRIMARY KEY,
            source_table TEXT NOT NULL,
            source_id BIGINT NOT NULL,
            leaf_hash BYTEA NOT NULL,
            processed_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
            UNIQUE (source_table, source_id)
        );
        SET client_min_messages = notice;
        "
    );

    conn.batch_execute(&create_table).await?;

    Ok(())
}

// ============================================================================
// Batch Statistics and Architectural Notes
// ============================================================================

/// Statistics from a batch processing operation
#[derive(Debug, Clone)]
pub struct BatchStats {
    /// Number of rows copied from source tables to `merkle_log_{log_name}`
    pub rows_copied: u64,
    /// Time spent on the INSERT operation (ms) - includes CTE query, sort, and insert
    pub insert_ms: u64,
}

/// CRITICAL ARCHITECTURAL NOTE: `merkle_log_{log_name}` is GROUND TRUTH
///
/// The ordering committed to `merkle_log_{log_name}` cannot be reconstructed deterministically from source tables alone.
/// This is because:
///
/// 1. **Batch boundaries matter**: Entries are sorted within each batch, not globally. The same entry might
///    end up at different positions depending on when batches run.
///
///    Example:
///
///    Entry X (no timestamp) with 9 timestamped entries in batch 1 → X at position 10
///    Same 20 entries in one batch → X at position 20 (after all 19 timestamped entries)
///
/// 2. **Late arrivals**: An entry with timestamp T1 might arrive AFTER entries with T2, T3 (T2 > T1).
///    Once T2, T3 are committed to the merkle tree, we cannot retroactively insert T1 before them.
///
/// 3. **Ordering is a point-in-time commitment**: When processing a batch, we commit to "these are all
///    the entries we know about right now, in this order." Future entries might logically belong earlier,
///    but they weren't available yet.
///
/// Therefore:
/// - Startup rebuild from `merkle_log_{log_name}` IS deterministic (correct behavior)
/// - Rebuild from source tables is NOT deterministic (would produce different merkle roots)
/// - `merkle_log_{log_name}` must be backed up and preserved for disaster recovery
/// - This is correct behavior for append-only transparency logs
///
/// Configuration for a source table that feeds into the merkle log table
#[derive(Debug, Clone)]
pub struct SourceConfig {
    /// The name of the source table in the database
    pub table_name: String,
    /// The column containing the pre-computed hash value
    pub hash_column: String,
    /// The unique ID column (must be unique per table, e.g., primary key)
    pub id_column: String,
    /// Optional timestamp column for chronological ordering across tables
    pub timestamp_column: Option<String>,
    /// The log this source belongs to
    pub log_name: String,
}

impl SourceConfig {
    /// Creates a new source configuration
    pub fn new(
        table_name: impl Into<String>,
        hash_column: impl Into<String>,
        id_column: impl Into<String>,
        timestamp_column: Option<String>,
        log_name: impl Into<String>,
    ) -> Self {
        Self {
            table_name: table_name.into(),
            hash_column: hash_column.into(),
            id_column: id_column.into(),
            timestamp_column,
            log_name: log_name.into(),
        }
    }
}

/// Batch processor entry point
///
/// Runs the batch processing loop that
/// 1) Retrieves enabled logs
/// 2) Processes each log (`process_log`)
/// 3) Updates global metrics
///
/// This loop runs until the processor state is set to Stopping via an admin endpoint.
///
/// Note: each log is processed serially, but errors are independently handled: an error
/// in one log does not affect others.
pub async fn run_batch_processor(app_state: AppState) {
    let batch_size: u32 = std::env::var("TRELLIS_BATCH_SIZE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(30000);
    let interval = std::env::var("TRELLIS_BATCH_INTERVAL_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);
    let interval_d = std::time::Duration::from_secs(interval);

    tracing::info!(
        "Batch processor started with batch size = '{batch_size}' and interval = '{interval_d:?}'"
    );

    loop {
        // Check processor state for pause/stop control
        let state_value = app_state
            .processor_state
            .load(std::sync::atomic::Ordering::Relaxed);
        match state_value {
            1 => {
                // Paused - sleep and continue
                tracing::info!("Batch processor paused");
                tokio::time::sleep(interval_d).await;
                continue;
            }
            2 => {
                // Stopping - break out of loop
                tracing::info!("Batch processor stopping");
                break;
            }
            _ => {
                // Running - continue normally
            }
        }

        // Load all enabled logs from the database
        let conn = match app_state.db_pool.get().await {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(error = %e, "Failed to get connection for loading logs");
                tokio::time::sleep(interval_d).await;
                continue;
            }
        };

        let logs_result = load_enabled_logs(&conn).await;
        // Release connection before processing
        drop(conn);

        let log_names = match logs_result {
            Ok(names) => names,
            Err(e) => {
                tracing::warn!(error = %e, "Failed to load enabled logs");
                tokio::time::sleep(interval_d).await;
                continue;
            }
        };

        // Remove any logs from in-memory state that are no longer enabled
        let in_memory = app_state
            .merkle_states
            .iter()
            .map(|entry| entry.key().clone())
            .collect::<Vec<_>>();
        for log in in_memory {
            if !log_names.contains(&log) {
                tracing::info!(log_name = %log, "Removing disabled log '{log}' from memory");
                if let Some(tree) = app_state.merkle_states.remove(&log) {
                    let bytes = LogMetrics::tree_size_bytes(tree.1.read().tree.len());
                    tracing::info!(log_name = %log, "Removed log from memory ({bytes} bytes freed)");
                    drop(tree);
                }
            }
        }

        let now = std::time::Instant::now();
        // Process each log independently
        for log_name in log_names.clone() {
            if let Err(e) = process_log(&app_state, &log_name, batch_size).await
                && !e
                    .to_string()
                    .contains("Another processing batch is running")
            {
                tracing::warn!(log_name, error = ?e, "Error processing log");
            }
        }
        let elapsed = now.elapsed();

        // Update global metrics
        app_state.metrics.update_global_metrics(|global| {
            global.record_cycle(
                u64_millis(elapsed.as_millis()),
                log_names.len(),
                &interval_d,
            );
        });

        // Wait for next interval
        tokio::time::sleep(interval_d).await;
    }

    tracing::info!("Batch processor stopped");
}

/// Process a single log: copy rows from source tables to `merkle_log_{log_name}` and update tree
///
/// This function
/// 1) Ensures merkle state exists for the log
/// 2) Copies pending entries from source tables into `merkle_log_{log_name}`
/// 3) Fetches new entries from `merkle_log_{log_name}` and updates the merkle tree
/// 4) Updates metrics for the log
///
/// Note: a log's merkle tree is not updated unless the entries have been successfully persisted
/// to `merkle_log_{log_name}`, ensuring that published log information can always be reconstructed
/// from persisted data.
async fn process_log(app_state: &AppState, log_name: &str, batch_size: u32) -> Result<()> {
    validate_log_name(log_name)?;
    // Time the entire processing cycle
    let total_start = std::time::Instant::now();

    // Ensure merkle state exists for this log (create if first time)
    if !app_state.merkle_states.contains_key(log_name) {
        app_state.merkle_states.insert(
            log_name.to_string(),
            std::sync::Arc::new(parking_lot::RwLock::new(crate::service::MerkleState::new())),
        );
    }

    let merkle_state_arc = app_state
        .merkle_states
        .get(log_name)
        .expect("merkle state should exist")
        .clone();

    // Get the current merkle state and last processed ID
    let current_last_id = merkle_state_arc.read().last_processed_id;

    // Time the copy from source tables to per-log merkle table
    let copy_start = std::time::Instant::now();

    // Copy pending entries into merkle_log_{log_name}

    let batch_stats = copy_source_rows(app_state, log_name, batch_size).await?;

    let copy_duration = copy_start.elapsed();

    // Always reconcile persisted leaves, including after failures and on replicas.
    // Time the retrieval from merkle_log_{log_name}
    let fetch_start = std::time::Instant::now();

    // Query everything after our last known processed ID for this log
    let conn = app_state.db_pool.get().await?;
    let table_name = merkle_log_table_name(log_name);
    let fetch_query =
        format!("SELECT id, leaf_hash FROM {table_name} WHERE id > $1 ORDER BY id LIMIT $2");
    let fetch_result = conn
        .query(&fetch_query, &[&current_last_id, &i64::from(batch_size)])
        .await;

    let fetch_duration = fetch_start.elapsed();

    let rows = fetch_result?;

    let leaves_added = rows.len();
    // Start timing the tree construction
    let tree_start = std::time::Instant::now();

    // Update the merkle state and maps
    let mut merkle_state = merkle_state_arc.write();

    for row in rows {
        let id: i64 = row.get("id");
        let hash: Vec<u8> = row.get("leaf_hash");
        let leaf_hash = LeafHash::new(hash);

        // Another caller may have advanced this tree while the query was in flight.
        if id > merkle_state.last_processed_id {
            merkle_state.update_with_entry(leaf_hash, id);
        }
    }
    let tree_duration = tree_start.elapsed();

    // Update metrics for this log
    let final_tree_size = merkle_state.tree.len();

    // Release lock asap
    drop(merkle_state);

    let total_duration = total_start.elapsed();

    app_state
        .metrics
        .update_log_metrics(log_name, |log_metrics| {
            log_metrics.record_batch(
                batch_stats.rows_copied as u64,
                leaves_added as u64,
                u64_millis(total_duration.as_millis()),
                u64_millis(copy_duration.as_millis()),
                batch_stats.insert_ms,
                u64_millis(fetch_duration.as_millis()),
                u64_millis(tree_duration.as_millis()),
                final_tree_size,
            );
        });

    Ok(())
}

/// Copies pending rows from source tables to the per-log merkle table using pure SQL.
///
/// This implementation uses a CTE with UNION ALL to combine all source tables,
/// ordered according to the Rust ordering semantics, and inserted directly
/// into the per-log table `merkle_log_{log_name}`.
///
/// Returns statistics about the batch processing operation
async fn copy_source_rows(
    app_state: &AppState,
    log_name: &str,
    batch_size: u32,
) -> Result<BatchStats> {
    // Validate log name before any database operations
    validate_log_name(log_name)?;

    let mut conn = app_state.db_pool.get().await?;

    // Ensure the per-log table exists (idempotent)
    ensure_merkle_log_table_exists(&conn, log_name).await?;

    let table_name = merkle_log_table_name(log_name);

    // Load source configurations from database for this log
    let source_configs = load_source_configs(&conn, log_name).await?;

    if source_configs.is_empty() {
        return Ok(BatchStats {
            rows_copied: 0,
            insert_ms: 0,
        });
    }

    // Filter to only tables that actually exist
    let valid_configs = get_valid_source_tables(&conn, &source_configs).await?;

    if valid_configs.is_empty() {
        return Ok(BatchStats {
            rows_copied: 0,
            insert_ms: 0,
        });
    }

    let txn = conn.transaction().await?;

    // Advisory lock prevents concurrent execution for this specific log
    get_advisory_lock(log_name, &txn).await?;

    // Increase work_mem for the sort operation in the CTE
    // Default PostgreSQL work_mem (4MB) may cause disk-based sorts for large batches
    // Testing showed minimal impact (~8ms difference), so 32MB is sufficient
    txn.execute("SET LOCAL work_mem = '32MB'", &[]).await?;

    // Time the entire INSERT operation (query building, CTE execution, and insert)
    let insert_start = std::time::Instant::now();

    // Build UNION ALL query dynamically for all source tables
    // Each source table needs its own LIMIT to avoid fetching unbounded rows
    let mut union_parts = Vec::new();
    let mut source_names = Vec::new();

    for config in &valid_configs {
        source_names.push(config.table_name.clone());
        let source_param = source_names.len();
        let source = quote_identifier(&config.table_name);
        let id = quote_identifier(&config.id_column);
        let hash = quote_identifier(&config.hash_column);
        let timestamp = config.timestamp_column.as_ref().map_or_else(
            || "NULL::timestamptz".to_owned(),
            |column| format!("s.{}", quote_identifier(column)),
        );
        // Reconcile every visible source identity against durable receipts. A source
        // transaction can commit a smaller ID after a larger ID has been copied.
        // No high-water mark on source IDs can safely rule those rows out.
        union_parts.push(format!(
            "(SELECT s.{id}::bigint AS source_id, s.{hash} AS leaf_hash, \
             {timestamp} AS timestamp_col, ${source_param}::text AS source_table \
             FROM public.{source} s WHERE NOT EXISTS (\
                 SELECT 1 FROM {table_name} copied \
                 WHERE copied.source_table = ${source_param}::text \
                   AND copied.source_id = s.{id}) \
             ORDER BY s.{id} LIMIT ${limit_param}::bigint)",
            limit_param = valid_configs.len() + 1,
        ));
    }

    let query = format!(
        "WITH combined AS ({}) \
         INSERT INTO {table_name} (source_table, source_id, leaf_hash) \
         SELECT source_table, source_id, leaf_hash FROM combined \
         ORDER BY timestamp_col NULLS LAST, source_id, source_table",
        union_parts.join(" UNION ALL "),
    );
    let mut all_params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = source_names
        .iter()
        .map(|name| name as &(dyn tokio_postgres::types::ToSql + Sync))
        .collect();
    let batch_size_i64 = i64::from(batch_size);
    all_params.push(&batch_size_i64);

    let rows_affected = txn.execute(&query, &all_params).await?;

    let insert_duration = insert_start.elapsed();

    txn.commit().await?;

    Ok(BatchStats {
        rows_copied: rows_affected,
        insert_ms: u64_millis(insert_duration.as_millis()),
    })
}

/// Rebuilds all enabled logs from the database on startup
///
/// # Errors
///
/// Returns an error if any database operations fail, including getting a connection,
/// loading log configurations, validating sources, or rebuilding individual logs.
pub async fn rebuild_all_logs(app_state: &AppState) -> Result<()> {
    tracing::info!("Rebuilding all logs from database");

    let conn = app_state.db_pool.get().await?;
    let log_names = load_enabled_logs(&conn).await?;

    // Validate all source configurations and report issues
    tracing::info!("Validating source configurations");
    let validations = crate::service::validate_all_logs(&conn).await?;
    drop(conn);

    for validation in &validations {
        if !validation.enabled {
            // Skip disabled logs
            continue;
        }

        let invalid_sources: Vec<_> = validation
            .sources
            .iter()
            .filter(|s| !s.is_valid())
            .collect();

        if !invalid_sources.is_empty() {
            tracing::warn!(
                log_name = validation.log_name,
                invalid_count = invalid_sources.len(),
                "Log has invalid sources"
            );
            for source in invalid_sources {
                for error in source.errors() {
                    tracing::warn!(
                        source_table = source.source_table,
                        error,
                        "Source validation error"
                    );
                }
            }
        }
    }
    tracing::info!("Validation complete");

    if log_names.is_empty() {
        tracing::info!("No logs to rebuild");
        return Ok(());
    }

    for log_name in &log_names {
        rebuild_log(app_state, log_name).await?;
    }

    tracing::info!(log_count = log_names.len(), "Rebuilt logs");
    Ok(())
}

/// Rebuilds a single log's merkle tree from the database
///
/// The log will be reconstructed from all entries in the per-log merkle table
/// (`merkle_log_{log_name}`) and placed into the in-memory merkle state map.
async fn rebuild_log(app_state: &AppState, log_name: &str) -> Result<()> {
    // Validate log name
    validate_log_name(log_name)?;

    let conn = app_state.db_pool.get().await?;
    let table_name = merkle_log_table_name(log_name);

    // Check if the table exists before trying to query it
    let table_exists: bool = conn
        .query_one(
            "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = $1)",
            &[&table_name],
        )
        .await?
        .get(0);

    if !table_exists {
        tracing::debug!(log_name, "Log table does not exist yet");
        return Ok(());
    }

    // Fetch all entries for this log
    let query = format!("SELECT id, leaf_hash FROM {table_name} ORDER BY id");
    let rows = conn.query(&query, &[]).await?;

    if rows.is_empty() {
        tracing::debug!(log_name, "Log has no entries");
        return Ok(());
    }

    // Create new merkle state
    let mut merkle_state = crate::service::MerkleState::new();

    for row in rows {
        let id: i64 = row.get(0);
        let hash: Vec<u8> = row.get(1);
        let leaf_hash = LeafHash::new(hash);
        // we must checkpoint after each addition, because in a rebuild
        // scenario we do not know which roots have been published
        merkle_state.update_with_entry(leaf_hash, id);
    }

    // Store in DashMap
    let merkle_state_arc = std::sync::Arc::new(parking_lot::RwLock::new(merkle_state));
    let tree_size = merkle_state_arc.read().tree.len();
    let last_id = merkle_state_arc.read().last_processed_id;

    app_state
        .merkle_states
        .insert(log_name.to_string(), merkle_state_arc);

    tracing::info!(log_name, tree_size, last_id, "Log rebuilt");

    Ok(())
}

/// Load enabled log names from `verification_logs` table
///
/// A log is enabled if its 'enabled' column is true
async fn load_enabled_logs(conn: &PooledConnection) -> Result<Vec<String>> {
    let rows = conn
        .query(
            "SELECT log_name FROM verification_logs WHERE enabled = true ORDER BY log_name",
            &[],
        )
        .await?;

    let log_names: Vec<String> = rows.iter().map(|row| row.get(0)).collect();

    if !log_names.is_empty() {
        // tracing::info!("📋 Loaded {} enabled log(s)", log_names.len());
    }

    Ok(log_names)
}

/// Load enabled source configurations from `verification_sources` table for a specific log
///
/// A source is enabled if its 'enabled' column is true
async fn load_source_configs(conn: &PooledConnection, log_name: &str) -> Result<Vec<SourceConfig>> {
    let rows = conn
        .query(
            "SELECT source_table, hash_column, id_column, timestamp_column, log_name 
             FROM verification_sources 
             WHERE enabled = true AND log_name = $1 
             ORDER BY source_table",
            &[&log_name],
        )
        .await?;

    let mut configs = Vec::new();
    for row in rows {
        let table_name: String = row.get(0);
        let hash_column: String = row.get(1);
        let id_column: String = row.get(2);
        let timestamp_column: Option<String> = row.get(3);
        let log_name_: String = row.get(4);
        configs.push(SourceConfig::new(
            &table_name,
            &hash_column,
            &id_column,
            timestamp_column,
            &log_name_,
        ));
    }

    if !configs.is_empty() {
        // println!("📋 Loaded {} source config(s) for log '{}'", configs.len(), log_name);
    }

    Ok(configs)
}

/// Validates configured source tables, columns, types, and unique identities.
/// Invalid configuration fails the batch before copying any source rows.
async fn get_valid_source_tables(
    conn: &PooledConnection,
    configs: &[SourceConfig],
) -> Result<Vec<SourceConfig>> {
    let mut valid_configs = Vec::new();

    for config in configs {
        let validation = crate::service::validation::validate_source(
            conn,
            &config.log_name,
            &config.table_name,
            &config.id_column,
            &config.hash_column,
            config.timestamp_column.as_deref(),
        )
        .await?;
        anyhow::ensure!(
            validation.is_valid(),
            "Invalid source '{}': {}",
            config.table_name,
            validation.errors().join("; ")
        );
        valid_configs.push(config.clone());
    }

    Ok(valid_configs)
}

/// Hash a string to i64 for use as advisory lock ID
fn hash_string_to_i64(s: &str) -> i64 {
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish().cast_signed()
}

/// Acquires an advisory lock for the given log name within the provided transaction.
///
/// # Errors
/// Returns an error if the lock cannot be acquired, indicating that another
/// processing batch is already running for the specified log.
async fn get_advisory_lock(log_name: &str, txn: &tokio_postgres::Transaction<'_>) -> Result<()> {
    // Advisory lock prevents concurrent execution for this specific log
    let lock_id = hash_string_to_i64(log_name);
    let lock_acquired: bool = txn
        .query_one("SELECT pg_try_advisory_xact_lock($1)", &[&lock_id])
        .await?
        .get(0);

    if !lock_acquired {
        return Err(anyhow::anyhow!(
            "Another processing batch is running for log '{log_name}'"
        ));
    }

    Ok(())
}

/// Quotes a configured SQL identifier; configuration never supplies expressions.
fn quote_identifier(identifier: &str) -> String {
    format!("\"{}\"", identifier.replace('"', "\"\""))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::server::tests::test_state;

    #[test]
    fn log_names_cannot_alias_postgres_identifiers() {
        assert!(validate_log_name(&"a".repeat(52)).is_ok());
        for name in [
            format!("{}x", "a".repeat(52)),
            format!("{}y", "a".repeat(52)),
        ] {
            assert!(validate_log_name(&name).is_err());
        }
    }

    /// Run only against an empty disposable database; no application data is used.
    #[tokio::test]
    #[ignore = "requires TRELLIS_TEST_DATABASE_URL pointing to an empty disposable database"]
    async fn postgres_source_reconciliation_and_tree_recovery() -> Result<()> {
        let url = std::env::var("TRELLIS_TEST_DATABASE_URL")?;
        let state = test_state(&url);
        let conn = state.db_pool.get().await?;
        conn.batch_execute(
            "CREATE TABLE verification_logs (log_name text PRIMARY KEY, enabled boolean NOT NULL DEFAULT true);
             CREATE TABLE verification_sources (
                 source_table text NOT NULL, log_name text NOT NULL, hash_column text NOT NULL,
                 id_column text NOT NULL, timestamp_column text, enabled boolean NOT NULL DEFAULT true,
                 PRIMARY KEY (source_table, log_name));
             CREATE TABLE regression_source (id bigserial PRIMARY KEY, hash bytea NOT NULL);
             INSERT INTO verification_logs VALUES ('regression', true);
             INSERT INTO verification_sources (source_table, log_name, hash_column, id_column)
                 VALUES ('regression_source', 'regression', 'hash', 'id');"
        ).await?;
        let mut delayed = state.db_pool.get().await?;
        let txn = delayed.transaction().await?;
        txn.execute(
            "INSERT INTO regression_source (hash) VALUES ($1)",
            &[&vec![1u8; 32]],
        )
        .await?;
        conn.execute(
            "INSERT INTO regression_source (hash) VALUES ($1)",
            &[&vec![2u8; 32]],
        )
        .await?;
        process_log(&state, "regression", 10).await?;
        assert_eq!(
            state
                .merkle_states
                .get("regression")
                .unwrap()
                .read()
                .tree
                .len(),
            1
        );
        txn.commit().await?;
        process_log(&state, "regression", 10).await?;
        let ids: Vec<i64> = conn
            .query(
                "SELECT source_id FROM merkle_log_regression ORDER BY id",
                &[],
            )
            .await?
            .iter()
            .map(|row| row.get(0))
            .collect();
        assert_eq!(
            ids,
            vec![2, 1],
            "late commit is appended without rewriting history"
        );
        let expected_root = state
            .merkle_states
            .get("regression")
            .unwrap()
            .read()
            .tree
            .root();

        // A second instance must catch up even if it copied no rows locally.
        let replica = test_state(&url);
        process_log(&replica, "regression", 1).await?;
        assert_eq!(
            replica
                .merkle_states
                .get("regression")
                .unwrap()
                .read()
                .tree
                .len(),
            1
        );
        process_log(&replica, "regression", 1).await?;
        assert_eq!(
            replica
                .merkle_states
                .get("regression")
                .unwrap()
                .read()
                .tree
                .root(),
            expected_root
        );

        // Simulate disabling/unloading and re-enabling a populated log.
        state.merkle_states.remove("regression");
        process_log(&state, "regression", 10).await?;
        assert_eq!(
            state
                .merkle_states
                .get("regression")
                .unwrap()
                .read()
                .tree
                .root(),
            expected_root
        );

        // Commit the final batch without fetching it (e.g. a fetch/network failure).
        conn.execute(
            "INSERT INTO regression_source (hash) VALUES ($1)",
            &[&vec![3u8; 32]],
        )
        .await?;
        assert_eq!(
            copy_source_rows(&state, "regression", 10)
                .await?
                .rows_copied,
            1
        );
        assert_eq!(
            copy_source_rows(&state, "regression", 10)
                .await?
                .rows_copied,
            0
        );
        process_log(&state, "regression", 10).await?;
        assert_eq!(
            state
                .merkle_states
                .get("regression")
                .unwrap()
                .read()
                .tree
                .len(),
            3
        );

        // Supported ID types must work, including zero and negative identities.
        for (suffix, sql_type) in [("small", "smallint"), ("int", "integer"), ("big", "bigint")] {
            let table = format!("regression_{suffix}");
            let log = format!("regression_{suffix}");
            conn.batch_execute(&format!("CREATE TABLE {table} (id {sql_type} PRIMARY KEY, hash bytea NOT NULL, stamp timestamptz);\
                INSERT INTO {table} VALUES (-1, decode(repeat('01', 32), 'hex'), now()), (0, decode(repeat('02', 32), 'hex'), now()), (1, decode(repeat('03', 32), 'hex'), now());")).await?;
            conn.execute("INSERT INTO verification_sources (source_table, log_name, hash_column, id_column, timestamp_column) VALUES ($1, $2, 'hash', 'id', 'stamp')", &[&table, &log]).await?;
            process_log(&state, &log, 10).await?;
            assert_eq!(state.merkle_states.get(&log).unwrap().read().tree.len(), 3);
            assert_eq!(copy_source_rows(&state, &log, 10).await?.rows_copied, 0);
        }

        // Each source gets a bounded batch; tree catch-up remains bounded independently.
        conn.execute("INSERT INTO verification_sources (source_table, log_name, hash_column, id_column, timestamp_column) SELECT source_table, 'regression_multi', hash_column, id_column, timestamp_column FROM verification_sources WHERE log_name IN ('regression', 'regression_small', 'regression_int')", &[]).await?;
        for _ in 0..5 {
            let before = state
                .merkle_states
                .get("regression_multi")
                .map_or(0, |entry| entry.read().tree.len());
            process_log(&state, "regression_multi", 2).await?;
            let after = state
                .merkle_states
                .get("regression_multi")
                .unwrap()
                .read()
                .tree
                .len();
            assert!(after - before <= 2);
        }
        assert_eq!(
            state
                .merkle_states
                .get("regression_multi")
                .unwrap()
                .read()
                .tree
                .len(),
            9
        );

        // Identifier quoting and value parameters support literal punctuation safely.
        conn.batch_execute(
            r#"CREATE TABLE "odd'source" ("select" integer PRIMARY KEY, "leaf""hash" bytea);
            INSERT INTO "odd'source" VALUES (1, decode(repeat('04', 32), 'hex'));"#,
        )
        .await?;
        conn.execute("INSERT INTO verification_sources (source_table, log_name, hash_column, id_column) VALUES ($1, 'regression_quoted', $2, $3)", &[&"odd'source", &"leaf\"hash", &"select"]).await?;
        process_log(&state, "regression_quoted", 10).await?;
        assert_eq!(
            state
                .merkle_states
                .get("regression_quoted")
                .unwrap()
                .read()
                .tree
                .len(),
            1
        );

        // Fractional numeric IDs cannot be safely represented by persisted bigint IDs.
        conn.batch_execute(
            "CREATE TABLE regression_numeric (id numeric PRIMARY KEY, hash bytea NOT NULL);
            INSERT INTO verification_sources (source_table, log_name, hash_column, id_column)
                VALUES ('regression_numeric', 'regression_numeric', 'hash', 'id');",
        )
        .await?;
        assert!(
            copy_source_rows(&state, "regression_numeric", 10)
                .await
                .is_err()
        );

        // Step's journal requests proofs by persisted position, including duplicates.
        conn.batch_execute(crate::journal::SCHEMA).await?;
        conn.execute("INSERT INTO trellis_logs (name) VALUES ('duplicates')", &[])
            .await?;
        let journal = crate::journal::Journal::new(state.db_pool.clone());
        for (id, hash) in [
            (10_i64, vec![1u8; 32]),
            (11, vec![2u8; 32]),
            (12, vec![1u8; 32]),
        ] {
            let mut writer = state.db_pool.get().await?;
            let transaction = writer.transaction().await?;
            crate::journal::Journal::append(&transaction, "duplicates", id, &hash).await?;
            transaction.commit().await?;
        }
        for (id, index) in [(10_i64, 0_u64), (12, 2)] {
            let inclusion = journal.inclusion("duplicates", id).await?;
            assert_eq!(inclusion.proof.index, index);
            inclusion.verify(&[1; 32], &inclusion.checkpoint)?;
        }

        // Configuration expressions fail validation rather than being executed as SQL.
        conn.execute(
            "UPDATE verification_sources SET hash_column = $1 WHERE log_name = 'regression'",
            &[&"decode(repeat('00',32),'hex')"],
        )
        .await?;
        assert!(copy_source_rows(&state, "regression", 10).await.is_err());
        Ok(())
    }
}
