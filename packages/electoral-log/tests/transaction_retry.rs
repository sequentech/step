// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Context;
use electoral_log::retry_electoral_log_transaction;
use std::cell::Cell;
use tonic::{Code, Status};

#[tokio::test(start_paused = true)]
async fn a_typed_read_conflict_retries_the_transaction_until_it_commits() {
    let attempts = Cell::new(0);
    let result = retry_electoral_log_transaction(|| {
        let attempt = attempts.get() + 1;
        attempts.set(attempt);
        async move {
            if attempt < 3 {
                Err(Status::unknown("tx read conflict")).context("committing board transaction")
            } else {
                Ok("committed batch")
            }
        }
    })
    .await
    .unwrap();
    assert_eq!(result, "committed batch");
    assert_eq!(attempts.get(), 3);
}

#[tokio::test(start_paused = true)]
async fn repeated_conflicts_are_bounded_and_preserve_the_final_error() {
    let attempts = Cell::new(0);
    let error = retry_electoral_log_transaction(|| {
        attempts.set(attempts.get() + 1);
        async {
            Err::<(), _>(Status::unknown("tx read conflict"))
                .context("committing board transaction")
        }
    })
    .await
    .unwrap_err();
    assert_eq!(attempts.get(), 6);
    let status = error.downcast_ref::<Status>().unwrap();
    assert_eq!(status.code(), Code::Unknown);
    assert_eq!(status.message(), "tx read conflict");
    assert_eq!(error.to_string(), "committing board transaction");
}

#[tokio::test(start_paused = true)]
async fn ambiguous_commit_failures_and_other_errors_are_never_replayed() {
    for status in [
        Status::unavailable("connection closed after commit"),
        Status::deadline_exceeded("commit timed out"),
        Status::unknown("duplicate key"),
        Status::unknown("tx read conflict: unrelated failure"),
        Status::unavailable("tx read conflict"),
    ] {
        let attempts = Cell::new(0);
        let error = retry_electoral_log_transaction(|| {
            attempts.set(attempts.get() + 1);
            let status = status.clone();
            async { Err::<(), _>(status).context("committing board transaction") }
        })
        .await
        .unwrap_err();
        assert_eq!(attempts.get(), 1);
        let actual = error.downcast_ref::<Status>().unwrap();
        assert_eq!(actual.code(), status.code());
        assert_eq!(actual.message(), status.message());
    }
    let attempts = Cell::new(0);
    let error = retry_electoral_log_transaction(|| {
        attempts.set(attempts.get() + 1);
        async { Err::<(), _>(anyhow::anyhow!("tx read conflict")) }
    })
    .await
    .unwrap_err();
    assert_eq!(attempts.get(), 1);
    assert_eq!(error.to_string(), "tx read conflict");
}

#[tokio::test(start_paused = true)]
async fn a_successful_transaction_is_not_repeated() {
    let attempts = Cell::new(0);
    let result = retry_electoral_log_transaction(|| {
        attempts.set(attempts.get() + 1);
        async { Ok(17) }
    })
    .await
    .unwrap();
    assert_eq!(result, 17);
    assert_eq!(attempts.get(), 1);
}
