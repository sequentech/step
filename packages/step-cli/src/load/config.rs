// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Operator configuration. Defaults are centralized here and emitted by `load init`.
//! Unknown keys are rejected so a misspelled goal cannot silently disable validation.
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
};

/// Supported journey implementations; both authenticate a distinct voter per iteration.
#[derive(Clone, Copy, Debug, clap::ValueEnum, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Engine {
    /// Authenticated HTTP with native encryption completed before measurement.
    K6,
    /// Full browser rendering, selection, encryption and confirmation.
    Chromium,
}

/// Complete reproducible workload; secrets are referenced by environment-variable name.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    /// Deployment endpoints and tenant scope.
    pub target: Target,
    /// Voter identity range, engine and finite iteration count.
    pub workload: Workload,
    /// Worker topology and container infrastructure.
    pub execution: Execution,
    /// Provisioning policy and optional existing event.
    pub preparation: Preparation,
    /// Executables and browser dependency location.
    pub runtime: Runtime,
    /// Aggregate rendering and optional database audit settings.
    pub reporting: Reporting,
    /// Maximum p50/p99 latency by stage; throughput is configured separately.
    pub goals: BTreeMap<String, BTreeMap<String, f64>>,
    /// Minimum accepted casts per second; zero disables this goal.
    pub min_casts_per_second: f64,
}

/// Upload routing policy. Direct preserves the deployment's signed upload URL.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UploadMode {
    /// Rewrite only upload hosts for the local devcontainer network.
    Local,
    /// Use the deployment's signed upload address unchanged.
    Direct,
}

/// JavaScript workers must represent every voter suffix exactly.
const MAX_EXACT_VOTER_INDEX: u64 = (1_u64 << 53) - 1;

/// Addresses permitted during a run. Storage origins never rewrite signed URLs.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Target {
    /// Existing synthetic tenant administered by the CLI session.
    pub tenant_id: String,
    /// Voting portal base URL, including a path prefix if deployed under one.
    pub portal_url: String,
    /// Keycloak base URL.
    pub keycloak_url: String,
    /// Full GraphQL endpoint.
    pub graphql_url: String,
    /// Public S3/CDN origins returned by publication signing.
    pub storage_origins: Vec<String>,
    /// Rewrite upload hosts for the devcontainer's local storage network only.
    pub upload_mode: UploadMode,
}
impl Default for Target {
    fn default() -> Self {
        Self {
            tenant_id: String::new(),
            portal_url: "http://localhost:3000".into(),
            keycloak_url: "http://keycloak:8090".into(),
            graphql_url: "http://graphql-engine:8080/v1/graphql".into(),
            storage_origins: vec!["http://minio-proxy:9002".into()],
            upload_mode: UploadMode::Local,
        }
    }
}

/// One iteration owns one patterned username and one cast attempt.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Workload {
    /// HTTP protocol or full browser journey.
    pub engine: Engine,
    /// `vote` executes the whole journey; `status` measures login and voter status.
    pub mode: String,
    /// Number of distinct voters, independent of worker count.
    pub count: usize,
    /// Numeric suffix of the first voter.
    pub start: u64,
    /// Prefix prepended to every numeric suffix.
    pub username_prefix: String,
    /// Maximum ballots loaded by one worker at a time.
    pub shard_size: usize,
    /// Concurrent voters per worker.
    pub concurrency: usize,
    /// Maximum duration of each finite k6 shard, in k6 duration notation.
    pub max_duration: String,
    /// HTTP request deadline for login, status and publication downloads.
    pub request_timeout: String,
    /// HTTP cast request deadline.
    pub cast_timeout: String,
    /// Language requested from Keycloak during protocol login.
    pub locale: String,
    /// Environment variable containing the shared synthetic voter password.
    pub password_env: String,
    /// PBKDF2-SHA256 rounds, also installed as the fixture realm's policy.
    pub hash_iterations: u32,
    /// OIDC client registered in the election realm.
    pub client_id: String,
    /// Browser journey timeout, in milliseconds.
    pub journey_timeout_ms: u64,
    /// Browser locator/expectation timeout, in milliseconds.
    pub action_timeout_ms: u64,
    /// Retain sanitized per-fetch diagnostics in private logs.
    pub trace_http: bool,
}
impl Default for Workload {
    fn default() -> Self {
        Self {
            engine: Engine::K6,
            mode: "vote".into(),
            count: 100,
            start: 0,
            username_prefix: "load-".into(),
            shard_size: 25,
            concurrency: 2,
            max_duration: "30m".into(),
            request_timeout: "30s".into(),
            cast_timeout: "60s".into(),
            locale: "en".into(),
            password_env: "LOAD_PASSWORD".into(),
            hash_iterations: 27500,
            client_id: "voting-portal".into(),
            journey_timeout_ms: 180000,
            action_timeout_ms: 15000,
            trace_http: false,
        }
    }
}

/// The same shard ownership scheme is used for all executor backends.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Execution {
    /// Independent worker processes or pods.
    pub workers: usize,
    /// `local`, `docker`, or `kubernetes`.
    pub executor: String,
    /// Worker image, built from the bundled package before remote execution.
    pub image: String,
    /// Docker network reachable from the target endpoints.
    pub network: String,
    /// Optional daemon-host path to prepared inputs; devcontainer bind mounts are otherwise discovered.
    pub docker_mount_source: Option<PathBuf>,
    /// Kubernetes namespace for this run.
    pub namespace: String,
    /// ReadWriteMany storage class; required for Kubernetes.
    pub storage_class: String,
    /// Kubernetes volume capacity, including results.
    pub storage_size: String,
    /// Maximum wait for Kubernetes resources, using kubectl duration notation.
    pub wait_timeout: String,
    /// Kubernetes per-worker requests and limits (cpu/memory resource quantities).
    pub resources: BTreeMap<String, BTreeMap<String, String>>,
}
impl Default for Execution {
    fn default() -> Self {
        Self {
            workers: 1,
            executor: "local".into(),
            image: "voting-load:k6".into(),
            network: "step_devcontainer_default".into(),
            docker_mount_source: None,
            namespace: "default".into(),
            storage_class: String::new(),
            storage_size: "20Gi".into(),
            wait_timeout: "1h".into(),
            resources: BTreeMap::from([
                (
                    "requests".into(),
                    BTreeMap::from([("cpu".into(), "1".into()), ("memory".into(), "1Gi".into())]),
                ),
                (
                    "limits".into(),
                    BTreeMap::from([("cpu".into(), "2".into()), ("memory".into(), "4Gi".into())]),
                ),
            ]),
        }
    }
}

/// Provisioning inputs. Paths are resolved relative to the workload YAML.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Preparation {
    /// Optional exported event template; the bundled single-election fixture is the default.
    pub template: Option<PathBuf>,
    /// Optional prior generated event config; provisions only a new voter range.
    pub existing_event: Option<PathBuf>,
    /// Optional explicit decoded ballot selections.
    pub choices: Option<PathBuf>,
    /// Automatic trustee threshold.
    pub threshold: usize,
    /// Key-ceremony polling interval in seconds.
    pub poll_interval_seconds: u64,
    /// Maximum ceremony wait in seconds.
    pub ceremony_timeout_seconds: u64,
    /// Optional application publication writer for deployments with separate S3 preparation.
    pub publication_preparer: Option<PathBuf>,
}
impl Default for Preparation {
    fn default() -> Self {
        Self {
            template: None,
            existing_event: None,
            choices: None,
            threshold: 2,
            poll_interval_seconds: 5,
            ceremony_timeout_seconds: 600,
            publication_preparer: None,
        }
    }
}

/// Runtime commands are argument-vector executables, never shell command strings.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Runtime {
    /// k6 executable.
    pub k6: String,
    /// Node.js executable for Chromium workers.
    pub node: String,
    /// Directory containing the installed @playwright/test package.
    pub playwright_dir: PathBuf,
    /// Optional system Chromium executable; otherwise use Playwright's matching browser.
    pub chromium: Option<PathBuf>,
    /// Executable used by `report --open`.
    pub opener: String,
}
impl Default for Runtime {
    fn default() -> Self {
        Self {
            k6: "k6".into(),
            node: "node".into(),
            playwright_dir: std::env::current_dir()
                .unwrap_or_default()
                .join("packages/voting-portal"),
            chromium: std::env::var_os("CHROMIUM_EXECUTABLE_PATH")
                .map(PathBuf::from)
                .or_else(|| {
                    std::env::var_os("PATH").and_then(|path| {
                        std::env::split_paths(&path)
                            .map(|directory| directory.join("chromium"))
                            .find(|path| path.is_file())
                    })
                }),
            opener: "xdg-open".into(),
        }
    }
}

/// Report resource limits, graph resolution and screenshot dimensions.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Reporting {
    /// Maximum failure details retained in a report; total failure counts remain exact.
    pub max_errors: usize,
    /// Quantile points per cumulative latency curve, including its endpoints.
    pub cdf_points: usize,
    /// Maximum throughput chart buckets; independent of voter count.
    pub bins: usize,
    /// SQLite aggregation cache in KiB; sorting spills to disk.
    pub sqlite_cache_kib: usize,
    /// Receipt IDs queried per read-only audit batch.
    pub audit_batch_size: usize,
    /// PostgreSQL statement timeout in milliseconds.
    pub audit_timeout_ms: u64,
    /// PostgreSQL connection timeout in seconds.
    pub audit_connect_timeout_seconds: u64,
    /// Screenshot viewport width in CSS pixels.
    pub screenshot_width: usize,
    /// Screenshot initial viewport height; the full report is captured.
    pub screenshot_height: usize,
}
impl Default for Reporting {
    fn default() -> Self {
        Self {
            max_errors: 20,
            cdf_points: 51,
            bins: 60,
            sqlite_cache_kib: 16384,
            audit_batch_size: 1000,
            audit_timeout_ms: 30000,
            audit_connect_timeout_seconds: 15,
            screenshot_width: 1200,
            screenshot_height: 1000,
        }
    }
}

/// Accept positive Go-style durations used by both k6 and kubectl.
fn valid_duration(mut value: &str) -> bool {
    let mut positive = false;
    while !value.is_empty() {
        let length = value
            .bytes()
            .take_while(|byte| byte.is_ascii_digit() || *byte == b'.')
            .count();
        if length == 0 {
            return false;
        }
        let Ok(number) = value[..length].parse::<f64>() else {
            return false;
        };
        if !number.is_finite() {
            return false;
        }
        positive |= number > 0.0;
        value = &value[length..];
        let Some(unit) = ["ms", "s", "m", "h"]
            .into_iter()
            .find(|unit| value.starts_with(unit))
        else {
            return false;
        };
        value = &value[unit.len()..];
    }
    positive
}

impl Settings {
    /// Parse YAML (including JSON), reject unknown fields, and validate before side effects.
    pub fn read(path: &Path) -> Result<Self> {
        let value: Self = serde_yaml::from_reader(
            std::fs::File::open(path).with_context(|| format!("Cannot read {}", path.display()))?,
        )?;
        value.validate()?;
        Ok(value)
    }
    /// Create a private, reviewable configuration without overwriting an existing workload.
    pub fn create(&self, path: &Path) -> Result<()> {
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path)?;
        writeln!(file, "# Voting load configuration. Secrets stay in the named environment variable.\n# Prepare freezes this configuration; use a fresh output directory for each run.\n# Latency goals use milliseconds: status_ms, cast_ms, journey_ms; quantiles p50/p99.\n# Example: goals: {{status_ms: {{p50: 100, p99: 500}}}}")?;
        file.write_all(serde_yaml::to_string(self)?.as_bytes())?;
        Ok(())
    }
    /// Enforce finite, safe ownership and meaningful goals before creating voters.
    pub fn validate(&self) -> Result<()> {
        let w = &self.workload;
        for duration in [
            &w.max_duration,
            &w.request_timeout,
            &w.cast_timeout,
            &self.execution.wait_timeout,
        ] {
            ensure!(
                valid_duration(duration),
                "Duration must be positive with ms, s, m or h units: {duration}"
            );
        }
        ensure!(
            self.reporting.max_errors > 0
                && self.reporting.cdf_points >= 2
                && self.reporting.bins > 0
                && self.reporting.sqlite_cache_kib > 0
                && self.reporting.audit_batch_size > 0
                && self.reporting.audit_timeout_ms > 0
                && self.reporting.audit_connect_timeout_seconds > 0
                && self.reporting.screenshot_width > 0
                && self.reporting.screenshot_height > 0,
            "Report limits and dimensions must be positive"
        );
        ensure!(
            !self.target.tenant_id.is_empty(),
            "target.tenant_id is required; authenticate with step-cli step config"
        );
        ensure!(
            w.count > 0 && w.shard_size > 0 && w.concurrency > 0 && self.execution.workers > 0,
            "count, shard_size, concurrency and workers must be positive"
        );
        ensure!(
            w.start
                .checked_add(w.count as u64)
                .is_some_and(|end| end <= MAX_EXACT_VOTER_INDEX),
            "Voter range exceeds JavaScript's exact integer range"
        );
        ensure!(
            w.hash_iterations > 0 && w.journey_timeout_ms > 0 && w.action_timeout_ms > 0,
            "Hash rounds and timeouts must be positive"
        );
        ensure!(
            !w.password_env.is_empty() && !w.username_prefix.is_empty(),
            "password_env and username_prefix must not be empty"
        );
        ensure!(
            matches!(w.mode.as_str(), "vote" | "status"),
            "mode must be vote or status"
        );
        ensure!(
            !(matches!(w.engine, Engine::Chromium) && w.mode == "status"),
            "status mode requires k6"
        );
        ensure!(
            matches!(
                self.execution.executor.as_str(),
                "local" | "docker" | "kubernetes"
            ),
            "Unknown executor"
        );
        ensure!(
            self.preparation.threshold > 0
                && self.preparation.poll_interval_seconds > 0
                && self.preparation.ceremony_timeout_seconds > 0,
            "Ceremony threshold and timing must be positive"
        );
        for endpoint in [
            &self.target.portal_url,
            &self.target.keycloak_url,
            &self.target.graphql_url,
        ]
        .into_iter()
        .chain(self.target.storage_origins.iter())
        {
            let url = url::Url::parse(endpoint).context("Invalid target URL")?;
            ensure!(
                matches!(url.scheme(), "http" | "https")
                    && url.host_str().is_some()
                    && url.username().is_empty()
                    && url.password().is_none()
                    && url.query().is_none()
                    && url.fragment().is_none(),
                "Target URLs must be HTTP(S) and contain no credentials, query or fragment"
            );
        }
        ensure!(
            self.min_casts_per_second.is_finite() && self.min_casts_per_second >= 0.0,
            "Throughput goal must be finite and nonnegative"
        );
        for (stage, goals) in &self.goals {
            ensure!(
                matches!(stage.as_str(), "status_ms" | "cast_ms" | "journey_ms"),
                "Unknown latency stage: {stage}"
            );
            ensure!(
                w.mode != "status" || stage != "cast_ms",
                "Status workloads cannot have cast latency goals"
            );
            for (quantile, limit) in goals {
                ensure!(
                    matches!(quantile.as_str(), "p50" | "p99") && limit.is_finite() && *limit > 0.0,
                    "Invalid latency goal: {stage}.{quantile}"
                );
            }
        }
        ensure!(
            w.mode != "status" || self.min_casts_per_second == 0.0,
            "Status workloads cannot have cast throughput goals"
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn valid() -> Settings {
        let mut s = Settings::default();
        s.target.tenant_id = "test".into();
        s
    }
    #[test]
    fn duration_errors_fail_before_provisioning() {
        for duration in ["30m", "1m30s", "500ms", ".5s"] {
            assert!(valid_duration(duration));
        }
        for duration in ["", "0s", "-1s", "NaNs", "30", "1week", "1..2s"] {
            assert!(!valid_duration(duration));
        }
        let mut settings = valid();
        settings.workload.request_timeout = "forever".into();
        assert!(settings.validate().is_err());
    }
    #[test]
    fn configuration_round_trip_preserves_custom_values() {
        let mut s = valid();
        s.workload.count = 1_000_000;
        s.execution.workers = 40;
        s.preparation.poll_interval_seconds = 2;
        let t: Settings = serde_yaml::from_str(&serde_yaml::to_string(&s).unwrap()).unwrap();
        t.validate().unwrap();
        assert_eq!(t.workload.count, 1_000_000);
        assert_eq!(t.execution.workers, 40);
        assert_eq!(t.preparation.poll_interval_seconds, 2);
    }
    #[test]
    fn misspelled_settings_are_errors() {
        assert!(serde_yaml::from_str::<Settings>("workload:\n  concurrncy: 4").is_err());
    }
    #[test]
    fn zero_workers_and_unsafe_voter_ranges_are_rejected() {
        let mut s = valid();
        s.execution.workers = 0;
        assert!(s.validate().is_err());
        s.execution.workers = 1;
        s.workload.start = u64::MAX;
        assert!(s.validate().is_err());
    }
    #[test]
    fn credentials_in_urls_are_rejected() {
        let mut s = valid();
        s.target.graphql_url = "https://user:secret@example.org/graphql".into();
        assert!(s.validate().is_err());
    }
    #[test]
    fn irrelevant_status_goals_are_rejected() {
        let mut s = valid();
        s.workload.mode = "status".into();
        s.min_casts_per_second = 1.;
        assert!(s.validate().is_err());
    }
    #[test]
    fn unknown_and_nonfinite_goals_are_rejected() {
        let mut s = valid();
        s.goals
            .insert("cast_ms".into(), BTreeMap::from([("p95".into(), 100.)]));
        assert!(s.validate().is_err());
        s.goals.clear();
        s.min_casts_per_second = f64::NAN;
        assert!(s.validate().is_err());
    }
}
