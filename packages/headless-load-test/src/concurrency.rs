// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The rate limiter and voter pool behind Phase 2. Generic over the outcome
//! type so it can be tested without a network call — the real caller
//! instantiates it with `vote::VoteOutcome`.
//!
//! A voter lives in an `mpsc` channel while idle and is only handed to
//! `work` by taking it out of that channel; `work` hands it back once its
//! cast completes. Two in-flight casts can therefore never share a voter —
//! not because of a lock around each voter, but because there is only ever
//! one copy of it and it is only ever in one place (in the channel or in
//! exactly one in-flight task) at a time. `votes_per_second` gates how
//! often a *new* cast is allowed to start; if every voter is already
//! in-flight when a tick fires, that tick is skipped rather than blocked
//! on — configuring a rate the voter pool can't sustain shows up as a
//! lower actual rate, not as the run stalling.

use std::future::Future;
use std::time::Duration;

use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tokio::time::{interval, Instant};

use crate::provision::VoterCredential;

/// Runs `work` at up to `votes_per_second`, one call per available voter,
/// for `duration`, then waits for whatever is still in flight. Returns
/// every outcome `work` produced, in completion order (not start order).
pub async fn run_rate_limited<F, Fut, T>(
    voters: Vec<VoterCredential>,
    votes_per_second: f64,
    duration: Duration,
    work: F,
) -> Vec<T>
where
    F: Fn(VoterCredential) -> Fut + Clone + Send + 'static,
    Fut: Future<Output = (VoterCredential, T)> + Send + 'static,
    T: Send + 'static,
{
    let (voter_tx, mut voter_rx) = mpsc::unbounded_channel::<VoterCredential>();
    for voter in voters {
        let _ = voter_tx.send(voter);
    }

    let tick_period = Duration::from_secs_f64((1.0 / votes_per_second).max(0.0));
    let mut ticker = interval(tick_period.max(Duration::from_micros(1)));
    let deadline = Instant::now() + duration;
    let mut in_flight = JoinSet::new();
    let mut results = Vec::new();

    loop {
        tokio::select! {
            _ = ticker.tick() => {
                if Instant::now() >= deadline {
                    break;
                }
                if let Ok(voter) = voter_rx.try_recv() {
                    let work = work.clone();
                    let voter_tx = voter_tx.clone();
                    in_flight.spawn(async move {
                        let (voter, outcome) = work(voter).await;
                        let _ = voter_tx.send(voter);
                        outcome
                    });
                }
                // No voter free right now: this tick is skipped, not queued.
            }
            Some(finished) = in_flight.join_next(), if !in_flight.is_empty() => {
                if let Ok(outcome) = finished {
                    results.push(outcome);
                }
            }
        }
    }

    while let Some(finished) = in_flight.join_next().await {
        if let Ok(outcome) = finished {
            results.push(outcome);
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::sync::{Arc, Mutex};

    fn voters(count: u32) -> Vec<VoterCredential> {
        (0..count).map(crate::provision::voter_credential).collect()
    }

    #[tokio::test]
    async fn no_two_in_flight_calls_ever_share_a_voter() {
        let currently_in_flight: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));
        let violation = Arc::new(Mutex::new(false));

        let in_flight = currently_in_flight.clone();
        let violation_flag = violation.clone();
        let results = run_rate_limited(
            voters(5),
            200.0, // fast enough to stress the pool against its own size
            Duration::from_millis(80),
            move |voter| {
                let in_flight = in_flight.clone();
                let violation_flag = violation_flag.clone();
                async move {
                    let newly_inserted = in_flight.lock().unwrap().insert(voter.username.clone());
                    if !newly_inserted {
                        *violation_flag.lock().unwrap() = true;
                    }
                    tokio::time::sleep(Duration::from_millis(5)).await;
                    in_flight.lock().unwrap().remove(&voter.username);
                    (voter, ())
                }
            },
        )
        .await;

        assert!(!*violation.lock().unwrap(), "a voter was used concurrently");
        assert!(
            !results.is_empty(),
            "the run should have cast at least one vote"
        );
    }

    #[tokio::test]
    async fn the_rate_is_capped_by_votes_per_second_not_just_voter_count() {
        // Plenty of voters, but the rate itself should still gate how many
        // casts start in the window.
        let results = run_rate_limited(
            voters(1000),
            50.0,
            Duration::from_millis(200),
            |voter| async move { (voter, ()) },
        )
        .await;

        // ~10 ticks expected (50/s * 0.2s); allow generous scheduling slack.
        assert!(
            results.len() <= 20,
            "expected roughly 10 casts at 50/s over 200ms, got {}",
            results.len()
        );
    }

    #[tokio::test]
    async fn an_exhausted_voter_pool_skips_ticks_instead_of_blocking() {
        // One voter, a slow call, and a fast tick: most ticks must find no
        // free voter and skip rather than queue up. The window is shorter
        // than the call itself, so there's no way for the voter to free up
        // and start a second cast before the deadline — exactly one must
        // run (and still gets drained after the window closes).
        let results = run_rate_limited(
            voters(1),
            1000.0,
            Duration::from_millis(30),
            |voter| async move {
                tokio::time::sleep(Duration::from_millis(100)).await;
                (voter, ())
            },
        )
        .await;

        assert_eq!(
            results.len(),
            1,
            "only one cast should fit for the single voter in this window"
        );
    }
}
