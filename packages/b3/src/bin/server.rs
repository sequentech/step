// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
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
        use b3::grpc::B3Server;
        use b3::grpc::MAX_MESSAGE_SIZE;
        use sequent_core::util::init_log::init_log;

        #[derive(Debug, Deserialize)]
        #[serde(default)]
        struct ServerConfig {
            pg_host: String,
            pg_port: u32,
            pg_user: String,
            pg_password: String,
            pg_database: String,
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

            info!("Starting b3 with configuration: {:#?}", config);

            let c = PgsqlConnectionParams::new(&config.pg_host, config.pg_port, &config.pg_user, &config.pg_password);
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

    // cfg-if
    } else {
        fn main() {
            println!("Requires the 'server' feature");
        }
    }
}
