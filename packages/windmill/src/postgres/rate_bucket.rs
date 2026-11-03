// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The per-account token bucket in `messaging_rate_bucket`, shared by every
//! harvest and windmill process. The rules (capacity, refill, the OTP
//! reserve) are in [`messaging::rate_limit`]; here they are applied in one
//! `UPDATE … RETURNING`, so concurrent takers never spend more than the
//! bucket holds.
//!
//! Each statement reads its own `statement_timestamp()`. A taker that
//! waited for another one's row lock sees that taker's later `refilled_at`
//! and adds nothing for the time between: tokens are never added twice for
//! the same interval, and `refilled_at` never goes back.

use anyhow::Result;
use deadpool_postgres::Client as DbClient;
use messaging::rate_limit::{BucketLimits, BucketShare};
use sequent_core::types::messaging::AccountLimits;
use std::time::Duration;
use tokio::time::Instant;
use uuid::Uuid;

/// Why a message was not handed to its provider: its sender gave up
/// waiting for the account's capacity (harvest: 5 s for a code). Shared by
/// the ledger, the log and the Admin Portal.
pub const CAPACITY_BUSY: &str = "Not sent: the account stayed at its sending limit for too long";

/// The shortest and longest pause between two tries of a waiting taker.
const MIN_PAUSE: Duration = Duration::from_millis(5);
const MAX_PAUSE: Duration = Duration::from_secs(1);

/// What one try at the bucket gave.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TryTake {
    /// The account has no `messages_per_second`: nothing to take.
    Unthrottled,
    /// One token taken; `tokens_left` remain.
    Taken { tokens_left: f64 },
    /// Not enough tokens for the share: try again after this long.
    Wait(Duration),
}

/// What waiting for a token gave.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Take {
    Unthrottled,
    Taken,
    /// The deadline passed first: nothing was taken.
    DeadlinePassed,
}

/// Refills the account's bucket by the time elapsed and takes one token if
/// `share` allows it, in one `UPDATE … RETURNING`. The first try of an
/// account creates its bucket full.
pub async fn try_take(
    client: &DbClient,
    account_id: &Uuid,
    limits: &AccountLimits,
    share: BucketShare,
) -> Result<TryTake> {
    let Some(bucket) = BucketLimits::of(limits) else {
        return Ok(TryTake::Unthrottled);
    };
    let floor = bucket.floor(share);
    for _ in 0..2 {
        let taken = client
            .query_opt(
                r#"
                UPDATE sequent_backend.messaging_rate_bucket AS bucket
                SET tokens = LEAST(
                        $2::float8,
                        bucket.tokens + GREATEST(
                            0,
                            EXTRACT(EPOCH FROM statement_timestamp() - bucket.refilled_at)::float8
                        ) * $3::float8
                    ) - 1,
                    refilled_at = GREATEST(bucket.refilled_at, statement_timestamp())
                WHERE bucket.account_id = $1
                  AND LEAST(
                        $2::float8,
                        bucket.tokens + GREATEST(
                            0,
                            EXTRACT(EPOCH FROM statement_timestamp() - bucket.refilled_at)::float8
                        ) * $3::float8
                    ) >= $4::float8 + 1
                RETURNING bucket.tokens
                "#,
                &[account_id, &bucket.capacity, &bucket.per_second, &floor],
            )
            .await?;
        if let Some(row) = taken {
            return Ok(TryTake::Taken {
                tokens_left: row.get(0),
            });
        }
        let current = client
            .query_opt(
                r#"
                SELECT LEAST(
                    $2::float8,
                    tokens + GREATEST(
                        0,
                        EXTRACT(EPOCH FROM statement_timestamp() - refilled_at)::float8
                    ) * $3::float8
                )
                FROM sequent_backend.messaging_rate_bucket
                WHERE account_id = $1
                "#,
                &[account_id, &bucket.capacity, &bucket.per_second],
            )
            .await?;
        match current {
            Some(row) => return Ok(TryTake::Wait(bucket.wait(row.get(0), share))),
            None => {
                client
                    .execute(
                        r#"
                        INSERT INTO sequent_backend.messaging_rate_bucket
                            (account_id, tokens, refilled_at)
                        VALUES ($1, $2, statement_timestamp())
                        ON CONFLICT (account_id) DO NOTHING
                        "#,
                        &[account_id, &bucket.capacity],
                    )
                    .await?;
            }
        }
    }
    // Created and still not takeable: a full bucket below the share's
    // floor can't happen, but never spin.
    Ok(TryTake::Wait(MIN_PAUSE))
}

/// Takes one token for `share`, waiting as long as needed, or until
/// `deadline`.
pub async fn take(
    client: &DbClient,
    account_id: &Uuid,
    limits: &AccountLimits,
    share: BucketShare,
    deadline: Option<Instant>,
) -> Result<Take> {
    loop {
        let wait = match try_take(client, account_id, limits, share).await? {
            TryTake::Unthrottled => return Ok(Take::Unthrottled),
            TryTake::Taken { .. } => return Ok(Take::Taken),
            TryTake::Wait(wait) => wait,
        };
        // A little jitter keeps concurrent takers from waking together.
        let jitter = Duration::from_micros(u64::from(rand::random::<u16>() % 2_000));
        let pause = wait.clamp(MIN_PAUSE, MAX_PAUSE) + jitter;
        match deadline {
            Some(deadline) => {
                let now = Instant::now();
                if now >= deadline {
                    return Ok(Take::DeadlinePassed);
                }
                // One last try at the deadline.
                tokio::time::sleep_until(deadline.min(now + pause)).await;
            }
            None => tokio::time::sleep(pause).await,
        }
    }
}
