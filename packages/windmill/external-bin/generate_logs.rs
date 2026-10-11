// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{Context, Result};
use base64::engine::general_purpose;
use base64::Engine;
use chrono::{TimeZone, Utc};
use clap::Parser;
use csv::Writer;
use electoral_log::messages::message::Message;
use electoral_log::BoardClient;
use electoral_log::ElectoralLogMessage;
use serde::Deserialize;
use std::collections::HashMap; // Added for HashMap
use std::fs::{self, File};
use std::path::PathBuf;
use tracing::{debug, error, info, warn};
use tracing_subscriber::EnvFilter;
use voting_crypto::RecordDeserialize;
use windmill::services::electoral_log::ElectoralLogRow;
use windmill::services::reports::activity_log::{ActivityLogRow, LogZones};

/// Generates a CSV report of activity logs from native audit.
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    /// Tenant ID
    #[clap(long)]
    tenant_id: String,

    /// Election Event ID (used to identify the native audit log)
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
    audit_database_url: String,
    elections: HashMap<String, String>, // election_id -> election_name (for CSV filename)
    /// IANA zone for this standalone export; absent settings use UTC.
    #[serde(default)]
    time_zone: Option<String>,
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

/// Constructs the native audit board name from tenant_id and election_event_id.
/// Replicates logic from `packages/windmill/src/services/electoral_log_board.rs`.
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

/// Establishes a connection to native audit using the provided configuration.
/// Replicates logic from `packages/windmill/src/services/electoral_log_board.rs`.
async fn connect_audit(config: &Config) -> Result<BoardClient> {
    BoardClient::new(&config.audit_database_url)
        .await
        .context("Failed to connect to the native audit database")
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
    let config: Config = toml::from_str(&config_content).with_context(|| {
        format!(
            "Failed to parse TOML configuration from {}",
            cli.config.display()
        )
    })?;

    let chosen_zone = config
        .time_zone
        .as_deref()
        .map(windmill::services::time_zones::parse_zone)
        .transpose()?;
    let log_zones = LogZones::new(chosen_zone, None, &[]);
    info!("Configuration loaded successfully.");

    // Get board name
    let board_name = get_event_board_name(&cli.tenant_id, &cli.election_event_id);
    info!(%board_name, "Target native audit board name determined.");

    // Create output directory
    fs::create_dir_all(&cli.output_folder_path).with_context(|| {
        format!(
            "Failed to create output folder: {}",
            cli.output_folder_path.display()
        )
    })?;
    info!(output_folder = %cli.output_folder_path.display(), "Output folder ensured.");

    // HashMap to store CSV writers, keyed by sanitized filename stem
    let mut csv_writers: HashMap<String, Writer<File>> = HashMap::new();

    let mut client = connect_audit(&config).await?;
    let mut total_rows_fetched = 0;
    let mut activity_log_written_counts: HashMap<String, usize> = HashMap::new();
    let mut after_id = 0;
    loop {
        let rows = client
            .get_electoral_log_messages_batch(&board_name, 1000, after_id)
            .await?;
        if rows.is_empty() {
            break;
        }
        for individual_row in rows {
            after_id = individual_row.id;
            // Iterate over individual rows within the batch
            total_rows_fetched += 1;
            if total_rows_fetched % 1000 == 0 {
                info!(total_rows_fetched, "Processed rows from stream...");
            }

            let elog_row = match ElectoralLogRow::try_from(individual_row) {
                Ok(elog_row) => elog_row,
                Err(e) => {
                    warn!(error = %e, "Failed to parse NativeAuditRow into ElectoralLogRow from stream batch.");
                    continue;
                }
            };
            debug!(
                log_id = elog_row.id,
                "Successfully parsed ElectoralLogRow from stream batch."
            );
            let message_bytes = match general_purpose::STANDARD_NO_PAD.decode(&elog_row.data) {
                Ok(message_bytes) => message_bytes,
                Err(e) => {
                    warn!(error = %e, "Error decoding ElectoralLogRow base64 data into bytes.");
                    continue;
                }
            };

            let message: Message = match Message::record_deserialize(&message_bytes) {
                Ok(message) => message,
                Err(e) => {
                    warn!(error = %e, "Error deserializing ElectoralLogRow data bytes into a Message.");
                    continue;
                }
            };
            let extracted_election_id_opt = message.election_id.clone();

            let activity_log_row = match ElectoralLogMessage::try_from(&message)
                .and_then(|entry| ActivityLogRow::new(entry, &log_zones))
            {
                Ok(activity_log_row) => activity_log_row,
                Err(e) => {
                    warn!(log_id = elog_row.id, error = %e, "Failed to transform ElectoralLogRow.");
                    continue;
                }
            };

            let filename_stem_key = match &extracted_election_id_opt {
                Some(id) => config
                    .elections
                    .get(id)
                    .map(|s| s.as_str())
                    .unwrap_or(id)
                    .to_string(),
                None => "general_logs".to_string(),
            };
            let sanitized_stem = sanitize_filename(&filename_stem_key);

            if !csv_writers.contains_key(&sanitized_stem) {
                let csv_path = cli
                    .output_folder_path
                    .join(format!("{}.csv", sanitized_stem));
                info!(file_path = %csv_path.display(), election_id_key = %filename_stem_key, "Creating new CSV file.");
                let file = File::create(&csv_path).with_context(|| {
                    format!("Failed to create CSV file: {}", csv_path.display())
                })?;
                csv_writers.insert(sanitized_stem.clone(), Writer::from_writer(file));
            }

            if let Some(writer) = csv_writers.get_mut(&sanitized_stem) {
                if let Err(e) = writer.serialize(&activity_log_row) {
                    error!(log_id = elog_row.id, election_file_stem = %sanitized_stem, error = %e, "Failed to serialize ActivityLogRow to CSV.");
                } else {
                    *activity_log_written_counts
                        .entry(sanitized_stem.clone())
                        .or_insert(0) += 1;
                }
            }
        }
    }

    for (filename_stem, writer) in csv_writers.iter_mut() {
        writer
            .flush()
            .with_context(|| format!("Failed to flush CSV writer for {}", filename_stem))?;
        info!(
            filename_stem,
            count = activity_log_written_counts.get(filename_stem).unwrap_or(&0),
            "CSV file flushed."
        );
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

    Ok(())
}
