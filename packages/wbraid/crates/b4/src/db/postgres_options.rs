// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{bail, Context, Result};
use sqlx::{postgres::PgSslMode, ConnectOptions};

/// Shared board-server and application-pool policy: verify TLS unless the URL
/// explicitly declares the local plaintext exception. Retains SQLx certificate
/// and credential options without inheriting its fallback-capable default.
pub fn postgres_connect_options(database_url: &str) -> Result<sqlx::postgres::PgConnectOptions> {
    let url = url::Url::parse(database_url).context("invalid PostgreSQL URL")?;
    if !<sqlx::Postgres as sqlx::Database>::URL_SCHEMES.contains(&url.scheme()) {
        bail!("board PostgreSQL URL must use postgres or postgresql");
    }
    let mut modes = url
        .query_pairs()
        .filter(|(key, _)| key == "sslmode" || key == "ssl-mode");
    let mode = match modes.next() {
        None => PgSslMode::VerifyFull,
        Some((_, value)) => match value.parse::<PgSslMode>() {
            Ok(PgSslMode::VerifyFull) => PgSslMode::VerifyFull,
            Ok(PgSslMode::Disable) => PgSslMode::Disable,
            _ => bail!("board PostgreSQL sslmode must be verify-full or explicit disable"),
        },
    };
    if modes.next().is_some() {
        bail!("board PostgreSQL sslmode must be specified only once");
    }
    Ok(sqlx::postgres::PgConnectOptions::from_url(&url)?.ssl_mode(mode))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tls_is_verified_when_mode_is_omitted_or_explicit() {
        for suffix in ["", "?sslmode=verify-full", "?ssl-mode=verify-full"] {
            let options =
                postgres_connect_options(&format!("postgres://user@localhost/board{suffix}"))
                    .unwrap();
            assert!(matches!(options.get_ssl_mode(), PgSslMode::VerifyFull));
        }
    }

    #[test]
    fn local_plaintext_requires_an_explicit_url_mode() {
        let options =
            postgres_connect_options("postgres://user@localhost/board?sslmode=disable").unwrap();
        assert!(matches!(options.get_ssl_mode(), PgSslMode::Disable));
    }

    #[test]
    fn unsafe_empty_and_conflicting_modes_are_rejected() {
        for mode in ["prefer", "allow", "require", "verify-ca", "", "invalid"] {
            let error = postgres_connect_options(&format!(
                "postgres://user@localhost/board?sslmode={mode}"
            ))
            .unwrap_err();
            assert!(error.to_string().contains("sslmode must be verify-full"));
        }
        for modes in [
            "sslmode=verify-full&sslmode=disable",
            "sslmode=verify-full&ssl-mode=verify-full",
        ] {
            let error =
                postgres_connect_options(&format!("postgres://user@localhost/board?{modes}"))
                    .unwrap_err();
            assert!(error.to_string().contains("specified only once"));
        }
        assert!(postgres_connect_options("").is_err());
        assert!(postgres_connect_options("https://localhost/board").is_err());
    }

    #[test]
    fn url_credentials_remain_literal() {
        let options = postgres_connect_options(
            "postgres://user%20sslmode%3Ddisable:pass%20word@localhost/board%20name",
        )
        .unwrap();
        assert_eq!(options.get_username(), "user sslmode=disable");
        assert_eq!(options.get_database(), Some("board name"));
        assert!(matches!(options.get_ssl_mode(), PgSslMode::VerifyFull));
    }
}
