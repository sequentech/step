use clap::Parser;
use tonic::transport::Server;
use tracing::info;

use board_messages::grpc::pgsql::{PgsqlConnectionParams, XPgsqlB3Client};
use board_messages::grpc::server::PgsqlB3Server;
use board_messages::grpc::B3Server;

const PG_DATABASE: &'static str = "protocoldb";
const PG_HOST: &'static str = "localhost";
const PG_USER: &'static str = "postgres";
const PG_PASSW: &'static str = "postgrespw";
const PG_PORT: u32 = 49154;
const BIND: &'static str = "127.0.0.1:50051";

const MAX_MESSAGE_SIZE: usize = 10 * 1024 * 1024 * 1024;

#[derive(Parser)]
struct Cli {
    #[arg(long, default_value_t = PG_HOST.to_string())]
    host: String,

    #[arg(long, default_value_t = PG_PORT)]
    port: u32,

    #[arg(short, long, default_value_t = PG_USER.to_string())]
    username: String,

    #[arg(long, default_value_t = PG_PASSW.to_string())]
    password: String,

    #[arg(long, default_value_t = PG_DATABASE.to_string())]
    database: String,

    #[arg(long, default_value_t = BIND.to_string())]
    bind: String,

    #[arg(long, default_value_t = MAX_MESSAGE_SIZE)]
    max_message_size_bytes: usize,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_log(true);

    let args = Cli::parse();

    let host = &args.host;
    let port = args.port;
    let username = &args.username;
    let database = &args.database;
    let bind = &args.bind;
    let max_message_size = &args.max_message_size_bytes;

    info!("Starting b3");
    info!("pgsql host: '{host}'");
    info!("pgsql port: {port}");
    info!("pgsql username: '{username}'");
    info!("pgsql database: '{database}'");
    info!("grpc socket: {bind}");
    info!("grpc max_message_size: {} MB", (max_message_size / 1000));

    let c = PgsqlConnectionParams::new(host, port, username, &args.password);
    let c_db = c.with_database(&database);
    let client = XPgsqlB3Client::new(&c_db).await?;
    info!("pgsql connection ok");
    let boards = client.get_boards().await?;
    info!("there are {} boards in the index", boards.len());
    drop(client);

    let addr = bind.parse()?;
    let b3_impl = PgsqlB3Server::new(c_db).await?;
    let service = B3Server::new(b3_impl)
        .max_encoding_message_size(MAX_MESSAGE_SIZE)
        .max_decoding_message_size(MAX_MESSAGE_SIZE);

    Server::builder().add_service(service).serve(addr).await?;

    Ok(())
}

use std::str::FromStr;
use tracing::Level;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::reload::Handle;
use tracing_subscriber::{filter, reload};
use tracing_subscriber::{layer::SubscriberExt, registry::Registry};
use tracing_tree::HierarchicalLayer;

pub fn init_log(set_global: bool) -> Handle<LevelFilter, Registry> {
    let layer = HierarchicalLayer::default()
        .with_writer(std::io::stdout)
        .with_indent_lines(true)
        .with_indent_amount(3)
        .with_thread_names(false)
        .with_thread_ids(false)
        .with_verbose_exit(false)
        .with_verbose_entry(false)
        .with_targets(false);

    let level_str = std::env::var("LOG_LEVEL").unwrap_or("info".to_string());
    let level = Level::from_str(&level_str).unwrap();
    let filter = filter::LevelFilter::from_level(level);
    let (filter, reload_handle) = reload::Layer::new(filter);
    let subscriber = Registry::default().with(filter).with(layer);

    if set_global {
        tracing::subscriber::set_global_default(subscriber).unwrap();
    }
    tracing_log::LogTracer::init().unwrap();
    reload_handle
}
