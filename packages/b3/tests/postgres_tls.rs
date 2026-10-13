// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

#![cfg(feature = "server")]

#[path = "support/tls_postgres.rs"]
mod tls_postgres;

use b3::client::pgsql::{
    create_database, drop_database, PgsqlB3Client, PgsqlConnectionParams, PooledPgsqlB3Client,
};
use b3::grpc::{proto::b3_server::B3, server::PgsqlB3Server, GetBoardsRequest};
use b3::postgres::{self, PgTlsConfig, PgTlsMode};
use tls_postgres::TlsPostgres;
use tonic::Request;

fn params(server: &TlsPostgres, host: &str, tls: PgTlsConfig) -> PgsqlConnectionParams {
    PgsqlConnectionParams::new(host, server.port as u32, "b3_test", "synthetic").with_tls(tls)
}

fn trusted(server: &TlsPostgres) -> PgTlsConfig {
    PgTlsConfig {
        ca_path: Some(server.ca("ca.pem")),
        ..Default::default()
    }
}

#[tokio::test]
#[ignore = "requires local PostgreSQL and OpenSSL executables"]
async fn default_policy_rejects_plaintext_postgres() {
    let server = TlsPostgres::start(false);
    let connection =
        PgsqlConnectionParams::new("localhost", server.port as u32, "b3_test", "synthetic");
    assert!(PgsqlB3Client::new(&connection.with_database("postgres"))
        .await
        .is_err());
}

#[tokio::test]
#[ignore = "requires local PostgreSQL and OpenSSL executables"]
async fn valid_ca_supports_server_direct_pooled_and_database_helpers() {
    let server = TlsPostgres::start(true);
    let params = params(&server, "localhost", trusted(&server));
    let db = params.with_database("postgres");
    let pool = postgres::pool(db.config().unwrap(), db.tls_config(), 20)
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
    let mut client = PgsqlB3Client::new(&db).await.unwrap();
    client.create_index_ine().await.unwrap();
    client.create_board_ine("tls_direct").await.unwrap();
    let mut pooled = PooledPgsqlB3Client::new(pool.get().await.unwrap());
    pooled.create_board_ine("tls_pooled").await.unwrap();
    assert_eq!(pooled.get_boards().await.unwrap().len(), 2);
    assert!(client.get_board("tls_pooled").await.unwrap().is_some());
    let board = PgsqlB3Server::new(db.clone(), None).await.unwrap();
    assert_eq!(
        board
            .get_boards(Request::new(GetBoardsRequest {}))
            .await
            .unwrap()
            .get_ref()
            .boards
            .len(),
        2
    );

    let admin = postgres::connect(db.config().unwrap(), db.tls_config())
        .await
        .unwrap();
    admin.execute("CREATE DATABASE b3_test", &[]).await.unwrap();
    let name = "tls created 'database'";
    create_database(&params, name).await.unwrap();
    let created = PgsqlB3Client::new(&params.with_database(name))
        .await
        .unwrap();
    drop(created);
    drop_database(&params, name).await.unwrap();
}

#[tokio::test]
#[ignore = "requires local PostgreSQL and OpenSSL executables"]
async fn all_paths_reject_untrusted_wrong_ca_and_wrong_hostname() {
    let server = TlsPostgres::start(true);
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
        let db = params.with_database("postgres");
        let error = PgsqlB3Client::new(&db).await.err().unwrap();
        assert!(format!("{error:#}").contains(expected), "{error:#}");
        for result in [
            create_database(&params, "denied").await,
            drop_database(&params, "denied").await,
        ] {
            let error = result.unwrap_err();
            assert!(format!("{error:#}").contains(expected), "{error:#}");
        }
        // bb8 connects on acquisition. The server's public call must fail too.
        let board = PgsqlB3Server::new(db, None).await.unwrap();
        let error = board
            .get_boards(Request::new(GetBoardsRequest {}))
            .await
            .unwrap_err();
        assert_eq!(error.code(), tonic::Code::Internal);
        assert!(error.message().contains("Pgsql connection failed"));
    }
}

#[tokio::test]
#[ignore = "requires local PostgreSQL and OpenSSL executables"]
async fn tls_refusal_never_downgrades_and_plaintext_requires_explicit_disable() {
    let server = TlsPostgres::start(false);
    let params = params(&server, "localhost", PgTlsConfig::default());
    let db = params.with_database("postgres");
    let error = PgsqlB3Client::new(&db).await.err().unwrap();
    assert!(
        format!("{error:#}").contains("does not support TLS"),
        "{error:#}"
    );
    for result in [
        create_database(&params, "denied").await,
        drop_database(&params, "denied").await,
    ] {
        assert!(format!("{:#}", result.unwrap_err()).contains("does not support TLS"));
    }
    let board = PgsqlB3Server::new(db, None).await.unwrap();
    assert!(board
        .get_boards(Request::new(GetBoardsRequest {}))
        .await
        .is_err());

    let params = params.with_tls(PgTlsConfig {
        mode: PgTlsMode::Disable,
        ca_path: None,
    });
    let db = params.with_database("postgres");
    let pool = postgres::pool(db.config().unwrap(), db.tls_config(), 20)
        .await
        .unwrap();
    let conn = pool.get().await.unwrap();
    assert!(!conn
        .query_one(
            "SELECT ssl FROM pg_stat_ssl WHERE pid = pg_backend_pid()",
            &[]
        )
        .await
        .unwrap()
        .get::<_, bool>(0));
    let mut client = PgsqlB3Client::new(&db).await.unwrap();
    client.create_index_ine().await.unwrap();
    let board = PgsqlB3Server::new(db, None).await.unwrap();
    assert!(board
        .get_boards(Request::new(GetBoardsRequest {}))
        .await
        .is_ok());
}

#[tokio::test]
async fn invalid_configuration_fails_before_connecting() {
    let directory = tempfile::tempdir().unwrap();
    let invalid_ca = directory.path().join("invalid.pem");
    std::fs::write(&invalid_ca, "not a certificate").unwrap();
    for tls in [
        PgTlsConfig {
            ca_path: Some(directory.path().join("missing.pem")),
            ..Default::default()
        },
        PgTlsConfig {
            ca_path: Some(invalid_ca),
            ..Default::default()
        },
        PgTlsConfig {
            mode: PgTlsMode::Disable,
            ca_path: Some(directory.path().join("missing.pem")),
        },
    ] {
        let params =
            PgsqlConnectionParams::new("localhost", 1, "synthetic", "synthetic").with_tls(tls);
        let db = params.with_database("postgres");
        assert!(PgsqlB3Client::new(&db).await.is_err());
        assert!(PgsqlB3Server::new(db, None).await.is_err());
        assert!(create_database(&params, "denied").await.is_err());
        assert!(drop_database(&params, "denied").await.is_err());
    }
    for (host, port) in [("", 5432), ("localhost", 0), ("localhost", 65536)] {
        let params = PgsqlConnectionParams::new(host, port, "synthetic", "synthetic");
        assert!(params.config().is_err());
        assert!(PgsqlB3Client::new(&params.with_database("postgres"))
            .await
            .is_err());
    }
}

#[tokio::test]
#[ignore = "requires local PostgreSQL and OpenSSL executables"]
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
            .execute("SELECT pg_advisory_xact_lock(hashtext('b3-tls-test'))", &[])
            .await
            .unwrap();
        assert!(!observer
            .query_one(
                "SELECT pg_try_advisory_xact_lock(hashtext('b3-tls-test'))",
                &[]
            )
            .await
            .unwrap()
            .get::<_, bool>(0));
        if commit {
            transaction.commit().await.unwrap();
        } else {
            transaction.rollback().await.unwrap();
        }
        assert!(observer
            .query_one(
                "SELECT pg_try_advisory_xact_lock(hashtext('b3-tls-test'))",
                &[]
            )
            .await
            .unwrap()
            .get::<_, bool>(0));
    }
}

#[test]
fn typed_connection_fields_cannot_override_transport_or_credentials() {
    let user = "user name'";
    let password = "space ' quote \\ sslmode=disable host=attacker";
    let db = "database name'";
    let params = PgsqlConnectionParams::new("localhost", 5432, user, password).with_database(db);
    let mut config = params.config().unwrap();
    assert_eq!(config.get_user(), Some(user));
    assert_eq!(config.get_password(), Some(password.as_bytes()));
    assert_eq!(config.get_dbname(), Some(db));
    params.tls_config().configure(&mut config).unwrap();
    assert_eq!(
        config.get_ssl_mode(),
        tokio_postgres::config::SslMode::Require
    );
}

#[tokio::test]
#[ignore = "requires local PostgreSQL and OpenSSL executables"]
async fn postgres_17_scram_auth_preserves_literal_credentials_over_verified_tls() {
    let server = TlsPostgres::start(true);
    let params = params(&server, "localhost", trusted(&server)).with_database("postgres");
    let admin = postgres::connect(params.config().unwrap(), params.tls_config())
        .await
        .unwrap();
    admin.batch_execute("SET password_encryption = 'scram-sha-256'; CREATE ROLE \"tls user\" LOGIN PASSWORD 'space '' quote \\ sslmode=disable';").await.unwrap();
    admin
        .execute("CREATE DATABASE \"tls user\" OWNER \"tls user\"", &[])
        .await
        .unwrap();
    server.require_password_auth();
    let params = PgsqlConnectionParams::new(
        "localhost",
        server.port as u32,
        "tls user",
        "space ' quote \\ sslmode=disable",
    )
    .with_tls(trusted(&server))
    .with_database("tls user");
    let mut client = PgsqlB3Client::new(&params).await.unwrap();
    client.create_index_ine().await.unwrap();
    client.create_board_ine("scram_board").await.unwrap();
    let board = PgsqlB3Server::new(params.clone(), None).await.unwrap();
    assert_eq!(
        board
            .get_boards(Request::new(GetBoardsRequest {}))
            .await
            .unwrap()
            .get_ref()
            .boards,
        vec!["scram_board"]
    );
    let wrong = PgsqlConnectionParams::new(
        "localhost",
        server.port as u32,
        "tls user",
        "wrong synthetic password",
    )
    .with_tls(trusted(&server))
    .with_database("tls user");
    let error = PgsqlB3Client::new(&wrong).await.err().unwrap();
    assert!(
        format!("{error:#}").contains("password authentication failed"),
        "{error:#}"
    );
}
