// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use tokio_postgres::NoTls;
use tracing::{error, instrument};

/// Opens a PostgreSQL connection without TLS, runs `SELECT 1`, then closes.
///
/// Does not log `connection` (may contain credentials). Use only when the server
/// accepts `NoTls` (same as typical `deadpool_postgres` `NoTls` pools).
#[instrument(skip(connection), ret)]
pub async fn check_postgres_select_one_no_tls(
    connection: &tokio_postgres::Config,
) -> bool {
    let (client, conn) = match connection.connect(NoTls).await {
        Ok(v) => v,
        Err(e) => {
            error!("postgres probe: connect failed: {e:?}");
            return false;
        }
    };
    tokio::spawn(async move {
        if let Err(e) = conn.await {
            error!("postgres probe: connection task error: {e:?}");
        }
    });
    match client.simple_query("SELECT 1").await {
        Ok(_) => true,
        Err(e) => {
            error!("postgres probe: SELECT 1 failed: {e:?}");
            false
        }
    }
}

#[cfg(feature = "s3")]
#[instrument(ret)]
pub async fn check_s3(should_run: bool) -> Option<bool> {
    if !should_run {
        return None;
    }

    let s3_bucket = match crate::services::s3::get_public_bucket() {
        Ok(s3_bucket) => s3_bucket,
        Err(err) => {
            error!("s3 error: {err:?}");
            return Some(false);
        }
    };
    let path = std::env::var("AWS_S3_JWKS_CERTS_PATH")
        .unwrap_or_else(|_| "certs.json".to_string());
    match crate::services::s3::get_file_from_s3(s3_bucket, path).await {
        Ok(_) => Some(true),
        Err(error) => {
            error!("s3 error: {error:?}");
            Some(false)
        }
    }
}

#[cfg(not(feature = "s3"))]
pub async fn check_s3(_should_run: bool) -> Option<bool> {
    None
}
