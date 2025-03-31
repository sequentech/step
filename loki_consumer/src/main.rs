use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::time::Duration;
use thiserror::Error;
use url::Url;

// --- Data Structures to Match Loki's API Response ---

/// Represents the overall structure of the response from the Loki tail endpoint.
#[derive(Deserialize, Debug)]
struct LokiTailResponse {
    streams: Vec<LokiStream>,
}

/// Represents a single log stream with its labels and log entries.
#[derive(Deserialize, Debug)]
struct LokiStream {
    /// Labels associated with the log stream (e.g., {"app": "myapp", "level": "info"}).
    stream: HashMap<String, String>,
    /// A list of log entries, where each entry is a pair:
    /// [0]: Timestamp string (nanoseconds since epoch).
    /// [1]: Log line content string.
    values: Vec<[String; 2]>,
}

// --- Custom Error Type for Better Error Handling ---

#[derive(Error, Debug)]
enum LokiError {
    #[error("Network or HTTP request error: {0}")]
    RequestError(#[from] reqwest::Error),
    #[error("URL parsing error: {0}")]
    UrlParseError(#[from] url::ParseError),
    #[error("Required environment variable not set: {0}")]
    EnvVarError(String),
    #[error("Loki API returned an error: Status={status}, Body={body}")]
    ApiError {
        status: reqwest::StatusCode,
        body: String,
    },
    #[error("Failed to parse JSON response: {0}")]
    JsonParseError(#[from] serde_json::Error), // Note: reqwest::Error can also represent JSON parsing errors from response.json()
    #[error("Failed to parse integer: {0}")]
    ParseIntError(#[from] std::num::ParseIntError),
}

// --- Main Application Logic ---

#[tokio::main]
async fn main() -> Result<(), LokiError> {
    // --- Configuration ---
    println!("Reading configuration from environment variables...");

    // Get Loki base URL (e.g., "http://localhost:3100")
    let loki_base_url_str = env::var("LOKI_URL").map_err(|_| {
        LokiError::EnvVarError(
            "LOKI_URL environment variable not set. Example: http://localhost:3100".to_string(),
        )
    })?;

    // Get the LogQL query (e.g., "{app=\"my-service\"}")
    let loki_query = env::var("LOKI_QUERY").map_err(|_| {
        LokiError::EnvVarError(
            "LOKI_QUERY environment variable not set. Example: {job=\"myapp\"}".to_string(),
        )
    })?;

    // Optional configuration with defaults
    let tail_limit_str = env::var("LOKI_TAIL_LIMIT").unwrap_or_else(|_| "100".to_string());
    let tail_limit: u32 = tail_limit_str.parse().unwrap_or(100); // Max logs per poll

    let poll_interval_secs_str =
        env::var("LOKI_POLL_INTERVAL_SECS").unwrap_or_else(|_| "5".to_string());
    let poll_interval_secs: u64 = poll_interval_secs_str.parse().unwrap_or(5); // Poll frequency
    let poll_interval = Duration::from_secs(poll_interval_secs);

    // --- Setup ---
    println!("Initializing HTTP client and URL...");
    let http_client = Client::builder()
        .timeout(Duration::from_secs(poll_interval_secs + 10)) // Set a timeout longer than the poll interval
        .build()?; // Use builder for potential future customizations

    // Construct the base URL for the tail endpoint
    let mut loki_tail_url = Url::parse(&loki_base_url_str)?;
    loki_tail_url.set_path("/loki/api/v1/tail");

    // Start tracking time from slightly in the past to catch logs potentially missed during startup
    // We store the timestamp of the *last successfully processed* log entry in nanoseconds.
    let mut last_timestamp_ns: u64 = (chrono::Utc::now() - chrono::Duration::seconds(10))
        .timestamp_nanos_opt()
        .unwrap_or(0) as u64; // Use i64 directly from chrono if needed, but u64 is fine for ns timestamps

    println!(
        "Starting Loki consumer:"
    );
    println!("  Loki URL: {}", loki_base_url_str);
    println!("  Query: {}", loki_query);
    println!("  Polling Interval: {:?}", poll_interval);
    println!("  Tail Limit: {}", tail_limit);
    println!("---");

    // --- Main Polling Loop ---
    loop {
        // Prepare query parameters for the tail request
        let params = [
            ("query", loki_query.as_str()),
            ("limit", &tail_limit.to_string()),
            // 'start' tells Loki the timestamp *after* which we want logs.
            // Use the timestamp of the last log we processed.
            ("start", &last_timestamp_ns.to_string()),
        ];

        // println!("DEBUG: Polling Loki with start_ns = {}", last_timestamp_ns); // Debugging line

        // Perform the asynchronous GET request
        let response_result = http_client
            .get(loki_tail_url.clone())
            .query(&params)
            .send()
            .await;

        match response_result {
            Ok(response) => {
                let status = response.status();
                if status.is_success() {
                    // Attempt to parse the successful response body as JSON
                    match response.json::<LokiTailResponse>().await {
                        Ok(loki_data) => {
                            let mut latest_ts_in_batch = last_timestamp_ns;
                            let mut logs_received_count = 0;

                            // Process each stream and its log entries
                            for stream in loki_data.streams {
                                for value_pair in stream.values {
                                    // value_pair[0] = timestamp (string, nanoseconds)
                                    // value_pair[1] = log line (string)
                                    match value_pair[0].parse::<u64>() {
                                        Ok(ts_ns) => {
                                            // IMPORTANT: Only process logs strictly newer than the last one seen.
                                            // Loki might re-send the boundary log entry.
                                            if ts_ns > last_timestamp_ns {
                                                // Format timestamp (optional, requires chrono features)
                                                // let dt = chrono::DateTime::from_timestamp_nanos(ts_ns as i64);
                                                // let formatted_ts = dt.format("%Y-%m-%d %H:%M:%S%.3f");

                                                println!(
                                                    // "[{}] Labels: {:?} | Log: {}", // Simple timestamp
                                                    "[{}] Labels: {:?} | Log: {}", // Simple timestamp
                                                    ts_ns, // Or formatted_ts
                                                    stream.stream,
                                                    value_pair[1]
                                                );
                                                // Update the latest timestamp seen in *this specific batch*
                                                latest_ts_in_batch = latest_ts_in_batch.max(ts_ns);
                                                logs_received_count += 1;
                                            }
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

                            // If we received new logs, update the global last timestamp marker
                            if logs_received_count > 0 {
                                // Add 1 nanosecond to the latest timestamp from the batch.
                                // This ensures the *next* poll starts immediately *after* this log,
                                // preventing fetching the exact same log again if multiple logs
                                // share the same timestamp.
                                last_timestamp_ns = latest_ts_in_batch + 1;
                                // println!("DEBUG: Updated last_timestamp_ns to: {}", last_timestamp_ns); // Debugging line
                            } else {
                                // println!("DEBUG: No new logs in this poll."); // Debugging line
                            }
                        }
                        Err(e) => {
                            // Handle JSON parsing errors specifically
                            eprintln!("Error parsing Loki JSON response: {}", e);
                            // It might be useful to log the raw response body here for debugging,
                            // but be cautious as logs can contain sensitive data.
                            // let body_text = response.text().await.unwrap_or_else(|_| "Failed to read body".to_string());
                            // eprintln!("Raw response body: {}", body_text);
                        }
                    }
                } else {
                    // Handle non-successful HTTP status codes (4xx, 5xx)
                    let body = response
                        .text()
                        .await
                        .unwrap_or_else(|_| "Could not read error body".to_string());
                    eprintln!( // Use eprintln for errors
                        "Loki API request failed: Status: {}, Body: {}",
                        status, body
                    );
                    // Consider specific error handling (e.g., backoff on 429 Too Many Requests)
                }
            }
            Err(e) => {
                // Handle errors during the request itself (network issues, DNS errors, timeouts)
                eprintln!("Error making request to Loki: {}", e);
                // Consider adding a longer delay here before retrying
            }
        }

        // Wait for the specified interval before the next poll
        tokio::time::sleep(poll_interval).await;
    }
    // Note: The loop is infinite, so Ok(()) is technically unreachable unless the loop is modified.
    // Ok(())
}

