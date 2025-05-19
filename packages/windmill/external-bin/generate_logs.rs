// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{Context, Result};
use base64::engine::general_purpose;
use base64::Engine;
// use chrono::{TimeZone, Utc}; // Not explicitly used for TimeZone after refactor, Utc might be from ActivityLogRow
use clap::Parser;
use csv::Writer;
use electoral_log::messages::message::Message;
use immudb_rs::{sql_value::Value as ImmudbSqlValue, Client}; // ImmudbSqlValue not directly used, but good to keep if future query types need it.
use serde::Deserialize;
use std::collections::HashMap;
use std::fs::{self, File};
use std::path::PathBuf;
use std::time::{Duration, Instant}; // Added for periodic reconnect
use strand::serialization::StrandDeserialize;
use tokio_stream::StreamExt;
use tracing::{debug, error, info, warn};
use tracing_subscriber::EnvFilter;
use windmill::services::electoral_log::ElectoralLogRow;
use windmill::services::reports::activity_log::ActivityLogRow;

const DEFAULT_SQL_LIMIT: usize = 100_000_000; // Default value seems very high, ensure this is intended.
                                              // Typical DB limits are much smaller for single queries.
                                              // However, this is for the *total* if not paginated by user args.
                                              // The code *does* paginate internally with this value.

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

    /// Maximum query limit for each paginated request to Immudb.
    #[clap(long)]
    limit: Option<usize>,

    /// Query offset for the initial query.
    #[clap(long)]
    offset: Option<usize>,

    /// Reconnect to immudb every X seconds. Set to 0 to disable periodic reconnections. Default is 0 (disabled).
    #[clap(long, default_value_t = 0)]
    reconnect_interval_seconds: u64,
}

#[derive(Deserialize, Debug)]
struct Config {
    immudb_url: String,
    immudb_user: String,
    immudb_password: String,
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

/// Constructs the immudb board name from tenant_id and election_event_id.
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
async fn connect_immudb(config: &Config) -> Result<Client> {
    let mut client = Client::new(
        &config.immudb_url,
        &config.immudb_user,
        &config.immudb_password,
    )
    .await
    .context(format!(
        "Failed to create immudb client with URL: {}",
        config.immudb_url
    ))?;
    client.login().await.context("Failed to login to immudb")?;
    Ok(client)
}

#[tokio::main]
async fn main() -> Result<()> {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"))
        .add_directive("generate_logs=info".parse()?);

    tracing_subscriber::fmt().with_env_filter(filter).init();

    let cli = Cli::parse();

    info!(
        tenant_id = %cli.tenant_id,
        election_event_id = %cli.election_event_id,
        output_folder_path = %cli.output_folder_path.display(),
        config_path = %cli.config.display(),
        limit = %cli.limit.clone().unwrap_or(DEFAULT_SQL_LIMIT), // This limit is per page
        offset = %cli.offset.clone().unwrap_or(0),
        reconnect_interval_seconds = cli.reconnect_interval_seconds,
        "Starting log generation process with CLI arguments."
    );

    let config_content = fs::read_to_string(&cli.config)
        .with_context(|| format!("Failed to read config file: {}", cli.config.display()))?;
    let config: Config = toml::from_str(&config_content).with_context(|| {
        format!(
            "Failed to parse TOML configuration from {}",
            cli.config.display()
        )
    })?;

    info!(config = ?config, "Configuration loaded successfully.");

    let board_name = get_event_board_name(&cli.tenant_id, &cli.election_event_id);
    info!(%board_name, "Target Immudb board name determined.");

    fs::create_dir_all(&cli.output_folder_path).with_context(|| {
        format!(
            "Failed to create output folder: {}",
            cli.output_folder_path.display()
        )
    })?;
    info!(output_folder = %cli.output_folder_path.display(), "Output folder ensured.");

    let mut csv_writers: HashMap<String, Writer<File>> = HashMap::new();

    let mut client = connect_immudb(&config).await?;
    info!("Successfully connected to Immudb.");
    let mut last_connection_time = Instant::now(); // Initialize connection timer

    client
        .open_session(&board_name)
        .await
        .with_context(|| format!("Failed to open session to board: {}", board_name))?;
    info!(%board_name, "Successfully opened session to board.");

    let mut total_rows_fetched = 0;
    let mut activity_log_written_counts: HashMap<String, usize> = HashMap::new();

    // --- Pagination Logic ---
    let immudb_page_limit: usize = cli.limit.unwrap_or(DEFAULT_SQL_LIMIT); // Renamed for clarity
    let mut current_offset: usize = cli.offset.unwrap_or(0);
    let mut continue_fetching = true;

    info!(
        page_limit = immudb_page_limit,
        "Starting paginated log retrieval from Immudb."
    );

    while continue_fetching {
        // --- Periodic Reconnection Logic ---
        if cli.reconnect_interval_seconds > 0 {
            if last_connection_time.elapsed() >= Duration::from_secs(cli.reconnect_interval_seconds)
            {
                info!(
                    interval = cli.reconnect_interval_seconds,
                    "Reconnect interval reached. Attempting to reconnect to Immudb."
                );

                // Best effort to close the old session
                match client.close_session().await {
                    Ok(_) => info!("Old Immudb session closed successfully before reconnecting."),
                    Err(e) => {
                        warn!(error = %e, "Failed to close old Immudb session (might have already expired or been invalid). Proceeding with reconnection.")
                    }
                }
                // Also good to ensure client is dropped if logout/close isn't enough, though assigning a new one should do it.
                // For immudb-rs, creating a new Client and logging in effectively gives a new connection.

                client = connect_immudb(&config).await.context(
                    "Failed to re-establish connection to Immudb during periodic reconnect",
                )?;
                info!("Successfully reconnected to Immudb.");

                client.open_session(&board_name).await.with_context(|| {
                    format!(
                        "Failed to open session on new connection to board: {}",
                        board_name
                    )
                })?;
                info!(%board_name, "Successfully opened session to board on new connection.");
                last_connection_time = Instant::now(); // Reset the timer
            }
        }

        let sql = format!(
            "SELECT id, created, statement_timestamp, statement_kind, message, user_id \
             FROM electoral_log_messages \
             ORDER BY id ASC \
             LIMIT {} OFFSET {}",
            immudb_page_limit,
            current_offset // Use page_limit here
        );

        debug!(
            offset = current_offset,
            "Executing paginated SQL query: {}", sql
        );

        let response_stream = match client.streaming_sql_query(&sql, Vec::new()).await {
            Ok(rs) => rs,
            Err(e) => {
                error!(error = %e, offset = current_offset, "Failed to execute paginated streaming_sql_query.");
                return Err(e)
                    .with_context(|| format!("Immudb query failed at offset {}", current_offset));
            }
        };

        let mut stream = response_stream.into_inner();
        let mut rows_in_current_page = 0;
        let mut received_data_in_current_page = false;

        while let Some(batch_result) = stream.next().await {
            match batch_result {
                Ok(sql_query_result_batch) => {
                    if !sql_query_result_batch.rows.is_empty() {
                        received_data_in_current_page = true;
                    } else {
                        debug!(
                            offset = current_offset,
                            "Received an empty batch in stream for current page."
                        );
                    }

                    for individual_row in &sql_query_result_batch.rows {
                        rows_in_current_page += 1;
                        total_rows_fetched += 1;

                        if total_rows_fetched % 1000 == 0 {
                            // Consider making this logging interval configurable or less frequent
                            info!(total_rows_fetched, "Processed rows from stream...");
                        }

                        let elog_row = match ElectoralLogRow::try_from(individual_row) {
                            Ok(elog_row) => elog_row,
                            Err(e) => {
                                warn!(error = %e, raw_row = ?individual_row, "Failed to parse ImmudbRow into ElectoralLogRow from stream batch.");
                                continue;
                            }
                        };
                        debug!(
                            log_id = elog_row.id,
                            "Successfully parsed ElectoralLogRow from stream batch."
                        );

                        let activity_log_row = match ActivityLogRow::try_from(elog_row.clone()) {
                            Ok(activity_log_row) => activity_log_row,
                            Err(e) => {
                                warn!(log_id = elog_row.id, error = %e, "Failed to transform ElectoralLogRow.");
                                continue;
                            }
                        };
                        let binary_message = general_purpose::STANDARD_NO_PAD
                            .decode(&elog_row.data) // Pass by reference
                            .with_context(|| {
                                format!(
                                    "Error reading base64 message into binary for log_id: {}",
                                    elog_row.id
                                )
                            })?;

                        let deserialized_message = Message::strand_deserialize(&binary_message)
                            .with_context(|| {
                                format!("Error deserializing message for log_id: {}", elog_row.id)
                            })?;

                        let extracted_election_id_opt = deserialized_message.election_id_string();

                        let filename_stem_key = match &extracted_election_id_opt {
                            Some(id) => config
                                .elections
                                .get(id)
                                .map(|s| s.as_str())
                                .unwrap_or(id) // Use the ID itself if not found in config map
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
                            // Write header row upon creating a new CSV
                            // Assuming ActivityLogRow has a method to get headers or you serialize a header struct
                            // For example:
                            // if let Some(writer) = csv_writers.get_mut(&sanitized_stem) {
                            //     writer.write_record(&["header1", "header2", ...])?;
                            // }
                            // Or if using serde, you might need to write headers based on struct fields.
                            // If `csv::Writer::serialize` writes headers automatically on first serialize, this is fine.
                            // Typically, `WriterBuilder::has_headers(true)` (default) handles this if the struct has serde attributes.
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
                Err(e) => {
                    error!(error = %e, offset = current_offset, "Error receiving batch from Immudb stream for current page.");
                    continue_fetching = false;
                    break;
                }
            }
        }

        if !continue_fetching {
            break;
        }

        info!(
            offset = current_offset,
            rows_fetched_this_page = rows_in_current_page,
            limit = immudb_page_limit, // Use page_limit here
            "Finished processing page from Immudb stream."
        );

        if !received_data_in_current_page || rows_in_current_page < immudb_page_limit {
            // Use page_limit
            debug!(
                "Fetched {} rows in the last page (page limit was {}). Assuming end of data.",
                rows_in_current_page,
                immudb_page_limit // Use page_limit
            );
            continue_fetching = false;
        } else {
            current_offset += immudb_page_limit; // Use page_limit
        }
    }

    info!("Finished processing all pages from Immudb.");

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

    // Attempt to close the final session, but don't fail the whole process if it errors out
    // as data processing is already complete.
    match client.close_session().await {
        Ok(_) => info!("Successfully closed immudb session."),
        Err(e) => {
            warn!(error = %e, "Failed to close immudb session after processing. This might be due to a prior session timeout but data processing should be complete.")
        }
    }

    Ok(())
}
