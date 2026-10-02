//! Example: Posting an entry and verifying inclusion
//!
//! This example demonstrates the core verification workflow:
//! 1. Setting up a test environment (log and source table)
//! 2. Posting a new entry (simulated via direct database insert)
//! 3. Waiting for the entry to be processed into the merkle log
//! 4. Fetching an inclusion proof
//! 5. Verifying the inclusion proof cryptographically
//!
//! Usage:
//!   cargo run --example post
#![allow(clippy::pedantic)]
#![allow(clippy::print_stdout)]
#![allow(clippy::print_stderr)]

use anyhow::Result;
use base64::Engine;
use trellis::service::Client;
use sha2::{Digest, Sha256};
use std::time::Duration;
use tokio_postgres::NoTls;

/// The name of the merkle log used in this example.
const LOG_NAME: &str = "example_post_log";
/// The name of the source table used in this example.
const SOURCE_TABLE: &str = "example_post_source";

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();

    println!("=== Trellis Post & Verify Example ===\n");

    let server_url =
        std::env::var("TRELLIS_SERVER_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

    // Step 1: Setup - Create database connection and ensure log exists
    println!("📋 Setting up test environment...");
    let db_client = setup_environment().await?;
    println!("✅ Environment ready\n");

    // Step 2: Create verification client
    let client = Client::new(&server_url)?;
    println!("🔗 Connected to server: {server_url}\n");

    // Step 3: Insert - Post a new entry (simulated via database)
    let data = format!("Example entry at {}", chrono::Utc::now());
    println!("📝 Inserting entry: \"{data}\"");
    let hash = insert_entry(&db_client, &data).await?;
    println!("✅ Entry inserted\n");

    // Step 4: Wait - Ensure entry is processed into the merkle log
    wait_for_entry_in_log(&client, &data).await?;

    // Step 5: Verify - Get inclusion proof and verify it
    println!("🔍 Fetching inclusion proof...");
    let proof = client.get_inclusion_proof(LOG_NAME, &hash).await?;
    println!("✅ Received proof:");
    println!("   - Leaf index: {}", proof.index);
    println!("   - Tree size: {}", proof.tree_size);
    let root_preview = proof.root.get(..8).unwrap_or(&proof.root);
    println!(
        "   - Root: {}...",
        base64::engine::general_purpose::STANDARD.encode(root_preview)
    );
    println!();

    println!("🔐 Verifying inclusion proof...");
    let result = proof.verify(&hash);

    if result.is_ok() {
        println!("✅ Proof verification PASSED");
        println!(
            "   Entry \"{data}\" is cryptographically verified to be in the log!"
        );
    } else {
        println!("❌ Proof verification FAILED");
        return Err(anyhow::anyhow!("Inclusion proof verification failed"));
    }

    println!("\n✨ Example completed successfully!");
    Ok(())
}

// ===== Helper Functions =====

/// Sets up database connection and idempotently creates log and source table
async fn setup_environment() -> Result<tokio_postgres::Client> {
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let (client, connection) = tokio_postgres::connect(&db_url, NoTls).await?;

    // Spawn connection handler
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("Database connection error: {e}");
        }
    });

    // Create log if it doesn't exist
    client
        .execute(
            "INSERT INTO verification_logs (log_name, description) 
             VALUES ($1, $2) 
             ON CONFLICT (log_name) DO NOTHING",
            &[&LOG_NAME, &"Example log for post demonstration"],
        )
        .await?;

    // Create source table if it doesn't exist
    client
        .batch_execute(&format!(
            r#"
            CREATE TABLE IF NOT EXISTS {SOURCE_TABLE} (
                id          BIGSERIAL PRIMARY KEY,
                data        TEXT NOT NULL,
                created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                leaf_hash   BYTEA NOT NULL
            );
            "#
        ))
        .await?;

    // Register source table with the log
    client
        .execute(
            "INSERT INTO verification_sources (source_table, log_name, hash_column, id_column, timestamp_column)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (source_table, log_name) DO NOTHING",
            &[&SOURCE_TABLE, &LOG_NAME, &"leaf_hash", &"id", &Some("created_at")],
        )
        .await?;

    Ok(client)
}

/// Inserts an entry into the source table (simulates posting by external means)
/// Returns the hash of the inserted data for later verification
async fn insert_entry(client: &tokio_postgres::Client, data: &str) -> Result<Vec<u8>> {
    let mut hasher = Sha256::new();
    hasher.update(data.as_bytes());
    let hash = hasher.finalize();

    client
        .execute(
            &format!(
                "INSERT INTO {SOURCE_TABLE} (data, leaf_hash) VALUES ($1, $2)"
            ),
            &[&data, &hash.as_slice()],
        )
        .await?;

    Ok(hash.to_vec())
}

/// Waits for the server's batch processor to discover the newly created log
async fn wait_for_log_discovery(client: &Client) -> Result<()> {
    println!("⏳ Waiting for server to discover the log...");

    for attempt in 1..=30 {
        match client.has_log(LOG_NAME).await {
            Ok(true) => {
                println!("✅ Log discovered by server\n");
                return Ok(());
            }
            Ok(false) if attempt < 30 => {
                if attempt == 1 {
                    println!("   (The batch processor needs to load the new log - this is normal)");
                }
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
            Ok(false) => {
                return Err(anyhow::anyhow!("Timeout: Server never discovered the log"));
            }
            Err(e) => {
                return Err(anyhow::anyhow!("Failed to check log existence: {e}"));
            }
        }
    }

    Ok(())
}

/// Waits for the specific entry to be processed into the merkle log
/// (also ensures the log itself has been discovered by the server)
async fn wait_for_entry_in_log(client: &Client, data: &str) -> Result<()> {
    wait_for_log_discovery(client).await?;

    println!("⏳ Waiting for entry to be processed into merkle log...");

    for attempt in 1..=30 {
        match client.has_leaf(LOG_NAME, data).await {
            Ok(true) => {
                println!("✅ Entry found in merkle log\n");
                return Ok(());
            }
            Ok(false) if attempt < 30 => {
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
            Ok(false) => {
                return Err(anyhow::anyhow!("Timeout waiting for entry to be processed"));
            }
            Err(e) => return Err(e),
        }
    }

    Ok(())
}
