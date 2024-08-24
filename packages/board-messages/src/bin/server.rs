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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_log(true);

    let host = PG_HOST;
    let port = PG_PORT;
    let user = PG_USER;
    let database = PG_DATABASE;
    let socket = "192.168.1.37:50051";

    info!("Starting b3");
    info!("pgsql host: '{host}'");
    info!("pgsql port: '{port}'");
    info!("pgsql user: '{user}'");
    info!("pgsql database: '{database}'");
    info!("grpc socket: '{socket}'");

    let c = PgsqlConnectionParams::new(PG_HOST, PG_PORT, PG_USER, PG_PASSW);
    let c_db = c.with_database(database);
    let client = XPgsqlB3Client::new(&c_db).await?;
    info!("pgsql connection ok");
    let boards = client.get_boards().await?;
    info!("there are {} boards in the index", boards.len());
    drop(client);

    let addr = socket.parse()?;
    let b3_impl = PgsqlB3Server::new(c_db).await?;
    let service = B3Server::new(b3_impl);

    let limit_mb = 100 * 1024 * 1024;
    let service = service.max_decoding_message_size(limit_mb);
    let service = service.max_encoding_message_size(limit_mb);

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
