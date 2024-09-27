// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{Context, Result};
use clap::Parser;
use std::env;
use std::path::PathBuf;
use tracing::{debug, instrument};
use tracing_subscriber::filter;

use electoral_log::util::init_log;
use electoral_log::BoardClient;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Path to the cache directory
    /// [example: path/to/dir]
    #[arg(short, long, value_name = "PATH")]
    cache_dir: PathBuf,

    /// Immugw Server URL
    /// [example: http://127.0.0.1:3323]
    /// [default: IMMUDB_SERVER_URL env var if set]
    #[arg(short, long, value_name = "URL")]
    server_url: Option<String>,

    /// Board dbname
    /// [example: board1]
    /// [default: IMMUDB_BOARD_DBNAME env var if set]
    #[arg(short, long, value_name = "DBNAME")]
    board_dbname: Option<String>,

    /// Immugw username
    /// [example: immudb]
    /// [default: IMMUDB_USERNAME env var if set]
    #[arg(short, long)]
    username: Option<String>,

    /// Immugw password
    /// [example: immudb]
    /// [default: IMMUDB_PASSWORD env var if set]
    #[arg(short, long)]
    password: Option<String>,

    /// Action to execute
    #[arg(value_enum)]
    actions: Vec<Action>,

    /// Verbosity level
    #[arg(short,long, value_enum, default_value_t=LogLevel::Info)]
    log_level: LogLevel,
}

#[derive(clap::ValueEnum, Clone)]
enum LogLevel {
    Off,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum Action {
    DeleteBoardDb,
    UpsertBoardDb,
}

impl Cli {
    fn init() -> Self {
        let log_reload = init_log(true);
        let args = Cli::parse();

        // set log level
        let new_log_level = match args.log_level.clone() {
            LogLevel::Off => filter::LevelFilter::OFF,
            LogLevel::Error => filter::LevelFilter::ERROR,
            LogLevel::Warn => filter::LevelFilter::WARN,
            LogLevel::Debug => filter::LevelFilter::DEBUG,
            LogLevel::Trace => filter::LevelFilter::TRACE,
            _ => filter::LevelFilter::INFO,
        };
        log_reload.modify(|filter| *filter = new_log_level).unwrap();

        return args;
    }
}

struct BBHelper {
    client: BoardClient,
    board_dbname: String,
    actions: Vec<Action>,
}

impl BBHelper {
    async fn new() -> Result<BBHelper> {
        let args = Cli::init();
        let server_url = match args.server_url.as_deref() {
            Some(server_url) => server_url.to_owned(),
            None => env::var("IMMUDB_SERVER_URL")
                .context("server_url not provided and IMMUDB_SERVER_URL env var not set")?,
        };
        let board_dbname = match args.board_dbname.as_deref() {
            Some(board_dbname) => board_dbname.to_owned(),
            None => env::var("IMMUDB_BOARD_DBNAME")
                .context("board_dbname not provided and IMMUDB_BOARD_DBNAME env var not set")?,
        };

        // Authenticate
        let username = match args.username.as_deref() {
            Some(username) => username.to_owned(),
            None => env::var("IMMUDB_USERNAME")
                .context("username not provided and IMMUDB_USERNAME env var not set")?,
        };
        let password = match args.password.as_deref() {
            Some(password) => password.to_owned(),
            None => env::var("IMMUDB_PASSWORD")
                .context("password not provided and IMMUDB_PASSWORD env var not set")?,
        };
        let client = BoardClient::new(&server_url, &username, &password).await?;
        Ok(BBHelper {
            client: client,
            board_dbname: board_dbname,
            actions: args.actions,
        })
    }

    async fn upsert_board_db(&mut self) -> Result<()> {
        self.client
            .upsert_electoral_log_db(self.board_dbname.clone().as_str())
            .await
    }

    async fn delete_board_db(&mut self) -> Result<()> {
        self.client
            .delete_database(self.board_dbname.clone().as_str())
            .await
    }

    // Run the given actions
    async fn run_actions(&mut self) -> Result<()> {
        for action in self.actions.clone().iter() {
            debug!("executing action {:?}:\n", action);

            match action {
                Action::DeleteBoardDb => self.delete_board_db().await?,
                Action::UpsertBoardDb => self.upsert_board_db().await?,
            }
        }
        Ok(())
    }
}

#[tokio::main]
#[instrument]
async fn main() -> Result<()> {
    let mut helper = BBHelper::new().await?;
    helper.run_actions().await?;
    Ok(())
}
