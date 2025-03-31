use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::sync::Arc; // Used for potential sharing if actions become async
use std::time::Duration;
use thiserror::Error;
use url::Url;

// --- Data Structures for Loki API Response ---

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
    JsonParseError(#[from] serde_json::Error),
    #[error("Failed to parse integer: {0}")]
    ParseIntError(#[from] std::num::ParseIntError),
}

// --- Event Consumer Trait and Implementations ---

/// Trait for defining actions to take based on log content or labels.
trait LogConsumer: Send + Sync {
    /// Returns a descriptive name for the consumer.
    fn name(&self) -> String;

    /// Processes a single log entry.
    /// Implementations should check if the log meets their criteria and perform an action.
    fn consume(&self, log_line: &str, labels: &HashMap<String, String>);
}

/// A simple consumer that checks for a specific keyword (case-insensitive).
struct KeywordConsumer {
    keyword: String,
}

impl KeywordConsumer {
    fn new(keyword: &str) -> Self {
        KeywordConsumer {
            keyword: keyword.to_lowercase(), // Store keyword in lowercase for case-insensitive comparison
        }
    }
}

impl LogConsumer for KeywordConsumer {
    fn name(&self) -> String {
        format!("KeywordConsumer({})", self.keyword)
    }

    fn consume(&self, log_line: &str, labels: &HashMap<String, String>) {
        // Perform case-insensitive check
        if log_line.to_lowercase().contains(&self.keyword) {
            // Action: Print a notification message
            // In a real application, this could trigger an alert, call a webhook, etc.
            println!(
                "--- !!! [{}] Found keyword '{}' in log from {:?}: {} ---",
                self.name(),
                self.keyword,
                labels, // Include labels for context
                log_line
            );
        }
    }
}

// --- Main Application Logic ---

#[tokio::main]
async fn main() -> Result<(), LokiError> {
    // --- Configuration ---
    println!("Reading configuration from environment variables...");
    let loki_base_url_str = env::var("LOKI_URL").map_err(|_| {
        LokiError::EnvVarError(
            "LOKI_URL environment variable not set. Example: http://localhost:3100".to_string(),
        )
    })?;
    let loki_query = env::var("LOKI_QUERY").map_err(|_| {
        LokiError::EnvVarError(
            "LOKI_QUERY environment variable not set. Example: {job=\"myapp\"}".to_string(),
        )
    })?;
    let tail_limit_str = env::var("LOKI_TAIL_LIMIT").unwrap_or_else(|_| "100".to_string());
    let tail_limit: u32 = tail_limit_str.parse().unwrap_or(100);
    let poll_interval_secs_str =
        env::var("LOKI_POLL_INTERVAL_SECS").unwrap_or_else(|_| "5".to_string());
    let poll_interval_secs: u64 = poll_interval_secs_str.parse().unwrap_or(5);
    let poll_interval = Duration::from_secs(poll_interval_secs);

    // --- Setup ---
    println!("Initializing HTTP client and URL...");
    let http_client = Client::builder()
        .timeout(Duration::from_secs(poll_interval_secs + 10))
        .build()?;
    let mut loki_tail_url = Url::parse(&loki_base_url_str)?;
    loki_tail_url.set_path("/loki/api/v1/tail");

    let mut last_timestamp_ns: u64 = (chrono::Utc::now() - chrono::Duration::seconds(10))
        .timestamp_nanos_opt()
        .unwrap_or(0) as u64;

    // --- Initialize Consumers ---
    // Create a vector to hold different consumers.
    // Use Arc for potential future async actions within consumers.
    // Use Box<dyn LogConsumer> for dynamic dispatch.
    let consumers: Vec<Arc<Box<dyn LogConsumer>>> = vec![
        // Add specific consumers here
        Arc::new(Box::new(KeywordConsumer::new("error"))),
        Arc::new(Box::new(KeywordConsumer::new("failed"))),
        // Add more consumers as needed (e.g., RegexConsumer, AlertConsumer)
    ];
    println!("Initialized {} log consumers:", consumers.len());
    for consumer in &consumers {
        println!("  - {}", consumer.name());
    }


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
        let params = [
            ("query", loki_query.as_str()),
            ("limit", &tail_limit.to_string()),
            ("start", &last_timestamp_ns.to_string()),
        ];

        let response_result = http_client
            .get(loki_tail_url.clone())
            .query(&params)
            .send()
            .await;

        match response_result {
            Ok(response) => {
                let status = response.status();
                if status.is_success() {
                    match response.json::<LokiTailResponse>().await {
                        Ok(loki_data) => {
                            let mut latest_ts_in_batch = last_timestamp_ns;
                            let mut logs_received_count = 0;

                            for stream in loki_data.streams {
                                for value_pair in stream.values {
                                    match value_pair[0].parse::<u64>() {
                                        Ok(ts_ns) => {
                                            if ts_ns > last_timestamp_ns {
                                                let log_line = &value_pair[1];
                                                let labels = &stream.stream;

                                                // 1. Print the raw log (optional)
                                                println!(
                                                    "[{}] Labels: {:?} | Log: {}",
                                                    ts_ns, labels, log_line
                                                );

                                                // 2. Pass the log to each consumer
                                                for consumer in &consumers {
                                                    // Clone Arc for potential async tasks later
                                                    let consumer_clone = Arc::clone(consumer);
                                                    // For now, call consume directly.
                                                    // If actions were async, you'd spawn tasks:
                                                    // tokio::spawn(async move {
                                                    //    consumer_clone.consume(log_line_owned, labels_owned).await;
                                                    // });
                                                    consumer_clone.consume(log_line, labels);
                                                }


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

                            if logs_received_count > 0 {
                                last_timestamp_ns = latest_ts_in_batch + 1;
                            }
                        }
                        Err(e) => {
                            eprintln!("Error parsing Loki JSON response: {}", e);
                        }
                    }
                } else {
                    let body = response
                        .text()
                        .await
                        .unwrap_or_else(|_| "Could not read error body".to_string());
                    eprintln!(
                        "Loki API request failed: Status: {}, Body: {}",
                        status, body
                    );
                }
            }
            Err(e) => {
                eprintln!("Error making request to Loki: {}", e);
            }
        }

        tokio::time::sleep(poll_interval).await;
    }
    // Ok(()) // Unreachable
}
