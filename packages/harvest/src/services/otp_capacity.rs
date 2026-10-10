// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! What Keycloak's sends, and the Messenger codes Get Started releases,
//! may wait for: their account's capacity, within a budget that ends well
//! before Keycloak gives up.
//!
//! - Every send Keycloak asks for (codes and notices: reset links,
//!   temporary passwords) is interactive: a voter is waiting. It holds one
//!   of the account's places (`limits.otp_concurrency`, default
//!   [`DEFAULT_OTP_CONCURRENCY`]) until it is recorded, and takes its token
//!   from the whole shared bucket (`windmill::postgres::rate_bucket`), the
//!   OTP reserve included. Only windmill's bulk sends and retries stay
//!   above the reserve.
//! - The place is taken before any database connection, so sends waiting
//!   for one hold no connection.
//! - The places are per harvest process: the shared bucket is the limit
//!   across replicas, and the places keep one process from having too many
//!   sends in flight at the provider.
//! - The wait for a place and a token ends [`CAPACITY_BUDGET`] after
//!   harvest got the request ([`Arrival`], before the token check and the
//!   body). A send still waiting then is never handed to the provider: the
//!   ledger records it FAILED with
//!   [`CAPACITY_BUSY`](windmill::postgres::rate_bucket::CAPACITY_BUSY),
//!   and Keycloak gets the same state and reason and shows Resend after
//!   its timer.
//! - The provider's answer is awaited until [`ANSWER_BUDGET`] after the
//!   request; later, the send is UNKNOWN (it may have been sent), so
//!   harvest answers before Keycloak's 10 s timeout (`timeout-seconds`
//!   of the Keycloak message sender).
//! - An account without `messages_per_second` has no bucket: sends then
//!   wait only for a place.

use rocket::request::{FromRequest, Outcome, Request};
use sequent_core::types::messaging::AccountLimits;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio::time::Instant;
use tracing::info;
use uuid::Uuid;
use windmill::services::messaging::dispatch::{Place, SendPolicy, ShareRule};

/// How long a send Keycloak asks for may wait for a place and a token,
/// counted from when harvest got the request.
pub const CAPACITY_BUDGET: Duration = Duration::from_secs(5);

/// How long after the request the provider's answer is awaited. Keycloak
/// waits 10 s from before it connected: harvest answers before that.
pub const ANSWER_BUDGET: Duration = Duration::from_secs(8);

/// Sends an account has in flight at once from one harvest process when
/// its limits don't say.
pub const DEFAULT_OTP_CONCURRENCY: usize = 8;

/// When harvest got the request: taken by the first guard of the route,
/// before the service token is checked and the body read.
#[derive(Debug, Clone, Copy)]
pub struct Arrival(pub Instant);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Arrival {
    type Error = std::convert::Infallible;

    async fn from_request(
        request: &'r Request<'_>,
    ) -> Outcome<Self, Self::Error> {
        Outcome::Success(*request.local_cache(|| Arrival(Instant::now())))
    }
}

/// Sends an account may have in flight at once from this harvest process:
/// `limits.otp_concurrency`, or [`DEFAULT_OTP_CONCURRENCY`] when it is
/// absent or 0. Per process, not per deployment: the shared bucket is the
/// account's limit across replicas.
pub fn otp_concurrency(limits: &AccountLimits) -> usize {
    match limits.otp_concurrency {
        Some(concurrency) if concurrency > 0 => concurrency as usize,
        _ => DEFAULT_OTP_CONCURRENCY,
    }
}

/// One account's places: one semaphore for the account's life in this
/// process, resized when its concurrency changes.
struct AccountPlaces {
    size: usize,
    semaphore: Arc<Semaphore>,
}

/// The per-account places for interactive sends in this process.
#[derive(Default)]
pub struct OtpCapacity {
    places: Mutex<HashMap<Uuid, AccountPlaces>>,
}

/// A place asked for: what the send is to be told ([`Place`]), and the
/// permit to keep until the send is recorded.
pub struct Admitted {
    pub place: Place,
    pub permit: Option<OwnedSemaphorePermit>,
}

impl OtpCapacity {
    /// The policy of an interactive send harvest got at `arrival`: the
    /// whole bucket, within the budgets.
    pub fn policy(&self, arrival: Arrival, place: Place) -> SendPolicy {
        SendPolicy {
            capacity_deadline: Some(arrival.0 + CAPACITY_BUDGET),
            share: ShareRule::WHOLE_BUCKET,
            place,
            answer_by: Some(arrival.0 + ANSWER_BUDGET),
            ..SendPolicy::default()
        }
    }

    /// The account's places, resized to `concurrency`: raising adds the
    /// difference at once; lowering removes free places at once and the
    /// rest as sends in flight return theirs, so the account never has
    /// more in flight than the larger of the two counts, and reaches the
    /// new one as sends finish.
    fn places(&self, account_id: Uuid, concurrency: usize) -> Arc<Semaphore> {
        let concurrency = concurrency.max(1);
        let mut places = self
            .places
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let places =
            places.entry(account_id).or_insert_with(|| AccountPlaces {
                size: concurrency,
                semaphore: Arc::new(Semaphore::new(concurrency)),
            });
        if places.size != concurrency {
            info!(
                %account_id,
                from = places.size,
                to = concurrency,
                "the account's places changed"
            );
            if concurrency > places.size {
                places.semaphore.add_permits(concurrency - places.size);
            } else {
                let excess = places.size - concurrency;
                let rest = excess - places.semaphore.forget_permits(excess);
                if rest > 0 {
                    // Taken as sends in flight return them, ahead of the
                    // sends waiting after this change (the semaphore is
                    // fair), and never given back.
                    let semaphore = places.semaphore.clone();
                    tokio::spawn(async move {
                        if let Ok(taken) =
                            semaphore.acquire_many_owned(rest as u32).await
                        {
                            taken.forget();
                        }
                    });
                }
            }
            places.size = concurrency;
        }
        places.semaphore.clone()
    }

    /// A place for one send of the account, or `None` once `deadline`
    /// passed without one.
    pub async fn admit_code(
        &self,
        account_id: Uuid,
        limits: &AccountLimits,
        deadline: Option<Instant>,
    ) -> Option<OwnedSemaphorePermit> {
        let places = self.places(account_id, otp_concurrency(limits));
        let place = match deadline {
            Some(deadline) => {
                tokio::time::timeout_at(deadline, places.acquire_owned())
                    .await
                    .ok()?
            }
            None => places.acquire_owned().await,
        };
        place.ok()
    }

    /// A place for one send of the account harvest got at `arrival`,
    /// within the capacity budget.
    pub async fn admit(
        &self,
        account_id: Uuid,
        limits: &AccountLimits,
        arrival: Arrival,
    ) -> Admitted {
        match self
            .admit_code(account_id, limits, Some(arrival.0 + CAPACITY_BUDGET))
            .await
        {
            Some(permit) => Admitted {
                place: Place::HELD,
                permit: Some(permit),
            },
            None => {
                info!(%account_id, "no place for the send within the budget");
                Admitted {
                    place: Place::NOT_GRANTED,
                    permit: None,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn an_account_sends_at_most_its_concurrency_of_codes_at_once() {
        let capacity = OtpCapacity::default();
        let account = Uuid::new_v4();
        let limits = AccountLimits::default();
        let received = Instant::now();
        let deadline = Some(received + CAPACITY_BUDGET);
        let mut held = Vec::new();
        for _ in 0..DEFAULT_OTP_CONCURRENCY {
            held.push(
                capacity
                    .admit_code(account, &limits, deadline)
                    .await
                    .expect("a free place"),
            );
        }
        // The next code waits for a place, and gives up when the budget
        // ends.
        assert!(capacity
            .admit_code(account, &limits, deadline)
            .await
            .is_none());
        assert_eq!(Instant::now() - received, CAPACITY_BUDGET);

        // Another account isn't held up by this one.
        assert!(capacity
            .admit_code(Uuid::new_v4(), &limits, Some(Instant::now()))
            .await
            .is_some());

        // A place freed while a code waits goes to it.
        let waiting = Instant::now();
        let freed = held.pop();
        let next = tokio::spawn({
            let capacity = Arc::new(capacity);
            async move {
                capacity
                    .admit_code(
                        account,
                        &AccountLimits::default(),
                        Some(Instant::now() + CAPACITY_BUDGET),
                    )
                    .await
                    .is_some()
            }
        });
        tokio::time::sleep(Duration::from_secs(2)).await;
        drop(freed);
        assert!(next.await.unwrap());
        assert!(Instant::now() - waiting < CAPACITY_BUDGET);
    }

    /// R5-02: Keycloak's sends wait at most 5 s for capacity and 8 s for
    /// the provider, both from the request: harvest answers before
    /// Keycloak's 10 s timeout. They take from the whole bucket.
    #[test]
    fn keycloaks_sends_answer_before_its_timeout() {
        let capacity = OtpCapacity::default();
        let arrival = Arrival(Instant::now());
        let policy = capacity.policy(arrival, Place::HELD);
        assert_eq!(
            policy.capacity_deadline,
            Some(arrival.0 + Duration::from_secs(5))
        );
        assert_eq!(policy.answer_by, Some(arrival.0 + Duration::from_secs(8)));
        assert!(ANSWER_BUDGET < Duration::from_secs(10));
        assert_eq!(policy.share, ShareRule::WHOLE_BUCKET);
        assert_eq!(policy.place, Place::HELD);
        assert_eq!(otp_concurrency(&AccountLimits::default()), 8);
    }

    #[test]
    fn the_accounts_concurrency_comes_from_its_limits() {
        let limits = |concurrency| AccountLimits {
            otp_concurrency: concurrency,
            ..Default::default()
        };
        assert_eq!(otp_concurrency(&limits(Some(3))), 3);
        assert_eq!(otp_concurrency(&limits(Some(0))), DEFAULT_OTP_CONCURRENCY);
        assert_eq!(otp_concurrency(&limits(None)), DEFAULT_OTP_CONCURRENCY);
    }

    #[tokio::test(start_paused = true)]
    async fn an_account_with_its_own_concurrency_gets_that_many_places() {
        let capacity = OtpCapacity::default();
        let account = Uuid::new_v4();
        let limits = AccountLimits {
            otp_concurrency: Some(2),
            ..Default::default()
        };
        let deadline = Some(Instant::now() + CAPACITY_BUDGET);
        let first = capacity.admit_code(account, &limits, deadline).await;
        let second = capacity.admit_code(account, &limits, deadline).await;
        assert!(first.is_some() && second.is_some());
        assert!(capacity
            .admit_code(account, &limits, deadline)
            .await
            .is_none());
        // Raising it takes effect for the next code.
        let raised = AccountLimits {
            otp_concurrency: Some(3),
            ..Default::default()
        };
        assert!(capacity
            .admit_code(account, &raised, Some(Instant::now()))
            .await
            .is_some());
    }

    /// R5-13: one set of places per account. Lowering the concurrency
    /// while codes hold places never lets more than the new count in: new
    /// codes wait until enough places came back.
    #[tokio::test(start_paused = true)]
    async fn lowering_the_concurrency_never_lets_more_codes_in_than_the_new_count(
    ) {
        let capacity = OtpCapacity::default();
        let account = Uuid::new_v4();
        let concurrency = |n| AccountLimits {
            otp_concurrency: Some(n),
            ..Default::default()
        };
        let soon = || Some(Instant::now() + Duration::from_millis(10));
        let mut held = Vec::new();
        for _ in 0..4 {
            held.push(
                capacity
                    .admit_code(account, &concurrency(4), soon())
                    .await
                    .expect("a free place"),
            );
        }
        // Lowered to 2 with 4 in flight: nothing more until only 1 is.
        assert!(capacity
            .admit_code(account, &concurrency(2), soon())
            .await
            .is_none());
        held.pop();
        tokio::task::yield_now().await;
        assert!(capacity
            .admit_code(account, &concurrency(2), soon())
            .await
            .is_none());
        held.pop();
        tokio::task::yield_now().await;
        assert!(capacity
            .admit_code(account, &concurrency(2), soon())
            .await
            .is_none());
        held.pop();
        tokio::task::yield_now().await;
        let second = capacity
            .admit_code(account, &concurrency(2), soon())
            .await
            .expect("one of 2 places in use: another code goes");
        assert!(capacity
            .admit_code(account, &concurrency(2), soon())
            .await
            .is_none());
        drop(second);
        drop(held);
    }
}
