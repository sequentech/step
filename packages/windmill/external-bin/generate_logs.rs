// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use csv::Writer;
use electoral_log::messages::message::Message;
use serde::Deserialize;
use std::collections::HashMap; // Added for HashMap
use std::fs;
use std::path::PathBuf;
use strand::serialization::StrandDeserialize;
use tracing::info;
use tracing_subscriber::EnvFilter;
use windmill::services::electoral_log::ElectoralLogRow;
use windmill::services::partial_file::PartialFile;
use windmill::services::protocol_manager::{get_board_client, get_event_board};
use windmill::services::reports::activity_log::ActivityLogRow;

/// Generates a CSV report of activity logs from PostgreSQL.
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    /// Tenant ID
    #[clap(long)]
    tenant_id: String,

    /// Election Event ID (used to identify the PostgreSQL log)
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
    elections: HashMap<String, String>, // election_id -> election_name (for CSV filename)
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

    info!(config = ?config, "Configuration loaded successfully."); // Use ? for Debug formatting of Config

    // Get board name
    let slug = std::env::var("ENV_SLUG").context("ENV_SLUG must be set")?;
    let board_name = get_event_board(&cli.tenant_id, &cli.election_event_id, &slug);
    info!(%board_name, "Target PostgreSQL board name determined.");

    // Create output directory
    fs::create_dir_all(&cli.output_folder_path).with_context(|| {
        format!(
            "Failed to create output folder: {}",
            cli.output_folder_path.display()
        )
    })?;
    info!(output_folder = %cli.output_folder_path.display(), "Output folder ensured.");

    // HashMap to store CSV writers, keyed by sanitized filename stem
    let mut csv_writers: HashMap<String, Writer<PartialFile>> = HashMap::new();

    let client = get_board_client().await?;
    let mut cursor = 0;
    let mut total_rows_fetched = 0;
    let mut activity_log_written_counts: HashMap<String, usize> = HashMap::new();
    loop {
        let rows = client
            .get_electoral_log_messages_batch(&board_name, 1000, cursor)
            .await?;
        if rows.is_empty() {
            break;
        }
        cursor = rows.last().unwrap().id;
        for row in rows {
            let message = Message::strand_deserialize(&row.message)?;
            let filename_stem_key = match message.election_id.as_ref() {
                Some(id) => config.elections.get(id).unwrap_or(id).clone(),
                None => "general_logs".to_owned(),
            };
            let sanitized_stem = sanitize_filename(&filename_stem_key);
            let elog_row = ElectoralLogRow::try_from(row)?;
            let activity_log_row = ActivityLogRow::try_from(elog_row)?;
            if !csv_writers.contains_key(&sanitized_stem) {
                let csv_path = cli
                    .output_folder_path
                    .join(format!("{}.csv", sanitized_stem));
                let file = PartialFile::create(&csv_path)?;
                csv_writers.insert(sanitized_stem.clone(), Writer::from_writer(file));
            }
            csv_writers
                .get_mut(&sanitized_stem)
                .unwrap()
                .serialize(&activity_log_row)?;
            *activity_log_written_counts
                .entry(sanitized_stem)
                .or_insert(0) += 1;
            total_rows_fetched += 1;
        }
    }

    // Each CSV file is written as `<name>.csv.partial` and renamed only here, so a
    // run that fails part-way leaves no file that looks complete.
    for (filename_stem, writer) in csv_writers {
        let path = writer
            .into_inner()
            .map_err(|error| {
                anyhow!(
                    "Failed to write CSV file for {}: {}",
                    filename_stem,
                    error.error()
                )
            })?
            .commit()?;
        info!(
            path = %path.display(),
            count = activity_log_written_counts.get(&filename_stem).unwrap_or(&0),
            "CSV file written."
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
