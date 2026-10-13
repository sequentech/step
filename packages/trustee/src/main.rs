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
mod verify;

use std::env;
use std::path::PathBuf;

use anyhow::{anyhow, Context as _, Result};
use clap::{Parser, Subcommand};
use tracing::info;
use tracing_subscriber::EnvFilter;

use crate::config::Config;
use crate::platform::PlatformClient;
use protocol_board::trustee_sessions::{B4Boards, SessionSet, SessionStores};

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
    /// Verify a public DKG/tally transcript without trustee keys.
    Verify(verify::Verify),
    /// Print both public keys of an existing local keys file as JSON.
    PublicKeys {
        #[arg(long)]
        trustee_config: PathBuf,
    },
    /// Confirm custody without sending private keys to the platform.
    Attest {
        /// Public custody request downloaded from the administration portal.
        #[arg(long)]
        request: PathBuf,
        #[arg(long)]
        trustee_config: PathBuf,
        /// Bind the proof to this tally's restore step.
        #[arg(long)]
        tally_session_id: Option<String>,
        /// A new proof file to upload to the portal.
        #[arg(long)]
        output: PathBuf,
    },
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
        Command::Verify(verify) => verify.run(),
        Command::PublicKeys { trustee_config } => {
            let keys = identity::load(&trustee_config)?.public_keys()?;
            println!(
                "{}",
                serde_json::json!({
                    "signing_public_key": keys.signing_public_key,
                    "share_encryption_public_key": keys.share_encryption_public_key,
                })
            );
            Ok(())
        }
        Command::Attest {
            request,
            trustee_config,
            tally_session_id,
            output,
        } => {
            let mut request: protocol_board::CustodyRequest =
                serde_json::from_str(
                    &std::fs::read_to_string(&request)
                        .context("reading custody request")?,
                )
                .context("invalid custody request")?;
            if tally_session_id.is_some() {
                request.tally_session_id = tally_session_id;
            }
            let proof = request.attest(&identity::load(&trustee_config)?)?;
            identity::write_new(&output, &proof)
        }
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
        platform = %config.platform_url,
        "session stores in {}",
        store_directory.display()
    );
    let boards =
        B4Boards::new(config.b4_url, SessionStores::new(store_directory)?);
    let platform =
        PlatformClient::new(&config.platform_url, config.credentials)?;
    daemon::run(
        platform,
        SessionSet::new(config.trustee_name.clone(), secrets.clone(), boards),
        config.trustee_name,
        secrets,
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
