//! Integration tests for the Trellis HTTP client
//!
//! Tests the Client API for interacting with the Trellis server, including
//! proof requests, verification, and error handling.
#![allow(clippy::pedantic)]
#![allow(clippy::print_stdout)]
#![allow(clippy::print_stderr)]
#![allow(clippy::arithmetic_side_effects)]

use anyhow::Result;
use trellis::service::Client;
use sha2::{Digest, Sha256};
use std::time::Duration;
use tokio_postgres::NoTls;

/// Test client that wraps the production HTTP client and adds database utilities
pub struct TestClient {
    /// Production HTTP client
    pub client: Client,
    /// Direct database connection for test setup and assertions
    pub db_client: tokio_postgres::Client,
}

impl TestClient {
    /// Creates a new test client with both HTTP and database access
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP client cannot be created or if the database
    /// connection cannot be established.
    ///
    /// # Panics
    ///
    /// Panics if `DATABASE_URL` is not set in the environment.
    pub async fn new(api_base_url: &str) -> Result<Self> {
        // Create production HTTP client
        let client = Client::new(api_base_url)?;

        // Set up database connection
        let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set in .env file");
        let (db_client, connection) = tokio_postgres::connect(&db_url, NoTls).await?;

        // Spawn the connection handler
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {e}");
            }
        });

        Ok(Self { client, db_client })
    }

    // ===== Test Setup Utilities =====

    /// Idempotently sets up test environment with logs and sources
    /// This can be called multiple times safely - does nothing if already configured
    ///
    /// # Errors
    ///
    /// Returns an error if database operations fail.
    pub async fn setup_test_environment(&self) -> Result<()> {
        // Define our test logs
        // Format: (log_name, description, [(source_table, has_timestamp)])
        #[allow(clippy::type_complexity)]
        let logs: Vec<(&str, &str, Vec<(&str, bool)>)> = vec![
            (
                "test_log_single_source",
                "Log for single-source tests",
                vec![("source_log", true)],
            ),
            (
                "test_log_multi_source",
                "Log for multi-source tests",
                vec![
                    ("source_log", true),
                    ("test_source_a", true),
                    ("test_source_b", true),
                    ("source_no_timestamp", false),
                ],
            ),
            (
                "test_log_no_timestamp",
                "Log for testing sources without timestamps",
                vec![
                    ("source_no_timestamp", false),
                    ("source_no_timestamp_b", false),
                ],
            ),
        ];

        for (log_name, description, source_configs) in logs {
            // Create log if it doesn't exist (idempotent)
            let insert_result = self.db_client
                .execute(
                    "INSERT INTO verification_logs (log_name, description) VALUES ($1, $2) ON CONFLICT (log_name) DO NOTHING",
                    &[&log_name, &description],
                )
                .await?;

            if insert_result > 0 {
                println!("✅ Created log '{log_name}'");
            }

            // Create and register source tables
            for (table_name, has_timestamp) in source_configs {
                // Create table if it doesn't exist (idempotent)
                let create_table_sql = if has_timestamp {
                    format!(
                        r#"
                        CREATE TABLE IF NOT EXISTS {table_name} (
                            id          BIGSERIAL PRIMARY KEY,
                            data        TEXT NOT NULL,
                            created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                            leaf_hash   BYTEA NOT NULL
                        );
                        "#
                    )
                } else {
                    format!(
                        r#"
                        CREATE TABLE IF NOT EXISTS {table_name} (
                            id          BIGSERIAL PRIMARY KEY,
                            data        TEXT NOT NULL,
                            leaf_hash   BYTEA NOT NULL
                        );
                        "#
                    )
                };

                self.db_client.batch_execute(&create_table_sql).await?;

                // Register in verification_sources (idempotent)
                let timestamp_column = if has_timestamp {
                    Some("created_at")
                } else {
                    None
                };
                let result = self.db_client
                    .execute(
                        "INSERT INTO verification_sources (source_table, log_name, hash_column, id_column, timestamp_column) 
                         VALUES ($1, $2, $3, $4, $5) 
                         ON CONFLICT (source_table, log_name) DO NOTHING",
                        &[&table_name, &log_name, &"leaf_hash", &"id", &timestamp_column],
                    )
                    .await?;

                if result > 0 {
                    println!(
                        "✅ Registered '{table_name}' in log '{log_name}' (timestamp: {has_timestamp})"
                    );
                }
            }
        }

        Ok(())
    }

    // ===== Database Insertion Utilities =====

    /// Adds a new entry to the `source_log` table
    ///
    /// # Errors
    ///
    /// Returns an error if the database insert fails.
    pub async fn add_entry(&self, data: &str) -> Result<i64> {
        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        let hash_result = hasher.finalize();

        let row = self
            .db_client
            .query_one(
                "INSERT INTO source_log (data, leaf_hash) VALUES ($1, $2) RETURNING id",
                &[&data, &hash_result.as_slice()],
            )
            .await?;

        Ok(row.get(0))
    }

    /// Adds an entry to a specified source table
    ///
    /// # Errors
    ///
    /// Returns an error if the database insert fails.
    pub async fn add_entry_to_source(&self, table_name: &str, data: &str) -> Result<i64> {
        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        let hash_result = hasher.finalize();

        let row = self
            .db_client
            .query_one(
                &format!(
                    "INSERT INTO {table_name} (data, leaf_hash) VALUES ($1, $2) RETURNING id"
                ),
                &[&data, &hash_result.as_slice()],
            )
            .await?;

        Ok(row.get(0))
    }

    // ===== Test Assertion Utilities =====

    /// Gets all sources from per-log merkle table for a given log
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub async fn get_sources(&self, log_name: &str) -> Result<Vec<tokio_postgres::Row>> {
        let table_name = format!("merkle_log_{}", log_name);
        let query = format!(
            "SELECT source_table, source_id, id FROM {} ORDER BY id",
            table_name
        );
        let result = self.db_client.query(&query, &[]).await?;

        Ok(result)
    }

    /// Diagnostic: Show recent entries for debugging
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub async fn show_recent_entries(&self, count: i64) -> Result<()> {
        // Query all merkle_log_* tables and combine results
        let tables = self.db_client
            .query(
                "SELECT tablename FROM pg_tables WHERE schemaname = 'public' AND tablename LIKE 'merkle_log_%'",
                &[],
            )
            .await?;

        println!("\n🔍 Last {count} entries across all merkle_log tables:");
        for table_row in &tables {
            let table_name: &str = table_row.get(0);
            let log_name = table_name.strip_prefix("merkle_log_").unwrap_or(table_name);
            
            let query = format!(
                "SELECT id, source_table, source_id FROM {} ORDER BY id DESC LIMIT $1",
                table_name
            );
            let rows = self.db_client.query(&query, &[&count]).await?;

            for row in rows.iter().rev() {
                let id: i64 = row.get(0);
                let source_table: String = row.get(1);
                let source_id: i64 = row.get(2);
                println!(
                    "  id={id}, log={log_name}, source={source_table}:{source_id}"
                );
            }
        }
        println!();
        Ok(())
    }

    // ===== Test Synchronization Utilities =====

    /// Waits until the log reaches the expected size by polling the size endpoint
    /// This is much more efficient than fixed-duration sleeps
    /// STRICT: Returns error if size exceeds expected (indicates entries from elsewhere)
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails, if the log size exceeds the expected
    /// size, or if the timeout is reached.
    pub async fn wait_until_log_size(&self, log_name: &str, expected_size: u64) -> Result<()> {
        let start_time = std::time::Instant::now();
        let timeout = Duration::from_secs(30); // Maximum wait time
        let poll_interval = Duration::from_millis(100); // Check every 100ms

        loop {
            let current_size = self.client.get_log_size(log_name).await?;

            if current_size == expected_size {
                println!(
                    "✅ Log reached expected size: {} (took {:?})",
                    current_size,
                    start_time.elapsed()
                );
                return Ok(());
            }

            if current_size > expected_size {
                return Err(anyhow::anyhow!(
                    "❌ Log size mismatch! Expected exactly {}, but got {} (extra {} entries from unknown source). This indicates entries are being processed that weren't tracked by this test.",
                    expected_size,
                    current_size,
                    current_size - expected_size
                ));
            }

            if start_time.elapsed() > timeout {
                return Err(anyhow::anyhow!(
                    "Timeout waiting for log size. Expected {expected_size}, got {current_size} after {timeout:?}"
                ));
            }

            tokio::time::sleep(poll_interval).await;
        }
    }
}
