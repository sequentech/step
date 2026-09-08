// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Standalone worker entry point; built from the same source modules as step-cli.
use anyhow::Context as _;
use clap::Parser;
use std::path::PathBuf;
mod load {
    pub mod config;
    pub mod files;
    pub mod input;
    pub mod worker;
    pub use config::Engine;
}

/// Consume the finite voter shards belonging to one local or indexed worker.
#[derive(Parser)]
struct Arguments {
    directory: PathBuf,
    #[arg(long)]
    workers: usize,
    /// Defaults to Kubernetes' JOB_COMPLETION_INDEX; required outside indexed pods.
    #[arg(long)]
    index: Option<usize>,
    /// Root containing the bundled engine adapters.
    #[arg(long, default_value = "/runner")]
    assets: PathBuf,
}
fn run() -> anyhow::Result<()> {
    let arguments = Arguments::parse();
    let index = match arguments.index {
        Some(index) => index,
        None => std::env::var("JOB_COMPLETION_INDEX")
            .context("Pass --index or set JOB_COMPLETION_INDEX")?
            .parse()
            .context("JOB_COMPLETION_INDEX must be a non-negative integer")?,
    };
    load::worker::node(
        &arguments.directory.canonicalize().with_context(|| {
            format!(
                "Run directory {} is not readable",
                arguments.directory.display()
            )
        })?,
        index,
        arguments.workers,
        &arguments.assets,
    )
}
fn main() {
    if let Err(error) = run() {
        eprintln!("{error:#}");
        std::process::exit(1);
    }
}
