// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

#![cfg(feature = "postgres")]

#[path = "support/tls_postgres.rs"]
mod tls_postgres;

use b4::db::{postgres_connect_options, BoardDb};
use sqlx::{postgres::PgPoolOptions, Acquire};
use std::time::Duration;
use tls_postgres::TlsPostgres;

#[tokio::test]
async fn server_requires_tls_unless_plaintext_is_explicit() {
    let server = TlsPostgres::start(false);
    let url = format!("postgres://b4_test@127.0.0.1:{}/postgres", server.port);
    let local = BoardDb::open_url(&format!("{url}?sslmode=disable"))
        .await
        .unwrap();
    assert!(local.create_board("local_control").await.is_ok());
    let connection = BoardDb::open_url(&url).await;
    let error = connection
        .err()
        .expect("the board server silently accepted plaintext");
    assert!(format!("{error:#}").contains("server does not support TLS"));
    assert_rejected_on_both_paths(&url, "server does not support TLS").await;
    local.postgres_pool().unwrap().close().await;
}

async fn archive_pool(url: &str) -> anyhow::Result<sqlx::PgPool> {
    // Matches Windmill's application-owned pool construction, with a bounded
    // acquisition time and two sessions for the lock contract.
    Ok(PgPoolOptions::new()
        .max_connections(2)
        .acquire_timeout(Duration::from_secs(5))
        .connect_with(postgres_connect_options(url)?)
        .await?)
}

async fn assert_rejected_on_both_paths(url: &str, expected: &str) {
    let server_error = BoardDb::open_url(url)
        .await
        .err()
        .expect("server accepted invalid TLS");
    let archive_error = archive_pool(url)
        .await
        .expect_err("archive pool accepted invalid TLS");
    for error in [server_error, archive_error] {
        let message = format!("{error:#}").to_lowercase();
        assert!(
            message.contains(&expected.to_lowercase()),
            "expected {expected}: {message}"
        );
    }
}

#[tokio::test]
async fn verified_tls_rejects_wrong_trust_hostname_and_missing_ca_on_both_paths() {
    let server = TlsPostgres::start(true);
    let url = format!("postgres://b4_test@localhost:{}/postgres", server.port);
    let ca = server.ca("ca.pem");
    let verified = format!("{url}?sslrootcert={}", ca.display());
    let board = BoardDb::open_url(&verified).await.unwrap();
    let pool = archive_pool(&verified).await.unwrap();
    let archive = BoardDb::from_postgres_pool(pool.clone()).await.unwrap();

    for db in [&board, &archive] {
        let ssl: bool =
            sqlx::query_scalar("SELECT ssl FROM pg_stat_ssl WHERE pid=pg_backend_pid()")
                .fetch_one(db.postgres_pool().unwrap())
                .await
                .unwrap();
        assert!(ssl, "verified connection must actually negotiate TLS");
    }
    board.create_board("tls_control").await.unwrap();
    assert!(archive.get_board("tls_control").await.unwrap().is_some());

    // Start every rejection from the same successful verified connection.
    let wrong_ca = format!("{url}?sslrootcert={}", server.ca("wrong-ca.pem").display());
    assert_rejected_on_both_paths(&wrong_ca, "unknownissuer").await;
    assert_rejected_on_both_paths(&url, "unknownissuer").await;
    let wrong_hostname = verified.replacen("localhost", "127.0.0.1", 1);
    assert_rejected_on_both_paths(&wrong_hostname, "not valid for name").await;
    let missing_ca = format!("{url}?sslrootcert={}", server.ca("missing.pem").display());
    assert_rejected_on_both_paths(&missing_ca, "no such file").await;

    assert_lock_lifetimes(board.postgres_pool().unwrap()).await;
    assert_lock_lifetimes(&pool).await;
    pool.close().await;
    board.postgres_pool().unwrap().close().await;
}

async fn assert_lock_lifetimes(pool: &sqlx::PgPool) {
    let mut owner = pool.acquire().await.unwrap();
    let mut other = pool.acquire().await.unwrap();
    let mut transaction = owner.begin().await.unwrap();
    sqlx::query("SELECT pg_advisory_xact_lock(13947)")
        .execute(&mut *transaction)
        .await
        .unwrap();
    let held: bool = sqlx::query_scalar("SELECT pg_try_advisory_xact_lock(13947)")
        .fetch_one(&mut *other)
        .await
        .unwrap();
    assert!(!held, "transaction lock must exclude another session");
    transaction.rollback().await.unwrap();
    let released: bool = sqlx::query_scalar("SELECT pg_try_advisory_xact_lock(13947)")
        .fetch_one(&mut *other)
        .await
        .unwrap();
    assert!(released, "rollback must release the transaction lock");

    sqlx::query("SELECT pg_advisory_lock(13947)")
        .execute(&mut *owner)
        .await
        .unwrap();
    let held: bool = sqlx::query_scalar("SELECT pg_try_advisory_lock(13947)")
        .fetch_one(&mut *other)
        .await
        .unwrap();
    assert!(!held, "session lock must exclude another session");
    let released: bool = sqlx::query_scalar("SELECT pg_advisory_unlock(13947)")
        .fetch_one(&mut *owner)
        .await
        .unwrap();
    assert!(released);
    let acquired: bool = sqlx::query_scalar("SELECT pg_try_advisory_lock(13947)")
        .fetch_one(&mut *other)
        .await
        .unwrap();
    assert!(acquired, "explicit unlock must release the session lock");
    sqlx::query("SELECT pg_advisory_unlock(13947)")
        .execute(&mut *other)
        .await
        .unwrap();
}
