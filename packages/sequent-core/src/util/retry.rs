// SPDX-FileCopyrightText: 2025 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, instrument};

/// Retries the given future-returning function `op` with exponential backoff.
///
/// - `op` is an async operation returning a `Result<T, E>`
/// - `max_retries` is the maximum number of retries before giving up
/// - `initial_backoff` is the delay for the first retry, which then doubles
///   each time (`initial_backoff`, `2 * initial_backoff`, etc.)
///
/// Returns `Ok(T)` on success or the last `Err(E)` on failure.
#[instrument(skip(op))]
pub async fn retry_with_exponential_backoff<F, Fut, T, E>(
    mut op: F,
    max_retries: usize,
    initial_backoff: Duration,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Debug,
{
    let mut attempts = 0;
    let mut backoff = initial_backoff;

    loop {
        match op().await {
            Ok(val) => {
                // Success on this attempt:
                return Ok(val);
            }
            Err(err) if attempts < max_retries => {
                // Failure, but we can try again after a backoff delay
                attempts += 1;
                info!(
                    "Failed attempt {attempts}, sleeping {:?} ms, error: {:?}",
                    backoff, err
                );
                sleep(backoff).await;
                // Exponential backoff: double the delay
                backoff *= 2;
            }
            Err(err) => {
                info!("Failed attempt {attempts}, run out of retries, error: {:?}", err);
                // We've run out of retries
                return Err(err);
            }
        }
    }
}
