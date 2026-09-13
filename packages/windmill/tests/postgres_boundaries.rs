// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Exercise the SQL escape helpers against PostgreSQL's parser, not a second
//! string-replacement implementation. Use the same local fixture as Rust CI.

use windmill::services::database::generate_hasura_pool;
use windmill::services::sql_utils::{
    assert_standard_conforming_strings, escape_sql_identifier, escape_sql_literal,
};

#[tokio::test]
async fn escaped_literals_round_trip_through_postgres_without_executing_sql_text() {
    let pool = generate_hasura_pool().await.unwrap();
    assert_standard_conforming_strings(&pool).await.unwrap();
    let mut client = pool.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    transaction
        .batch_execute("CREATE TEMP TABLE literal_control (value INTEGER);")
        .await
        .unwrap();

    for value in [
        "",
        "O'Connor",
        "'; DROP TABLE literal_control; --",
        "\\'; SELECT 1; --",
        "éλ中\nsecond line",
    ] {
        let sql = format!("SELECT '{}'::TEXT", escape_sql_literal(value));
        let row = transaction.query_one(&sql, &[]).await.unwrap();
        assert_eq!(row.get::<_, String>(0), value);
    }
    // If a payload had become a second SQL statement, this table could vanish.
    transaction
        .query_one("SELECT COUNT(*) FROM literal_control", &[])
        .await
        .unwrap();
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn quoted_identifiers_remain_one_identifier_even_with_quotes_and_semicolons() {
    let pool = generate_hasura_pool().await.unwrap();
    let mut client = pool.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    for name in [
        "normal",
        "name with spaces",
        "a\"; SELECT 1; --",
        "élection",
    ] {
        let sql = format!("SELECT 17 AS {}", escape_sql_identifier(name));
        let row = transaction.query_one(&sql, &[]).await.unwrap();
        assert_eq!(row.columns()[0].name(), name);
        assert_eq!(row.get::<_, i32>(0), 17);
    }
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn unsafe_postgres_string_mode_is_rejected_before_literal_interpolation() {
    // Give this pool its own session option. Altering a shared database default
    // would race other tests and could make their escaping checks meaningless.
    let mut config = deadpool_postgres::Config::new();
    config.host = Some("127.0.0.1".into());
    config.port = Some(3322);
    config.user = Some("test".into());
    config.password = Some("test".into());
    config.dbname = Some("test".into());
    config.options = Some("-c standard_conforming_strings=off".into());
    let pool = config
        .create_pool(
            Some(deadpool_postgres::Runtime::Tokio1),
            tokio_postgres::NoTls,
        )
        .unwrap();
    let error = assert_standard_conforming_strings(&pool).await.unwrap_err();
    assert!(error.to_string().contains("expected 'on'"));
}
