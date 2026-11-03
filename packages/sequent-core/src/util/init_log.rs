// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use std::str::FromStr;
use tracing::{Level, Metadata};
use tracing_log::NormalizeEvent;
use tracing_subscriber::filter::{FilterExt, LevelFilter};
use tracing_subscriber::layer::{Context, Filter};
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
        .with(layer.with_filter(request_filter.and(SqlParameterFilter)));

    if set_global {
        tracing::subscriber::set_global_default(subscriber).unwrap();
    }
    tracing_log::LogTracer::init().unwrap();
    reload_handle
}

/// SQL drivers log bound parameters at DEBUG. This sink filter is independent
/// of runtime verbosity, so enabling diagnostics cannot expose private records.
/// Warning and error diagnostics, and other modules' debug events, are retained.
#[derive(Clone, Copy, Debug)]
pub struct SqlParameterFilter;

fn safe_driver_metadata(metadata: &Metadata<'_>) -> bool {
    let driver = ["tokio_postgres", "postgres"].iter().any(|name| {
        metadata.target() == *name
            || metadata
                .target()
                .strip_prefix(name)
                .is_some_and(|rest| rest.starts_with("::"))
    });
    !driver || *metadata.level() <= Level::INFO
}

impl<S: tracing::Subscriber> Filter<S> for SqlParameterFilter {
    fn enabled(&self, metadata: &Metadata<'_>, _: &Context<'_, S>) -> bool {
        safe_driver_metadata(metadata)
    }

    fn event_enabled(
        &self,
        event: &tracing::Event<'_>,
        _: &Context<'_, S>,
    ) -> bool {
        // LogTracer dispatches with target `log`; its original driver target is
        // carried in fields. Recheck it even if another sink enabled the event.
        let normalized = event.normalized_metadata();
        safe_driver_metadata(
            normalized.as_ref().unwrap_or_else(|| event.metadata()),
        )
    }
}

#[cfg(test)]
mod privacy_tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use tracing::subscriber::with_default;
    use tracing_log::log::{Level as LogLevel, Log, Record};

    #[derive(Clone, Default)]
    struct Sink(Arc<Mutex<Vec<u8>>>);
    impl std::io::Write for Sink {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn privacy_driver_parameters_are_suppressed_after_runtime_trace_reload() {
        let sink = Sink::default();
        let writer = sink.clone();
        let (level, handle) = reload::Layer::new(LevelFilter::INFO);
        let format = tracing_subscriber::fmt::layer()
            .with_ansi(false)
            .without_time()
            .with_writer(move || writer.clone())
            .with_filter(SqlParameterFilter);
        let subscriber = Registry::default()
            .with(level)
            .with(format)
            // An independently enabled sink must not bypass the protected sink's
            // normalized-event filtering for the LogTracer `log` callsite.
            .with(tracing_subscriber::fmt::layer().with_writer(std::io::sink));
        with_default(subscriber, || {
            let bridge = tracing_log::LogTracer::new();
            handle.reload(LevelFilter::TRACE).unwrap();
            for target in ["tokio_postgres::query", "postgres::query"] {
                for level in [LogLevel::Debug, LogLevel::Trace] {
                    bridge.log(&Record::builder().args(format_args!("parameters: private-sql-voter 198.51.100.73 private-sql-ballot"))
                        .target(target).level(level).build());
                }
                bridge.log(
                    &Record::builder()
                        .args(format_args!("database warning preserved"))
                        .target(target)
                        .level(LogLevel::Warn)
                        .build(),
                );
                bridge.log(
                    &Record::builder()
                        .args(format_args!("database error preserved"))
                        .target(target)
                        .level(LogLevel::Error)
                        .build(),
                );
            }
            bridge.log(
                &Record::builder()
                    .args(format_args!("unrelated debug preserved"))
                    .target("privacy_test")
                    .level(LogLevel::Debug)
                    .build(),
            );
            tracing::debug!(target: "tokio_postgres::query", "private-native-trace-voter");
        });
        let logs = String::from_utf8(sink.0.lock().unwrap().clone()).unwrap();
        assert!(logs.contains("database warning preserved"));
        assert!(logs.contains("database error preserved"));
        assert!(logs.contains("unrelated debug preserved"));
        for private in [
            "private-sql-voter",
            "198.51.100.73",
            "private-sql-ballot",
            "private-native-trace-voter",
        ] {
            assert!(
                !logs.contains(private),
                "SQL driver disclosed private parameters"
            );
        }
    }
}
