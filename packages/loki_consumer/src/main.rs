use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::time::sleep;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use url::Url;

// --- Data Structures for Loki API Response ---
// These remain the same as they define the JSON structure within the messages

#[derive(Deserialize, Debug)]
struct LokiTailResponse {
    streams: Vec<LokiStream>,
}

#[derive(Deserialize, Debug)]
struct LokiStream {
    stream: HashMap<String, String>, // Labels
    values: Vec<[String; 2]>,       // [timestamp_ns_string, log_line_string]
}

// --- Custom Error Type ---
// Updated to include WebSocket errors

#[derive(Error, Debug)]
enum LokiError {
    #[error("WebSocket connection error: {0}")]
    WebSocketError(#[from] tokio_tungstenite::tungstenite::Error),
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
}

// --- Event Consumer Trait and Implementations ---
// These remain the same as they operate on parsed data

trait LogConsumer: Send + Sync {
    fn name(&self) -> String;
    fn consume(&self, log_line: &str, labels: &HashMap<String, String>);
}

struct KeywordConsumer {
    keyword: String,
}

impl KeywordConsumer {
    fn new(keyword: &str) -> Self {
        KeywordConsumer {
            keyword: keyword.to_lowercase(),
        }
    }
}

impl LogConsumer for KeywordConsumer {
    fn name(&self) -> String {
        format!("KeywordConsumer({})", self.keyword)
    }

    fn consume(&self, log_line: &str, labels: &HashMap<String, String>) {
        if log_line.to_lowercase().contains(&self.keyword) {
            println!(
                "--- !!! [{}] Found keyword '{}' in log from {:?}: {} ---",
                self.name(),
                self.keyword,
                labels,
                log_line
            );
        }
    }
}

// --- Main Application Logic ---

#[tokio::main]
async fn main() {
    // --- Configuration ---
    println!("Reading configuration from environment variables...");
    let loki_base_url_str = env::var("LOKI_URL")
        .map_err(|_| {
            LokiError::EnvVarError(
                "LOKI_URL environment variable not set. Example: http://localhost:3100".to_string(),
            )
        })
        .expect("Failed to get LOKI_URL"); // Simple panic on config error for brevity

    let loki_query = env::var("LOKI_QUERY")
        .map_err(|_| {
            LokiError::EnvVarError(
                "LOKI_QUERY environment variable not set. Example: {job=\"myapp\"}".to_string(),
            )
        })
        .expect("Failed to get LOKI_QUERY");

    let tail_limit_str = env::var("LOKI_TAIL_LIMIT").unwrap_or_else(|_| "100".to_string());
    // Note: Limit might behave differently or be less relevant for WebSocket streams vs polling
    let reconnect_delay_secs: u64 = env::var("RECONNECT_DELAY_SECS")
        .unwrap_or_else(|_| "5".to_string())
        .parse()
        .unwrap_or(5);

    // --- Initialize Consumers ---
    let consumers: Vec<Arc<Box<dyn LogConsumer>>> = vec![
        Arc::new(Box::new(KeywordConsumer::new("error"))),
        Arc::new(Box::new(KeywordConsumer::new("failed"))),
        // Add more consumers here
    ];
    println!("Initialized {} log consumers:", consumers.len());
    for consumer in &consumers {
        println!("  - {}", consumer.name());
    }


    // --- WebSocket Connection Loop ---
    // Keep track of the last timestamp for potential reconnections
    let mut last_timestamp_ns: u64 = (chrono::Utc::now() - chrono::Duration::seconds(10))
        .timestamp_nanos_opt()
        .unwrap_or(0) as u64;

    loop { // Outer loop for handling reconnections
        match connect_and_listen(
            &loki_base_url_str,
            &loki_query,
            &tail_limit_str,
            last_timestamp_ns,
            &consumers,
        ).await {
            Ok(last_ts) => {
                // If connect_and_listen exits cleanly (e.g., server closed connection),
                // update timestamp and try reconnecting immediately.
                println!("WebSocket connection closed cleanly. Reconnecting...");
                last_timestamp_ns = last_ts.saturating_add(1); // Use last known timestamp + 1ns
            }
            Err(e) => {
                // If there was an error, print it, wait, and try reconnecting.
                eprintln!("Error during WebSocket communication: {}. Attempting reconnect in {} seconds...", e, reconnect_delay_secs);
                // Keep the same last_timestamp_ns on error, as we might not have processed new logs
                sleep(Duration::from_secs(reconnect_delay_secs)).await;
            }
        }
    }
}

/// Establishes a WebSocket connection and listens for messages.
/// Returns the last processed timestamp on clean exit, or an error.
async fn connect_and_listen(
    loki_base_url_str: &str,
    loki_query: &str,
    tail_limit_str: &str,
    start_timestamp_ns: u64,
    consumers: &[Arc<Box<dyn LogConsumer>>],
) -> Result<u64, LokiError> {

    // --- Construct WebSocket URL ---
    let mut base_url = Url::parse(loki_base_url_str)?;
    let scheme = match base_url.scheme() {
        "http" => "ws",
        "https" => "wss",
        s => return Err(LokiError::InvalidScheme(s.to_string())),
    };
    base_url.set_scheme(scheme).map_err(|_| LokiError::InvalidScheme("Failed to set scheme".into()))?; // Handle potential error if scheme cannot be base
    base_url.set_path("/loki/api/v1/tail");

    // Add query parameters
    base_url
        .query_pairs_mut()
        .append_pair("query", loki_query)
        .append_pair("limit", tail_limit_str)
        .append_pair("start", &start_timestamp_ns.to_string());

    let ws_url = base_url; // ws_url is still type url::Url here
    println!("Connecting to WebSocket: {}", ws_url);

    // --- Connect to WebSocket ---
    // Convert the url::Url to a &str before passing to connect_async
    let (ws_stream, response) = connect_async(ws_url.as_str()).await?;
    //                                        ^^^^^^^^^^^^^^^-- FIX: Convert Url to &str

    println!("WebSocket handshake successful!");
    println!("HTTP Response Status: {}", response.status());
    // You could inspect response headers here if needed

    // Split the stream into a sender and receiver. We mostly use the receiver.
    let (mut write, mut read) = ws_stream.split();

    let mut current_last_ts = start_timestamp_ns;

    // --- Message Processing Loop ---
    while let Some(message_result) = read.next().await {
        match message_result {
            Ok(message) => {
                match message {
                    Message::Text(text) => {
                        // println!("Received Text: {}", text); // Debug raw message
                        match serde_json::from_str::<LokiTailResponse>(&text) {
                            Ok(loki_data) => {
                                let mut latest_ts_in_batch = current_last_ts;
                                for stream in loki_data.streams {
                                    for value_pair in stream.values {
                                        match value_pair[0].parse::<u64>() {
                                            Ok(ts_ns) => {
                                                // Process logs (no need to check ts > current_last_ts
                                                // as WebSocket streams new data, but update the latest seen)
                                                let log_line = &value_pair[1];
                                                let labels = &stream.stream;

                                                // 1. Print raw log (optional)
                                                println!(
                                                    "[{}] Labels: {:?} | Log: {}",
                                                    ts_ns, labels, log_line
                                                );

                                                // 2. Pass to consumers
                                                for consumer in consumers {
                                                    consumer.consume(log_line, labels);
                                                }

                                                latest_ts_in_batch = latest_ts_in_batch.max(ts_ns);
                                            }
                                            Err(e) => {
                                                eprintln!(
                                                    "Warning: Could not parse timestamp '{}': {}",
                                                    value_pair[0], e
                                                );
                                            }
                                        }
                                    }
                                }
                                // Update the latest timestamp seen in this batch
                                if latest_ts_in_batch > current_last_ts {
                                    current_last_ts = latest_ts_in_batch;
                                }
                            }
                            Err(e) => {
                                eprintln!("Error parsing Loki JSON from WebSocket: {}", e);
                                eprintln!("Raw message content: {}", text); // Log raw message on parse error
                            }
                        }
                    }
                    Message::Binary(bin) => {
                        println!("Received unexpected binary message: {} bytes", bin.len());
                    }
                    Message::Ping(ping_data) => {
                        // Respond to Pings to keep connection alive
                        // println!("Received Ping, sending Pong"); // Debug
                        if write.send(Message::Pong(ping_data)).await.is_err() {
                             // If sending Pong fails, the connection is likely broken
                             eprintln!("Failed to send Pong, connection likely closed.");
                             break; // Exit inner loop to trigger reconnect
                        }
                    }
                    Message::Pong(_) => {
                        // Usually we don't need to do anything with Pongs
                        // println!("Received Pong"); // Debug
                    }
                    Message::Close(close_frame) => {
                        println!("Received Close frame: {:?}", close_frame);
                        break; // Exit the inner loop cleanly
                    }
                    Message::Frame(_) => {
                        // Raw frame, usually not handled directly
                        println!("Received unexpected raw frame");
                    }
                }
            }
            Err(e) => {
                // Handle errors reading from the WebSocket stream
                eprintln!("Error reading from WebSocket: {}", e);
                // Return the error to the outer loop to trigger reconnection logic
                return Err(LokiError::WebSocketError(e));
            }
        }
    }

    // If the loop exits cleanly (e.g., received Close frame), return the last timestamp
    Ok(current_last_ts)
}
