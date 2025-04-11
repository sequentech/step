use aws_config::meta::region::RegionProviderChain;
use aws_sdk_sns::Client as SnsClient;
use futures_util::{SinkExt, StreamExt};
use reqwest::Client as HttpClient; // Alias to avoid confusion with WebSocket client
use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex; // To protect access to last_timestamp and stream state
use tokio::time::{interval, sleep};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use url::Url;

// --- Constants ---
const DEFAULT_STATE_FILE: &str = "loki_consumer.state";
const STATE_SAVE_INTERVAL_SECS: u64 = 60; // How often to save the timestamp

// --- Data Structures ---

// For WebSocket /loki/api/v1/tail response
#[derive(Deserialize, Debug, Clone)]
struct LokiTailResponse {
    streams: Vec<LokiStream>,
}

// For /loki/api/v1/query_range response
#[derive(Deserialize, Debug, Clone)]
struct LokiQueryRangeResponse {
    status: String,
    data: QueryRangeData,
}

#[derive(Deserialize, Debug, Clone)]
struct QueryRangeData {
    #[serde(rename = "resultType")]
    result_type: String, // Should be "streams"
    result: Vec<QueryResultStream>,
    // stats: serde_json::Value, // Ignore stats for now
}

// Common stream structure used by both tail and query_range results
#[derive(Deserialize, Debug, Clone)]
struct LokiStream {
    stream: HashMap<String, String>, // Labels
    values: Vec<[String; 2]>,       // [timestamp_ns_string, log_line_string]
}
// Alias QueryResultStream to LokiStream as they are compatible for our needs
type QueryResultStream = LokiStream;

// State for tracking pending phone numbers per stream
#[derive(Debug, Clone, Default)]
struct StreamState {
    pending_phone_number: Option<String>,
    // We could add timestamp here to prune old pending numbers, but keeping it simple for now
}
type StreamProcessingState = HashMap<String, StreamState>;


// --- Custom Error Type ---
#[derive(Error, Debug)]
enum LokiError {
    #[error("WebSocket connection error: {0}")]
    WebSocketError(#[from] tokio_tungstenite::tungstenite::Error),
    #[error("HTTP request error: {0}")]
    HttpRequestError(#[from] reqwest::Error),
    #[error("HTTP API Error: Status={status}, Body={body}")]
    HttpApiError { status: reqwest::StatusCode, body: String },
    #[error("URL parsing error: {0}")]
    UrlParseError(#[from] url::ParseError),
    #[error("Required environment variable not set: {0}")]
    EnvVarError(String),
    #[error("Failed to parse JSON response: {0}")]
    JsonParseError(#[from] serde_json::Error),
    #[error("Failed to parse integer: {0}")]
    ParseIntError(#[from] std::num::ParseIntError),
    #[error("Invalid URL scheme for WebSocket: {0}")]
    InvalidScheme(String),
    #[error("File I/O error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Loki API returned status '{status}' for query_range")]
    ApiError { status: String }, // For errors within successful JSON response
    #[error("AWS SDK Error: {0}")]
    AwsSdkError(String), // Generic wrapper for AWS errors
}

// --- AWS SNS Action ---

/// Sends an OTP message using AWS SNS.
/// Assumes AWS credentials are configured in the environment.
async fn send_otp_via_sns(phone_number: &str, message: &str) -> Result<(), LokiError> {
    println!("Attempting to send OTP to {}...", phone_number); // Mask number in real app
    let region_provider = RegionProviderChain::default_provider().or_else("us-east-1"); // Example region
    let config = aws_config::from_env().region(region_provider).load().await;
    let client = SnsClient::new(&config);

    // Extract only the OTP message part if needed, or send the whole line
    // Example: Assuming message format is "\t- message=Your OTP is XXXXXX and is valid..."
    let otp_message = message
        .split_once("=")
        .map(|(_, msg_part)| msg_part.trim())
        .unwrap_or(message); // Fallback to the full message

    match client
        .publish()
        .phone_number(phone_number)
        .message(otp_message)
        // Optionally set MessageAttributes (e.g., for Transactional SMS)
        // .message_attributes(
        //     "AWS.SNS.SMS.SMSType",
        //     aws_sdk_sns::types::MessageAttributeValue::builder()
        //         .data_type("String")
        //         .string_value("Transactional") // Or "Promotional"
        //         .build()?, // Handle potential build error
        // )
        .send()
        .await
    {
        Ok(output) => {
            println!(
                "Successfully sent message to {}. Message ID: {}",
                phone_number, // Mask number in real app
                output.message_id().unwrap_or("N/A")
            );
            Ok(())
        }
        Err(e) => {
            eprintln!("Failed to send SNS message to {}: {:?}", phone_number, e); // Mask number
            // Convert SDK error to our custom error type
            Err(LokiError::AwsSdkError(e.to_string()))
        }
    }
}


// --- State Management ---

/// Loads the last known timestamp from the state file.
async fn load_state(state_file_path: &str) -> Result<Option<u64>, LokiError> {
    // ... (load_state implementation remains the same)
    match File::open(state_file_path).await {
        Ok(mut file) => {
            let mut contents = String::new();
            file.read_to_string(&mut contents).await?;
            match contents.trim().parse::<u64>() {
                Ok(timestamp) => {
                    println!("Loaded last timestamp {} from state file {}", timestamp, state_file_path);
                    Ok(Some(timestamp))
                },
                Err(e) => {
                    eprintln!("Warning: Could not parse timestamp from state file {}: {}. Starting fresh.", state_file_path, e);
                    Ok(None)
                }
            }
        }
        Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => {
            println!("State file {} not found. Starting fresh.", state_file_path);
            Ok(None)
        }
        Err(e) => Err(LokiError::IoError(e)),
    }
}

/// Saves the given timestamp to the state file.
async fn save_state(state_file_path: &str, timestamp: u64) -> Result<(), LokiError> {
    // ... (save_state implementation remains the same)
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(state_file_path)
        .await?;
    file.write_all(timestamp.to_string().as_bytes()).await?;
    Ok(())
}


// --- Log Processing ---

/// Generates a canonical string key for a stream based on its labels.
fn get_stream_key(stream_labels: &HashMap<String, String>) -> String {
    let mut labels: Vec<_> = stream_labels.iter().collect();
    // Sort by label key for consistency
    labels.sort_by(|a, b| a.0.cmp(b.0));
    // Format into a reproducible string (e.g., key1="value1",key2="value2")
    labels
        .iter()
        .map(|(k, v)| format!("{}={:?}", k, v)) // Use debug format for value to handle quotes
        .collect::<Vec<_>>()
        .join(",")
}


/// Processes a batch of streams, pairing phoneNumber and message lines within each stream.
/// Updates the last timestamp and triggers SNS action.
/// Returns the latest timestamp processed within this batch.
async fn process_log_streams(
    streams: Vec<LokiStream>,
    print_raw_logs: bool,
    mut current_last_ts: u64, // Pass current last timestamp known *before* this batch
    stream_state: Arc<Mutex<StreamProcessingState>>, // Shared state for pending numbers
) -> u64 { // Return the new last timestamp *after* this batch
    let mut latest_ts_in_batch = current_last_ts;

    for stream in streams {
        let stream_key = get_stream_key(&stream.stream);
        let labels = &stream.stream; // For potential logging

        // Sort values by timestamp to process chronologically within the stream
        // Clone values to sort them, or process directly if Loki guarantees order (safer to sort)
        let mut sorted_values = stream.values.clone();
        sorted_values.sort_by(|a, b| a[0].cmp(&b[0]));

        for value_pair in sorted_values {
            match value_pair[0].parse::<u64>() {
                Ok(ts_ns) => {
                    // Only process if strictly newer than the last timestamp known before this batch
                    if ts_ns > current_last_ts {
                        let log_line = &value_pair[1];

                        // Optionally print the raw log
                        if print_raw_logs {
                            println!("[{}] Labels: {:?} | Raw Line: {}", ts_ns, labels, log_line);
                        }

                        // --- Pairing Logic ---
                        // Trim whitespace before checking prefixes
                        let trimmed_line = log_line.trim();

                        if let Some(num_part) = trimmed_line.strip_prefix("- phoneNumber=") {
                            let phone_number = num_part.trim().to_string();
                            // Lock state, store pending number for this stream
                            let mut state = stream_state.lock().await;
                            println!("Found phone number for stream [{}]: {}", stream_key, phone_number); // Mask number
                            // Insert or update the state for this stream
                            state.entry(stream_key.clone()).or_default().pending_phone_number = Some(phone_number);
                            // Unlock happens when state goes out of scope
                        } else if let Some(msg_part) = trimmed_line.strip_prefix("- message=") {
                            let message = msg_part.trim().to_string();
                            // Lock state, check for pending number
                            let mut state = stream_state.lock().await;
                            // Check if an entry exists and if it has a pending number
                            if let Some(stream_data) = state.get_mut(&stream_key) {
                                if let Some(phone_number) = stream_data.pending_phone_number.take() {
                                    // Found a pair! Clear pending number and trigger action
                                    println!("Found message for stream [{}], pairing with pending number {}", stream_key, phone_number); // Mask number
                                    // Spawn SNS task so it doesn't block log processing
                                    tokio::spawn(async move {
                                        if let Err(e) = send_otp_via_sns(&phone_number, &message).await {
                                            eprintln!("SNS Error: {}", e);
                                        }
                                    });
                                } else {
                                     // Message found, but no number was pending for this stream
                                     // eprintln!("Warning: Found message for stream [{}] but no pending phone number.", stream_key);
                                }
                            } else {
                                // Message found, but no state entry for this stream (e.g., message came before phone)
                                // eprintln!("Warning: Found message for stream [{}] but no prior state.", stream_key);
                            }
                            // Unlock happens when state goes out of scope
                        }

                        // Update the latest timestamp seen for persistence, regardless of pairing success
                        latest_ts_in_batch = latest_ts_in_batch.max(ts_ns);
                    }
                }
                Err(e) => {
                    eprintln!("Warning: Could not parse timestamp '{}' in stream {:?}: {}", value_pair[0], stream.stream, e);
                }
            }
        }
    }
    // Return the latest timestamp found across all processed lines in this batch
    latest_ts_in_batch
}


// --- Historical Catch-up ---

/// Fetches logs using query_range and processes them using the pairing logic.
async fn fetch_historical_logs(
    http_client: &HttpClient,
    loki_base_url_str: &str,
    loki_query: &str,
    start_ns: u64,
    end_ns: u64,
    print_raw_logs: bool,
    current_last_ts: u64,
    stream_state: Arc<Mutex<StreamProcessingState>>, // Pass shared state
) -> Result<u64, LokiError> {
    if start_ns >= end_ns {
         println!("Skipping historical fetch: start time {}ns is not before end time {}ns", start_ns, end_ns);
         return Ok(current_last_ts);
    }
    println!("Fetching historical logs from {}ns to {}ns...", start_ns, end_ns);
    let mut query_range_url = Url::parse(loki_base_url_str)?;
    query_range_url.set_path("/loki/api/v1/query_range");
    query_range_url.query_pairs_mut()
        .append_pair("query", loki_query)
        .append_pair("start", &start_ns.to_string())
        .append_pair("end", &end_ns.to_string())
        // Ensure direction is forward for chronological processing if needed, though sorting is done later
        // .append_pair("direction", "forward")
        .append_pair("limit", "5000"); // Adjust limit as needed

    let response = http_client.get(query_range_url).send().await?;
    if !response.status().is_success() {
        let status = response.status();
        let body_text = response.text().await.unwrap_or_else(|_| "Failed to read error body".to_string());
        eprintln!("Error response from query_range: {} - {}", status, body_text);
        return Err(LokiError::HttpApiError { status, body: body_text });
    }
    let response_data = response.json::<LokiQueryRangeResponse>().await?;
    if response_data.status != "success" {
        eprintln!("Loki query_range API returned status: {}", response_data.status);
        return Err(LokiError::ApiError { status: response_data.status });
    }
    if response_data.data.result_type != "streams" {
        eprintln!("Warning: Expected resultType 'streams' from query_range, got '{}'", response_data.data.result_type);
        return Ok(current_last_ts);
    }

    println!("Processing {} streams from historical fetch.", response_data.data.result.len());
    let new_last_ts = process_log_streams( // Call the updated processing function
        response_data.data.result,
        print_raw_logs,
        current_last_ts,
        stream_state, // Pass the stream state map
    ).await; // Now async

    println!("Finished processing historical logs. New last timestamp: {}", new_last_ts);
    Ok(new_last_ts)
}


// --- WebSocket Tailing ---

/// Establishes a WebSocket connection and listens for messages.
async fn connect_and_listen(
    loki_base_url_str: &str,
    loki_query: &str,
    tail_limit_str: &str,
    start_timestamp_ns: u64,
    print_raw_logs: bool,
    state_file_path: Arc<String>,
    last_processed_ts: Arc<Mutex<u64>>, // For saving state
    stream_state: Arc<Mutex<StreamProcessingState>>, // For pairing logic state
) -> Result<(), LokiError> {
    // --- Construct WebSocket URL ---
    let mut base_url = Url::parse(loki_base_url_str)?;
    let scheme = match base_url.scheme() { "http" => "ws", "https" => "wss", s => return Err(LokiError::InvalidScheme(s.to_string())), };
    base_url.set_scheme(scheme).map_err(|_| LokiError::InvalidScheme("Failed to set scheme".into()))?;
    base_url.set_path("/loki/api/v1/tail");
    base_url.query_pairs_mut()
        .append_pair("query", loki_query)
        .append_pair("limit", tail_limit_str)
        .append_pair("start", &start_timestamp_ns.to_string());

    let ws_url = base_url;
    println!("Connecting to WebSocket: {}", ws_url);

    // --- Connect to WebSocket ---
    let (ws_stream, response) = connect_async(ws_url.as_str()).await?;
    println!("WebSocket handshake successful! Status: {}", response.status());
    let (mut write, mut read) = ws_stream.split();

    // --- Periodic State Saving Task ---
    let save_interval = Duration::from_secs(STATE_SAVE_INTERVAL_SECS);
    let mut interval_timer = interval(save_interval);
    let state_path_clone = Arc::clone(&state_file_path);
    let last_ts_save_clone = Arc::clone(&last_processed_ts); // Clone Arc for save task
    let mut save_task = tokio::spawn(async move {
        loop {
            interval_timer.tick().await;
            let ts_to_save = { *last_ts_save_clone.lock().await };
            if let Err(e) = save_state(&state_path_clone, ts_to_save).await {
                 eprintln!("Error saving state periodically: {}", e);
            }
        }
    });

    // --- Message Processing Loop ---
    loop {
       tokio::select! {
           biased;
           message_result = read.next() => {
               match message_result {
                   Some(Ok(message)) => {
                       match message {
                           Message::Text(text) => {
                               match serde_json::from_str::<LokiTailResponse>(&text) {
                                   Ok(loki_data) => {
                                       // Lock the shared timestamp *before* processing
                                       let mut ts_guard = last_processed_ts.lock().await;
                                       let new_last_ts = process_log_streams( // Call updated processing function
                                           loki_data.streams,
                                           print_raw_logs,
                                           *ts_guard, // Pass current value
                                           Arc::clone(&stream_state), // Pass stream state
                                       ).await; // Now async
                                       *ts_guard = new_last_ts; // Update shared value
                                   }
                                   Err(e) => { eprintln!("Error parsing Loki JSON from WebSocket: {}", e); eprintln!("Raw message content: {}", text); }
                               }
                           }
                           Message::Ping(ping_data) => {
                               if write.send(Message::Pong(ping_data)).await.is_err() {
                                    eprintln!("Failed to send Pong, connection likely closed.");
                                    save_task.abort();
                                    return Err(LokiError::WebSocketError(tokio_tungstenite::tungstenite::Error::ConnectionClosed));
                               }
                           }
                           Message::Close(close_frame) => { println!("Received Close frame: {:?}", close_frame); save_task.abort(); return Ok(()); }
                           _ => {} // Ignore other message types
                       }
                   }
                   Some(Err(e)) => { eprintln!("Error reading from WebSocket: {}", e); save_task.abort(); return Err(LokiError::WebSocketError(e)); }
                   None => { println!("WebSocket stream ended."); save_task.abort(); return Ok(()); }
               }
           }
           _ = &mut save_task => {
                eprintln!("Save state task completed unexpectedly.");
                return Err(LokiError::IoError(std::io::Error::new(std::io::ErrorKind::Other, "Save state task ended unexpectedly")));
           }
       }
    }
}


// --- Main Application Entry Point ---

#[tokio::main]
async fn main() {
    // --- Configuration ---
    println!("Reading configuration...");
    let loki_base_url_str = env::var("LOKI_URL").expect("LOKI_URL not set");
    let loki_query = env::var("LOKI_QUERY").expect("LOKI_QUERY not set");
    let tail_limit_str = env::var("LOKI_TAIL_LIMIT").unwrap_or_else(|_| "100".to_string());
    let reconnect_delay_secs: u64 = env::var("RECONNECT_DELAY_SECS").unwrap_or_else(|_| "5".to_string()).parse().unwrap_or(5);
    let log_level = env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string()).to_lowercase();
    let print_raw_logs = log_level == "trace";
    let state_file_path = env::var("STATE_FILE").unwrap_or_else(|_| DEFAULT_STATE_FILE.to_string());
    let state_file_path_arc = Arc::new(state_file_path.clone());

    println!("--- Configuration ---");
    println!("Loki URL: {}", loki_base_url_str);
    println!("Loki Query: {}", loki_query);
    println!("Log Level (Trace Print): {}", print_raw_logs);
    println!("State File: {}", state_file_path);
    println!("---------------------");

    // --- Initialize State ---
    let initial_timestamp_ns = match load_state(&state_file_path).await {
        Ok(Some(ts)) => ts,
        Ok(None) => (chrono::Utc::now() - chrono::Duration::seconds(10)).timestamp_nanos_opt().unwrap_or(0) as u64,
        Err(e) => { eprintln!("Error loading state: {}. Starting fresh.", e); (chrono::Utc::now() - chrono::Duration::seconds(10)).timestamp_nanos_opt().unwrap_or(0) as u64 }
    };
    // Timestamp for persistence
    let last_processed_ts = Arc::new(Mutex::new(initial_timestamp_ns));
    // Shared state map for pending phone numbers per stream
    let stream_state = Arc::new(Mutex::new(StreamProcessingState::new()));

    // Create the HTTP client once
    let http_client = HttpClient::new();

    // --- Perform Initial Historical Catch-up ---
    let catch_up_start_ns_initial = initial_timestamp_ns.saturating_add(1);
    let catch_up_end_ns_initial = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64;
    if catch_up_start_ns_initial < catch_up_end_ns_initial {
        println!("Performing initial historical catch-up...");
        match fetch_historical_logs(
            &http_client,
            &loki_base_url_str,
            &loki_query,
            catch_up_start_ns_initial,
            catch_up_end_ns_initial,
            print_raw_logs,
            initial_timestamp_ns,
            Arc::clone(&stream_state), // Pass stream state map
        ).await {
            Ok(new_last_ts) => {
                 if new_last_ts > initial_timestamp_ns {
                    let mut ts_guard = last_processed_ts.lock().await;
                    *ts_guard = new_last_ts;
                    println!("Initial catch-up complete. Updated last timestamp to: {}", new_last_ts);
                    if let Err(e) = save_state(&state_file_path, new_last_ts).await { eprintln!("Error saving state after initial catch-up: {}", e); }
                 } else { println!("Initial catch-up complete. No new logs found."); }
            }
            Err(e) => { eprintln!("Error during initial historical catch-up: {}. Proceeding without catch-up.", e); }
        }
        println!("---------------------");
    } else { println!("No initial historical catch-up needed."); println!("---------------------"); }


    // --- Start Main WebSocket Loop ---
    println!("Starting WebSocket listener...");
    loop { // Outer loop handles reconnections and error recovery
        let start_ts_for_ws = { *last_processed_ts.lock().await };

        match connect_and_listen(
            &loki_base_url_str,
            &loki_query,
            &tail_limit_str,
            start_ts_for_ws,
            print_raw_logs,
            Arc::clone(&state_file_path_arc),
            Arc::clone(&last_processed_ts),
            Arc::clone(&stream_state), // Pass stream state map
        ).await {
            Ok(()) => {
                println!("WebSocket connection closed cleanly. Reconnecting...");
                sleep(Duration::from_secs(1)).await;
            }
            Err(e) => {
                eprintln!("Error in WebSocket connection/processing: {}", e);
                println!("Attempting historical catch-up due to error...");

                // --- Perform Historical Catch-up on Error ---
                let last_known_ts_before_error = { *last_processed_ts.lock().await };
                let catch_up_start_ns_error = last_known_ts_before_error.saturating_add(1);
                let catch_up_end_ns_error = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64;

                match fetch_historical_logs(
                    &http_client,
                    &loki_base_url_str,
                    &loki_query,
                    catch_up_start_ns_error,
                    catch_up_end_ns_error,
                    print_raw_logs,
                    last_known_ts_before_error,
                    Arc::clone(&stream_state), // Pass stream state map
                ).await {
                    Ok(new_last_ts) => {
                         if new_last_ts > last_known_ts_before_error {
                            let mut ts_guard = last_processed_ts.lock().await;
                            *ts_guard = new_last_ts;
                            println!("Error recovery catch-up complete. Updated last timestamp to: {}", new_last_ts);
                            if let Err(e_save) = save_state(&state_file_path, new_last_ts).await { eprintln!("Error saving state after error recovery catch-up: {}", e_save); }
                         } else { println!("Error recovery catch-up complete. No new logs found."); }
                    }
                    Err(e_catchup) => { eprintln!("Error during error recovery catch-up: {}. Proceeding to reconnect.", e_catchup); }
                }
                println!("Attempting WebSocket reconnect in {} seconds...", reconnect_delay_secs);
                sleep(Duration::from_secs(reconnect_delay_secs)).await;
            }
        }
    }
    // Main loop is infinite
}
