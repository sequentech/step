//! Database setup utility for Trellis
//!
//! Creates the necessary `PostgreSQL` tables for Trellis operation.
//! The schema includes `verification_logs` and `verification_sources` tables.
//!
//! Note: Per-log merkle tables (`merkle_log_{log_name}`) are created dynamically
//! by the batch processor when a log is first processed.
//!
#![allow(clippy::pedantic)]
#![allow(clippy::print_stdout)]
#![allow(clippy::print_stderr)]
//! # Usage
//!
//! ```bash
//! cargo run --bin setup                    # Create/update schema
//! cargo run --bin setup -- --reset         # Drop and recreate all tables
//! ```

use anyhow::Result;
use std::env;
use tokio_postgres::{Client, NoTls};

/// Connects to the database and returns a client.
async fn connect() -> Result<Client> {
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set in .env file");
    let (client, connection) = tokio_postgres::connect(&db_url, NoTls).await?;

    // The connection object performs the actual communication with the database,
    // so spawn it off to run on its own.
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {e}");
        }
    });

    Ok(client)
}

/// Sets up the database schema, including the configuration tables.
/// If reset is true, it will drop and recreate all objects.
///
/// Note: Per-log merkle tables (merkle_log_{log_name}) are created dynamically
/// by the batch processor, not by this setup script.
async fn setup_database(client: &Client, reset: bool) -> Result<()> {
    if reset {
        println!("🗑️ Dropping existing objects...");

        // Drop all merkle_log_* tables dynamically
        let tables = client
            .query(
                "SELECT tablename FROM pg_tables WHERE schemaname = 'public' AND tablename LIKE 'merkle_log_%'",
                &[],
            )
            .await?;

        for row in tables {
            let table_name: &str = row.get(0);
            let drop_query = format!("DROP TABLE IF EXISTS {} CASCADE", table_name);
            client.batch_execute(&drop_query).await?;
            println!("  Dropped table '{}'", table_name);
        }

        // Drop legacy shared merkle_log table if it exists
        client
            .batch_execute("DROP TABLE IF EXISTS merkle_log CASCADE;")
            .await?;

        client
            .batch_execute(
                r#"
            DROP TABLE IF EXISTS verification_sources CASCADE;
            DROP TABLE IF EXISTS verification_logs CASCADE;
        "#,
            )
            .await?;
        println!("✅ Existing objects dropped.");
    }

    // 1. Create the verification logs table - defines independent merkle logs
    client
        .batch_execute(
            r#"
        CREATE TABLE IF NOT EXISTS verification_logs (
            log_name TEXT PRIMARY KEY,
            created_at TIMESTAMP NOT NULL DEFAULT NOW(),
            description TEXT,
            enabled BOOLEAN NOT NULL DEFAULT true
        );
    "#,
        )
        .await?;
    println!("✅ Table 'verification_logs' is ready.");

    // 2. Create the verification sources configuration table - sources can belong to multiple logs
    client
        .batch_execute(
            r#"
        CREATE TABLE IF NOT EXISTS verification_sources (
            source_table TEXT NOT NULL,
            log_name TEXT NOT NULL REFERENCES verification_logs(log_name) ON DELETE CASCADE,
            hash_column TEXT NOT NULL,
            id_column TEXT NOT NULL,
            timestamp_column TEXT,
            enabled BOOLEAN NOT NULL DEFAULT true,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            PRIMARY KEY (source_table, log_name)
        );
    "#,
        )
        .await?;
    println!("✅ Table 'verification_sources' is ready.");

    // Note: Per-log merkle tables (merkle_log_{log_name}) are created dynamically
    // by the batch processor when a log is first processed.
    println!("ℹ️  Per-log merkle tables will be created automatically when logs are processed.");

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file
    dotenv::dotenv().ok();

    // Check if --reset flag is provided
    let reset = env::args().any(|arg| arg == "--reset");

    let client = connect().await?;

    println!("--- Setting up database schema ---");
    setup_database(&client, reset).await?;
    println!("----------------------------------");

    Ok(())
}
