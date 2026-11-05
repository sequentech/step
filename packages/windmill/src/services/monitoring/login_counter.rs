// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Sign-in attempts per election event, counted from the Keycloak events the
//! electoral log receives, per 15 minutes of UTC, event type, whether the
//! attempt named an account, and the account's area.
//!
//! A batch's attempts are counted in the transaction that reads its
//! deliveries, behind a receipt of each delivery: a delivery counts once
//! however often the queue delivers it, within the window receipts are kept
//! for, and not at all if the transaction does not commit (it is then
//! delivered again). The batch's receipts, then its counters, are each
//! written by one statement in key order, just before the transaction
//! commits: batches that count the same counters wait on one another in
//! the same order, so they never deadlock, and hold them only briefly.

use crate::tasks::electoral_log::{LogEventInput, LogMessageType};
use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use deadpool_postgres::Transaction;
use std::collections::{BTreeMap, HashSet};
use std::fmt;
use tracing::{error, instrument};
use uuid::Uuid;

/// How long a bucket is. Every UTC offset in use is a whole number of
/// quarter hours, so hourly buckets rebuild exactly in any time zone.
const BUCKET_SECONDS: i64 = 15 * 60;

/// Whether an attempt named an account.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Registration {
    Registered,
    /// No account: the name given matched none. Such an attempt belongs to
    /// no area.
    Unregistered,
}

impl fmt::Display for Registration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Registration::Registered => "REGISTERED",
            Registration::Unregistered => "UNREGISTERED",
        })
    }
}

/// One sign-in attempt, as it is counted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginAttempt {
    pub tenant_id: Uuid,
    pub election_event_id: Uuid,
    pub bucket_start: DateTime<Utc>,
    /// The Keycloak event type, such as `LOGIN` or `LOGIN_ERROR`.
    pub event_type: String,
    pub registration: Registration,
    /// The account's area; `None` when it has none, or there is no account.
    pub area_id: Option<Uuid>,
}

/// The start of the quarter hour of UTC `time` falls in.
pub fn bucket_start(time: DateTime<Utc>) -> DateTime<Utc> {
    let seconds = time.timestamp().div_euclid(BUCKET_SECONDS) * BUCKET_SECONDS;
    DateTime::from_timestamp(seconds, 0).unwrap_or(time)
}

/// A Keycloak event type as the counters keep it: upper case, as Keycloak
/// names its types.
fn is_event_type(event_type: &str) -> bool {
    let mut characters = event_type.chars();
    characters.next().is_some_and(|c| c.is_ascii_uppercase())
        && event_type.len() <= 64
        && characters.all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

/// The Keycloak listener's stand-in for a missing value.
fn named(value: Option<&str>) -> Option<&str> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != "null")
}

/// What a delivered event counts as; `None` for an event that is not a
/// Keycloak event of an election event. `user_area_id` is the area of the
/// event's account, and `received` when the event arrived, which is when it
/// counts unless the event says when it happened.
pub fn login_attempt(
    input: &LogEventInput,
    user_area_id: Option<&str>,
    received: DateTime<Utc>,
) -> Option<LoginAttempt> {
    let LogMessageType::KeycloakEvent(event_type) = &input.message_type else {
        return None;
    };
    if !is_event_type(event_type) {
        return None;
    }
    let tenant_id = Uuid::parse_str(&input.tenant_id).ok()?;
    let election_event_id = Uuid::parse_str(&input.election_event_id).ok()?;
    let registration = match named(input.user_id.as_deref()) {
        Some(_) => Registration::Registered,
        None => Registration::Unregistered,
    };
    let area_id = match registration {
        Registration::Registered => named(user_area_id)
            .and_then(|area| Uuid::parse_str(area).ok())
            .filter(|area| !area.is_nil()),
        Registration::Unregistered => None,
    };
    let happened = input
        .event_time_ms
        .filter(|millis| *millis > 0)
        .and_then(DateTime::from_timestamp_millis)
        .unwrap_or(received);
    Some(LoginAttempt {
        tenant_id,
        election_event_id,
        bucket_start: bucket_start(happened),
        event_type: event_type.clone(),
        registration,
        area_id,
    })
}

/// Counts each delivery's attempt unless the delivery was counted already
/// (a delivery that appears twice counts as its first): how many counted. A
/// delivery another transaction is counting waits for that transaction, and
/// counts only if it rolls back.
#[instrument(err, skip_all, fields(deliveries = deliveries.len()))]
pub async fn count_login_attempts(
    transaction: &Transaction<'_>,
    deliveries: &[(String, LoginAttempt)],
) -> Result<u64> {
    let mut seen = HashSet::new();
    let deliveries: Vec<&(String, LoginAttempt)> = deliveries
        .iter()
        .filter(|(delivery_id, _)| seen.insert(delivery_id.as_str()))
        .collect();
    if deliveries.is_empty() {
        return Ok(0);
    }
    let ids: Vec<&str> = deliveries.iter().map(|(id, _)| id.as_str()).collect();
    let tenants: Vec<Uuid> = deliveries.iter().map(|(_, a)| a.tenant_id).collect();
    let events: Vec<Uuid> = deliveries
        .iter()
        .map(|(_, a)| a.election_event_id)
        .collect();
    let received: HashSet<String> = transaction
        .query(
            "INSERT INTO sequent_backend.monitoring_login_counter_receipt
                 (delivery_id, tenant_id, election_event_id)
             SELECT d.delivery_id, d.tenant_id, d.election_event_id
             FROM unnest($1::text[], $2::uuid[], $3::uuid[])
                 AS d(delivery_id, tenant_id, election_event_id)
             ORDER BY d.delivery_id
             ON CONFLICT (delivery_id) DO NOTHING
             RETURNING delivery_id",
            &[&ids, &tenants, &events],
        )
        .await
        .context("Failed to keep the receipts of sign-in attempts")?
        .iter()
        .map(|row| row.get(0))
        .collect();

    let mut counters: BTreeMap<Counter<'_>, i64> = BTreeMap::new();
    for (_, attempt) in deliveries
        .iter()
        .filter(|(delivery_id, _)| received.contains(delivery_id))
    {
        *counters.entry(Counter::of(attempt)).or_default() += 1;
    }
    let counted = counters.values().sum::<i64>();
    if counters.is_empty() {
        return Ok(0);
    }
    let keys: Vec<&Counter<'_>> = counters.keys().collect();
    let tenants: Vec<Uuid> = keys.iter().map(|key| key.tenant_id).collect();
    let events: Vec<Uuid> = keys.iter().map(|key| key.election_event_id).collect();
    let buckets: Vec<DateTime<Utc>> = keys.iter().map(|key| key.bucket_start).collect();
    let types: Vec<&str> = keys.iter().map(|key| key.event_type).collect();
    let registrations: Vec<String> = keys
        .iter()
        .map(|key| key.registration.to_string())
        .collect();
    let areas: Vec<Option<Uuid>> = keys.iter().map(|key| key.area_id).collect();
    let attempts: Vec<i64> = counters.values().copied().collect();
    transaction
        .execute(
            "INSERT INTO sequent_backend.monitoring_login_counter AS c
                 (tenant_id, election_event_id, bucket_start, event_type, registration,
                  area_id, attempts)
             SELECT a.tenant_id, a.election_event_id, a.bucket_start, a.event_type,
                    a.registration, a.area_id, a.attempts
             FROM unnest($1::uuid[], $2::uuid[], $3::timestamptz[], $4::text[], $5::text[],
                         $6::uuid[], $7::bigint[]) WITH ORDINALITY
                 AS a(tenant_id, election_event_id, bucket_start, event_type, registration,
                      area_id, attempts, place)
             ORDER BY a.place
             ON CONFLICT (tenant_id, election_event_id, bucket_start, event_type,
                          registration, area_key)
             DO UPDATE SET attempts = c.attempts + EXCLUDED.attempts",
            &[
                &tenants,
                &events,
                &buckets,
                &types,
                &registrations,
                &areas,
                &attempts,
            ],
        )
        .await
        .context("Failed to count sign-in attempts")?;
    Ok(u64::try_from(counted).unwrap_or_default())
}

/// Counts as [`count_login_attempts`] does, apart from the rest of the
/// transaction: when counting fails, what it wrote is undone, the failure
/// logged, and the transaction left to go on, so the electoral log never
/// waits on the monitoring counters. The batch's deliveries are then counted
/// if they are delivered again.
pub async fn count_login_attempts_apart(
    transaction: &mut Transaction<'_>,
    deliveries: &[(String, LoginAttempt)],
) -> u64 {
    let counted = async {
        let savepoint = transaction
            .savepoint("monitoring_login_counter")
            .await
            .context("Failed to start counting sign-in attempts")?;
        match count_login_attempts(&savepoint, deliveries).await {
            Ok(counted) => {
                savepoint
                    .commit()
                    .await
                    .context("Failed to keep the sign-in attempts counted")?;
                Ok(counted)
            }
            Err(error) => {
                savepoint
                    .rollback()
                    .await
                    .context("Failed to undo counting sign-in attempts")?;
                Err(error)
            }
        }
    }
    .await;
    counted.unwrap_or_else(|error: anyhow::Error| {
        error!(
            deliveries = deliveries.len(),
            "The sign-in attempts of an electoral log batch were not counted: {error:?}"
        );
        0
    })
}

/// A counter's key, in the order batches write counters in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Counter<'a> {
    tenant_id: Uuid,
    election_event_id: Uuid,
    bucket_start: DateTime<Utc>,
    event_type: &'a str,
    registration: Registration,
    area_id: Option<Uuid>,
}

impl<'a> Counter<'a> {
    fn of(attempt: &'a LoginAttempt) -> Self {
        Counter {
            tenant_id: attempt.tenant_id,
            election_event_id: attempt.election_event_id,
            bucket_start: attempt.bucket_start,
            event_type: &attempt.event_type,
            registration: attempt.registration,
            area_id: attempt.area_id,
        }
    }
}

/// Forgets the receipts of deliveries received more than `window` ago, the
/// longest the queue redelivers after: how many. What they counted stays.
#[instrument(err, skip(transaction))]
pub async fn prune_login_counter_receipts(
    transaction: &Transaction<'_>,
    window: Duration,
) -> Result<u64> {
    let seconds = window.num_seconds() as f64;
    transaction
        .execute(
            "DELETE FROM sequent_backend.monitoring_login_counter_receipt
             WHERE received_at < now() - make_interval(secs => $1)",
            &[&seconds],
        )
        .await
        .context("Failed to prune the receipts of counted sign-in attempts")
}
