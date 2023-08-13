cfg_if::cfg_if! {
    if #[cfg(feature = "bb-test")] {

use anyhow::{Context, Result};
use braid::util::init_log;
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

use crate::immudb::immu_service_client::ImmuServiceClient;
use crate::immudb::{
    CreateDatabaseRequest,
    Database,
    LoginRequest,
    SqlExecRequest,
};

pub mod immudb {
    tonic::include_proto!("immudb.schema");
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Path to the cache directory. Example: path/to/dir
    #[arg(short, long, value_name="PATH")]
    cache_dir: PathBuf,

    /// Immugw Server URL. Example: http://127.0.0.1:3323
    #[arg(short, long, value_name="URL")]
    server_url: Option<String>,

    /// Index dbname. Example: bb_index
    #[arg(short, long, value_name="DBNAME")]
    index_dbname: Option<String>,

    /// Immugw username. Example: immudb
    #[arg(short, long)]
    username: Option<String>,

    /// Immugw password. Example: immudb
    #[arg(short, long)]
    password: Option<String>,
    
    /// Action to execute
    #[arg(value_enum)]
    action: Action,

    /// Set verbosity level
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


#[derive(clap::ValueEnum, Clone)]
enum Action {
    Init,
    Ballots,
    List,
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
    server_url: String,
    index_dbname: String,
    login_token: String,
    database_tokens: HashMap<String, String>
}

impl BBHelper {
    async fn create() -> Result<BBHelper> {
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
                server_url: server_url,
                index_dbname: index_dbname,
                login_token: login_token,
                database_tokens: Default::default()
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

    /// Creates the index database, only if it doesn't exist. It also creates
    /// the appropriate tables if they don't exist.
    async fn upsert_index_db(&mut self) -> Result<()> {
        let use_db_request = self.get_request(
            Database { database_name: self.index_dbname.clone() },
            None
        )?;
        
        // Index database doesn't seem to exist, so we have to create it
        match self.client.use_database(use_db_request).await {
            Err(_) => {
                debug!("Index Database doesn't exist, creating it");
                let create_db_request = self.get_request(
                    CreateDatabaseRequest {
                        name: self.index_dbname.clone(),
                        ..Default::default()
                    },
                    None
                )?;
                let _create_db_response = self.client
                    .create_database_v2(create_db_request).await?;
                debug!("Index Database created! try obtaining token again..");

                let use_db_request = self.get_request(
                    Database { database_name: self.index_dbname.clone() },
                    None
                )?;        
                let use_db_response = self.client
                    .use_database(use_db_request)
                    .await?;
                self.database_tokens.insert(
                    self.index_dbname.clone(),
                    use_db_response.get_ref().token.clone()
                );
            },
            Ok(use_db_response) => {
                debug!("Index Database already exists, obtaining token..");
                self.database_tokens.insert(
                    self.index_dbname.clone(),
                    use_db_response.get_ref().token.clone()
                );
            }
        };

        // List tables and create them if missing
        let list_tables_request = self.get_request(
            (),
            Some(self.index_dbname.clone())
        )?;
        let list_tables_response = self.client
            .list_tables(list_tables_request)
            .await?;
        debug!("list_tables_response={:?}", list_tables_response);
        if list_tables_response.get_ref().rows.is_empty() {
            debug!("no tables! let's create them");
            let create_tables_request = self.get_request(
                SqlExecRequest {
                    sql: String::from(r#"
                    CREATE TABLE IF NOT EXISTS bulletin_boards (
                        id INTEGER,
                        database_name VARCHAR,
                        trustee_names VARCHAR,
                        is_archived BOOLEAN,
                        PRIMARY KEY id
                    );
                    "#),
                    no_wait: true,
                    params: vec![],
                },
                Some(self.index_dbname.clone())
            )?;
            let create_tables_response = self.client
                .sql_exec(create_tables_request)
                .await?;
            debug!("create_tables_response={:?}", create_tables_response);
        } else {
            debug!(
                "index database has the following tables: {:?}",
                list_tables_response.get_ref().rows
            );
        }

        Ok(())
    }
}

#[tokio::main]
#[instrument]
async fn main() -> Result<()> {
    let mut helper = BBHelper::create().await?;
    helper.upsert_index_db().await?;

    Ok(())
}

    } else {

fn main() {
    println!("Requires the 'bb-test' feature");
}

    }
}
