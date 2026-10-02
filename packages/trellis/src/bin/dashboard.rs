//! Real-time metrics dashboard for Trellis
//!
//! A terminal-based dashboard that displays live metrics from the Trellis server,
//! including per-log processing statistics and global system metrics.
//!
#![allow(clippy::pedantic)]
#![allow(clippy::print_stdout)]
#![allow(clippy::print_stderr)]
#![allow(clippy::arithmetic_side_effects)]
//! # Usage
//!
//! ```bash
//! cargo run --bin dashboard
//! ```
//!
//! Press Ctrl+C to exit.

use anyhow::Result;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent},
    execute,
    terminal::{self, ClearType},
};
use std::collections::{HashMap, VecDeque};
use std::io::{Write, stdout};
use std::time::Duration;
use trellis::service::Client;
use trellis::service::responses::MetricsResponse;

/// Display mode for metrics
#[derive(Debug, Clone, Copy, PartialEq)]
enum DisplayMode {
    /// Show last reported values
    Last,
    /// Show rolling averages
    Average,
}

/// Rolling window of metrics for averaging
struct MetricsHistory {
    /// Maximum number of samples to keep
    window_size: usize,
    /// Per-log metric samples
    log_samples: HashMap<String, LogSamples>,
    /// Global metric samples
    global_samples: GlobalSamples,
}

/// Rolling samples for a single log
struct LogSamples {
    /// Total processing time samples
    total_ms: VecDeque<u64>,
    /// Copy operation time samples
    copy_ms: VecDeque<u64>,
    /// Insert operation time samples
    insert_ms: VecDeque<u64>,
    /// Rows processed samples (for per-row calculations)
    rows: VecDeque<u64>,
    /// Fetch operation time samples
    fetch_ms: VecDeque<u64>,
    /// Tree update time samples
    tree_ms: VecDeque<u64>,
}

/// Rolling samples for global metrics
struct GlobalSamples {
    /// Cycle duration samples
    cycle_duration_ms: VecDeque<u64>,
}

impl MetricsHistory {
    /// Creates a new metrics history with the specified window size
    fn new(window_size: usize) -> Self {
        Self {
            window_size,
            log_samples: HashMap::new(),
            global_samples: GlobalSamples {
                cycle_duration_ms: VecDeque::with_capacity(window_size),
            },
        }
    }

    /// Adds a new metrics sample to the rolling window
    fn add_sample(&mut self, metrics: &MetricsResponse) {
        // Add global sample (always add, as cycles always run)
        Self::push_sample(
            &mut self.global_samples.cycle_duration_ms,
            metrics.global.last_cycle_duration_ms,
            self.window_size,
        );

        // Add per-log samples - ONLY if the log processed data this cycle
        for (log_name, log_metrics) in &metrics.logs {
            // Skip idle cycles - only track when log actually processed rows
            if log_metrics.last_batch_rows == 0 {
                continue;
            }

            let samples = self
                .log_samples
                .entry(log_name.clone())
                .or_insert_with(|| LogSamples {
                    total_ms: VecDeque::with_capacity(self.window_size),
                    copy_ms: VecDeque::with_capacity(self.window_size),
                    insert_ms: VecDeque::with_capacity(self.window_size),
                    rows: VecDeque::with_capacity(self.window_size),
                    fetch_ms: VecDeque::with_capacity(self.window_size),
                    tree_ms: VecDeque::with_capacity(self.window_size),
                });

            Self::push_sample(
                &mut samples.total_ms,
                log_metrics.last_total_ms,
                self.window_size,
            );
            Self::push_sample(
                &mut samples.copy_ms,
                log_metrics.last_copy_ms,
                self.window_size,
            );
            Self::push_sample(
                &mut samples.insert_ms,
                log_metrics.last_insert_ms,
                self.window_size,
            );
            Self::push_sample(
                &mut samples.rows,
                log_metrics.last_batch_rows,
                self.window_size,
            );
            Self::push_sample(
                &mut samples.fetch_ms,
                log_metrics.last_fetch_merkle_log_ms,
                self.window_size,
            );
            Self::push_sample(
                &mut samples.tree_ms,
                log_metrics.last_tree_update_ms,
                self.window_size,
            );
        }
    }

    /// Pushes a sample into a queue, removing oldest if at capacity
    fn push_sample(queue: &mut VecDeque<u64>, value: u64, max_size: usize) {
        if queue.len() >= max_size {
            queue.pop_front();
        }
        queue.push_back(value);
    }

    /// Computes the average of values in a queue
    fn average(queue: &VecDeque<u64>) -> u64 {
        if queue.is_empty() {
            return 0;
        }
        let sum: u64 = queue.iter().sum();
        sum / queue.len() as u64
    }

    /// Gets the average global cycle duration
    fn get_global_cycle_avg(&self) -> u64 {
        Self::average(&self.global_samples.cycle_duration_ms)
    }

    /// Gets the average metrics for a specific log (total, copy, insert, fetch, tree, rows)
    fn get_log_averages(&self, log_name: &str) -> Option<(u64, u64, u64, u64, u64, u64)> {
        self.log_samples.get(log_name).map(|samples| {
            (
                Self::average(&samples.total_ms),
                Self::average(&samples.copy_ms),
                Self::average(&samples.insert_ms),
                Self::average(&samples.fetch_ms),
                Self::average(&samples.tree_ms),
                Self::average(&samples.rows),
            )
        })
    }
}

/// Formats a number with K/M suffixes for readability.
fn format_number(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}k", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

/// Formats bytes as KB/MB/GB for readability.
fn format_memory(bytes: u64) -> String {
    if bytes >= 1_073_741_824 {
        format!("{:.1}GB", bytes as f64 / 1_073_741_824.0)
    } else if bytes >= 1_048_576 {
        format!("{:.1}MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1_024 {
        format!("{:.1}KB", bytes as f64 / 1_024.0)
    } else {
        format!("{bytes}B")
    }
}

#[allow(clippy::uninlined_format_args)]
/// Displays metrics in the terminal.
fn display_metrics(
    metrics: &MetricsResponse,
    history: &MetricsHistory,
    mode: DisplayMode,
    refresh_interval: u64,
) -> Result<()> {
    let mut stdout = stdout();

    // Clear screen and move to top (simple approach, no raw mode needed)
    execute!(
        stdout,
        terminal::Clear(ClearType::All),
        cursor::MoveTo(0, 0)
    )?;

    // Mode indicator
    let mode_str = match mode {
        DisplayMode::Last => "LAST",
        DisplayMode::Average => "AVERAGE",
    };
    println!("Mode: {} | Press 'a' for average, 'l' for last", mode_str);
    println!();

    // HTTP Metrics section (if available)
    if let Some(http) = &metrics.http {
        println!("─── HTTP Metrics ───");
        println!(
            "{:<12} {:>10} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8}",
            "ENDPOINT", "REQUESTS", "ERRORS", "<1ms", "<5ms", "<10ms", "<50ms", "<100ms", "≥100ms"
        );

        // Helper to format latency distribution as percentages
        let format_latency = |ep: &trellis::service::metrics::EndpointMetricsSnapshot| {
            let total = ep.requests;
            if total == 0 {
                return vec!["-".to_string(); 6];
            }
            ep.latency_buckets
                .iter()
                .map(|&count| {
                    let pct = (count as f64 / total as f64) * 100.0;
                    format!("{:.0}%", pct)
                })
                .collect()
        };

        let inc_latency = format_latency(&http.inclusion);
        println!(
            "{:<12} {:>10} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8}",
            "inclusion",
            format_number(http.inclusion.requests),
            format_number(http.inclusion.errors),
            inc_latency.first().map_or("-", String::as_str),
            inc_latency.get(1).map_or("-", String::as_str),
            inc_latency.get(2).map_or("-", String::as_str),
            inc_latency.get(3).map_or("-", String::as_str),
            inc_latency.get(4).map_or("-", String::as_str),
            inc_latency.get(5).map_or("-", String::as_str)
        );

        let con_latency = format_latency(&http.consistency);
        println!(
            "{:<12} {:>10} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8}",
            "consistency",
            format_number(http.consistency.requests),
            format_number(http.consistency.errors),
            con_latency.first().map_or("-", String::as_str),
            con_latency.get(1).map_or("-", String::as_str),
            con_latency.get(2).map_or("-", String::as_str),
            con_latency.get(3).map_or("-", String::as_str),
            con_latency.get(4).map_or("-", String::as_str),
            con_latency.get(5).map_or("-", String::as_str)
        );
        println!();
    }

    // Header with global stats
    println!("─── Batch Processor ───");
    let cycle_value = match mode {
        DisplayMode::Last => metrics.global.last_cycle_duration_ms,
        DisplayMode::Average => history.get_global_cycle_avg(),
    };

    println!(
        "Cycle: {}ms ({:.2}x) | Active Logs: {}",
        cycle_value, metrics.global.last_cycle_fraction, metrics.global.last_active_log_count
    );
    println!();

    // Table header
    // INSERT shows total ms, INS µs/row shows microseconds per row for normalized comparison
    // TREE/WORK shows normalized tree update cost: µs per (row × tree level)
    println!(
        "{:<20} {:>8} {:>8} {:>8} {:>8} {:>10} {:>8} {:>8} {:>10} {:>12} {:>10} {:>12}",
        "LOG",
        "ROWS",
        "TOTAL",
        "COPY",
        "INSERT",
        "(µs/row)",
        "FETCH",
        "TREE",
        "TREE/WORK",
        "SIZE",
        "MEMORY",
        "UPDATE"
    );
    println!("{}", "-".repeat(144));

    // Sort logs by name for consistent display
    let mut log_names: Vec<_> = metrics.logs.keys().collect();
    log_names.sort();

    // Display each log's metrics
    for log_name in log_names {
        if let Some(log_metrics) = metrics.logs.get(log_name) {
            // Get values based on mode
            let (total, copy, insert, fetch, tree, rows) = match mode {
                DisplayMode::Last => (
                    log_metrics.last_total_ms,
                    log_metrics.last_copy_ms,
                    log_metrics.last_insert_ms,
                    log_metrics.last_fetch_merkle_log_ms,
                    log_metrics.last_tree_update_ms,
                    log_metrics.last_batch_rows,
                ),
                DisplayMode::Average => history
                    .get_log_averages(log_name)
                    .unwrap_or((0, 0, 0, 0, 0, 0)),
            };

            // Calculate normalized tree update cost: µs per (row × tree level)
            // This measures efficiency of tree updates accounting for both batch size and tree depth
            // Formula: (tree_ms * 1000) / (rows * log2(tree_size)) = µs per unit of tree work
            let tree_work_str = if log_metrics.tree_size > 1 && rows > 0 {
                let log2_size = (log_metrics.tree_size as f64).log2();
                let tree_us = tree as f64 * 1000.0;
                let work_units = rows as f64 * log2_size;
                let us_per_work = tree_us / work_units;
                format!("{:.2}", us_per_work)
            } else {
                // For empty/single-element trees or zero rows, show dash
                "-".to_string()
            };

            // Extract time from ISO8601 timestamp (HH:MM:SS)
            let time_str = log_metrics
                .last_update
                .split('T')
                .nth(1)
                .and_then(|t| t.split('.').next())
                .unwrap_or("--:--:--");

            // Compute INSERT time per row in microseconds
            let insert_us_per_row = if rows > 0 {
                (insert as f64 * 1000.0) / rows as f64
            } else {
                0.0
            };

            println!(
                "{:<20} {:>8} {:>8} {:>8} {:>8} {:>10} {:>8} {:>8} {:>10} {:>12} {:>10} {:>12}",
                if log_name.len() > 20 {
                    &log_name[..20]
                } else {
                    log_name
                },
                rows,
                total,
                copy,
                insert,
                format!("{:.1}", insert_us_per_row),
                fetch,
                tree,
                tree_work_str,
                format_number(log_metrics.tree_size),
                format_memory(log_metrics.tree_memory_bytes),
                time_str
            );
        }
    }

    println!();
    println!(
        "Press Ctrl+C to exit | Refreshing every {}s",
        refresh_interval
    );

    stdout.flush()?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();

    // Get server URL from environment or use default
    let server_url =
        std::env::var("TRELLIS_SERVER_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

    // Refresh interval in seconds
    let refresh_interval = std::env::var("TRELLIS_DASHBOARD_REFRESH_INTERVAL")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);

    // Window size for averaging (number of samples)
    let window_size = 30;

    println!("Connecting to {server_url}...",);
    println!("Refresh interval: {refresh_interval}s");
    println!("Average window: {window_size} samples");
    println!();

    // Create client
    let client = Client::new(&server_url)?;

    // Initialize metrics history and display mode
    let mut history = MetricsHistory::new(window_size);
    let mut mode = DisplayMode::Last;

    // No raw mode - allows normal terminal interactions (text selection, etc.)

    loop {
        // Check for key press (non-blocking)
        if event::poll(Duration::from_millis(0))?
            && let Event::Key(KeyEvent { code, .. }) = event::read()?
        {
            match code {
                KeyCode::Char('a') | KeyCode::Char('A') => {
                    mode = DisplayMode::Average;
                }
                KeyCode::Char('l') | KeyCode::Char('L') => {
                    mode = DisplayMode::Last;
                }
                _ => {}
            }
        }

        match client.get_metrics().await {
            Ok(metrics) => {
                // Add to history
                history.add_sample(&metrics);

                // Display with current mode
                if let Err(e) = display_metrics(&metrics, &history, mode, refresh_interval) {
                    eprintln!("Display error: {e}");
                }
            }
            Err(e) => {
                execute!(
                    stdout(),
                    terminal::Clear(ClearType::All),
                    cursor::MoveTo(0, 0)
                )?;
                println!("⚠️  Failed to fetch metrics: {e}");
                println!("Retrying in {refresh_interval}s...");
                println!("\nPress Ctrl+C to exit");
            }
        }

        tokio::time::sleep(Duration::from_secs(refresh_interval)).await;
    }
}
