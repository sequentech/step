// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Finite voting workloads with an explicit preparation/execution boundary.
//!
//! `init` creates editable YAML using the existing tenant administrator session.
//! `prepare` provisions a synthetic election and freezes one unique voter range.
//! `run` consumes that range once; a failed worker never makes it reusable.
//! `report` merges individual samples and renders a portable HTML artifact.
//!
//! Orchestration, encryption and reporting are native Rust. The bundled k6 and
//! Playwright adapters are checked before provisioning.
//! Administrator credentials remain on the coordinator; workers receive only the
//! synthetic password named by the configuration. See [`config::Settings`].
mod census;
mod config;
mod coordinator;
mod encryption;
mod executor;
mod files;
mod image;
mod input;
mod presentation;
mod provision;
mod reference;
mod report;
mod worker;

use anyhow::{bail, Context, Result};
use clap::{Subcommand, ValueEnum};
use config::Settings;
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command as Process,
};
include!(concat!(env!("OUT_DIR"), "/load_assets.rs"));

/// Supported journey implementations; both authenticate a distinct voter per iteration.
#[derive(Clone, Copy, Debug, ValueEnum, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Engine {
    /// Authenticated HTTP with native encryption completed before measurement.
    K6,
    /// Full browser rendering, selection, encryption and confirmation.
    Chromium,
}

/// Execution location. Worker ownership is identical across all locations.
#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum Executor {
    /// Parallel worker processes on the coordinator.
    Local,
    /// Isolated containers sharing prepared inputs through a bind mount.
    Docker,
    /// Indexed pods with durable shared input and attempt storage.
    Kubernetes,
}

/// Public load-testing lifecycle. Every operation exits nonzero on failure.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Generate command/configuration Markdown from help text and Rust field documentation.
    Reference {
        /// Destination file; stdout when omitted.
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Internal coordinator subprocess; legacy client diagnostics remain in private logs.
    #[command(hide = true)]
    Setup {
        settings: PathBuf,
        base: PathBuf,
        output: PathBuf,
    },
    /// Build a source-only worker image from the bundled runtime.
    Image {
        /// Engine to include in the image.
        #[arg(long)]
        engine: Engine,
        /// Docker tag; include a registry for Kubernetes workers.
        #[arg(long)]
        tag: String,
        /// Push the built image using the current Docker registry credentials.
        #[arg(long)]
        push: bool,
        /// Rust builder image for the standalone worker.
        #[arg(long, default_value = "rust:1.90-bookworm")]
        rust_image: String,
        /// Image supplying the k6 executable.
        #[arg(long, default_value = "grafana/k6:1.6.0")]
        k6_image: String,
        /// Base image for protocol workers.
        #[arg(long, default_value = "debian:bookworm-slim")]
        worker_image: String,
        /// Base image for browser workers.
        #[arg(long, default_value = "node:22-bookworm-slim")]
        browser_image: String,
        /// Playwright version, kept in sync with the portal test runner.
        #[arg(long, default_value = "1.62.1")]
        playwright_version: String,
    },
    /// Generate a documented workload using the configured tenant administrator.
    Init {
        /// Local devcontainer defaults or remote deployment URLs from CLI configuration.
        #[arg(long, default_value = "local", value_parser = ["local", "remote"])]
        target: String,
        /// YAML destination; existing files are never overwritten.
        #[arg(long, default_value = "voting-load.yaml")]
        output: PathBuf,
        /// Voting-portal base URL; required for remote targets.
        #[arg(long)]
        portal_url: Option<String>,
        /// Public publication-storage origin; required for remote targets.
        #[arg(long)]
        storage_origin: Option<String>,
    },
    /// Validate dependencies, create the election/census, and prepare immutable worker inputs.
    Prepare {
        /// Workload YAML written by init.
        config: PathBuf,
        /// Fresh run directory, including private provisioning logs and worker inputs.
        #[arg(long)]
        output: PathBuf,
        /// Override the engine before preparing inputs.
        #[arg(long)]
        engine: Option<Engine>,
        /// Override parallel preparation workers.
        #[arg(long)]
        workers: Option<usize>,
    },
    /// Execute a prepared run once and automatically write its HTML report.
    Run {
        /// Prepared run directory.
        directory: PathBuf,
        /// Override the worker count recorded in the workload.
        #[arg(long)]
        workers: Option<usize>,
        /// Execution backend; defaults to the workload's configured executor.
        #[arg(long)]
        executor: Option<Executor>,
    },
    /// Rebuild aggregate results; optional database auditing remains coordinator-only.
    Report {
        /// Completed or interrupted run directory.
        directory: PathBuf,
        /// Environment variable containing a read-only backend PostgreSQL DSN.
        #[arg(long)]
        dsn_env: Option<String>,
        /// Open the standalone report with the configured browser opener.
        #[arg(long)]
        open: bool,
        /// Capture the report as a PNG using the configured Playwright browser.
        #[arg(long)]
        screenshot: Option<PathBuf>,
    },
    /// Validate configuration and check installed engine dependencies.
    Check { config: PathBuf },
    /// Internal streaming encryption worker; emits only castable ciphertext JSONL.
    #[command(hide = true)]
    Encrypt {
        style: PathBuf,
        choices: PathBuf,
        count: usize,
    },
}

/// Extract content-addressed bundled files without requiring a repository checkout.
fn runtime() -> Result<PathBuf> {
    let mut hash = Sha256::new();
    for (name, bytes) in ASSETS {
        hash.update(name);
        hash.update(bytes);
    }
    use std::os::unix::fs::DirBuilderExt;
    let cache = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))
        .context("Set XDG_CACHE_HOME for the bundled runtime cache")?;
    let root = cache
        .join("step-cli/load")
        .join(hex::encode(hash.finalize()));
    for (name, bytes) in ASSETS {
        let destination = root.join(name);
        let parent = destination.parent().context("Invalid bundled path")?;
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(parent)?;
        match fs::symlink_metadata(&destination) {
            Ok(metadata) => {
                if !metadata.is_file() || fs::read(&destination)? != *bytes {
                    bail!("Bundled runtime cache is modified: {}; remove this cache directory and retry", destination.display());
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                use std::io::Write;
                use std::os::unix::fs::OpenOptionsExt;
                let mut file = fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .mode(0o600)
                    .open(&destination)?;
                file.write_all(bytes)?;
            }
            Err(error) => return Err(error.into()),
        }
    }
    Ok(root)
}

/// Resolve a run configuration from its immutable operator-settings snapshot.
fn run_settings(directory: &Path) -> Result<Settings> {
    Settings::read(&directory.join("settings.yaml"))
}

impl Command {
    /// Dispatch a lifecycle operation without shell interpolation or implicit cast retries.
    pub fn run(&self) -> Result<()> {
        match self {
            Self::Reference { output } => {
                let text = reference::markdown()?;
                if let Some(path) = output {
                    fs::write(path, text)?;
                } else {
                    print!("{text}");
                }
                Ok(())
            }
            Self::Setup {
                settings,
                base,
                output,
            } => provision::setup(settings, base, output, &runtime()?),
            Self::Image {
                engine,
                tag,
                push,
                rust_image,
                k6_image,
                worker_image,
                browser_image,
                playwright_version,
            } => image::build(
                &runtime()?,
                *engine,
                tag,
                *push,
                rust_image,
                k6_image,
                worker_image,
                browser_image,
                playwright_version,
            ),
            Self::Encrypt {
                style,
                choices,
                count,
            } => encryption::encrypt(style, choices, *count),
            Self::Init {
                target,
                output,
                portal_url,
                storage_origin,
            } => {
                let session = crate::utils::read_config::read_config()
                    .map_err(|error| anyhow::anyhow!(error.to_string()))?;
                let mut settings = Settings::default();
                settings.target.tenant_id = session.tenant_id;
                if target == "local" {
                    settings.execution.network = if Path::new("/.dockerenv").exists() {
                        format!("container:{}", fs::read_to_string("/etc/hostname")?.trim())
                    } else {
                        "host".into()
                    };
                }
                settings.target.graphql_url = session.endpoint_url;
                settings.target.keycloak_url = session.keycloak_url;
                settings.target.upload_mode = if target == "local" {
                    config::UploadMode::Local
                } else {
                    config::UploadMode::Direct
                };
                if target == "remote" && (portal_url.is_none() || storage_origin.is_none()) {
                    bail!("Remote init requires --portal-url and --storage-origin");
                }
                if let Some(url) = portal_url {
                    settings.target.portal_url = url.clone();
                }
                if let Some(url) = storage_origin {
                    settings.target.storage_origins = vec![url.clone()];
                }
                settings.validate()?;
                settings.create(output)?;
                println!(
                    "Created {}. Review its target and goals, then run step-cli load check {}.",
                    output.display(),
                    output.display()
                );
                Ok(())
            }
            Self::Prepare {
                config,
                output,
                engine,
                workers,
            } => {
                let mut settings = Settings::read(config)?;
                if let Some(engine) = engine {
                    settings.workload.engine = *engine;
                }
                if let Some(workers) = workers {
                    settings.execution.workers = *workers;
                }
                settings.validate()?;
                if output.exists() {
                    bail!("Run directory already exists; choose a fresh --output");
                }
                coordinator::prepare(&settings, config, output, &runtime()?)
            }
            Self::Run {
                directory,
                workers,
                executor,
            } => {
                let settings = run_settings(directory)?;
                let workers = workers.unwrap_or(settings.execution.workers);
                if workers == 0 {
                    bail!("workers must be positive");
                }
                let executor = executor
                    .map(|e| format!("{e:?}").to_lowercase())
                    .unwrap_or(settings.execution.executor.clone());
                executor::run(directory, &settings, workers, &executor, &runtime()?)
            }
            Self::Report {
                directory,
                dsn_env,
                open,
                screenshot,
            } => {
                let settings = run_settings(directory)?;
                executor::report(directory, dsn_env.as_deref())?;
                if let Some(destination) = screenshot {
                    coordinator::screenshot(
                        &settings,
                        &directory.join("report.html"),
                        destination,
                    )?;
                }
                if *open {
                    let status = Process::new(&settings.runtime.opener)
                        .arg(directory.join("report.html"))
                        .status()?;
                    if !status.success() {
                        bail!("Browser opener failed; open report.html directly");
                    }
                }
                Ok(())
            }
            Self::Check { config } => {
                let settings = Settings::read(config)?;
                coordinator::check(&settings)
            }
        }
    }
}

#[cfg(test)]
mod tests;
