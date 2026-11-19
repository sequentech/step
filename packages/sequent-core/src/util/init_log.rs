// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Logging setup shared by the Rust services.
//!
//! `LOG_LEVEL` accepts a plain level (`info`, `debug`, ...) and
//! `target=level` directives, separated by commas, for example
//! `info,hyper=warn,windmill::services::ceremonies=warn`. A directive applies
//! to every target (module path) that starts with it.
//!
//! Span and field filters (`target[span]=level`) are not supported: inside a
//! matching span they enable events from any target, which would bypass the
//! directives that mute a module. Invalid directives are dropped one by one,
//! so a typo in one doesn't undo the others.
//!
//! A base level more verbose than `info` applies only to our own crates
//! (see [`FIRST_PARTY_TARGETS`]); every dependency stays at `info`. At
//! `debug`, `rocket` logs every request with its headers, including bearer
//! tokens, and other dependencies print payloads too. A directive naming a
//! dependency turns it on deliberately: `LOG_LEVEL=debug,rocket=debug`.

use tracing::level_filters::LevelFilter;
use tracing_subscriber::reload::{self, Handle};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::{layer::SubscriberExt, registry::Registry};
use tracing_tree::HierarchicalLayer;

/// Environment variable read by [`init_log`].
pub const LOG_LEVEL_ENV: &str = "LOG_LEVEL";

/// Level used when `LOG_LEVEL` sets no valid base level.
pub const DEFAULT_LOG_LEVEL: LevelFilter = LevelFilter::INFO;

/// Target prefixes of our own crates: the only ones a `debug` or `trace`
/// base level applies to. Everything else stays at `info` unless
/// `LOG_LEVEL` names it. `main` and `beat` are windmill's binaries, whose
/// targets are the binary name.
pub const FIRST_PARTY_TARGETS: &[&str] = &[
    "harvest",
    "windmill",
    "main",
    "beat",
    "velvet",
    "orare",
    "doc_renderer",
    "b3",
    "braid",
    "strand",
    "sequent_core",
    "electoral_log",
    "immu_board",
    "immudb_rs",
];

/// Handle to change the log filter while the service runs.
pub type LogReloadHandle = Handle<EnvFilter, Registry>;

/// A valid `LOG_LEVEL` directive.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Directive {
    Base(LevelFilter),
    Target { target: String, level: LevelFilter },
}

impl Directive {
    fn parse(directive: &str) -> Result<Self, String> {
        if let Ok(level) = directive.parse::<LevelFilter>() {
            return Ok(Directive::Base(level));
        }
        let Some((target, level)) = directive.split_once('=') else {
            return Err(format!(
                "{directive:?} is not a level or target=level"
            ));
        };
        let target = target.trim();
        let is_target_char =
            |c: char| c.is_ascii_alphanumeric() || matches!(c, '_' | ':' | '-');
        if target.is_empty() || !target.chars().all(is_target_char) {
            return Err(format!(
                "{directive:?} has an unsupported target (only module paths, \
                 without span or field filters)"
            ));
        }
        // `LevelFilter` parses an empty string as `error`.
        let level = level.trim();
        if level.is_empty() {
            return Err(format!("{directive:?} has no level"));
        }
        let level = level
            .parse::<LevelFilter>()
            .map_err(|_| format!("{directive:?} has an unknown level"))?;
        Ok(Directive::Target {
            target: target.to_string(),
            level,
        })
    }
}

impl std::fmt::Display for Directive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Directive::Base(level) => write!(f, "{level}"),
            Directive::Target { target, level } => {
                write!(f, "{target}={level}")
            }
        }
    }
}

/// Builds the filter for a `LOG_LEVEL` value.
///
/// Returns the filter and, when some directives were invalid and dropped,
/// a message to log once logging is up. The service never fails to start
/// because of `LOG_LEVEL`.
pub fn build_filter(spec: Option<&str>) -> (EnvFilter, Option<String>) {
    let mut valid = Vec::new();
    let mut invalid = Vec::new();
    for part in spec.unwrap_or("").split(',').map(str::trim) {
        if part.is_empty() {
            continue;
        }
        match Directive::parse(part) {
            Ok(directive) => valid.push(directive),
            Err(err) => invalid.push(err),
        }
    }

    let base = valid
        .iter()
        .filter_map(|d| match d {
            Directive::Base(level) => Some(*level),
            Directive::Target { .. } => None,
        })
        .last()
        .unwrap_or(DEFAULT_LOG_LEVEL);

    // `LevelFilter` orders more verbose levels as greater. A base more
    // verbose than `info` is applied to our own crates only.
    let verbose = base > DEFAULT_LOG_LEVEL;
    let global = if verbose { DEFAULT_LOG_LEVEL } else { base };
    let mut directives = vec![Directive::Base(global).to_string()];
    if verbose {
        for first_party in FIRST_PARTY_TARGETS {
            // A directive for the crate, or for a prefix of its name, wins.
            let overridden = valid.iter().any(|d| {
                matches!(d, Directive::Target { target, .. } if first_party.starts_with(target.as_str()))
            });
            if !overridden {
                directives.push(format!("{first_party}={base}"));
            }
        }
    }
    directives.extend(valid.iter().filter_map(|d| match d {
        Directive::Target { .. } => Some(d.to_string()),
        Directive::Base(_) => None,
    }));

    let filter = EnvFilter::builder()
        .with_default_directive(DEFAULT_LOG_LEVEL.into())
        .parse_lossy(directives.join(","));
    let error = (!invalid.is_empty()).then(|| {
        format!(
            "ignored invalid {LOG_LEVEL_ENV} directives: {}",
            invalid.join("; ")
        )
    });
    (filter, error)
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
    // Written directly to stderr: the configured filter could hide it, for
    // example with `LOG_LEVEL=error,windmill=wrn`.
    if let Some(error) = error {
        use std::io::Write;
        let _ = writeln!(std::io::stderr(), "WARN {} {error}", module_path!());
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
                "aws_smithy_runtime",
                "celery::broker"
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
        // Not only the well-known noisy ones: any dependency.
        assert!(!has(&seen, "DEBUG", "celery::broker"));
        assert!(has(&seen, "INFO", "celery::broker"));
        assert!(has(&seen, "DEBUG", "harvest::routes"));
        assert!(has(&passed(Some("trace")), "TRACE", "harvest::routes"));
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
        let seen = passed(Some("debug,celery=debug"));
        assert!(has(&seen, "DEBUG", "celery::broker"));
        // A directive for one of our crates beats the base level.
        let seen = passed(Some("debug,windmill=warn"));
        assert!(!has(&seen, "INFO", "windmill::services::tally"));
        assert!(has(&seen, "DEBUG", "harvest::routes"));
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
        for spec in [
            "verbose",
            "infoo",
            "info,hyper=loud",
            "windmill",
            "info,windmill=",
            "info,windmill= ",
        ] {
            let (_, error) = build_filter(Some(spec));
            assert!(error.is_some(), "{spec} should be reported");
            let seen = passed(Some(spec));
            assert!(has(&seen, "INFO", "harvest::routes"));
            assert!(!has(&seen, "DEBUG", "harvest::routes"));
            assert!(has(&seen, "INFO", "windmill::services::tally"));
        }
        assert!(build_filter(Some("info,hyper=warn")).1.is_none());
        assert!(build_filter(Some("info,")).1.is_none());
    }

    #[test]
    fn invalid_directive_keeps_the_valid_ones() {
        let spec = Some("info,windmill::services::ceremonies=warn,rocket=wrn");
        assert!(build_filter(spec).1.is_some());
        let seen = passed(spec);
        let ceremony = "windmill::services::ceremonies::keys_ceremony";
        assert!(!has(&seen, "INFO", ceremony));
        assert!(has(&seen, "INFO", "harvest::routes"));
    }

    #[test]
    fn whitespace_around_directives_is_ignored() {
        let seen = passed(Some(" info , windmill = debug "));
        assert!(has(&seen, "DEBUG", "windmill::services::tally"));
        assert!(!has(&seen, "DEBUG", "harvest::routes"));
        let (_, error) = build_filter(Some("info, é=debug"));
        assert!(error.is_some());
    }

    #[test]
    fn span_directives_are_rejected() {
        for spec in [
            "debug,rocket[request]=warn",
            "debug,windmill[work]=debug",
            "info,windmill::services::probe[check_celery]",
            "info,windmill{task}=debug",
        ] {
            assert!(build_filter(Some(spec)).1.is_some(), "{spec}");
        }
        // The rejected span directive doesn't turn the dependency on.
        let seen = passed(Some("debug,rocket[request]=warn"));
        assert!(!has(&seen, "DEBUG", "rocket::server"));
    }

    #[test]
    fn span_directives_cannot_bypass_a_mute() {
        let (filter, _) =
            build_filter(Some("debug,windmill[work]=debug,rocket=off"));
        let seen = Seen::default();
        let subscriber = Registry::default().with(filter).with(seen.clone());
        tracing::subscriber::with_default(subscriber, || {
            let span = tracing::info_span!(target: "windmill::tasks", "work");
            let _guard = span.enter();
            tracing::debug!(target: "rocket::server", "e");
        });
        let seen = seen.0.lock().map(|s| s.clone()).unwrap_or_default();
        assert!(!has(&seen, "DEBUG", "rocket::server"));
    }
}
