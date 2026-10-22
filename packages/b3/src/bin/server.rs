// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

cfg_if::cfg_if! {
    if #[cfg(feature = "server")] {
        use std::path::PathBuf;
        use tonic::transport::Server;
        use tracing::info;
        use config::{Config, Environment};
        use serde::Deserialize;
        use b3::client::pgsql::{PgsqlB3Client, PgsqlConnectionParams};
        use b3::grpc::server::PgsqlB3Server;
        use b3::postgres::{PgTlsConfig, PgTlsMode};
        use b3::grpc::B3Server;
        use b3::grpc::MAX_MESSAGE_SIZE;

        #[derive(Debug, Deserialize)]
        #[serde(default)]
        struct ServerConfig {
            pg_host: String,
            pg_port: u32,
            pg_user: String,
            pg_password: String,
            pg_database: String,
            pg_ssl_mode: PgTlsMode,
            pg_ca_path: Option<PathBuf>,
            bind: String,
            blob_root: Option<PathBuf>,
            max_message_size_bytes: usize,
        }

        impl Default for ServerConfig {
            fn default() -> Self {
                ServerConfig {
                    pg_host: "localhost".to_string(),
                    pg_port: 49154,
                    pg_user: "postgres".to_string(),
                    pg_password: "postgrespw".to_string(),
                    pg_database: "protocoldb".to_string(),
                    pg_ssl_mode: PgTlsMode::default(),
                    pg_ca_path: None,
                    bind: "127.0.0.1:50051".to_string(),
                    blob_root: None,
                    max_message_size_bytes: MAX_MESSAGE_SIZE,
                }
            }
        }

        impl ServerConfig {
            pub fn from_env() -> Result<Self, config::ConfigError> {
                Config::builder()
                    .add_source(Environment::default().prefix("B3"))
                    .build()?
                    .try_deserialize()
            }
        }

        #[tokio::main]
        async fn main() -> Result<(), Box<dyn std::error::Error>> {
            init_log(true);

            let config = ServerConfig::from_env()?;

            info!(host = %config.pg_host, port = config.pg_port, database = %config.pg_database, tls_mode = ?config.pg_ssl_mode, "Starting b3");

            let c = PgsqlConnectionParams::new(&config.pg_host, config.pg_port, &config.pg_user, &config.pg_password)
                .with_tls(PgTlsConfig { mode: config.pg_ssl_mode, ca_path: config.pg_ca_path.filter(|path| !path.as_os_str().is_empty()) });
            let c_db = c.with_database(&config.pg_database);
            let mut client = PgsqlB3Client::new(&c_db).await?;
            info!("pgsql connection ok");

            client.create_index_ine().await?;
            info!("pgsql index db asserted");
            let boards = client.get_boards().await?;
            info!("there are {} boards in the index", boards.len());
            drop(client);

            let addr = config.bind.parse()?;
            let b3_impl = PgsqlB3Server::new(c_db, config.blob_root).await?;
            let service = B3Server::new(b3_impl)
                .max_encoding_message_size(config.max_message_size_bytes)
                .max_decoding_message_size(config.max_message_size_bytes);

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

        #[cfg(test)]
        mod tests {
            use super::*;

            #[test]
            fn server_configuration_defaults_to_verified_tls_and_rejects_unsafe_modes() {
                let config: ServerConfig = Config::builder().build().unwrap().try_deserialize().unwrap();
                assert_eq!(config.pg_ssl_mode, PgTlsMode::VerifyFull);
                assert!(config.pg_ca_path.is_none());
                for mode in ["verify-full", "require", "disable"] {
                    let config: ServerConfig = Config::builder()
                        .set_override("pg_ssl_mode", mode).unwrap()
                        .set_override("pg_ca_path", "/synthetic/ca.pem").unwrap()
                        .build().unwrap().try_deserialize().unwrap();
                    assert_eq!(config.pg_ssl_mode, mode.parse::<PgTlsMode>().unwrap());
                    assert_eq!(config.pg_ca_path, Some(PathBuf::from("/synthetic/ca.pem")));
                }
                for mode in ["prefer", "verify-ca", "allow", ""] {
                    assert!(Config::builder().set_override("pg_ssl_mode", mode).unwrap()
                        .build().unwrap().try_deserialize::<ServerConfig>().is_err());
                }
            }
        }

    // cfg-if
    } else {
        fn main() {
            println!("Requires the 'server' feature");
        }
    }
}
