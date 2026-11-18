// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Electoral-log events that wait for their signer.
//!
//! A board whose signer keeps its key elsewhere can be unable to sign for a
//! while (its key service is down: [`electoral_log::signer::SignerUnavailable`]).
//! Its events are neither dead-lettered nor retried a fixed number of times:
//! they wait, however long the outage lasts.
//!
//! - **Where they wait.** Each waiting event is put back on the event queue
//!   (`electoral_log_event_queue`) as the same Celery envelope producers send,
//!   due after a delay, with PGMQ headers that say why it waits. It is durable
//!   from the moment the batch task commits the put-back: a worker that stops
//!   loses nothing, and the dispatcher hands the event to a batch again when
//!   it is due. The event carries how long it has waited ([`Deferral`]).
//! - **Backoff.** The delay doubles from `ELECTORAL_LOG_DEFER_FIRST_SECS`
//!   (5 s) up to `ELECTORAL_LOG_DEFER_MAX_SECS` (5 min), and stays there for
//!   as many rounds as the outage lasts: there is no retry budget.
//! - **Order.** The events of one scope (an election event, or a tenant's
//!   own events) keep their order: an event waits behind any event of its
//!   scope that started waiting before it (in the queue or earlier in the
//!   batch), even if its own signing would work. Waiting events are ordered
//!   by when they first waited, then by their place in that batch.
//! - **Idempotency.** An event is put back at most once while a copy waits
//!   (by delivery ID), and storing is idempotent by delivery ID, so a batch
//!   retried after an uncertain commit stores nothing twice.
//! - **Observability.** Each round logs how many events wait, why, the
//!   oldest one's age and the next attempt; past
//!   `ELECTORAL_LOG_DEFER_ALERT_SECS` (15 min) the line is an error. The
//!   queue's own metrics (`pgmq.metrics`) count them too.

use crate::services::electoral_log_dead_letter::event_message;
use crate::services::tenant_log::names_no_election_event;
use crate::tasks::electoral_log::LogEventInput;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::time::Duration;
use tokio_postgres::GenericClient;

/// Environment variable with the first wait, in seconds.
pub const DEFER_FIRST_ENV: &str = "ELECTORAL_LOG_DEFER_FIRST_SECS";
/// Environment variable with the longest wait, in seconds.
pub const DEFER_MAX_ENV: &str = "ELECTORAL_LOG_DEFER_MAX_SECS";
/// Environment variable with the age, in seconds, past which waiting events
/// are logged as an error.
pub const DEFER_ALERT_ENV: &str = "ELECTORAL_LOG_DEFER_ALERT_SECS";
pub const DEFAULT_DEFER_FIRST: Duration = Duration::from_secs(5);
pub const DEFAULT_DEFER_MAX: Duration = Duration::from_secs(300);
pub const DEFAULT_DEFER_ALERT: Duration = Duration::from_secs(900);

/// Header with the scope whose order a waiting event keeps.
pub const SCOPE_HEADER: &str = "x-electoral-log-deferred-scope";
/// Header with a waiting event's delivery ID.
pub const DELIVERY_HEADER: &str = "x-electoral-log-deferred-delivery";
/// Header with when the event first waited, in milliseconds since the epoch.
pub const SINCE_HEADER: &str = "x-electoral-log-deferred-since-ms";
/// Header with the event's place in the batch it first waited in.
pub const ORDER_HEADER: &str = "x-electoral-log-deferred-order";
/// Header with how many times the event was put back.
pub const ROUNDS_HEADER: &str = "x-electoral-log-deferred-rounds";
/// Header with why the event waits ([`WaitReason`]).
pub const KIND_HEADER: &str = "x-electoral-log-deferred-kind";
/// Header with the last error.
pub const REASON_HEADER: &str = "x-electoral-log-deferred-reason";
/// Longest error kept in a header.
const MAX_REASON_CHARS: usize = 2_000;

/// How long an event has waited for its signer.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Deferral {
    /// Times it was put back.
    pub rounds: u32,
    /// When it first waited, in milliseconds since the Unix epoch.
    pub since_ms: i64,
    /// Its place among the events that first waited in the same batch.
    pub order: u32,
}

impl Deferral {
    /// The deferral after one more round: `previous` one round on, or a
    /// first one at `now_ms`, `order`-th of its batch.
    pub fn next(previous: Option<Deferral>, now_ms: i64, order: u32) -> Deferral {
        match previous {
            Some(previous) => Deferral {
                rounds: previous.rounds.saturating_add(1),
                ..previous
            },
            None => Deferral {
                rounds: 1,
                since_ms: now_ms,
                order,
            },
        }
    }
}

/// Where an event stands among the waiting events of its scope: those that
/// first waited earlier come first; an event that never waited comes last.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct WaitKey {
    pub since_ms: i64,
    pub order: u32,
    pub delivery_id: String,
}

impl WaitKey {
    pub fn of(event: &LogEventInput) -> WaitKey {
        let delivery_id = event.delivery_id.clone().unwrap_or_default();
        match event.deferral {
            Some(deferral) => WaitKey {
                since_ms: deferral.since_ms,
                order: deferral.order,
                delivery_id,
            },
            None => WaitKey {
                since_ms: i64::MAX,
                order: u32::MAX,
                delivery_id,
            },
        }
    }
}

/// The scope whose events keep their order: the tenant and the election
/// event, or the tenant alone for events that name no election event.
pub fn scope_of(event: &LogEventInput) -> String {
    let election_event_id = if names_no_election_event(&event.election_event_id) {
        ""
    } else {
        event.election_event_id.as_str()
    };
    format!("{}/{election_event_id}", event.tenant_id)
}

/// The order a batch's events are built in: those that waited first, by
/// [`WaitKey`]; the others after them, in their own order.
pub fn in_wait_order(events: &[LogEventInput]) -> Vec<&LogEventInput> {
    let mut ordered: Vec<&LogEventInput> = events.iter().collect();
    ordered.sort_by_key(|event| {
        event
            .deferral
            .map_or((i64::MAX, u32::MAX), |d| (d.since_ms, d.order))
    });
    ordered
}

/// Why an event waits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WaitReason {
    /// Its board's signer cannot sign now.
    SignerUnavailable,
    /// An earlier event of its scope waits.
    Held,
}

impl WaitReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            WaitReason::SignerUnavailable => "signer-unavailable",
            WaitReason::Held => "held",
        }
    }
}

/// An event of a batch that waits.
#[derive(Clone, Debug)]
pub struct WaitingEvent {
    pub event: LogEventInput,
    pub reason: WaitReason,
    pub error: String,
}

/// The events already waiting in the queue, by scope.
#[derive(Clone, Debug, Default)]
pub struct Waiting {
    by_scope: HashMap<String, Vec<WaitKey>>,
}

impl Waiting {
    pub fn add(&mut self, scope: String, key: WaitKey) {
        self.by_scope.entry(scope).or_default().push(key);
    }

    /// The earliest waiting event `event` must stay behind, if any: one of
    /// its scope that first waited before it (never a copy of itself).
    pub fn ahead_of(&self, event: &LogEventInput) -> Option<&WaitKey> {
        let key = WaitKey::of(event);
        self.by_scope
            .get(&scope_of(event))?
            .iter()
            .filter(|waiting| waiting.delivery_id != key.delivery_id && **waiting < key)
            .min()
    }

    /// How many events wait.
    pub fn len(&self) -> usize {
        self.by_scope.values().map(Vec::len).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// When the longest-waiting event first waited.
    pub fn oldest_since_ms(&self) -> Option<i64> {
        self.by_scope
            .values()
            .flatten()
            .map(|key| key.since_ms)
            .min()
    }
}

/// How long waiting events wait between rounds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeferralBackoff {
    pub first: Duration,
    pub max: Duration,
    /// Waiting longer than this is logged as an error.
    pub alert: Duration,
}

impl Default for DeferralBackoff {
    fn default() -> Self {
        DeferralBackoff {
            first: DEFAULT_DEFER_FIRST,
            max: DEFAULT_DEFER_MAX,
            alert: DEFAULT_DEFER_ALERT,
        }
    }
}

impl DeferralBackoff {
    /// Read from the environment; an unset or empty variable takes its
    /// default, any other value must be a positive number of seconds.
    pub fn from_env() -> Result<Self> {
        Self::from_values(
            std::env::var(DEFER_FIRST_ENV).ok().as_deref(),
            std::env::var(DEFER_MAX_ENV).ok().as_deref(),
            std::env::var(DEFER_ALERT_ENV).ok().as_deref(),
        )
    }

    pub fn from_values(
        first: Option<&str>,
        max: Option<&str>,
        alert: Option<&str>,
    ) -> Result<Self> {
        let backoff = DeferralBackoff {
            first: seconds(DEFER_FIRST_ENV, first, DEFAULT_DEFER_FIRST)?,
            max: seconds(DEFER_MAX_ENV, max, DEFAULT_DEFER_MAX)?,
            alert: seconds(DEFER_ALERT_ENV, alert, DEFAULT_DEFER_ALERT)?,
        };
        anyhow::ensure!(
            backoff.first <= backoff.max,
            "{DEFER_FIRST_ENV} must not be longer than {DEFER_MAX_ENV}"
        );
        Ok(backoff)
    }

    /// The wait before round `rounds` (1 is the first): doubling from
    /// `first`, never longer than `max`, however many rounds.
    pub fn delay(&self, rounds: u32) -> Duration {
        let factor = 1u32
            .checked_shl(rounds.saturating_sub(1))
            .unwrap_or(u32::MAX);
        self.first.saturating_mul(factor).min(self.max)
    }
}

fn seconds(name: &str, value: Option<&str>, default: Duration) -> Result<Duration> {
    match value.map(str::trim).filter(|value| !value.is_empty()) {
        None => Ok(default),
        Some(value) => {
            let seconds: u64 = value
                .parse()
                .with_context(|| format!("{name} must be a positive integer, got {value:?}"))?;
            anyhow::ensure!(
                seconds > 0,
                "{name} must be a positive integer, got {value:?}"
            );
            Ok(Duration::from_secs(seconds))
        }
    }
}

/// The events waiting in `queue` (the event queue).
pub async fn waiting_events<C: GenericClient + Sync>(client: &C, queue: &str) -> Result<Waiting> {
    pgmq_broker::validate_queue_name(queue).map_err(anyhow::Error::msg)?;
    let rows = client
        .query(
            &format!(
                "SELECT headers->>'{SCOPE_HEADER}', headers->>'{DELIVERY_HEADER}', \
                 (headers->>'{SINCE_HEADER}')::bigint, (headers->>'{ORDER_HEADER}')::bigint \
                 FROM pgmq.q_{queue} WHERE headers->>'{SCOPE_HEADER}' IS NOT NULL"
            ),
            &[],
        )
        .await
        .context("Error reading the electoral-log events that wait")?;
    let mut waiting = Waiting::default();
    for row in rows {
        let scope: Option<String> = row.get(0);
        let delivery_id: Option<String> = row.get(1);
        let since_ms: Option<i64> = row.get(2);
        let order: Option<i64> = row.get(3);
        if let (Some(scope), Some(delivery_id)) = (scope, delivery_id) {
            waiting.add(
                scope,
                WaitKey {
                    since_ms: since_ms.unwrap_or(i64::MAX),
                    order: order
                        .and_then(|order| u32::try_from(order).ok())
                        .unwrap_or(u32::MAX),
                    delivery_id,
                },
            );
        }
    }
    Ok(waiting)
}

/// What one round of putting events back did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PutBack {
    /// Events whose signer cannot sign now.
    pub signer_unavailable: usize,
    /// Events held behind an earlier one of their scope.
    pub held: usize,
    /// Events put back now.
    pub put_back: usize,
    /// Events not put back: a copy already waits.
    pub already_waiting: usize,
    /// When the longest-waiting of them first waited.
    pub oldest_since_ms: Option<i64>,
    /// The shortest wait before one of them is tried again.
    pub next_attempt: Option<Duration>,
}

/// The event as it is put back for its next round, and that round's wait.
pub fn next_round(
    event: &LogEventInput,
    order: u32,
    backoff: &DeferralBackoff,
    now_ms: i64,
) -> (LogEventInput, Duration) {
    let mut event = event.clone();
    let deferral = Deferral::next(event.deferral, now_ms, order);
    event.deferral = Some(deferral);
    (event, backoff.delay(deferral.rounds))
}

/// Put the waiting events of a batch back on `queue` (the event queue), each
/// due after its next wait, in the caller's transaction. An event of which a
/// copy already waits there is not put back again.
pub async fn put_back<C: GenericClient + Sync>(
    client: &C,
    queue: &str,
    waiting: &[WaitingEvent],
    backoff: &DeferralBackoff,
    now_ms: i64,
) -> Result<PutBack> {
    pgmq_broker::validate_queue_name(queue).map_err(anyhow::Error::msg)?;
    let send = format!(
        "SELECT pgmq.send($1::text, $2::jsonb, $3::jsonb, $4::integer) \
         WHERE NOT EXISTS (SELECT 1 FROM pgmq.q_{queue} WHERE headers->>'{DELIVERY_HEADER}' = $5::text)"
    );
    let mut summary = PutBack::default();
    for (index, item) in waiting.iter().enumerate() {
        let order = u32::try_from(index).unwrap_or(u32::MAX);
        let (event, delay) = next_round(&item.event, order, backoff, now_ms);
        let deferral = event.deferral.unwrap_or(Deferral {
            rounds: 1,
            since_ms: now_ms,
            order,
        });
        let delivery_id = event.delivery_id.clone().unwrap_or_default();
        let message = event_message(&event)?;
        let headers = deferral_headers(&event, &deferral, item);
        let delay_secs = i32::try_from(delay.as_secs()).unwrap_or(i32::MAX);
        let sent = client
            .query(
                &send,
                &[&queue, &message, &headers, &delay_secs, &delivery_id],
            )
            .await
            .context("Error putting back an electoral-log event that waits")?;
        match item.reason {
            WaitReason::SignerUnavailable => summary.signer_unavailable += 1,
            WaitReason::Held => summary.held += 1,
        }
        if sent.is_empty() {
            summary.already_waiting += 1;
        } else {
            summary.put_back += 1;
        }
        summary.oldest_since_ms = Some(
            summary
                .oldest_since_ms
                .map_or(deferral.since_ms, |oldest| oldest.min(deferral.since_ms)),
        );
        summary.next_attempt = Some(summary.next_attempt.map_or(delay, |next| next.min(delay)));
    }
    Ok(summary)
}

fn deferral_headers(event: &LogEventInput, deferral: &Deferral, item: &WaitingEvent) -> Value {
    let mut headers = Map::new();
    headers.insert(SCOPE_HEADER.into(), Value::String(scope_of(event)));
    headers.insert(
        DELIVERY_HEADER.into(),
        Value::String(event.delivery_id.clone().unwrap_or_default()),
    );
    headers.insert(SINCE_HEADER.into(), Value::from(deferral.since_ms));
    headers.insert(ORDER_HEADER.into(), Value::from(deferral.order));
    headers.insert(ROUNDS_HEADER.into(), Value::from(deferral.rounds));
    headers.insert(KIND_HEADER.into(), Value::String(item.reason.as_str().into()));
    headers.insert(
        REASON_HEADER.into(),
        Value::String(item.error.chars().take(MAX_REASON_CHARS).collect()),
    );
    Value::Object(headers)
}

/// Log one round: what waits, why, since when, and when it is tried next.
/// Past the alert age, it is an error, so that a long outage is seen.
pub fn log_round(summary: &PutBack, waiting_before: &Waiting, backoff: &DeferralBackoff, now_ms: i64) {
    let oldest_since_ms = match (summary.oldest_since_ms, waiting_before.oldest_since_ms()) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (a, b) => a.or(b),
    };
    let oldest_secs = oldest_since_ms
        .map(|since| (now_ms.saturating_sub(since) / 1000).max(0))
        .unwrap_or(0);
    let next_attempt_secs = summary.next_attempt.map_or(0, |next| next.as_secs());
    let alert = u64::try_from(oldest_secs).unwrap_or(0) >= backoff.alert.as_secs();
    if alert {
        tracing::error!(
            signer_unavailable = summary.signer_unavailable,
            held = summary.held,
            put_back = summary.put_back,
            already_waiting = summary.already_waiting,
            waiting_in_queue = waiting_before.len(),
            oldest_waiting_secs = oldest_secs,
            next_attempt_secs,
            "Electoral-log events have waited for their signer for too long; they are kept and tried again"
        );
    } else {
        tracing::warn!(
            signer_unavailable = summary.signer_unavailable,
            held = summary.held,
            put_back = summary.put_back,
            already_waiting = summary.already_waiting,
            waiting_in_queue = waiting_before.len(),
            oldest_waiting_secs = oldest_secs,
            next_attempt_secs,
            "Electoral-log events wait for their signer; they are kept and tried again"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tasks::electoral_log::{LogEventBody, LogMessageType};

    fn event(delivery_id: &str, election_event_id: &str, deferral: Option<Deferral>) -> LogEventInput {
        LogEventInput {
            delivery_id: Some(delivery_id.into()),
            election_event_id: election_event_id.into(),
            message_type: LogMessageType::KeycloakEvent("LOGIN".into()),
            user_id: None,
            username: None,
            tenant_id: "tenant".into(),
            body: LogEventBody::Plain("null".into()),
            event_time_ms: None,
            details: None,
            admin_event: None,
            deferral,
        }
    }

    #[test]
    fn the_wait_doubles_and_stops_at_its_longest_for_any_number_of_rounds() {
        let backoff = DeferralBackoff::default();
        let delays: Vec<u64> = (1..=9).map(|round| backoff.delay(round).as_secs()).collect();
        assert_eq!(delays, vec![5, 10, 20, 40, 80, 160, 300, 300, 300]);
        for round in [10, 11, 32, 33, 1_000, u32::MAX] {
            assert_eq!(backoff.delay(round), DEFAULT_DEFER_MAX, "round {round}");
        }
        assert_eq!(backoff.delay(0), DEFAULT_DEFER_FIRST);
    }

    #[test]
    fn the_backoff_is_configured_by_the_environment() {
        assert_eq!(
            DeferralBackoff::from_values(None, Some(""), None).unwrap(),
            DeferralBackoff::default()
        );
        let backoff = DeferralBackoff::from_values(Some("1"), Some("8"), Some("60")).unwrap();
        assert_eq!(backoff.delay(10), Duration::from_secs(8));
        assert_eq!(backoff.alert, Duration::from_secs(60));
        assert!(DeferralBackoff::from_values(Some("0"), None, None).is_err());
        assert!(DeferralBackoff::from_values(Some("x"), None, None).is_err());
        assert!(DeferralBackoff::from_values(Some("600"), Some("300"), None).is_err());
    }

    #[test]
    fn a_deferral_keeps_when_and_where_it_first_waited() {
        let first = Deferral::next(None, 1_000, 3);
        assert_eq!(first, Deferral { rounds: 1, since_ms: 1_000, order: 3 });
        let later = Deferral::next(Some(first), 99_000, 0);
        assert_eq!(later, Deferral { rounds: 2, since_ms: 1_000, order: 3 });
        let last = Deferral::next(Some(Deferral { rounds: u32::MAX, ..first }), 0, 0);
        assert_eq!(last.rounds, u32::MAX);
    }

    #[test]
    fn an_event_waits_behind_earlier_ones_of_its_scope_only() {
        let early = Deferral { rounds: 4, since_ms: 1_000, order: 0 };
        let late = Deferral { rounds: 1, since_ms: 5_000, order: 0 };
        let mut waiting = Waiting::default();
        let x = event("x", "e1", Some(early));
        waiting.add(scope_of(&x), WaitKey::of(&x));
        // A new event, and one that waited later, stay behind it.
        assert_eq!(waiting.ahead_of(&event("y", "e1", None)).unwrap().delivery_id, "x");
        assert!(waiting.ahead_of(&event("z", "e1", Some(late))).is_some());
        // Not a copy of itself, nor one of another scope.
        assert!(waiting.ahead_of(&event("x", "e1", Some(early))).is_none());
        assert!(waiting.ahead_of(&event("y", "e2", None)).is_none());
        // One that waited earlier goes first: never two blocking each other.
        let z = event("z", "e1", Some(late));
        waiting.add(scope_of(&z), WaitKey::of(&z));
        assert!(waiting.ahead_of(&event("x", "e1", Some(early))).is_none());
        assert_eq!(waiting.ahead_of(&z).unwrap().delivery_id, "x");
        assert_eq!(waiting.len(), 2);
        assert_eq!(waiting.oldest_since_ms(), Some(1_000));
        // The tenant's own events are one scope, whatever "no event" is spelled as.
        assert_eq!(scope_of(&event("a", "", None)), scope_of(&event("b", "null", None)));
    }

    #[test]
    fn a_batch_builds_the_events_that_waited_first_in_their_order() {
        let events = vec![
            event("new-1", "e1", None),
            event("held", "e1", Some(Deferral { rounds: 2, since_ms: 1_000, order: 1 })),
            event("new-2", "e2", None),
            event("first", "e1", Some(Deferral { rounds: 9, since_ms: 1_000, order: 0 })),
            event("later", "e1", Some(Deferral { rounds: 1, since_ms: 2_000, order: 0 })),
        ];
        let order: Vec<&str> = in_wait_order(&events)
            .into_iter()
            .map(|event| event.delivery_id.as_deref().unwrap())
            .collect();
        assert_eq!(order, vec!["first", "held", "later", "new-1", "new-2"]);
    }

    #[test]
    fn the_deferral_survives_the_queue_format() {
        let deferral = Deferral { rounds: 7, since_ms: 1_234, order: 2 };
        let waited = event("d1", "e1", Some(deferral));
        let encoded = serde_json::to_value(&waited).unwrap();
        assert_eq!(encoded["deferral"]["rounds"], 7);
        let decoded: LogEventInput = serde_json::from_value(encoded).unwrap();
        assert_eq!(decoded, waited);
        // Producers that never set it are read as not waiting.
        let fresh = serde_json::to_value(event("d2", "e1", None)).unwrap();
        assert!(fresh.get("deferral").is_none());
    }
}
