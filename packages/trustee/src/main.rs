// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! `trustee`: one trustee of the platform's ceremonies on the crypto core.
//!
//! `trustee generate` makes the trustee's keys file; `trustee start` runs the
//! trustee. It asks harvest which boards are its own, runs braid's session
//! over each of them against the board service, and reports to harvest a
//! session that halted.

mod config;
mod daemon;
mod identity;
mod login;
mod platform;
mod sessions;
mod transport;

use std::env;
use std::path::PathBuf;

use anyhow::{anyhow, Context as _, Result};
use clap::{Parser, Subcommand};
use tracing::info;
use tracing_subscriber::EnvFilter;

use crate::config::Config;
use crate::platform::PlatformClient;
use crate::sessions::{B4Boards, SessionSet, SessionStores};

/// The variable the log level is read from, as by the platform's services.
const LOG_LEVEL: &str = "LOG_LEVEL";
const DEFAULT_LOG_LEVEL: &str = "info";

/// Where the sessions' stores are kept, under the working directory.
const STORE_DIRECTORY: &str = "message_store";

#[derive(Parser)]
#[command(about = "A trustee of the platform's ceremonies")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Generate a keys file and print its public keys.
    Generate {
        /// Write the keys file here, readable by its owner only; without it,
        /// the keys file is printed to standard output.
        #[arg(long)]
        trustee_config: Option<PathBuf>,
    },
    /// Run the trustee.
    Start {
        /// The board service; B4_URL when absent.
        #[arg(long)]
        b4_url: Option<String>,
        /// The keys file; TRUSTEE_CONFIG_PATH when absent.
        #[arg(long)]
        trustee_config: Option<PathBuf>,
    },
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Generate { trustee_config } => {
            identity::generate(trustee_config.as_deref())
        }
        Command::Start {
            b4_url,
            trustee_config,
        } => {
            init_logging()?;
            let config = Config::read(b4_url, trustee_config)?;
            // braid's transport and persistence futures are not `Send`.
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .context("building the runtime")?
                .block_on(start(config))
        }
    }
}

async fn start(config: Config) -> Result<()> {
    let secrets = identity::load(&config.keys_file)?;
    let store_directory = env::current_dir()
        .context("reading the working directory")?
        .join(STORE_DIRECTORY);
    info!(
        trustee = %config.trustee_name,
        board_service = %config.b4_url,
        platform = %config.harvest_url,
        "session stores in {}",
        store_directory.display()
    );
    let boards =
        B4Boards::new(config.b4_url, SessionStores::new(store_directory)?);
    let platform =
        PlatformClient::new(&config.harvest_url, config.credentials)?;
    daemon::run(
        platform,
        SessionSet::new(config.trustee_name, secrets, boards),
    )
    .await
}

fn init_logging() -> Result<()> {
    let level = env::var(LOG_LEVEL)
        .ok()
        .filter(|level| !level.is_empty())
        .unwrap_or_else(|| DEFAULT_LOG_LEVEL.to_string());
    let filter = EnvFilter::try_new(&level)
        .with_context(|| format!("{LOG_LEVEL} {level:?}"))?;
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .try_init()
        .map_err(|err| anyhow!(err).context("setting up the log"))
}
