cfg_if::cfg_if! {
    if #[cfg(feature = "bb-test")] {

use anyhow::{anyhow, Context, Result};
use braid::util::init_log;
use clap::Parser;
use reqwest::{
    Client,
    header,
    header::HeaderMap,
    header::HeaderValue
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_with::{serde_as, base64::Base64};
use std::env;
use std::path::PathBuf;
use tracing_subscriber::filter;
use tracing::{debug, instrument};

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


/// These structs allow to serialize/deserialize requests with Base64 encoding

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
struct ImmudbLogin {
    #[serde_as(as = "Base64")]
    user: Vec<u8>,

    #[serde_as(as = "Base64")]
    password: Vec<u8>,
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
struct ImmudbCreate {
    #[serde_as(as = "Base64")]
    name: Vec<u8>,

    ifNotExists: bool,
}


struct BBHelper {
    client: Client,
    server_url: String,
    index_dbname: String,
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

        let client = BBHelper::get_client(&args, &server_url).await?;

        Ok(
            BBHelper {
                client: client,
                server_url: server_url,
                index_dbname: index_dbname,
            }
        )
    }

    /// Returns an authenticated reqwest client
    async fn get_client(args: &Cli, server_url: &String) -> Result<Client> {
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

        let login_credentials = ImmudbLogin {
            user: username.into(),
            password: password.into()
        };
        let client = reqwest::Client::new();
        let response = client.post(server_url.to_owned() + "/login")
            .json(&login_credentials)
            .send()
            .await?;

        let status = response.status();
        let body_bytes = &response.bytes().await?;
        debug!(
            "authentication returned: status={:?} body={:?}",
            status,
            body_bytes
        );
        if status != 200 {
            return Err(anyhow!("auth-error"));
        }

        let body_json: Value = serde_json::from_slice(&body_bytes)?;
        let auth_token = body_json["token"]
            .as_str()
            .ok_or(anyhow!("Authentication token key not found"))?;

        let mut headers = HeaderMap::new();
        let mut auth_value = HeaderValue::from_str(auth_token)?;
        auth_value.set_sensitive(true);
        headers.insert(header::AUTHORIZATION, auth_value);

        Ok(
            Client::builder()
                .default_headers(headers)
                .build()?
        )
    }

    /// Creates the index database, only if it doesn't exist
    async fn create_index_db(&self) -> Result<()> {
        let url = format!("{}/db/{}/health", self.server_url, self.index_dbname);
        let response = self.client
            .get(&url)
            .send()
            .await?;

        let status = response.status();
        let body_bytes = &response.bytes().await?;
        debug!(
            "GET request:\n\t- url={}\n\t- status={:?}\n\t- body={:?}",
            url,
            status,
            body_bytes
        );
        if status == 404 {
            // Create the missing index database
            let data = ImmudbCreate {
                name: self.index_dbname.clone().into(),
                ifNotExists: true,
            };
            let url = format!(
                "{}/db/{}/create/v2", self.server_url, self.index_dbname
            );
            let response = self.client.post(&url)
                .json(&data)
                .send()
                .await?;
            let status = response.status();
            let body_bytes = &response.bytes().await?;
            debug!(
                "POST request:\n\t- url={}\n\t- status={:?}\n\t- body={:?}",
                url,
                status,
                body_bytes
            );
            if status != 200 {
                return Err(anyhow!("db-create-error"));
            }        
        }

        Ok(())
    }
}

#[tokio::main]
#[instrument]
async fn main() -> Result<()> {
    let helper = BBHelper::create().await?;
    helper.create_index_db().await?;

    Ok(())
}

    } else {

fn main() {
    println!("Requires the 'bb-test' feature");
}

    }
}
