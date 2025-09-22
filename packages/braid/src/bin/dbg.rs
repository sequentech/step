// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use strand::backend::ristretto::RistrettoCtx;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::reload::Handle;
use tracing_subscriber::{filter, reload};
use tracing_subscriber::{layer::SubscriberExt, registry::Registry};
use tracing_tree::HierarchicalLayer;

// Run with cargo run --bin dbg --release
/// Runs a simple interactive ncurses terminal to
/// simulate or debug a protocol execution.
fn main() {
    let log_reload = init_log();
    let ctx = RistrettoCtx;
    braid::test::dbg::dbg(ctx, log_reload).unwrap();
}

/// Initialize the tracing log, returning a handle that
/// allows changing log levels at run time.
fn init_log() -> Handle<LevelFilter, Registry> {
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

    tracing::subscriber::set_global_default(subscriber).unwrap();
    tracing_log::LogTracer::init().unwrap();
    reload_handle
}
