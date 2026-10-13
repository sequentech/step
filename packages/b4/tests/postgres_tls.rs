// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
#![cfg(all(feature = "native", feature = "client"))]

#[path = "support/postgres.rs"]
mod postgres;

use b4::client::pgsql::{PgsqlB3Client, PgsqlConnectionParams};
use b4::{
    db,
    postgres::{PgTlsConfig, PgTlsMode},
};
#[path = "support/tls_postgres.rs"]
mod tls_postgres;
use tls_postgres::TlsPostgres;

fn params(server: &TlsPostgres, host: &str, tls: PgTlsConfig) -> db::PgConnectionParams {
    db::PgConnectionParams::new(host, server.port, "b4_test", "synthetic", "postgres").with_tls(tls)
}

fn direct(params: &db::PgConnectionParams) -> PgsqlConnectionParams {
    PgsqlConnectionParams::new(
        &params.host,
        u32::from(params.port),
        &params.username,
        &params.password,
    )
    .with_tls(params.tls.clone())
}

fn trusted(server: &TlsPostgres) -> PgTlsConfig {
    PgTlsConfig {
        ca_path: Some(server.ca("ca.pem")),
        ..Default::default()
    }
}

#[tokio::test]
async fn direct_client_requires_verified_tls_by_default() {
    let server = postgres::Postgres::start();
    let params = server.parameters();
    let connection = PgsqlConnectionParams::new(
        &params.host,
        u32::from(params.port),
        &params.username,
        &params.password,
    )
    .with_database(&params.database);
    assert!(PgsqlB3Client::new(&connection).await.is_err());
}

#[tokio::test]
async fn valid_ca_supports_server_direct_pooled_and_database_helpers() {
    use b4::client::pgsql::{create_database, drop_database, PooledPgsqlB3Client};
    let server = TlsPostgres::start(true);
    let params = params(&server, "localhost", trusted(&server));
    let pool = db::init_db_with_params(&params).await.unwrap();
    let conn = pool.get().await.unwrap();
    let row = conn
        .query_one(
            "SELECT ssl FROM pg_stat_ssl WHERE pid = pg_backend_pid()",
            &[],
        )
        .await
        .unwrap();
    assert!(row.get::<_, bool>(0));
    drop(conn);
    db::create_board(&pool, "tls_board").await.unwrap();
    let client = PgsqlB3Client::new(&direct(&params).with_database("postgres"))
        .await
        .unwrap();
    assert!(client.get_board("tls_board").await.unwrap().is_some());
    let mut pooled = PooledPgsqlB3Client::new(pool.get().await.unwrap());
    assert!(!pooled.get_boards().await.unwrap().is_empty());
    pooled.create_board_ine("tls_pooled").await.unwrap();
    assert!(client.get_board("tls_pooled").await.unwrap().is_some());
    drop(pooled);
    drop(client);
    // Database provisioning uses the same verified transport and propagates errors.
    let admin = b4::postgres::connect(params.config(), &params.tls)
        .await
        .unwrap();
    admin.execute("CREATE DATABASE b4_test", &[]).await.unwrap();
    create_database(&direct(&params), "tls_created")
        .await
        .unwrap();
    let created = PgsqlB3Client::new(&direct(&params).with_database("tls_created"))
        .await
        .unwrap();
    drop(created);
    drop_database(&direct(&params), "tls_created")
        .await
        .unwrap();
}

#[tokio::test]
async fn both_paths_reject_untrusted_wrong_ca_and_wrong_hostname() {
    let server = TlsPostgres::start(true);
    // The valid CA/hostname control above uses exactly the same fixture setup.
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
        let error = PgsqlB3Client::new(&direct(&params).with_database("postgres"))
            .await
            .err()
            .unwrap();
        assert!(format!("{error:#}").contains(expected), "{error:#}");
        let error = db::init_db_with_params(&params).await.err().unwrap();
        // bb8 retries failed TLS handshakes and reports its bounded acquisition
        // timeout. The same server accepts the valid CA/hostname control.
        assert!(
            format!("{error:#}").contains("Timed out in bb8"),
            "{error:#}"
        );
    }
}

#[tokio::test]
async fn tls_refusal_never_downgrades_and_plaintext_requires_explicit_disable() {
    let server = TlsPostgres::start(false);
    let params = params(&server, "localhost", PgTlsConfig::default());
    let error = PgsqlB3Client::new(&direct(&params).with_database("postgres"))
        .await
        .err()
        .unwrap();
    assert!(
        format!("{error:#}").contains("does not support TLS"),
        "{error:#}"
    );
    let error = db::init_db_with_params(&params).await.err().unwrap();
    assert!(
        format!("{error:#}").contains("Timed out in bb8"),
        "{error:#}"
    );
    for operation in [
        b4::client::pgsql::create_database(&direct(&params), "denied").await,
        b4::client::pgsql::drop_database(&direct(&params), "denied").await,
    ] {
        assert!(format!("{:#}", operation.unwrap_err()).contains("does not support TLS"));
    }
    let params = params.with_tls(PgTlsConfig {
        mode: PgTlsMode::Disable,
        ca_path: None,
    });
    let pool = db::init_db_with_params(&params).await.unwrap();
    let conn = pool.get().await.unwrap();
    assert!(!conn
        .query_one(
            "SELECT ssl FROM pg_stat_ssl WHERE pid = pg_backend_pid()",
            &[]
        )
        .await
        .unwrap()
        .get::<_, bool>(0));
    let client = PgsqlB3Client::new(&direct(&params).with_database("postgres"))
        .await
        .unwrap();
    assert!(client.get_boards().await.unwrap().is_empty());
}

#[tokio::test]
async fn tls_keeps_pool_session_and_transaction_advisory_lock_lifetimes() {
    let server = TlsPostgres::start(true);
    let params = params(&server, "localhost", trusted(&server));
    let pool = db::init_db_with_params(&params).await.unwrap();
    let observer = b4::postgres::connect(params.config(), &params.tls)
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
    let transaction = conn.transaction().await.unwrap();
    transaction
        .execute("SELECT pg_advisory_xact_lock(hashtext('b4-tls-test'))", &[])
        .await
        .unwrap();
    assert!(!observer
        .query_one(
            "SELECT pg_try_advisory_xact_lock(hashtext('b4-tls-test'))",
            &[]
        )
        .await
        .unwrap()
        .get::<_, bool>(0));
    transaction.commit().await.unwrap();
    assert!(observer
        .query_one(
            "SELECT pg_try_advisory_xact_lock(hashtext('b4-tls-test'))",
            &[]
        )
        .await
        .unwrap()
        .get::<_, bool>(0));
    let transaction = conn.transaction().await.unwrap();
    transaction
        .execute("SELECT pg_advisory_xact_lock(hashtext('b4-tls-test'))", &[])
        .await
        .unwrap();
    transaction.rollback().await.unwrap();
    assert!(observer
        .query_one(
            "SELECT pg_try_advisory_xact_lock(hashtext('b4-tls-test'))",
            &[]
        )
        .await
        .unwrap()
        .get::<_, bool>(0));
}

#[test]
fn credentials_and_database_names_cannot_inject_connection_options() {
    let password = "quoted ' \\ password sslmode=disable";
    let connection = PgsqlConnectionParams::new("localhost", 5432, "user name", password);
    let config = connection
        .with_database("db sslmode=disable")
        .config()
        .unwrap();
    assert_eq!(config.get_password(), Some(password.as_bytes()));
    assert_eq!(config.get_user(), Some("user name"));
    assert_eq!(config.get_dbname(), Some("db sslmode=disable"));
    let mut config = config;
    PgTlsConfig::default().configure(&mut config).unwrap();
    assert_eq!(
        config.get_ssl_mode(),
        tokio_postgres::config::SslMode::Require
    );
    assert!(
        PgsqlConnectionParams::new("localhost", 65536, "user", "pass")
            .config()
            .is_err()
    );
    let params = db::PgConnectionParams::new(
        "localhost",
        5432,
        "user name",
        password,
        "db sslmode=disable",
    );
    assert_eq!(params.config().get_password(), Some(password.as_bytes()));
    assert_eq!(params.config().get_dbname(), Some("db sslmode=disable"));
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
        ["B4_PG_SSL_MODE", "B4_PG_CA_PATH"]
            .into_iter()
            .map(|name| (name, std::env::var_os(name)))
            .collect(),
    );
    for name in ["B4_PG_SSL_MODE", "B4_PG_CA_PATH"] {
        std::env::remove_var(name);
    }
    assert_eq!(PgTlsConfig::from_env().unwrap().mode, PgTlsMode::VerifyFull);
    for mode in ["", "prefer", "Require", "unexpected"] {
        std::env::set_var("B4_PG_SSL_MODE", mode);
        assert!(PgTlsConfig::from_env().is_err());
    }
    std::env::set_var("B4_PG_SSL_MODE", "verify-full");
    std::env::set_var("B4_PG_CA_PATH", "/b4-test-missing/ca.pem");
    let tls = PgTlsConfig::from_env().unwrap();
    let mut config = tokio_postgres::Config::new();
    config.host("localhost");
    assert!(tls.configure(&mut config).is_err());
    std::env::set_var("B4_PG_SSL_MODE", "disable");
    assert!(PgTlsConfig::from_env()
        .unwrap()
        .configure(&mut config)
        .is_err());
    for name in ["B4_PG_SSL_MODE", "B4_PG_CA_PATH"] {
        std::env::remove_var(name);
    }
}

#[tokio::test]
async fn both_paths_report_missing_ca_before_attempting_a_connection() {
    let params = db::PgConnectionParams::new("localhost", 1, "synthetic", "synthetic", "postgres")
        .with_tls(PgTlsConfig {
            ca_path: Some("/b4-test-missing/ca.pem".into()),
            ..Default::default()
        });
    let error = PgsqlB3Client::new(&direct(&params).with_database("postgres"))
        .await
        .err()
        .unwrap();
    assert!(format!("{error:#}").contains("Unable to load B4_PG_CA_PATH"));
    let error = db::init_db_with_params(&params).await.err().unwrap();
    assert!(format!("{error:#}").contains("Unable to load B4_PG_CA_PATH"));
}
