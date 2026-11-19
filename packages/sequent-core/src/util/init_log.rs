// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Logging setup shared by the Rust services.
//!
//! `LOG_LEVEL` accepts either a plain level (`info`, `debug`, ...) or
//! `tracing_subscriber` `EnvFilter` directives, for example
//! `info,hyper=warn,windmill::services::ceremonies=warn`. Each directive
//! applies to the targets (module paths) that start with it.
//!
//! When the base level is more verbose than `info`, noisy dependencies are
//! kept at `info` (see [`QUIET_DEPENDENCIES`]): at `debug`, `rocket` logs
//! every request with its headers, including bearer tokens. A directive
//! for the same target in `LOG_LEVEL` overrides that default, so
//! `LOG_LEVEL=debug,rocket=debug` turns it back on deliberately.

use tracing::level_filters::LevelFilter;
use tracing_subscriber::reload::{self, Handle};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::{layer::SubscriberExt, registry::Registry};
use tracing_tree::HierarchicalLayer;

/// Environment variable read by [`init_log`].
pub const LOG_LEVEL_ENV: &str = "LOG_LEVEL";

/// Level used when `LOG_LEVEL` is unset or invalid.
pub const DEFAULT_LOG_LEVEL: LevelFilter = LevelFilter::INFO;

/// Dependency targets kept at `info` when the base level is `debug` or
/// `trace`, unless `LOG_LEVEL` names them explicitly.
pub const QUIET_DEPENDENCIES: &[&str] = &[
    "rocket",
    "hyper",
    "h2",
    "rustls",
    "tokio_postgres",
    "reqwest",
    "lapin",
    "aws_",
];

/// Handle to change the log filter while the service runs.
pub type LogReloadHandle = Handle<EnvFilter, Registry>;

/// Builds the filter for a `LOG_LEVEL` value.
///
/// Returns the filter and, when the value could not be parsed, an error
/// message to log once logging is up. An invalid value falls back to
/// [`DEFAULT_LOG_LEVEL`] instead of stopping the service.
pub fn build_filter(spec: Option<&str>) -> (EnvFilter, Option<String>) {
    let spec = spec.map(str::trim).filter(|s| !s.is_empty());
    let Some(spec) = spec else {
        return (filter_with_defaults("", DEFAULT_LOG_LEVEL), None);
    };

    if let Err(err) = validate(spec) {
        let message = format!(
            "invalid {LOG_LEVEL_ENV}={spec:?} ({err}); using {DEFAULT_LOG_LEVEL}"
        );
        return (filter_with_defaults("", DEFAULT_LOG_LEVEL), Some(message));
    }

    match base_level(spec) {
        Some(base) => (filter_with_defaults(spec, base), None),
        None => {
            let spec = format!("{DEFAULT_LOG_LEVEL},{spec}");
            (filter_with_defaults(&spec, DEFAULT_LOG_LEVEL), None)
        }
    }
}

/// Rejects the whole value if any directive is invalid, since EnvFilter
/// would otherwise drop it silently. A bare word that isn't a level (a
/// typo such as `infoo`) is also rejected: EnvFilter would read it as a
/// target enabled at `trace`.
fn validate(spec: &str) -> Result<(), String> {
    for directive in spec.split(',').map(str::trim).filter(|d| !d.is_empty()) {
        let is_level = directive.parse::<LevelFilter>().is_ok();
        if !is_level && !directive.contains('=') {
            return Err(format!(
                "{directive:?} is not a level or target=level"
            ));
        }
    }
    EnvFilter::builder()
        .parse(spec)
        .map(|_| ())
        .map_err(|err| err.to_string())
}

/// The level of the last directive without a target, if any.
fn base_level(spec: &str) -> Option<LevelFilter> {
    spec.split(',')
        .map(str::trim)
        .filter_map(|d| d.parse::<LevelFilter>().ok())
        .last()
}

/// Names the targets that `spec` sets explicitly, so the quiet defaults
/// don't add a second directive for them.
fn names_target(spec: &str, target: &str) -> bool {
    spec.split(',').map(str::trim).any(|d| {
        let name = d.split('=').next().unwrap_or("");
        let name = name.split('[').next().unwrap_or("");
        name == target
    })
}

fn filter_with_defaults(spec: &str, base: LevelFilter) -> EnvFilter {
    let mut directives: Vec<String> = Vec::new();
    // `LevelFilter` orders more verbose levels as greater.
    if spec.is_empty() {
        directives.push(base.to_string().to_lowercase());
    }
    if base > LevelFilter::INFO {
        for target in QUIET_DEPENDENCIES {
            if !names_target(spec, target) {
                directives.push(format!("{target}=info"));
            }
        }
    }
    if !spec.is_empty() {
        directives.push(spec.to_string());
    }
    EnvFilter::builder()
        .with_default_directive(DEFAULT_LOG_LEVEL.into())
        .parse_lossy(directives.join(","))
}

/// Sets up logging for a service from `LOG_LEVEL`.
///
/// Lines show their target (module path), so the module that wrote a line
/// can be muted with a directive. Returns a handle that can replace the
/// filter at runtime.
pub fn init_log(set_global: bool) -> LogReloadHandle {
    let layer = HierarchicalLayer::default()
        .with_writer(std::io::stdout)
        .with_indent_lines(true)
        .with_indent_amount(3)
        .with_thread_names(false)
        .with_thread_ids(true)
        .with_verbose_exit(false)
        .with_verbose_entry(false)
        .with_targets(true);

    let spec = std::env::var(LOG_LEVEL_ENV).ok();
    let (filter, error) = build_filter(spec.as_deref());
    let (filter, reload_handle) = reload::Layer::new(filter);
    let subscriber = Registry::default().with(filter).with(layer);

    if set_global {
        tracing::subscriber::set_global_default(subscriber).unwrap();
    }
    tracing_log::LogTracer::init().unwrap();
    if let Some(error) = error {
        tracing::warn!("{error}");
    }
    reload_handle
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use tracing::{Event, Subscriber};
    use tracing_subscriber::layer::{Context, Layer};

    #[derive(Clone, Default)]
    struct Seen(Arc<Mutex<Vec<String>>>);

    impl<S: Subscriber> Layer<S> for Seen {
        fn on_event(&self, event: &Event<'_>, _: Context<'_, S>) {
            let meta = event.metadata();
            if let Ok(mut seen) = self.0.lock() {
                seen.push(format!("{} {}", meta.level(), meta.target()));
            }
        }
    }

    /// Emits one event per level and target and returns the ones that
    /// passed the filter built from `spec`.
    fn passed(spec: Option<&str>) -> Vec<String> {
        let (filter, _) = build_filter(spec);
        let seen = Seen::default();
        let subscriber = Registry::default().with(filter).with(seen.clone());
        tracing::subscriber::with_default(subscriber, || {
            macro_rules! emit {
                ($($target:literal),*) => {$(
                    tracing::trace!(target: $target, "e");
                    tracing::debug!(target: $target, "e");
                    tracing::info!(target: $target, "e");
                    tracing::warn!(target: $target, "e");
                )*};
            }
            emit!(
                "harvest::routes",
                "rocket::server",
                "windmill::services::ceremonies::keys_ceremony",
                "windmill::services::tally",
                "hyper::proto::h1::role",
                "hyper_util::client",
                "h2::codec",
                "aws_smithy_runtime"
            );
        });
        let seen = seen.0.lock().map(|s| s.clone()).unwrap_or_default();
        seen
    }

    fn has(seen: &[String], level: &str, target: &str) -> bool {
        seen.iter().any(|s| s == &format!("{level} {target}"))
    }

    #[test]
    fn unset_or_empty_is_info() {
        for spec in [None, Some(""), Some("  ")] {
            let seen = passed(spec);
            assert!(has(&seen, "INFO", "harvest::routes"));
            assert!(!has(&seen, "DEBUG", "harvest::routes"));
        }
    }

    #[test]
    fn plain_level_still_works() {
        let seen = passed(Some("debug"));
        assert!(has(&seen, "DEBUG", "harvest::routes"));
        assert!(!has(&seen, "TRACE", "harvest::routes"));
        assert!(has(&passed(Some("DEBUG")), "DEBUG", "harvest::routes"));
        let seen = passed(Some("warn"));
        assert!(!has(&seen, "INFO", "harvest::routes"));
        assert!(has(&seen, "WARN", "harvest::routes"));
    }

    #[test]
    fn module_directive_mutes_only_that_module() {
        let seen = passed(Some("info,windmill::services::ceremonies=warn"));
        let ceremony = "windmill::services::ceremonies::keys_ceremony";
        assert!(!has(&seen, "INFO", ceremony));
        assert!(has(&seen, "WARN", ceremony));
        assert!(has(&seen, "INFO", "windmill::services::tally"));
        assert!(has(&seen, "INFO", "harvest::routes"));
    }

    #[test]
    fn debug_keeps_dependencies_quiet() {
        let seen = passed(Some("debug"));
        assert!(!has(&seen, "DEBUG", "rocket::server"));
        assert!(has(&seen, "INFO", "rocket::server"));
        assert!(!has(&seen, "DEBUG", "hyper::proto::h1::role"));
        assert!(has(&seen, "INFO", "hyper::proto::h1::role"));
        assert!(!has(&seen, "DEBUG", "hyper_util::client"));
        assert!(!has(&seen, "DEBUG", "h2::codec"));
        assert!(has(&seen, "DEBUG", "windmill::services::tally"));
        assert!(!has(&passed(Some("trace")), "DEBUG", "aws_smithy_runtime"));
    }

    #[test]
    fn explicit_directive_overrides_quiet_default() {
        let seen = passed(Some("debug,hyper=debug"));
        assert!(has(&seen, "DEBUG", "hyper::proto::h1::role"));
        assert!(!has(&seen, "DEBUG", "h2::codec"));
        let seen = passed(Some("info,hyper=debug"));
        assert!(has(&seen, "DEBUG", "hyper::proto::h1::role"));
        assert!(!has(&seen, "DEBUG", "harvest::routes"));
    }

    #[test]
    fn quiet_defaults_never_raise_verbosity() {
        assert!(!has(
            &passed(Some("warn")),
            "INFO",
            "hyper::proto::h1::role"
        ));
        assert!(!has(
            &passed(Some("info")),
            "DEBUG",
            "hyper::proto::h1::role"
        ));
    }

    #[test]
    fn directives_without_base_use_info() {
        let seen = passed(Some("windmill=debug"));
        assert!(has(&seen, "DEBUG", "windmill::services::tally"));
        assert!(has(&seen, "INFO", "harvest::routes"));
        assert!(!has(&seen, "DEBUG", "harvest::routes"));
    }

    #[test]
    fn invalid_value_falls_back_to_info() {
        for spec in ["verbose", "info,hyper=loud", "info,windmill[="] {
            let (_, error) = build_filter(Some(spec));
            assert!(error.is_some(), "{spec} should be rejected");
            let seen = passed(Some(spec));
            assert!(has(&seen, "INFO", "harvest::routes"));
            assert!(!has(&seen, "DEBUG", "harvest::routes"));
        }
        assert!(build_filter(Some("info,hyper=warn")).1.is_none());
        assert!(build_filter(Some("info,")).1.is_none());
    }
}
