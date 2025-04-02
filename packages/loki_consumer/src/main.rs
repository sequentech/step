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
use tokio::sync::Mutex; // To protect access to last_timestamp between tasks
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


// --- Custom Error Type ---
#[derive(Error, Debug)]
enum LokiError {
    #[error("WebSocket connection error: {0}")]
    WebSocketError(#[from] tokio_tungstenite::tungstenite::Error),
    #[error("HTTP request error: {0}")]
    HttpRequestError(#[from] reqwest::Error),
    #[error("HTTP API Error: Status={status}, Body={body}")] // FIX: Added specific variant for HTTP errors
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
}

// --- Event Consumer Trait and Implementations ---
// (Remains the same)
trait LogConsumer: Send + Sync {
    fn name(&self) -> String;
    fn consume(&self, log_line: &str, labels: &HashMap<String, String>);
}
struct KeywordConsumer { keyword: String }
impl KeywordConsumer { fn new(keyword: &str) -> Self { KeywordConsumer { keyword: keyword.to_lowercase() } } }
impl LogConsumer for KeywordConsumer {
    fn name(&self) -> String { format!("KeywordConsumer({})", self.keyword) }
    fn consume(&self, log_line: &str, labels: &HashMap<String, String>) {
        if log_line.to_lowercase().contains(&self.keyword) {
            println!("--- !!! [{}] Found keyword '{}' in log from {:?}: {} ---", self.name(), self.keyword, labels, log_line);
        }
    }
}


// --- State Management ---

/// Loads the last known timestamp from the state file.
/// Returns Ok(None) if the file doesn't exist or is invalid.
async fn load_state(state_file_path: &str) -> Result<Option<u64>, LokiError> {
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
                    Ok(None) // Treat parse error as no state
                }
            }
        }
        Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => {
            println!("State file {} not found. Starting fresh.", state_file_path);
            Ok(None) // No state file, start fresh
        }
        Err(e) => Err(LokiError::IoError(e)), // Other file read error
    }
}

/// Saves the given timestamp to the state file, overwriting previous content.
async fn save_state(state_file_path: &str, timestamp: u64) -> Result<(), LokiError> {
    // Use OpenOptions to create or truncate the file
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(state_file_path)
        .await?;
    file.write_all(timestamp.to_string().as_bytes()).await?;
    // println!("DEBUG: Saved timestamp {} to state file {}", timestamp, state_file_path); // Optional debug log
    Ok(())
}


// --- Log Processing ---

/// Processes a batch of streams (from either tail or query_range)
/// Updates the last timestamp and calls consumers.
/// Returns the latest timestamp processed within this batch.
fn process_log_streams(
    streams: Vec<LokiStream>,
    consumers: &[Arc<Box<dyn LogConsumer>>],
    print_raw_logs: bool,
    mut current_last_ts: u64, // Pass current last timestamp known *before* this batch
) -> u64 { // Return the new last timestamp *after* this batch
    let mut latest_ts_in_batch = current_last_ts;
    for stream in streams {
        for value_pair in stream.values {
            match value_pair[0].parse::<u64>() {
                Ok(ts_ns) => {
                    // Only process if strictly newer than the last timestamp known before this batch
                    if ts_ns > current_last_ts {
                        let log_line = &value_pair[1];
                        let labels = &stream.stream;

                        // Optionally print the raw log based on log level
                        if print_raw_logs {
                            println!("[{}] Labels: {:?} | Log: {}", ts_ns, labels, log_line);
                        }
                        // Always pass to consumers
                        for consumer in consumers {
                            consumer.consume(log_line, labels);
                        }
                        // Update the latest timestamp seen within this specific batch processing run
                        latest_ts_in_batch = latest_ts_in_batch.max(ts_ns);
                    }
                }
                Err(e) => {
                    eprintln!("Warning: Could not parse timestamp '{}': {}", value_pair[0], e);
                }
            }
        }
    }
    // Return the latest timestamp found in this batch, or the original if no newer logs were found/processed
    latest_ts_in_batch
}


// --- Historical Catch-up ---

/// Fetches logs using query_range and processes them.
/// Returns the timestamp of the last log processed during catch-up.
async fn fetch_historical_logs(
    http_client: &HttpClient,
    loki_base_url_str: &str,
    loki_query: &str,
    start_ns: u64, // Start of the catch-up range (exclusive)
    end_ns: u64,   // End of the catch-up range (inclusive)
    consumers: &[Arc<Box<dyn LogConsumer>>],
    print_raw_logs: bool,
    current_last_ts: u64, // The timestamp *before* starting catch-up
) -> Result<u64, LokiError> { // Return the updated last timestamp
    println!(
        "Fetching historical logs from {} to {}...",
        start_ns, end_ns
    );
    let mut query_range_url = Url::parse(loki_base_url_str)?;
    query_range_url.set_path("/loki/api/v1/query_range");
    query_range_url
        .query_pairs_mut()
        .append_pair("query", loki_query)
        .append_pair("start", &start_ns.to_string())
        .append_pair("end", &end_ns.to_string())
        .append_pair("limit", "5000"); // Adjust limit as needed

    let response = http_client.get(query_range_url).send().await?;

    // Handle HTTP errors from Loki
    if !response.status().is_success() {
        let status = response.status();
        // Read body text for error reporting *before* potentially consuming response with error_for_status
        let body_text = response.text().await.unwrap_or_else(|_| "Failed to read error body".to_string());
        eprintln!("Error response from query_range: {} - {}", status, body_text);
        // FIX: Return the specific HttpApiError variant
        return Err(LokiError::HttpApiError { status, body: body_text });
    }

    // Parse the successful JSON response (handles JSON parsing errors)
    let response_data = response.json::<LokiQueryRangeResponse>().await?;

    // Check the status field within the JSON payload
    if response_data.status != "success" {
        eprintln!("Loki query_range API returned status: {}", response_data.status);
        return Err(LokiError::ApiError { status: response_data.status });
    }

    // Ensure the result type is streams, as expected
    if response_data.data.result_type != "streams" {
        eprintln!("Warning: Expected resultType 'streams' from query_range, got '{}'", response_data.data.result_type);
        return Ok(current_last_ts); // Return original timestamp if type is wrong
    }

    println!("Processing {} streams from historical fetch.", response_data.data.result.len());
    // Process the streams and get the timestamp of the last log within this historical batch
    let new_last_ts = process_log_streams(
        response_data.data.result,
        consumers,
        print_raw_logs,
        current_last_ts, // Pass the timestamp known *before* this fetch
    );

    println!("Finished processing historical logs. New last timestamp: {}", new_last_ts);
    Ok(new_last_ts) // Return the updated timestamp
}


// --- WebSocket Tailing ---

/// Establishes a WebSocket connection and listens for messages.
/// Handles periodic state saving in a background task.
async fn connect_and_listen(
    loki_base_url_str: &str,
    loki_query: &str,
    tail_limit_str: &str,
    start_timestamp_ns: u64, // Timestamp to start WebSocket from
    consumers: &[Arc<Box<dyn LogConsumer>>],
    print_raw_logs: bool,
    state_file_path: Arc<String>, // Use Arc for sharing path with save task
    last_processed_ts: Arc<Mutex<u64>>, // Use Arc<Mutex> to share/update the timestamp
) -> Result<(), LokiError> { // Return only errors; clean exit/reconnect handled by outer loop

    // --- Construct WebSocket URL ---
    let mut base_url = Url::parse(loki_base_url_str)?;
    let scheme = match base_url.scheme() {
        "http" => "ws", "https" => "wss",
        s => return Err(LokiError::InvalidScheme(s.to_string())),
    };
    base_url.set_scheme(scheme).map_err(|_| LokiError::InvalidScheme("Failed to set scheme".into()))?;
    base_url.set_path("/loki/api/v1/tail");
    base_url.query_pairs_mut()
        .append_pair("query", loki_query)
        .append_pair("limit", tail_limit_str)
        .append_pair("start", &start_timestamp_ns.to_string()); // Use the potentially updated start time

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
    let last_ts_clone = Arc::clone(&last_processed_ts);

    // Spawn a background task for saving state periodically
    // FIX: Declare save_task as mutable
    let mut save_task = tokio::spawn(async move {
        loop {
            interval_timer.tick().await; // Wait for the next interval
            let ts_to_save = { *last_ts_clone.lock().await }; // Read timestamp under lock
            if let Err(e) = save_state(&state_path_clone, ts_to_save).await {
                 eprintln!("Error saving state periodically: {}", e);
                 // Consider if this error should stop the main loop or just be logged
            }
        }
    });


    // --- Message Processing Loop ---
    loop {
       // Use tokio::select! to handle both WebSocket messages and potential task completion/errors
       tokio::select! {
           // Biased select prioritizes checking for messages
           biased;
           // Read next message from WebSocket stream
           message_result = read.next() => {
               match message_result {
                   Some(Ok(message)) => { // Successfully received a message
                       match message {
                           Message::Text(text) => {
                               // Attempt to parse the JSON payload
                               match serde_json::from_str::<LokiTailResponse>(&text) {
                                   Ok(loki_data) => {
                                       // Lock the shared timestamp, process logs, update timestamp
                                       let mut ts_guard = last_processed_ts.lock().await;
                                       let new_last_ts = process_log_streams(
                                           loki_data.streams,
                                           consumers,
                                           print_raw_logs,
                                           *ts_guard, // Pass current value known before this message
                                       );
                                       *ts_guard = new_last_ts; // Update shared value under lock
                                   }
                                   Err(e) => {
                                       // Log JSON parsing errors
                                       eprintln!("Error parsing Loki JSON from WebSocket: {}", e);
                                       eprintln!("Raw message content: {}", text);
                                   }
                               }
                           }
                           Message::Ping(ping_data) => {
                               // Respond to Ping messages to keep the connection alive
                               if write.send(Message::Pong(ping_data)).await.is_err() {
                                    eprintln!("Failed to send Pong, connection likely closed.");
                                    save_task.abort(); // Stop save task on connection error
                                    // Return error to trigger reconnect in the outer loop
                                    return Err(LokiError::WebSocketError(tokio_tungstenite::tungstenite::Error::ConnectionClosed));
                               }
                           }
                           Message::Close(close_frame) => {
                               // Server initiated close
                               println!("Received Close frame: {:?}", close_frame);
                               save_task.abort(); // Stop save task on clean close
                               return Ok(()); // Clean exit from this function
                           }
                           // Ignore other message types
                           Message::Binary(_) | Message::Pong(_) | Message::Frame(_) => {}
                       }
                   }
                   Some(Err(e)) => { // Error reading from WebSocket stream
                       eprintln!("Error reading from WebSocket: {}", e);
                       save_task.abort(); // Stop save task on error
                       return Err(LokiError::WebSocketError(e)); // Propagate error to outer loop
                   }
                   None => { // WebSocket stream ended (connection closed)
                       println!("WebSocket stream ended.");
                       save_task.abort(); // Stop save task
                       return Ok(()); // Treat as clean exit for reconnection purposes
                   }
               }
           }
           // Monitor the save task in case it finishes unexpectedly (e.g., panics)
           // FIX: Use &mut save_task here (requires save_task to be mutable)
           _ = &mut save_task => {
                eprintln!("Save state task completed unexpectedly.");
                // This indicates a problem; return an error to the outer loop
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
    let state_file_path_arc = Arc::new(state_file_path.clone()); // Use Arc for sharing path with save task

    println!("--- Configuration ---");
    println!("Loki URL: {}", loki_base_url_str);
    println!("Loki Query: {}", loki_query);
    println!("Log Level (Trace Print): {}", print_raw_logs);
    println!("State File: {}", state_file_path);
    println!("---------------------");

    // --- Initialize Consumers ---
    let consumers: Vec<Arc<Box<dyn LogConsumer>>> = vec![
        Arc::new(Box::new(KeywordConsumer::new("error"))),
        Arc::new(Box::new(KeywordConsumer::new("failed"))),
    ];
    println!("Initialized {} log consumers:", consumers.len());
    for consumer in &consumers { println!("  - {}", consumer.name()); }
    println!("---------------------");

    // --- Load Initial State ---
    // Determine the starting point: from state file or default (now - 10s)
    let initial_timestamp_ns = match load_state(&state_file_path).await {
        Ok(Some(ts)) => ts, // Use timestamp from file
        Ok(None) => (chrono::Utc::now() - chrono::Duration::seconds(10)).timestamp_nanos_opt().unwrap_or(0) as u64, // Default
        Err(e) => {
            eprintln!("Error loading state: {}. Starting fresh.", e); // Log error and default
            (chrono::Utc::now() - chrono::Duration::seconds(10)).timestamp_nanos_opt().unwrap_or(0) as u64
        }
    };
     // Use Arc<Mutex> for the timestamp shared between catch-up, websocket, and save tasks
    let last_processed_ts = Arc::new(Mutex::new(initial_timestamp_ns));


    // --- Perform Historical Catch-up (if applicable) ---
    let catch_up_start_ns = initial_timestamp_ns.saturating_add(1); // Start query just after last known state
    let catch_up_end_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64;

    // Only run catch-up if the loaded state timestamp is meaningfully before the current time
    if catch_up_start_ns < catch_up_end_ns {
        println!("Performing historical catch-up...");
        let http_client = HttpClient::new(); // Create HTTP client for query_range
        match fetch_historical_logs(
            &http_client,
            &loki_base_url_str,
            &loki_query,
            catch_up_start_ns, // Start after last known
            catch_up_end_ns,   // End now
            &consumers,
            print_raw_logs,
            initial_timestamp_ns, // Pass the timestamp known *before* catch-up
        ).await {
            Ok(new_last_ts) => {
                // Update the shared timestamp only if catch-up processed newer logs
                 if new_last_ts > initial_timestamp_ns {
                    let mut ts_guard = last_processed_ts.lock().await;
                    *ts_guard = new_last_ts;
                    println!("Catch-up complete. Updated last timestamp to: {}", new_last_ts);
                 } else {
                    println!("Catch-up complete. No new logs found in the historical range.");
                 }
                 // Save state immediately after successful catch-up
                 if let Err(e) = save_state(&state_file_path, new_last_ts).await {
                     eprintln!("Error saving state after catch-up: {}", e);
                 }
            }
            Err(e) => {
                eprintln!("Error during historical catch-up: {}. Proceeding without catch-up.", e);
                // Keep the initial_timestamp_ns in last_processed_ts if catch-up fails
            }
        }
        println!("---------------------");
    } else {
         println!("No historical catch-up needed (state file timestamp is recent).");
         println!("---------------------");
    }


    // --- Start Main WebSocket Loop ---
    println!("Starting WebSocket listener...");
    loop { // Outer loop handles reconnections
        let start_ts_for_ws = { *last_processed_ts.lock().await }; // Get latest timestamp for WS connection

        match connect_and_listen(
            &loki_base_url_str,
            &loki_query,
            &tail_limit_str,
            start_ts_for_ws, // Use potentially updated timestamp
            &consumers,
            print_raw_logs,
            Arc::clone(&state_file_path_arc), // Clone Arc for the state file path
            Arc::clone(&last_processed_ts), // Clone Arc for the shared timestamp
        ).await {
            Ok(()) => {
                // connect_and_listen exits cleanly (e.g., server closed connection)
                println!("WebSocket connection closed cleanly by server or client. Reconnecting...");
                // Add a small delay before reconnecting immediately after clean close?
                sleep(Duration::from_secs(1)).await;
            }
            Err(e) => {
                // If there was an error (e.g., connection drop), print it, wait, and try reconnecting.
                eprintln!("Error in WebSocket connection/processing: {}. Attempting reconnect in {} seconds...", e, reconnect_delay_secs);
                sleep(Duration::from_secs(reconnect_delay_secs)).await;
            }
        }
    }
    // Main loop is infinite in this design, so code below is unreachable
}
