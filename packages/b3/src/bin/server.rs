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
            fn tls_config(&self) -> PgTlsConfig {
                PgTlsConfig {
                    mode: self.pg_ssl_mode,
                    ca_path: self.pg_ca_path.clone().filter(|path| !path.as_os_str().is_empty()),
                }
            }

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
                .with_tls(config.tls_config());
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

        #[cfg(test)]
        mod tests {
            use super::*;

            #[test]
            #[serial_test::serial]
            fn server_environment_enforces_transport_policy() {
                struct Restore(Vec<(&'static str, Option<std::ffi::OsString>)>);
                impl Drop for Restore {
                    fn drop(&mut self) {
                        for (name, value) in &self.0 {
                            match value {
                                Some(value) => std::env::set_var(name, value),
                                None => std::env::remove_var(name),
                            }
                        }
                    }
                }
                let names = ["B3_PG_SSL_MODE", "B3_PG_CA_PATH"];
                let _restore = Restore(names.iter().map(|name| (*name, std::env::var_os(name))).collect());
                for name in names { std::env::remove_var(name); }
                assert_eq!(ServerConfig::from_env().unwrap().tls_config().mode, PgTlsMode::VerifyFull);
                for value in ["verify-full", "require"] {
                    std::env::set_var("B3_PG_SSL_MODE", value);
                    assert_eq!(ServerConfig::from_env().unwrap().tls_config().mode, PgTlsMode::VerifyFull);
                }
                for value in ["", "prefer", "allow", "verify-ca", "unexpected"] {
                    std::env::set_var("B3_PG_SSL_MODE", value);
                    assert!(ServerConfig::from_env().is_err());
                }
                std::env::set_var("B3_PG_SSL_MODE", "disable");
                std::env::set_var("B3_PG_CA_PATH", "");
                let tls = ServerConfig::from_env().unwrap().tls_config();
                assert_eq!(tls.mode, PgTlsMode::Disable);
                assert!(tls.ca_path.is_none());
                std::env::set_var("B3_PG_CA_PATH", "/b3-test-missing/ca.pem");
                let tls = ServerConfig::from_env().unwrap().tls_config();
                let mut config = tokio_postgres::Config::new(); config.host("localhost");
                assert!(tls.configure(&mut config).is_err());
            }
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

    // cfg-if
    } else {
        fn main() {
            println!("Requires the 'server' feature");
        }
    }
}
