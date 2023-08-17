// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{Context, Result};
use clap::Parser;
use std::env;
use std::path::PathBuf;
use tracing_subscriber::filter;
use tracing::{debug, instrument};

use immudb_rs::Client;

use immu_board::util::init_log;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Path to the cache directory
    /// [example: path/to/dir]
    #[arg(short, long, value_name="PATH")]
    cache_dir: PathBuf,

    /// Immugw Server URL
    /// [example: http://127.0.0.1:3323]
    /// [default: IMMUDB_SERVER_URL env var if set]
    #[arg(short, long, value_name="URL")]
    server_url: Option<String>,

    /// Index dbname
    /// [example: bbindex]
    /// [default: IMMUDB_INDEX_DBNAME env var if set]
    #[arg(short, long, value_name="DBNAME")]
    index_dbname: Option<String>,

    /// Board dbname
    /// [example: board1]
    /// [default: IMMUDB_BOARD_DBNAME env var if set]
    #[arg(short, long, value_name="DBNAME")]
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
    DeleteInitDb,
    UpsertInitDb,
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
        log_reload
            .modify(|filter| *filter = new_log_level)
            .unwrap();

        return args;
    }
}

struct BBHelper {
    client: Client,
    index_dbname: String,
    board_dbname: String,
    actions: Vec<Action>,
}

impl BBHelper {
    async fn new() -> Result<BBHelper> {
        let args = Cli::init();
        let server_url = match args.server_url.as_deref() {
            Some(server_url) => server_url.to_owned(),
            None => env::var("IMMUDB_SERVER_URL")
                .context("server_url not provided and IMMUDB_SERVER_URL env var not set")?
        };
        let index_dbname = match args.index_dbname.as_deref() {
            Some(index_dbname) => index_dbname.to_owned(),
            None => env::var("IMMUDB_INDEX_DBNAME")
                .context("index_dbname not provided and IMMUDB_INDEX_DBNAME env var not set")?
        };
        let board_dbname = match args.board_dbname.as_deref() {
            Some(board_dbname) => board_dbname.to_owned(),
            None => env::var("IMMUDB_BOARD_DBNAME")
                .context("board_dbname not provided and IMMUDB_BOARD_DBNAME env var not set")?
        };

        // Authenticate
        let username = match args.username.as_deref() {
            Some(username) => username.to_owned(),
            None => env::var("IMMUDB_USERNAME")
            .context("username not provided and IMMUDB_USERNAME env var not set")?
        };
        let password = match args.password.as_deref() {
            Some(password) => password.to_owned(),
            None => env::var("IMMUDB_PASSWORD")
                .context("password not provided and IMMUDB_PASSWORD env var not set")?
        };
        let mut client = Client::new(&server_url).await?;
        client.login(&username, &password).await?;
        Ok(
            BBHelper {
                client: client,
                index_dbname: index_dbname,
                board_dbname: board_dbname,
                actions: args.actions
            }
        )
    }

    /// Creates the database, only if it doesn't exist. It also creates
    /// the appropriate tables if they don't exist.
    async fn upsert_database(
        &mut self, database_name: &str, tables: &str
    ) -> Result<()>
    {
        // create database if it doesn't exist
        if !self.client.has_database(database_name).await? {
            self.client.create_database(database_name).await?;
            debug!("Database created!");
        };
        self.client.use_database(database_name).await?;

        // List tables and create them if missing
        if !self.client.has_tables().await? {
            debug!("no tables! let's create them");
            self.client.sql_exec(&tables, vec![]).await?;
        }
        Ok(())
    }

    async fn delete_database(&mut self, database_name: &str) -> Result<()>
    {
        if !self.client.has_database(database_name).await?
        {
            self.client.delete_database(database_name).await?;
        }
        Ok(())
    }

    async fn upsert_index_db(&mut self) -> Result<()> {
        self.upsert_database(
            self.index_dbname.clone().as_str(),
            r#"
            CREATE TABLE IF NOT EXISTS bulletin_boards (
                id INTEGER,
                database_name VARCHAR,
                is_archived BOOLEAN,
                PRIMARY KEY id
            );
            "#
        ).await
    }

    async fn delete_index_db(&mut self) -> Result<()> {
        self.delete_database(self.index_dbname.clone().as_str()).await
    }

    async fn upsert_board_db(&mut self) -> Result<()> {
        self.upsert_database(
            self.board_dbname.clone().as_str(),
            r#"
            CREATE TABLE IF NOT EXISTS messages (
                id INTEGER AUTO_INCREMENT,
                created TIMESTAMP,
                signer_key BLOB,
                statement_timestamp TIMESTAMP,
                statement_kind VARCHAR,
                message BLOB,
                PRIMARY KEY id
            );
            "#
        ).await
    }

    async fn delete_board_db(&mut self) -> Result<()> {
        self.delete_database(self.board_dbname.clone().as_str()).await
    }

    // Run the given actions
    async fn run_actions(&mut self) -> Result<()> {
        for action in self.actions.clone().iter() {
            debug!("executing action {:?}:\n", action);

            match action {
                Action::DeleteInitDb => self.delete_index_db().await?,
                Action::UpsertInitDb => self.upsert_index_db().await?,
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
