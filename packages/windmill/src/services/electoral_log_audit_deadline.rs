// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! One deadline spans acquisition and work; successive stages never reset it.

use std::future::Future;
use std::time::Duration;
use tokio::time::Instant;

#[derive(Debug, Clone, Copy)]
pub struct AuditDeadline(Instant);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuditTimeout;

impl std::fmt::Display for AuditTimeout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("The audit operation exceeded its time limit")
    }
}

impl std::error::Error for AuditTimeout {}

impl AuditDeadline {
    pub fn after(duration: Duration) -> Self {
        Self(Instant::now() + duration)
    }

    /// Queue/record operations are bounded separately from the configured scan.
    pub fn database() -> Self {
        Self::after(Duration::from_secs(10))
    }

    pub async fn run<F: Future>(&self, operation: F) -> Result<F::Output, AuditTimeout> {
        // Tokio polls a ready operation before its timer. Refuse an already
        // exhausted budget before polling, especially before a result write.
        if Instant::now() >= self.0 {
            return Err(AuditTimeout);
        }
        tokio::time::timeout_at(self.0, operation)
            .await
            .map_err(|_| AuditTimeout)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    struct ReleaseOnDrop<'a>(&'a AtomicBool);
    impl Drop for ReleaseOnDrop<'_> {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    #[tokio::test(start_paused = true)]
    async fn delayed_acquisition_times_out_before_the_scan_and_releases_its_future() {
        let released = AtomicBool::new(false);
        let scan_started = AtomicBool::new(false);
        let deadline = AuditDeadline::after(Duration::from_millis(100));
        let result = deadline
            .run(async {
                let _release = ReleaseOnDrop(&released);
                // Acquiring a pool slot or beginning a transaction is part of the
                // operation, not a precondition outside its timeout.
                tokio::time::sleep(Duration::from_secs(1)).await;
                scan_started.store(true, Ordering::SeqCst);
            })
            .await;
        assert_eq!(result, Err(AuditTimeout));
        assert!(!scan_started.load(Ordering::SeqCst));
        assert!(released.load(Ordering::SeqCst));
    }

    #[tokio::test(start_paused = true)]
    async fn acquisition_and_listing_share_the_same_budget() {
        let deadline = AuditDeadline::after(Duration::from_millis(100));
        assert!(deadline
            .run(tokio::time::sleep(Duration::from_millis(60)))
            .await
            .is_ok());
        assert_eq!(
            deadline
                .run(tokio::time::sleep(Duration::from_millis(60)))
                .await,
            Err(AuditTimeout)
        );
    }

    #[tokio::test(start_paused = true)]
    async fn an_exhausted_deadline_does_not_poll_a_new_write() {
        let started = AtomicBool::new(false);
        let deadline = AuditDeadline::after(Duration::from_millis(100));
        tokio::time::advance(Duration::from_millis(101)).await;
        let result = deadline
            .run(async {
                started.store(true, Ordering::SeqCst);
            })
            .await;
        assert_eq!(result, Err(AuditTimeout));
        assert!(!started.load(Ordering::SeqCst));
    }

    #[tokio::test(start_paused = true)]
    async fn immediate_success_and_failure_are_preserved() {
        let deadline = AuditDeadline::database();
        assert_eq!(deadline.run(async { Ok::<_, &str>(42) }).await, Ok(Ok(42)));
        assert_eq!(
            deadline
                .run(async { Err::<u32, _>("database unavailable") })
                .await,
            Ok(Err("database unavailable"))
        );
    }

    #[tokio::test(start_paused = true)]
    async fn database_result_writes_have_a_fixed_upper_bound() {
        let result = AuditDeadline::database()
            .run(tokio::time::sleep(Duration::from_secs(11)))
            .await;
        assert_eq!(result, Err(AuditTimeout));
    }
}
