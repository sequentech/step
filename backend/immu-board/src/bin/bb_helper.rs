// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{Context, Result};
use clap::Parser;
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use tracing_subscriber::filter;
use tracing::{debug, instrument};
use tonic::{
    metadata::MetadataValue,
    transport::Channel,
    Request
};

use immudb_rs::immu_service_client::ImmuServiceClient;
use immudb_rs::{
    CreateDatabaseRequest,
    Database,
    DatabaseListRequestV2,
    UnloadDatabaseRequest,
    DeleteDatabaseRequest,
    LoginRequest,
    SqlExecRequest,
};

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
    client: ImmuServiceClient<Channel>,
    index_dbname: String,
    board_dbname: String,
    login_token: String,
    database_tokens: HashMap<String, String>,
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
        let login_request = Request::new(LoginRequest {
            user: username.clone().into(),
            password: password.clone().into()
        });
        let mut client = ImmuServiceClient::connect(server_url.clone())
            .await?;
        let response = client.login(login_request).await?;
        debug!("grpc-login-response={:?}", response);
        let login_token = format!("Bearer {}", response.get_ref().token);

        Ok(
            BBHelper {
                client: client,
                index_dbname: index_dbname,
                board_dbname: board_dbname,
                login_token: login_token,
                database_tokens: Default::default(),
                actions: args.actions
            }
        )
    }

    /// Creates an Authenticated request, with the proper Auth token
    fn get_request<T: std::fmt::Debug>(
        &self,
        data: T,
        database_name: Option<String>
    ) -> Result<Request<T>>
    {
        let mut request = Request::new(data);
        let token: MetadataValue<_> = match &database_name {
            None => self.login_token.clone(),
            Some(database_name) =>
                self.database_tokens.get(database_name).unwrap().clone()
        }.parse()?;
        request.metadata_mut().insert("authorization", token);
        debug!(
            "BBHelper::get_request(database_name={:?}): request={:?}",
            database_name,
            request,
        );

        return Ok(request);
    }

    /// Creates the database, only if it doesn't exist. It also creates
    /// the appropriate tables if they don't exist.
    async fn upsert_database(&mut self, name: &str, tables: &str) -> Result<()>
    {
        let use_db_request = self.get_request(
            Database { database_name: name.to_string() },
            None
        )?;

        // database doesn't seem to exist, so we have to create it
        match self.client.use_database(use_db_request).await {
            Err(_) => {
                debug!("Database doesn't exist, creating it");
                let create_db_request = self.get_request(
                    CreateDatabaseRequest {
                        name: name.to_string(),
                        ..Default::default()
                    },
                    None
                )?;
                let _create_db_response = self.client
                    .create_database_v2(create_db_request).await?;
                debug!("Database created! try obtaining token again..");

                let use_db_request = self.get_request(
                    Database { database_name: name.to_string() },
                    None
                )?;
                let use_db_response = self.client
                    .use_database(use_db_request)
                    .await?;
                self.database_tokens.insert(
                    name.to_string(),
                    use_db_response.get_ref().token.clone()
                );
            },
            Ok(use_db_response) => {
                debug!("Index Database already exists, refreshing token..");
                self.database_tokens.insert(
                    name.to_string(),
                    use_db_response.get_ref().token.clone()
                );
            }
        };

        // List tables and create them if missing
        let list_tables_request = self.get_request(
            (),
            Some(name.to_string())
        )?;
        let list_tables_response = self.client
            .list_tables(list_tables_request)
            .await?;
        debug!("list_tables_response={:?}", list_tables_response);
        if list_tables_response.get_ref().rows.is_empty() {
            debug!("no tables! let's create them");
            let create_tables_request = self.get_request(
                SqlExecRequest {
                    sql: tables.to_string(),
                    no_wait: true,
                    params: vec![],
                },
                Some(name.to_string())
            )?;
            let create_tables_response = self.client
                .sql_exec(create_tables_request)
                .await?;
            debug!("create_tables_response={:?}", create_tables_response);
        } else {
            debug!(
                "Database has the following tables: {:?}",
                list_tables_response.get_ref().rows
            );
        }

        Ok(())
    }

    async fn delete_database(&mut self, name: &str) -> Result<()>
    {
        debug!("Listing databases..");
        let list_dbs_request = self.get_request(
            DatabaseListRequestV2 {}, None
        )?;

        let list_dbs_response = self.client
            .database_list_v2(list_dbs_request)
            .await?;
        let find_index_db = list_dbs_response
            .get_ref()
            .databases
            .iter()
            .find(|&database| database.name == name);
        if find_index_db.is_some() {
            debug!("Index Database found, unloading it before deletion...");
            let unload_db_request = self.get_request(
                UnloadDatabaseRequest { database: name.to_string() },
                None,
            )?;
            let _unload_db_response = self.client
                .unload_database(unload_db_request).await?;
            debug!("Deleting index database...");
            let delete_db_request = self.get_request(
                DeleteDatabaseRequest { database: name.to_string() },
                None,
            )?;
            let _delete_db_response = self.client
                .delete_database(delete_db_request).await?;
            debug!("Index Database deleted!");
        } else {
            debug!("Index Database doesn't exist, nothing to do");
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
                signer_key INTEGER,
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
