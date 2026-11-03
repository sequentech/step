// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The component's log.
//!
//! Rocket logs every request's full URI: at INFO when it arrives, at ERROR
//! when no route matches, and at DEBUG and TRACE while routing. A callback
//! URI carries the account's whole webhook key and every query value, Meta's
//! verify token among them, so Rocket's records are dropped at every level.
//! Only its launch summary and launch failures are kept; neither names a
//! request. Other crates keep their warnings and errors; the component's own
//! lines name the key's first characters only.

use std::cmp::min;
use std::str::FromStr;
use tracing_subscriber::filter::{LevelFilter, Targets};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::Layer;
use tracing_tree::HierarchicalLayer;

/// The variable naming the log level, as for the other services.
pub const LOG_LEVEL_ENV: &str = "LOG_LEVEL";

/// What the log keeps at `level`.
pub fn filter(level: LevelFilter) -> Targets {
    Targets::new()
        // Other crates (hyper, reqwest, …): warnings and errors only.
        .with_default(min(level, LevelFilter::WARN))
        .with_target("messaging_webhooks", level)
        .with_target("messaging", level)
        // Rocket's request records carry the full URI: none of them.
        .with_target("rocket", LevelFilter::OFF)
        .with_target("rocket::launch", level)
        .with_target("rocket::error", level)
}

/// Sends the `log` records of Rocket and other crates to tracing, through
/// [`filter`]. Once per process, before Rocket starts, so Rocket's own
/// logger (which would print request lines) is never installed.
pub fn bridge_log_records() {
    static BRIDGE: std::sync::Once = std::sync::Once::new();
    BRIDGE.call_once(|| {
        let _ = tracing_log::LogTracer::init();
    });
}

/// Installs the component's log: [`LOG_LEVEL_ENV`] (default `info`) through
/// [`filter`], in the other services' format.
pub fn init() {
    let raw = std::env::var(LOG_LEVEL_ENV).ok();
    let parsed = raw
        .as_deref()
        .map(|raw| LevelFilter::from_str(raw.trim()).ok());
    let unreadable = matches!(parsed, Some(None));
    let level = parsed.flatten().unwrap_or(LevelFilter::INFO);
    let layer = HierarchicalLayer::default()
        .with_writer(std::io::stdout)
        .with_indent_lines(true)
        .with_indent_amount(3)
        .with_thread_names(false)
        .with_thread_ids(true)
        .with_verbose_exit(false)
        .with_verbose_entry(false)
        .with_targets(false)
        .with_filter(filter(level));
    let subscriber = tracing_subscriber::registry().with(layer);
    if tracing::subscriber::set_global_default(subscriber).is_err() {
        eprintln!("messaging-webhooks: a log was already installed");
    }
    bridge_log_records();
    if unreadable {
        tracing::warn!("{LOG_LEVEL_ENV} is not a log level; using info");
    }
}
