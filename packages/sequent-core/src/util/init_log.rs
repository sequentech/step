// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use std::str::FromStr;
use tracing::{Level, Metadata};
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::reload::Handle;
use tracing_subscriber::{filter, reload, Layer};
use tracing_subscriber::{layer::SubscriberExt, registry::Registry};
use tracing_tree::HierarchicalLayer;

pub fn init_log(set_global: bool) -> Handle<LevelFilter, Registry> {
    init_log_with_filter(set_global, |_, _| true)
}

/// Initialize the usual logger with an additional request-aware filter. The
/// callback receives the current reloadable level, so debug checks follow
/// runtime log-level changes as well as LOG_LEVEL at startup.
pub fn init_log_with_filter(
    set_global: bool,
    enabled: impl Fn(&Metadata<'_>, LevelFilter) -> bool + Send + Sync + 'static,
) -> Handle<LevelFilter, Registry> {
    let layer = HierarchicalLayer::default()
        .with_writer(std::io::stdout)
        .with_indent_lines(true)
        .with_indent_amount(3)
        .with_thread_names(false)
        .with_thread_ids(true)
        .with_verbose_exit(false)
        .with_verbose_entry(false)
        .with_targets(false);

    let level_str = std::env::var("LOG_LEVEL").unwrap_or("info".to_string());
    let level = Level::from_str(&level_str).unwrap();
    let filter = filter::LevelFilter::from_level(level);
    let (filter, reload_handle) = reload::Layer::new(filter);
    let current_level = reload_handle.clone();
    let request_filter = filter::dynamic_filter_fn(move |metadata, _| {
        current_level
            .with_current(|level| enabled(metadata, *level))
            .unwrap_or(true)
    });
    let subscriber = Registry::default()
        .with(filter)
        .with(layer.with_filter(request_filter));

    if set_global {
        tracing::subscriber::set_global_default(subscriber).unwrap();
    }
    tracing_log::LogTracer::init().unwrap();
    reload_handle
}
