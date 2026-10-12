// SPDX-FileCopyrightText: 2021 David Ruescas <david@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Logging utilities for native platforms
//!
//! This module provides tracing/logging infrastructure that is only
//! available on native platforms (not WebAssembly).

use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::reload::Handle;
use tracing_subscriber::{filter, reload};
use tracing_subscriber::{layer::SubscriberExt, registry::Registry};
use tracing_tree::HierarchicalLayer;

/// Initialize the tracing log, returning a handle that
/// allows changing log levels at run time.
///
/// The log can display messages within a tree representation of the
/// call stack. To do this you must mark function definitions
/// you wish to track with the #[instrument] annotation.
/// See https://docs.rs/tracing-attributes/latest/tracing_attributes/attr.instrument.html
pub fn init_log(set_global: bool) -> Handle<LevelFilter, Registry> {
    let layer = HierarchicalLayer::default()
        .with_writer(std::io::stdout)
        .with_indent_lines(true)
        .with_indent_amount(3)
        .with_thread_names(false)
        .with_thread_ids(false)
        .with_verbose_exit(false)
        .with_verbose_entry(false)
        .with_targets(false);

    let filter = filter::LevelFilter::INFO;
    let (filter, reload_handle) = reload::Layer::new(filter);
    let subscriber = Registry::default().with(filter).with(layer);

    // Idempotent: a process (e.g. a test binary with several `#[test]`s) may call
    // this more than once; the global dispatcher can only be set once, so ignore a
    // repeat rather than panic.
    if set_global {
        let _ = tracing::subscriber::set_global_default(subscriber);
    }
    let _ = tracing_log::LogTracer::init();
    reload_handle
}
