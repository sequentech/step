// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{Context, Result};
use clap::Parser;
use chrono::{DateTime, Utc, TimeZone};
use csv::Writer;
use immudb_rs::{Client, Row as ImmudbRow, sql_value::Value as ImmudbSqlValue, NamedParam};
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap; // Added for HashMap
use std::fs::{self, File};
use std::path::PathBuf;
use tracing::{debug, error, info, warn}; // Added tracing macros
use tracing_subscriber::EnvFilter; // Added for tracing initialization

/// Generates a CSV report of activity logs from immudb.
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    /// Tenant ID
    #[clap(long)]
    tenant_id: String,

    /// Election Event ID (used to identify the immudb log)
    #[clap(long)]
    election_event_id: String,

    /// Path to the output folder where CSV files will be saved
    #[clap(long)]
    output_folder_path: PathBuf,

    /// Path to the configuration TOML file
    #[clap(long)]
    config: PathBuf,
}

#[derive(Deserialize, Debug)]
struct Config {
    immudb_url: String,
    immudb_user: String,
    immudb_password: String,
    batch_size: Option<usize>,
    elections: HashMap<String, String>, // election_id -> election_name (for CSV filename)
}

// --- Data Structures ---

#[derive(Serialize, Debug)]
struct ActivityLogRow {
    id: i64,
    created: String,
    statement_timestamp: String,
    statement_kind: String,
    event_type: String,
    log_type: String,
    description: String,
    message: String, // The original JSON message from ElectoralLogRow
    user_id: String,
}

#[derive(Deserialize, Debug, Clone)]
struct ElectoralLogRow {
    id: i64,
    created: i64, // Unix timestamp
    statement_timestamp: i64, // Unix timestamp
    statement_kind: String,
    message: String, // JSON string content of the log message
    user_id: Option<String>,
    // data: String, // Not strictly needed for ActivityLogRow, can be omitted from SELECT
    // username: Option<String>, // Not strictly needed for ActivityLogRow
}

// Structs for parsing the JSON content of ElectoralLogRow.message
#[derive(Deserialize, Debug, Clone)]
struct StatementHeadData {
    event_type: String,
    log_type: String,
    description: String,
}

#[derive(Deserialize, Debug)]
struct LogMessageBody {
    election_id: Option<String>,
}

#[derive(Deserialize, Debug)]
struct StatementWrapper {
    head: StatementHeadData,
    body: Option<LogMessageBody>,
}

#[derive(Deserialize, Debug)]
struct MessageWrapper {
    statement: StatementWrapper,
}

// --- Helper Functions ---

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => c,
            _ => '_', // Replace other characters with underscore
        })
        .collect()
}

fn timestamp_to_rfc3339(timestamp_secs: i64) -> Result<String> {
    Ok(Utc.timestamp_opt(timestamp_secs, 0).single()
        .ok_or_else(|| anyhow::anyhow!("Invalid timestamp: {}", timestamp_secs))?
        .to_rfc3339())
}

impl ElectoralLogRow {
    fn try_from_immudb_row(row: &ImmudbRow) -> Result<Self> {
        let mut id = 0;
        let mut created = 0;
        let mut statement_timestamp = 0;
        let mut statement_kind = String::new();
        let mut message_bytes: Vec<u8> = Vec::new();
        let mut user_id: Option<String> = None;

        for (col_idx, col_name_qual) in row.columns.iter().enumerate() {
            // Column name might be qualified, e.g., "electoral_log_messages.id"
            // We'll split by '.' and take the last part if present, or use the full name.
            let col_name = col_name_qual.split('.').last().unwrap_or(col_name_qual);
            let value = &row.values[col_idx];

            match col_name {
                "id" => id = value.try_into_i64().context("Failed to parse 'id' as i64")?,
                "created" => created = value.try_into_i64_timestamp().context("Failed to parse 'created' as timestamp")?,
                "statement_timestamp" => statement_timestamp = value.try_into_i64_timestamp().context("Failed to parse 'statement_timestamp' as timestamp")?,
                "statement_kind" => statement_kind = value.try_into_string().context("Failed to parse 'statement_kind' as String")?,
                "message" => message_bytes = value.try_into_bytes().context("Failed to parse 'message' as bytes")?,
                "user_id" => user_id = value.try_into_opt_string().context("Failed to parse 'user_id' as Option<String>")?,
                _ => { // Log or ignore unknown columns
                    // println!("Ignoring unknown column: {}", col_name);
                }
            }
        }

        let message_str = String::from_utf8(message_bytes)
            .with_context(|| "Failed to convert message bytes to UTF-8 string")?;

        Ok(ElectoralLogRow {
            id,
            created,
            statement_timestamp,
            statement_kind,
            message: message_str,
            user_id,
        })
    }
}

impl ActivityLogRow {
fn try_from_electoral_log(elog: &ElectoralLogRow) -> Result<Option<(Self, Option<String>)>> {
        let parsed_message: MessageWrapper = serde_json::from_str(&elog.message)
            .with_context(|| format!("Failed to parse ElectoralLogRow.message JSON for log id {}: {}", elog.id, elog.message))?;

        let extracted_election_id = parsed_message
            .statement
            .body
            .as_ref()
            .and_then(|b| b.election_id.clone());

        let head = parsed_message.statement.head;

        let activity_log_row = ActivityLogRow {
            id: elog.id,
            created: timestamp_to_rfc3339(elog.created)?,
            statement_timestamp: timestamp_to_rfc3339(elog.statement_timestamp)?,
            statement_kind: elog.statement_kind.clone(),
            event_type: head.event_type,
            log_type: head.log_type,
            description: head.description,
            message: elog.message.clone(), // Keep original JSON message
            user_id: elog.user_id.clone().unwrap_or_else(|| "-".to_string()),
        };
        
        Ok(Some((activity_log_row, extracted_election_id)))
    }
}

// Trait and impl for easier conversion from ImmudbSqlValue
trait ImmudbSqlValueExt {
    fn try_into_i64(&self) -> Result<i64>;
    fn try_into_i64_timestamp(&self) -> Result<i64>; // Assumes timestamp is N or Ts
    fn try_into_string(&self) -> Result<String>;
    fn try_into_opt_string(&self) -> Result<Option<String>>;
    fn try_into_bytes(&self) -> Result<Vec<u8>>;
}

impl ImmudbSqlValueExt for ImmudbSqlValue {
    fn try_into_i64(&self) -> Result<i64> {
        match &self.value {
            Some(ImmudbSqlValue::N(n)) => Ok(*n),
            _ => Err(anyhow::anyhow!("Expected N (i64), found {:?}", self.value)),
        }
    }
    fn try_into_i64_timestamp(&self) -> Result<i64> {
        match &self.value {
            Some(ImmudbSqlValue::N(n)) => Ok(*n),      // Timestamps might be stored as N
            Some(ImmudbSqlValue::Ts(ts)) => Ok(*ts), // Or as Ts
            _ => Err(anyhow::anyhow!("Expected N or Ts (timestamp), found {:?}", self.value)),
        }
    }
    fn try_into_string(&self) -> Result<String> {
        match &self.value {
            Some(ImmudbSqlValue::S(s)) => Ok(s.clone()),
            _ => Err(anyhow::anyhow!("Expected S (String), found {:?}", self.value)),
        }
    }
    fn try_into_opt_string(&self) -> Result<Option<String>> {
        match &self.value {
            Some(ImmudbSqlValue::S(s)) => Ok(Some(s.clone())),
            Some(ImmudbSqlValue::Null(_)) => Ok(None),
            None => Ok(None),
            _ => Err(anyhow::anyhow!("Expected S (String) or Null, found {:?}", self.value)),
        }
    }
    fn try_into_bytes(&self) -> Result<Vec<u8>> {
        match &self.value {
            Some(ImmudbSqlValue::Bs(bs)) => Ok(bs.clone()),
            _ => Err(anyhow::anyhow!("Expected Bs (Bytes), found {:?}", self.value)),
        }
    }
}

/// Constructs the immudb board name from tenant_id and election_event_id.
/// Replicates logic from `packages/windmill/src/services/protocol_manager.rs`.
fn get_event_board_name(tenant_id: &str, election_event_id: &str) -> String {
    let tenant: String = tenant_id
        .to_string()
        .chars()
        .filter(|&c| c != '-')
        .take(17)
        .collect();
    format!("tenant{}event{}", tenant, election_event_id)
        .chars()
        .filter(|&c| c != '-')
        .collect()
}

/// Establishes a connection to immudb using the provided configuration.
/// Replicates logic from `packages/windmill/src/services/protocol_manager.rs`.
async fn connect_immudb(config: &Config) -> Result<Client> {
    let mut client = Client::new(&config.immudb_url, &config.immudb_user, &config.immudb_password)
        .await
        .with_context(|| format!("Failed to create immudb client with URL: {}", config.immudb_url))?;
    client.login().await.with_context("Failed to login to immudb")?;
    Ok(client)
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing subscriber
    // Default to `info` level for this crate if RUST_LOG is not set.
    // Example: RUST_LOG=generate_logs=debug,warn
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info")) // Default to info if RUST_LOG is not set
        .add_directive("generate_logs=info".parse()?); // Ensure this crate's info logs are shown by default

    tracing_subscriber::fmt().with_env_filter(filter).init();

    let cli = Cli::parse();

    info!(
        tenant_id = %cli.tenant_id,
        election_event_id = %cli.election_event_id,
        output_folder_path = %cli.output_folder_path.display(),
        config_path = %cli.config.display(),
        "Starting log generation process with CLI arguments."
    );

    // Load configuration
    let config_content = fs::read_to_string(&cli.config)
        .with_context(|| format!("Failed to read config file: {}", cli.config.display()))?;
    let config: Config = toml::from_str(&config_content)
        .with_context(|| format!("Failed to parse TOML configuration from {}", cli.config.display()))?;

    info!(config = ?config, "Configuration loaded successfully."); // Use ? for Debug formatting of Config

    // Get board name
    let board_name = get_event_board_name(&cli.tenant_id, &cli.election_event_id);
    info!(%board_name, "Target Immudb board name determined.");

    // Create output directory
    fs::create_dir_all(&cli.output_folder_path)
        .with_context(|| format!("Failed to create output folder: {}", cli.output_folder_path.display()))?;
    info!(output_folder = %cli.output_folder_path.display(), "Output folder ensured.");

    // HashMap to store CSV writers, keyed by sanitized filename stem
    let mut csv_writers: HashMap<String, Writer<File>> = HashMap::new();

    // Connect to immudb and open session
    let mut client = connect_immudb(&config).await?;
    info!("Successfully connected to Immudb.");

    client
        .open_session(&board_name)
        .await
        .with_context(|| format!("Failed to open session to board: {}", board_name))?;
    info!(%board_name, "Successfully opened session to board.");

    let mut offset: i64 = 0;
    let batch_size = config.batch_size.unwrap_or(1000);
    let mut total_rows_fetched = 0;
    let mut activity_log_written_counts: HashMap<String, usize> = HashMap::new();

    info!(batch_size, "Starting log retrieval from Immudb.");

    loop {
        let sql = format!(
            "SELECT id, created, statement_timestamp, statement_kind, message, user_id \
             FROM electoral_log_messages \
             ORDER BY id ASC \
             LIMIT @limit OFFSET @offset"
        );

        let params = vec![
            NamedParam { name: "limit".to_string(), value: Some(ImmudbSqlValue { value: Some(ImmudbSqlValue::N(batch_size as i64)) }) },
            NamedParam { name: "offset".to_string(), value: Some(ImmudbSqlValue { value: Some(ImmudbSqlValue::N(offset)) }) },
        ];

        debug!(offset, "Fetching log batch from Immudb.");
        let response = client.sql_query(&sql, params)
            .await
            .with_context(|| format!("Failed to execute SQL query (offset: {})", offset))?;

        if response.rows.is_empty() {
            info!("No more rows found in Immudb. Log retrieval complete.");
            break;
        }

        let num_rows_in_batch = response.rows.len();
        total_rows_fetched += num_rows_in_batch;
        debug!(num_rows_in_batch, "Fetched rows in this batch.");

        for row in response.rows {
            match ElectoralLogRow::try_from_immudb_row(&row) {
                Ok(elog_row) => {
                    debug!(log_id = elog_row.id, "Successfully parsed ElectoralLogRow.");
                    match ActivityLogRow::try_from_electoral_log(&elog_row) {
                        Ok(Some((activity_log_row, extracted_election_id_opt))) => {
                            let filename_stem_key = match &extracted_election_id_opt {
                                Some(id) => config.elections.get(id).map(|s| s.as_str()).unwrap_or(id).to_string(),
                                None => "general_logs".to_string(),
                            };
                            let sanitized_stem = sanitize_filename(&filename_stem_key);

                            if !csv_writers.contains_key(&sanitized_stem) {
                                let csv_path = cli.output_folder_path.join(format!("{}.csv", sanitized_stem));
                                info!(file_path = %csv_path.display(), election_id_key = %filename_stem_key, "Creating new CSV file.");
                                let file = File::create(&csv_path)
                                    .with_context(|| format!("Failed to create CSV file: {}", csv_path.display()))?;
                                csv_writers.insert(sanitized_stem.clone(), Writer::from_writer(file));
                            }

                            if let Some(writer) = csv_writers.get_mut(&sanitized_stem) {
                                if let Err(e) = writer.serialize(&activity_log_row) {
                                    error!(log_id = elog_row.id, election_file_stem = %sanitized_stem, error = %e, "Failed to serialize ActivityLogRow to CSV.");
                                } else {
                                    *activity_log_written_counts.entry(sanitized_stem.clone()).or_insert(0) += 1;
                                }
                            }
                        }
                        Ok(None) => { 
                            // This case should ideally not happen if try_from_electoral_log always returns Some on success
                            debug!(log_id = elog_row.id, "Transformation to ActivityLogRow returned None unexpectedly.");
                         }
                        Err(e) => {
                            warn!(log_id = elog_row.id, error = %e, "Failed to transform ElectoralLogRow.");
                        }
                    }
                }
                Err(e) => {
                    error!(error = %e, "Failed to parse ImmudbRow into ElectoralLogRow.");
                }
            }
        }

        offset += batch_size as i64;
    }

    for (filename_stem, writer) in csv_writers.iter_mut() {
        writer.flush().with_context(|| format!("Failed to flush CSV writer for {}", filename_stem))?;
        info!(filename_stem, count = activity_log_written_counts.get(filename_stem).unwrap_or(&0), "CSV file flushed.");
    }

    info!(
        total_rows_fetched,
        num_csv_files = activity_log_written_counts.len(),
        output_folder = %cli.output_folder_path.display(),
        "Processing complete. CSV files generated."
    );
    for (file, count) in activity_log_written_counts {
        info!("  -> {}.csv : {} records", file, count);
    }

    client
        .close_session()
        .await
        .with_context("Failed to close immudb session")?;
    info!("Successfully closed immudb session.");

    Ok(())
}
