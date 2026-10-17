// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
#![cfg(all(feature = "server", feature = "client"))]

#[path = "support/tls_postgres.rs"]
mod tls_postgres;
use b3::{
    client::pgsql::{
        create_database, drop_database, PgsqlB3Client, PgsqlConnectionParams, PooledPgsqlB3Client,
    },
    grpc::{proto::b3_server::B3, server::PgsqlB3Server},
    postgres::{self, PgTlsConfig, PgTlsMode},
};
use tls_postgres::TlsPostgres;

fn params(server: &TlsPostgres, host: &str, tls: PgTlsConfig) -> PgsqlConnectionParams {
    PgsqlConnectionParams::new(host, u32::from(server.port), "postgres", "synthetic").with_tls(tls)
}

fn trusted(server: &TlsPostgres) -> PgTlsConfig {
    PgTlsConfig {
        ca_path: Some(server.ca("ca.pem")),
        ..Default::default()
    }
}

#[tokio::test]
async fn direct_client_requires_verified_tls_by_default() {
    let server = TlsPostgres::start(false);
    let params =
        PgsqlConnectionParams::new("localhost", u32::from(server.port), "postgres", "synthetic")
            .with_database("postgres");
    let error = PgsqlB3Client::new(&params)
        .await
        .err()
        .expect("Plaintext must not be accepted by default");
    assert!(
        format!("{error:#}").contains("does not support TLS"),
        "{error:#}"
    );
}

#[tokio::test]
async fn valid_ca_supports_server_direct_pooled_and_database_helpers() {
    let server = TlsPostgres::start(true);
    let params = params(&server, "localhost", trusted(&server));
    let database = params.with_database("postgres");
    let pool = postgres::pool(database.config().unwrap(), database.tls_config(), 20)
        .await
        .unwrap();
    let conn = pool.get().await.unwrap();
    assert!(conn
        .query_one(
            "SELECT ssl FROM pg_stat_ssl WHERE pid = pg_backend_pid()",
            &[]
        )
        .await
        .unwrap()
        .get::<_, bool>(0));
    drop(conn);
    let mut direct = PgsqlB3Client::new(&database).await.unwrap();
    direct.create_index_ine().await.unwrap();
    direct.create_board_ine("tls_direct").await.unwrap();
    let mut pooled = PooledPgsqlB3Client::new(pool.get().await.unwrap());
    pooled.create_board_ine("tls_pooled").await.unwrap();
    assert!(direct.get_board("tls_pooled").await.unwrap().is_some());
    assert_eq!(pooled.get_boards().await.unwrap().len(), 2);
    drop(pooled);
    let service = PgsqlB3Server::new(database, None).await.unwrap();
    let boards = service
        .get_boards(tonic::Request::new(b3::grpc::GetBoardsRequest {}))
        .await
        .unwrap();
    assert_eq!(boards.get_ref().boards.len(), 2);
    let created_name = "tls created \"quoted\"";
    create_database(&params, created_name).await.unwrap();
    let created = PgsqlB3Client::new(&params.with_database(created_name))
        .await
        .unwrap();
    drop(created);
    drop_database(&params, created_name).await.unwrap();
    let admin = postgres::connect(params.config().unwrap(), params.tls_config())
        .await
        .unwrap();
    assert!(!admin
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM pg_database WHERE datname=$1)",
            &[&created_name]
        )
        .await
        .unwrap()
        .get::<_, bool>(0));
}

#[tokio::test]
async fn all_paths_reject_untrusted_wrong_ca_and_wrong_hostname() {
    let server = TlsPostgres::start(true);
    // Control differs from each rejection only in trust material or host.
    let control = params(&server, "localhost", trusted(&server));
    drop(
        PgsqlB3Client::new(&control.with_database("postgres"))
            .await
            .unwrap(),
    );
    drop(
        PgsqlB3Server::new(control.with_database("postgres"), None)
            .await
            .unwrap(),
    );
    for (host, tls, expected) in [
        (
            "localhost",
            PgTlsConfig::default(),
            "certificate verify failed",
        ),
        (
            "localhost",
            PgTlsConfig {
                ca_path: Some(server.ca("wrong-ca.pem")),
                ..Default::default()
            },
            "certificate verify failed",
        ),
        ("127.0.0.1", trusted(&server), "IP address mismatch"),
    ] {
        let params = params(&server, host, tls);
        let error = PgsqlB3Client::new(&params.with_database("postgres"))
            .await
            .err()
            .unwrap();
        assert!(format!("{error:#}").contains(expected), "{error:#}");
        for operation in [
            create_database(&params, "denied").await,
            drop_database(&params, "denied").await,
        ] {
            let error = operation.unwrap_err();
            assert!(format!("{error:#}").contains(expected), "{error:#}");
        }
        let service = PgsqlB3Server::new(params.with_database("postgres"), None)
            .await
            .unwrap();
        // The server pool connects lazily; acquisition retries are bounded.
        let error = service
            .get_boards(tonic::Request::new(b3::grpc::GetBoardsRequest {}))
            .await
            .unwrap_err();
        assert!(
            error.message().contains("Pgsql connection failed"),
            "{error:#}"
        );
    }
}

#[tokio::test]
async fn tls_refusal_never_downgrades_and_plaintext_requires_explicit_disable() {
    let server = TlsPostgres::start(false);
    let params = params(&server, "localhost", PgTlsConfig::default());
    let service = PgsqlB3Server::new(params.with_database("postgres"), None)
        .await
        .unwrap();
    let error = service
        .get_boards(tonic::Request::new(b3::grpc::GetBoardsRequest {}))
        .await
        .unwrap_err();
    assert!(
        error.message().contains("Pgsql connection failed"),
        "{error:#}"
    );
    for operation in [
        create_database(&params, "denied").await,
        drop_database(&params, "denied").await,
    ] {
        let error = operation.unwrap_err();
        assert!(
            format!("{error:#}").contains("does not support TLS"),
            "{error:#}"
        );
    }
    let params = params.with_tls(PgTlsConfig {
        mode: PgTlsMode::Disable,
        ca_path: None,
    });
    let database = params.with_database("postgres");
    let pool = postgres::pool(database.config().unwrap(), database.tls_config(), 20)
        .await
        .unwrap();
    let conn = pool.get().await.unwrap();
    assert!(!conn
        .query_one(
            "SELECT ssl FROM pg_stat_ssl WHERE pid=pg_backend_pid()",
            &[]
        )
        .await
        .unwrap()
        .get::<_, bool>(0));
    drop(conn);
    let mut client = PgsqlB3Client::new(&database).await.unwrap();
    client.create_index_ine().await.unwrap();
    let service = PgsqlB3Server::new(database, None).await.unwrap();
    assert!(service
        .get_boards(tonic::Request::new(b3::grpc::GetBoardsRequest {}))
        .await
        .unwrap()
        .get_ref()
        .boards
        .is_empty());
    create_database(&params, "local_created").await.unwrap();
    drop_database(&params, "local_created").await.unwrap();
}

#[tokio::test]
async fn tls_keeps_pool_session_and_transaction_advisory_lock_lifetimes() {
    let server = TlsPostgres::start(true);
    let params = params(&server, "localhost", trusted(&server)).with_database("postgres");
    let pool = postgres::pool(params.config().unwrap(), params.tls_config(), 1)
        .await
        .unwrap();
    let observer = postgres::connect(params.config().unwrap(), params.tls_config())
        .await
        .unwrap();
    let conn = pool.get().await.unwrap();
    let pid: i32 = conn
        .query_one("SELECT pg_backend_pid()", &[])
        .await
        .unwrap()
        .get(0);
    conn.execute("SELECT pg_advisory_lock(31947)", &[])
        .await
        .unwrap();
    drop(conn);
    let mut conn = pool.get().await.unwrap();
    assert_eq!(
        pid,
        conn.query_one("SELECT pg_backend_pid()", &[])
            .await
            .unwrap()
            .get::<_, i32>(0)
    );
    assert!(!observer
        .query_one("SELECT pg_try_advisory_lock(31947)", &[])
        .await
        .unwrap()
        .get::<_, bool>(0));
    conn.execute("SELECT pg_advisory_unlock(31947)", &[])
        .await
        .unwrap();
    for commit in [true, false] {
        let transaction = conn.transaction().await.unwrap();
        transaction
            .execute("SELECT pg_advisory_xact_lock(31948)", &[])
            .await
            .unwrap();
        assert!(!observer
            .query_one("SELECT pg_try_advisory_xact_lock(31948)", &[])
            .await
            .unwrap()
            .get::<_, bool>(0));
        if commit {
            transaction.commit().await.unwrap();
        } else {
            transaction.rollback().await.unwrap();
        }
        assert!(observer
            .query_one("SELECT pg_try_advisory_xact_lock(31948)", &[])
            .await
            .unwrap()
            .get::<_, bool>(0));
    }
}

#[test]
fn credentials_and_database_names_cannot_inject_connection_options() {
    let password = "quoted ' \\ password sslmode=disable";
    let params = PgsqlConnectionParams::new("localhost", 5432, "user name", password)
        .with_database("db sslmode=disable");
    let mut config = params.config().unwrap();
    assert_eq!(config.get_password(), Some(password.as_bytes()));
    assert_eq!(config.get_user(), Some("user name"));
    assert_eq!(config.get_dbname(), Some("db sslmode=disable"));
    params.tls_config().configure(&mut config).unwrap();
    assert_eq!(
        config.get_ssl_mode(),
        tokio_postgres::config::SslMode::Require
    );
    for (host, port, user) in [
        ("", 5432, "user"),
        ("localhost", 0, "user"),
        ("localhost", 65536, "user"),
        ("localhost", 5432, ""),
    ] {
        assert!(PgsqlConnectionParams::new(host, port, user, password)
            .config()
            .is_err());
    }
    assert!(
        PgsqlConnectionParams::new("localhost", 5432, "user", password)
            .with_database("")
            .config()
            .is_err()
    );
}

#[test]
#[serial_test::serial]
fn missing_transport_settings_are_secure_and_bad_settings_fail_closed() {
    struct RestoreEnv(Vec<(&'static str, Option<std::ffi::OsString>)>);
    impl Drop for RestoreEnv {
        fn drop(&mut self) {
            for (name, value) in &self.0 {
                match value {
                    Some(value) => std::env::set_var(name, value),
                    None => std::env::remove_var(name),
                }
            }
        }
    }
    let _restore = RestoreEnv(
        ["B3_PG_SSL_MODE", "B3_PG_CA_PATH"]
            .into_iter()
            .map(|name| (name, std::env::var_os(name)))
            .collect(),
    );
    for name in ["B3_PG_SSL_MODE", "B3_PG_CA_PATH"] {
        std::env::remove_var(name);
    }
    assert_eq!(PgTlsConfig::from_env().unwrap().mode, PgTlsMode::VerifyFull);
    for mode in ["", "prefer", "allow", "verify-ca", "Require", "unexpected"] {
        std::env::set_var("B3_PG_SSL_MODE", mode);
        assert!(PgTlsConfig::from_env().is_err());
    }
    std::env::set_var("B3_PG_SSL_MODE", "require");
    assert_eq!(PgTlsConfig::from_env().unwrap().mode, PgTlsMode::VerifyFull);
    std::env::set_var("B3_PG_CA_PATH", "/b3-test-missing/ca.pem");
    let mut config = tokio_postgres::Config::new();
    config.host("localhost");
    assert!(PgTlsConfig::from_env()
        .unwrap()
        .configure(&mut config)
        .is_err());
    std::env::set_var("B3_PG_SSL_MODE", "disable");
    assert!(PgTlsConfig::from_env()
        .unwrap()
        .configure(&mut config)
        .is_err());
    std::env::set_var("B3_PG_CA_PATH", "");
    assert_eq!(PgTlsConfig::from_env().unwrap().mode, PgTlsMode::Disable);
}

#[tokio::test]
async fn all_paths_report_missing_or_invalid_ca_before_connecting() {
    let invalid_ca = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(invalid_ca.path(), "invalid PEM").unwrap();
    for ca_path in [
        std::path::PathBuf::from("/b3-test-missing/ca.pem"),
        invalid_ca.path().to_owned(),
    ] {
        let params = PgsqlConnectionParams::new("localhost", 1, "synthetic", "synthetic").with_tls(
            PgTlsConfig {
                ca_path: Some(ca_path),
                ..Default::default()
            },
        );
        let error = PgsqlB3Client::new(&params.with_database("postgres"))
            .await
            .err()
            .unwrap();
        assert!(
            format!("{error:#}").contains("Unable to load B3_PG_CA_PATH"),
            "{error:#}"
        );
        let error = PgsqlB3Server::new(params.with_database("postgres"), None)
            .await
            .err()
            .unwrap();
        assert!(
            format!("{error:#}").contains("Unable to load B3_PG_CA_PATH"),
            "{error:#}"
        );
        for operation in [
            create_database(&params, "denied").await,
            drop_database(&params, "denied").await,
        ] {
            let error = operation.unwrap_err();
            assert!(
                format!("{error:#}").contains("Unable to load B3_PG_CA_PATH"),
                "{error:#}"
            );
        }
    }
}

#[tokio::test]
async fn postgres_17_scram_auth_preserves_literal_credentials_over_verified_tls() {
    let server = TlsPostgres::start(true);
    let admin_params = params(&server, "localhost", trusted(&server)).with_database("postgres");
    let admin = postgres::connect(admin_params.config().unwrap(), admin_params.tls_config())
        .await
        .unwrap();
    admin.batch_execute("SET password_encryption = 'scram-sha-256'; CREATE ROLE \"tls user\" LOGIN PASSWORD 'space '' quote \\ sslmode=disable'; ").await.unwrap();
    admin
        .execute("CREATE DATABASE \"tls user\" OWNER \"tls user\"", &[])
        .await
        .unwrap();
    server.require_password_auth();
    let wrong = PgsqlConnectionParams::new(
        "localhost",
        u32::from(server.port),
        "tls user",
        "wrong synthetic password",
    )
    .with_tls(trusted(&server))
    .with_database("tls user");
    // Wait until PostgreSQL has applied the asynchronous HBA reload.
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            match PgsqlB3Client::new(&wrong).await {
                Ok(client) => drop(client),
                Err(error) => {
                    assert!(
                        format!("{error:#}").contains("password authentication failed"),
                        "{error:#}"
                    );
                    break;
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    let params = PgsqlConnectionParams::new(
        "localhost",
        u32::from(server.port),
        "tls user",
        "space ' quote \\ sslmode=disable",
    )
    .with_tls(trusted(&server))
    .with_database("tls user");
    let mut client = PgsqlB3Client::new(&params).await.unwrap();
    client.create_index_ine().await.unwrap();
    client.create_board_ine("scram_board").await.unwrap();
    let service = PgsqlB3Server::new(params, None).await.unwrap();
    assert_eq!(
        service
            .get_boards(tonic::Request::new(b3::grpc::GetBoardsRequest {}))
            .await
            .unwrap()
            .get_ref()
            .boards,
        vec!["scram_board"]
    );
}
