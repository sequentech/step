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
//!   (5 s) up to `ELECTORAL_LOG_DEFER_MAX_SECS` (30 s), and stays there for
//!   as many rounds as the outage lasts: there is no retry budget. The cap is
//!   the head's retry interval, so after the key service comes back a log's
//!   backlog drains within about 30 s (an attempt while it is still down fails
//!   at the signer and signs nothing).
//! - **Heads and why they wait.** A scope's head waits because its signer
//!   cannot sign now (`signer-unavailable`, no budget), because its tenant does
//!   not exist yet (`tenant-missing`: a Keycloak event can arrive while the
//!   tenant is being created), or because its record failed for another reason
//!   that may pass (`retry`: a signer refused or not loaded, or a batch that
//!   failed). The last two are bounded: past `ELECTORAL_LOG_BOUNDED_WAIT_SECS`
//!   (600 s) since the entry first waited it is dead-lettered (visible in the
//!   dead-letter queue with its reason, and its row is removed so its scope's
//!   later entries go on). Every head holds its scope: later events of the
//!   scope wait behind it (`held`), so a bounded wait never lets a later entry
//!   of its scope overtake it.
//! - **Order.** The events of one scope (an election event, or a tenant's
//!   own events) keep their order: an event waits behind any event of its
//!   scope that started waiting before it (in the queue or earlier in the
//!   batch), even if its own signing would work. Waiting events are ordered
//!   by when they first waited, then by their place in that batch. A held
//!   event has no backoff of its own: it is due just after the scope's
//!   earliest due waiting event; and when a batch stores a scope's entries
//!   and nothing of that scope waits for the signer any more, the scope's
//!   waiting entries are made due at once ([`release_scopes`]), so its
//!   backlog drains in order right after its head is stored.
//! - **Scale.** Only the waiting events of the batch's scopes are read, and
//!   the queue setup indexes the deferral headers (`pgmq-broker`'s
//!   `setup.sql`), so neither reading them nor putting one back scans the
//!   queue.
//! - **Concurrency.** A batch takes the processing lease of each of its
//!   scopes ([`take_leases`]: exclusive, sorted, held for its whole run), so
//!   two batches never store one scope at once and no gated append of the
//!   scope is admitted while a batch has its events in hand; what the batch
//!   could not store waits (as its scope's head) before the lease goes. Not
//!   covered (stated): an event still on the queue that no batch has picked
//!   up yet is not seen by admission; its statement time says when it happened.
//! - **Where waiting is recorded.** Every waiting entry has a row in
//!   `step_queue.electoral_log_waiting` (written in the same transaction as
//!   its put-back, removed once it is stored or dead-lettered). Only the
//!   workers' database role writes that table: Keycloak and Harvest, which
//!   can write the queues, can't make an entry wait or hold a log.
//! - **Direct posts keep the order (fail closed).** While a scope has a
//!   waiting row, a direct post to its log (`ElectoralLog::post_*` from a log
//!   that knows its scope) is refused with [`ScopeWaiting`] and stores
//!   nothing; it works again once the scope's waiting entries are stored.
//!   [`direct_post_gate`] is the one decision. It and every write that makes
//!   an entry wait take the scope's transaction-level advisory lock in the
//!   task-queue database (sorted, when several), and the direct post keeps
//!   its lock until its append is done: an entry that starts waiting
//!   meanwhile waits for the lock and comes after the post. Lock order: the
//!   scope locks are the only locks taken in the task-queue database by both
//!   paths, and the batch task never holds one while it appends, so there is
//!   no cycle with the log's own database.
//!
//!   A direct post takes the scope's lock shared (direct posts don't wait for
//!   each other) and gives up after [`GATE_LOCK_TIMEOUT`], refusing the post;
//!   writes that make an entry wait take it exclusive.
//!
//!   *Availability (operators).* While a log has entries waiting (its signer
//!   is down, its tenant does not exist yet, or a retry), every action that
//!   must record a direct entry in that log fails visibly: opening,
//!   closing or publishing an election, secret-attribute actions, readiness
//!   checks, key and tally ceremonies, configuration imports, among others.
//!   The error reads "the electoral log of <tenant>/<event> has N entries
//!   waiting for its signer; this entry is refused until they are stored".
//!   The waiting count and the oldest wait are in the "Electoral-log events
//!   wait for their signer" log line (an error after
//!   `ELECTORAL_LOG_DEFER_ALERT_SECS`). Once the key service is back, the
//!   waiting entries are stored within their backoff (at most
//!   `ELECTORAL_LOG_DEFER_MAX_SECS`) and the actions work again. Do not
//!   discard waiting or dead-lettered entries to unblock a log unless losing
//!   them is intended: they are the log's record.
//! - **Every append is gated** (`ScopeAdmission`, in `services/electoral_log.rs`),
//!   admitted BEFORE anything is signed, so a refused round signs nothing: direct
//!   posts (`post_built*`), the checkpoint and continuation records, the ballot
//!   box's sequencer (each round; refused, its ballots stay pending), the
//!   signing-log and audit-log outboxes (refused: no failure counted), the
//!   tally's sealed-box records, the ballot-box seal (refused: pending, busy),
//!   admin keys and the relay's announcements on the tenant log, and the CSV and
//!   transfer restores (nothing signed). Only the batch task (the drainer) appends
//!   without admission, under its scopes' leases. `electoral_log_append_coverage`
//!   lists every production append and fails on an unclassified one. A stored
//!   checkpoint whose record was refused is recorded later
//!   (`record_missing_checkpoints`, each periodic beat).
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
use std::collections::{HashMap, HashSet};
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
pub const DEFAULT_DEFER_MAX: Duration = Duration::from_secs(30);
/// Environment variable with how long, in seconds, a bounded head
/// (`tenant-missing`, `retry`) may wait before it is dead-lettered.
pub const BOUNDED_WAIT_ENV: &str = "ELECTORAL_LOG_BOUNDED_WAIT_SECS";
pub const DEFAULT_BOUNDED_WAIT: Duration = Duration::from_secs(600);
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
    /// When its current stretch as a bounded head (`tenant-missing`, `retry`)
    /// started, in milliseconds since the epoch (`None` outside one), and the
    /// bounded time it already spent in earlier stretches ([`Deferral::bounded_ms`]).
    /// Taken ONLY from its waiting row (kept by the workers; a message's values are
    /// ignored), so neither a queue writer nor a restart can move or extend them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bounded_since_ms: Option<i64>,
    #[serde(default)]
    pub bounded_used_ms: i64,
}

impl Deferral {
    /// The bounded time it has spent at `now_ms`: earlier stretches plus the
    /// current one (a clock that went back counts nothing for it).
    pub fn bounded_ms(&self, now_ms: i64) -> i64 {
        self.bounded_used_ms.max(0).saturating_add(
            self.bounded_since_ms
                .map_or(0, |since| now_ms.saturating_sub(since).max(0)),
        )
    }

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
                bounded_since_ms: None,
                bounded_used_ms: 0,
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
    scope_key(&event.tenant_id, election_event_id)
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
    /// Its board's signer cannot sign now (no budget: as long as the outage).
    SignerUnavailable,
    /// Its tenant does not exist yet (bounded: [`DeferralBackoff::bounded`]).
    TenantMissing,
    /// Its record failed for a reason that may pass (a signer refused or not
    /// loaded, or its batch failed): bounded, like `TenantMissing`.
    Retry,
    /// An earlier event of its scope waits.
    Held,
}

impl WaitReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            WaitReason::SignerUnavailable => "signer-unavailable",
            WaitReason::TenantMissing => "tenant-missing",
            WaitReason::Retry => "retry",
            WaitReason::Held => "held",
        }
    }

    /// Whether an event waiting for this reason is its scope's head: it waits
    /// on its own backoff and holds the scope's later events behind it (all
    /// reasons but `Held`).
    pub fn is_head(&self) -> bool {
        !matches!(self, WaitReason::Held)
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
    /// When each waiting entry is due, in milliseconds, by delivery ID, with its scope.
    due_by_delivery: HashMap<String, (String, i64)>,
    /// Each row's bounded budget: (`bounded_since_ms`, `bounded_used_ms`).
    bounded_since: HashMap<String, (Option<i64>, i64)>,
}

impl Waiting {
    pub fn add(&mut self, scope: String, key: WaitKey) {
        self.by_scope.entry(scope).or_default().push(key);
    }

    /// Notes the entry `delivery_id`'s bounded budget: its current stretch's
    /// start (if in one) and the bounded time it already spent.
    pub fn add_bounded(&mut self, delivery_id: String, since_ms: Option<i64>, used_ms: i64) {
        self.bounded_since.insert(delivery_id, (since_ms, used_ms));
    }

    /// Notes that the entry `delivery_id` of `scope` is due at `due_ms`.
    pub fn add_due(&mut self, scope: String, delivery_id: String, due_ms: i64) {
        self.due_by_delivery.insert(delivery_id, (scope, due_ms));
    }

    /// When the earliest waiting entry of `scope` is due.
    pub fn due_ms(&self, scope: &str) -> Option<i64> {
        self.due_by_delivery
            .values()
            .filter(|(entry_scope, _)| entry_scope == scope)
            .map(|(_, due)| *due)
            .min()
    }

    /// The wait keys of these delivery IDs' own rows, where they have one: the
    /// table is authoritative for when and where an entry first waited.
    pub fn keys_for(&self, delivery_ids: &HashSet<String>) -> HashMap<String, WaitKey> {
        self.by_scope
            .values()
            .flatten()
            .filter(|key| delivery_ids.contains(&key.delivery_id))
            .map(|key| (key.delivery_id.clone(), key.clone()))
            .collect()
    }

    /// The entries waiting besides these delivery IDs' own rows (a batch's own
    /// events don't wait behind themselves).
    pub fn without(mut self, delivery_ids: &HashSet<String>) -> Waiting {
        for keys in self.by_scope.values_mut() {
            keys.retain(|key| !delivery_ids.contains(&key.delivery_id));
        }
        self.by_scope.retain(|_, keys| !keys.is_empty());
        self.due_by_delivery
            .retain(|delivery_id, _| !delivery_ids.contains(delivery_id));
        self
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

/// A batch's events bound to the waiting table: an event's deferral is the
/// one its own row records (when and where it first waited), and an event
/// without a row has none, whatever its queue message says (a queue writer
/// can't backdate it, jump the queue, or keep it from the last retry). Returns
/// the bound events and the entries they wait behind (not their own rows).
pub fn bind_to_rows(events: &[LogEventInput], waiting: Waiting) -> (Vec<LogEventInput>, Waiting) {
    let ids: HashSet<String> = events
        .iter()
        .filter_map(|event| event.delivery_id.clone())
        .collect();
    let rows = waiting.keys_for(&ids);
    let bounded = waiting.bounded_since.clone();
    let bound = events
        .iter()
        .map(|event| {
            let mut event = event.clone();
            let row = event.delivery_id.as_ref().and_then(|id| rows.get(id));
            event.deferral = row.map(|key| Deferral {
                rounds: event.deferral.map_or(0, |deferral| deferral.rounds),
                since_ms: key.since_ms,
                order: key.order,
                // The row's own (worker-kept); never the message's.
                bounded_since_ms: bounded.get(&key.delivery_id).and_then(|budget| budget.0),
                bounded_used_ms: bounded.get(&key.delivery_id).map_or(0, |budget| budget.1),
            });
            event
        })
        .collect();
    (bound, waiting.without(&ids))
}

/// How long waiting events wait between rounds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeferralBackoff {
    pub first: Duration,
    pub max: Duration,
    /// Waiting longer than this is logged as an error.
    pub alert: Duration,
    /// A bounded head (`tenant-missing`, `retry`) waiting longer than this,
    /// since it first waited, is dead-lettered.
    pub bounded: Duration,
}

impl Default for DeferralBackoff {
    fn default() -> Self {
        DeferralBackoff {
            first: DEFAULT_DEFER_FIRST,
            max: DEFAULT_DEFER_MAX,
            alert: DEFAULT_DEFER_ALERT,
            bounded: DEFAULT_BOUNDED_WAIT,
        }
    }
}

impl DeferralBackoff {
    /// Read from the environment; an unset or empty variable takes its
    /// default, any other value must be a positive number of seconds.
    pub fn from_env() -> Result<Self> {
        let mut backoff = Self::from_values(
            std::env::var(DEFER_FIRST_ENV).ok().as_deref(),
            std::env::var(DEFER_MAX_ENV).ok().as_deref(),
            std::env::var(DEFER_ALERT_ENV).ok().as_deref(),
        )?;
        backoff.bounded = seconds(
            BOUNDED_WAIT_ENV,
            std::env::var(BOUNDED_WAIT_ENV).ok().as_deref(),
            DEFAULT_BOUNDED_WAIT,
        )?;
        Ok(backoff)
    }

    /// Whether a bounded head has spent more than its bound as one at `now_ms`
    /// (its row's budget: [`Deferral::bounded_ms`]; no deferral: none spent) and is
    /// dead-lettered instead of waiting again.
    pub fn bound_exceeded(&self, deferral: Option<&Deferral>, now_ms: i64) -> bool {
        let bound = i64::try_from(self.bounded.as_millis()).unwrap_or(i64::MAX);
        deferral.is_some_and(|deferral| deferral.bounded_ms(now_ms) > bound)
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
            bounded: DEFAULT_BOUNDED_WAIT,
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

/// The table of the entries that wait (`pgmq-broker`'s `setup.sql`): only the
/// workers write it, so queue writers can neither forge nor hold a waiting entry.
pub const WAITING_TABLE: &str = "step_queue.electoral_log_waiting";

/// The scope key of an election event's log (or a tenant's own, with an empty
/// election event).
pub fn scope_key(tenant_id: &str, election_event_id: &str) -> String {
    format!("{tenant_id}/{election_event_id}")
}

/// Takes the scopes' locks for the caller's queue-database transaction, in one
/// order (sorted), so two transactions never wait on each other in a cycle.
/// Every write that makes an entry wait, and every direct post, takes its
/// scope's lock first: see [`direct_post_gate`].
pub async fn lock_scopes<C: GenericClient + Sync>(client: &C, scopes: &[String]) -> Result<()> {
    let mut scopes: Vec<&String> = scopes.iter().collect();
    scopes.sort();
    scopes.dedup();
    for scope in scopes {
        client
            .query(
                "SELECT pg_advisory_xact_lock(hashtextextended($1, 0))",
                &[&format!("electoral-log-queue-scope:{scope}")],
            )
            .await
            .context("Error locking an electoral-log scope")?;
    }
    Ok(())
}

/// How long a direct post waits for its scope's lock before it is refused.
pub const GATE_LOCK_TIMEOUT: &str = "5s";
/// How long a batch waits for the leases of its scopes (a gated append holds
/// one, shared, while it appends) before it fails (its events then wait as
/// their scopes' heads: `retry`).
pub const LEASE_LOCK_TIMEOUT: &str = "60s";

/// The advisory-lock key of a scope's processing lease (task-queue database):
/// a batch holds it exclusive while it processes events of the scope; a gated
/// append holds it shared while it appends. Distinct from the scope lock.
fn lease_key(scope: &str) -> String {
    format!("electoral-log-lease:{scope}")
}

/// Takes the processing leases of `scopes` for the caller's transaction,
/// exclusive, sorted (so two batches never wait on each other in a cycle), and
/// holds them until it ends: a batch takes them first and keeps them for its
/// whole run (reading what waits, putting events back, appending, settling),
/// so no gated append of those scopes is admitted while the batch has events of
/// theirs in hand, and two batches never store one scope at once. Lock order:
/// lease, then the scope lock ([`lock_scopes`]), the same as a gated append's
/// ([`direct_post_gate`]): no cycle.
pub async fn take_leases<C: GenericClient + Sync>(client: &C, scopes: &[String]) -> Result<()> {
    let mut scopes: Vec<&String> = scopes.iter().collect();
    scopes.sort();
    scopes.dedup();
    client
        .batch_execute(&format!("SET LOCAL lock_timeout = '{LEASE_LOCK_TIMEOUT}'"))
        .await
        .context("Error setting the electoral-log lease timeout")?;
    for scope in scopes {
        client
            .query(
                "SELECT pg_advisory_xact_lock(hashtextextended($1, 0))",
                &[&lease_key(scope)],
            )
            .await
            .with_context(|| format!("Error taking the electoral-log lease of {scope}"))?;
    }
    Ok(())
}

/// A gated append was refused because its scope's lease or lock could not be
/// had in time (a batch is processing the scope, or an entry is starting to
/// wait): like [`ScopeWaiting`], the entry is not appended now and is tried
/// again; nothing is lost and it is not a failure of the entry.
#[derive(Debug)]
pub struct ScopeBusy {
    pub scope: String,
    pub what: &'static str,
}

impl std::fmt::Display for ScopeBusy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "the electoral log of {} is busy ({}); this entry is refused for now (the log keeps its order)",
            self.scope, self.what
        )
    }
}

impl std::error::Error for ScopeBusy {}

/// Whether `error` means a gated append was refused for its log's order (its
/// scope has entries waiting, or is busy), not that the entry failed: a durable
/// path keeps the entry and tries again, without counting a failure.
pub fn is_scope_gated(error: &anyhow::Error) -> bool {
    error
        .chain()
        .any(|cause| cause.is::<ScopeWaiting>() || cause.is::<ScopeBusy>())
}

/// Takes one shared advisory lock for the gate; a lock not had within the
/// transaction's lock_timeout is [`ScopeBusy`], any other error stays an error.
async fn gate_lock_shared<C: GenericClient + Sync>(
    client: &C,
    key: &str,
    scope: &str,
    what: &'static str,
) -> Result<()> {
    match client
        .query("SELECT pg_advisory_xact_lock_shared(hashtextextended($1, 0))", &[&key])
        .await
    {
        Ok(_) => Ok(()),
        Err(error)
            if error.code() == Some(&tokio_postgres::error::SqlState::LOCK_NOT_AVAILABLE) =>
        {
            Err(anyhow::Error::new(ScopeBusy {
                scope: scope.to_string(),
                what,
            }))
        }
        Err(error) => Err(anyhow::Error::new(error).context("Error taking an electoral-log gate lock")),
    }
}

/// The entries waiting in these `scopes` (the table's scope index).
pub async fn waiting_events<C: GenericClient + Sync>(
    client: &C,
    scopes: &[String],
) -> Result<Waiting> {
    let mut waiting = Waiting::default();
    if scopes.is_empty() {
        return Ok(waiting);
    }
    let rows = client
        .query(
            &format!(
                "SELECT scope, delivery_id, since_ms, ord, \
                 (extract(epoch FROM due_at) * 1000)::bigint, bounded_since_ms, bounded_used_ms \
                 FROM {WAITING_TABLE} WHERE scope = ANY($1)"
            ),
            &[&scopes],
        )
        .await
        .context("Error reading the electoral-log entries that wait")?;
    for row in rows {
        let scope: String = row.get(0);
        let delivery_id: String = row.get(1);
        let since_ms: i64 = row.get(2);
        let order: i64 = row.get(3);
        let due_ms: i64 = row.get(4);
        let bounded_since_ms: Option<i64> = row.get(5);
        let bounded_used_ms: i64 = row.get(6);
        if bounded_since_ms.is_some() || bounded_used_ms > 0 {
            waiting.add_bounded(delivery_id.clone(), bounded_since_ms, bounded_used_ms);
        }
        waiting.add_due(scope.clone(), delivery_id.clone(), due_ms);
        waiting.add(
            scope,
            WaitKey {
                since_ms,
                order: u32::try_from(order).unwrap_or(u32::MAX),
                delivery_id,
            },
        );
    }
    Ok(waiting)
}

/// Of these delivery IDs, those that wait (have a row).
pub async fn waiting_ids<C: GenericClient + Sync>(
    client: &C,
    delivery_ids: &[String],
) -> Result<HashSet<String>> {
    if delivery_ids.is_empty() {
        return Ok(HashSet::new());
    }
    Ok(client
        .query(
            &format!("SELECT delivery_id FROM {WAITING_TABLE} WHERE delivery_id = ANY($1)"),
            &[&delivery_ids],
        )
        .await
        .context("Error reading which electoral-log entries wait")?
        .iter()
        .map(|row| row.get(0))
        .collect())
}

/// Of these delivery IDs, those whose row is BOUNDED: only an in-flight marker
/// ([`mark_in_flight`]: no copy on the queue, no statement; attempted by a batch,
/// never put back to wait for its signer) or a head in an open bounded stretch
/// (`bounded_since_ms` set: `retry` / `tenant-missing`). At the last retry these go
/// back as bounded `retry` heads; the others (waits for the signer, held) as they were.
pub async fn bounded_row_ids<C: GenericClient + Sync>(
    client: &C,
    delivery_ids: &[String],
) -> Result<HashSet<String>> {
    if delivery_ids.is_empty() {
        return Ok(HashSet::new());
    }
    Ok(client
        .query(
            &format!(
                "SELECT delivery_id FROM {WAITING_TABLE} WHERE delivery_id = ANY($1) \
                 AND ((copy_msg_id IS NULL AND statement IS NULL) OR bounded_since_ms IS NOT NULL)"
            ),
            &[&delivery_ids],
        )
        .await
        .context("Error reading which electoral-log entries are in flight")?
        .iter()
        .map(|row| row.get(0))
        .collect())
}

/// Removes the rows of entries that no longer wait: stored, or dead-lettered.
pub async fn stop_waiting<C: GenericClient + Sync>(
    client: &C,
    delivery_ids: &[String],
) -> Result<()> {
    if delivery_ids.is_empty() {
        return Ok(());
    }
    client
        .execute(
            &format!("DELETE FROM {WAITING_TABLE} WHERE delivery_id = ANY($1)"),
            &[&delivery_ids],
        )
        .await
        .context("Error removing electoral-log entries that no longer wait")?;
    Ok(())
}

/// The waiting table's columns this worker needs (added by `setup-queue-database`).
pub const REQUIRED_WAITING_COLUMNS: [&str; 2] = ["bounded_since_ms", "bounded_used_ms"];

/// The columns of [`REQUIRED_WAITING_COLUMNS`] that the task-queue database's
/// waiting table lacks (all of them if the table itself is missing).
pub async fn missing_waiting_columns<C: GenericClient + Sync>(client: &C) -> Result<Vec<String>> {
    let present: HashSet<String> = client
        .query(
            "SELECT column_name::text FROM information_schema.columns \
             WHERE table_schema = 'step_queue' AND table_name = 'electoral_log_waiting'",
            &[],
        )
        .await
        .context("Error reading the waiting table's columns")?
        .iter()
        .map(|row| row.get(0))
        .collect();
    Ok(REQUIRED_WAITING_COLUMNS
        .iter()
        .filter(|column| !present.contains(**column))
        .map(|column| column.to_string())
        .collect())
}

/// Refuses to start a worker whose task-queue database's schema is older than it
/// (`setup-queue-database` not run since the upgrade): it would fail every
/// electoral-log batch and, after their retries, dead-letter them. Fails visibly,
/// BEFORE anything is consumed.
pub async fn ensure_waiting_table_schema() -> Result<()> {
    let client = crate::services::database::try_get_queue_pool()
        .await
        .context("The task-queue database is unavailable: the worker does not start")?
        .get()
        .await
        .context("The task-queue database is unavailable: the worker does not start")?;
    let missing = missing_waiting_columns(&**client).await?;
    anyhow::ensure!(
        missing.is_empty(),
        "The task-queue database's schema is older than this worker: \
         step_queue.electoral_log_waiting lacks {missing:?}. Run `setup-queue-database` \
         (the stack's task-queues-setup) first; this worker does not start (it would fail \
         every electoral-log batch)."
    );
    Ok(())
}

/// How long after a batch marks its events in flight their markers fall due:
/// longer than any batch runs, so only a batch that never finished (its worker
/// died and the task was not redelivered) leaves markers that are reported as
/// orphans ([`report_orphans`]).
pub const IN_FLIGHT_SECS: i32 = 900;

/// Marks a batch's events as in flight BEFORE it attempts any of them: each event
/// that has no waiting row gets one (no copy on the queue; due in
/// [`IN_FLIGHT_SECS`]). Committed by the caller before the batch signs or appends.
/// So an attempted event is never invisible to admission: if the worker dies mid
/// batch, its events' rows keep their logs' gated appends refused until the task
/// is redelivered and settles them (stored, put back or dead-lettered); a batch
/// that settles normally removes or updates every marker it made. Rows that
/// already exist are left as they are. Returns how many it marked.
pub async fn mark_in_flight<C: GenericClient + Sync>(
    client: &C,
    events: &[LogEventInput],
    now_ms: i64,
) -> Result<usize> {
    let events: Vec<&LogEventInput> = events
        .iter()
        .filter(|event| event.delivery_id.as_deref().is_some_and(|id| !id.is_empty()))
        .collect();
    if events.is_empty() {
        return Ok(0);
    }
    let scopes: Vec<String> = events.iter().map(|event| scope_of(event)).collect();
    lock_scopes(client, &scopes).await?;
    let mut marked = 0;
    for (index, event) in events.iter().enumerate() {
        marked += client
            .execute(
                &format!(
                    "INSERT INTO {WAITING_TABLE} (delivery_id, scope, since_ms, ord, due_at) \
                     VALUES ($1, $2, $3, $4, clock_timestamp() + make_interval(secs => $5::integer)) \
                     ON CONFLICT (delivery_id) DO NOTHING"
                ),
                &[
                    &event.delivery_id.clone().unwrap_or_default(),
                    &scope_of(event),
                    &now_ms,
                    &i64::try_from(index).unwrap_or(i64::MAX),
                    &IN_FLIGHT_SECS,
                ],
            )
            .await
            .context("Error marking an electoral-log event in flight")?;
    }
    Ok(usize::try_from(marked).unwrap_or(usize::MAX))
}

/// Makes the waiting entries of these scopes due now (their rows and their
/// copies on `queue`): called for the scopes whose head a batch just stored and
/// that have nothing waiting for the signer any more, so a log's backlog drains
/// right away, in order, instead of each entry at its own next due time.
pub async fn release_scopes<C: GenericClient + Sync>(
    client: &C,
    queue: &str,
    scopes: &[String],
) -> Result<()> {
    if scopes.is_empty() {
        return Ok(());
    }
    let copies: Vec<i64> = client
        .query(
            &format!(
                "UPDATE {WAITING_TABLE} SET due_at = clock_timestamp() \
                 WHERE scope = ANY($1) RETURNING copy_msg_id"
            ),
            &[&scopes],
        )
        .await
        .context("Error releasing a log's waiting entries")?
        .iter()
        .filter_map(|row| row.get::<_, Option<i64>>(0))
        .collect();
    if !copies.is_empty() {
        client
            .query("SELECT pgmq.set_vt($1, $2::bigint[], 0)", &[&queue, &copies])
            .await
            .context("Error making a log's waiting entries due")?;
    }
    Ok(())
}

/// The scopes a batch stored entries of and has nothing waiting for the
/// signer in (entries only held behind others don't count): their backlog may
/// go now ([`release_scopes`]).
pub fn released_scopes(
    events: &[LogEventInput],
    stored_ids: &HashSet<String>,
    waiting: &[WaitingEvent],
) -> Vec<String> {
    let still_waiting: HashSet<String> = waiting
        .iter()
        .filter(|item| item.reason.is_head())
        .map(|item| scope_of(&item.event))
        .collect();
    let mut scopes: Vec<String> = events
        .iter()
        .filter(|event| {
            event
                .delivery_id
                .as_ref()
                .is_some_and(|id| stored_ids.contains(id))
        })
        .map(scope_of)
        .filter(|scope| !still_waiting.contains(scope))
        .collect();
    scopes.sort();
    scopes.dedup();
    scopes
}

/// Alert class of waiting rows past due whose copy is no longer on the queue
/// (it was lost: dead-lettered and discarded, or an older worker's).
pub const ALERT_ORPHAN_WAITING: &str = "electoral-log-orphan-waiting";

/// Logs, with [`ALERT_ORPHAN_WAITING`], the waiting rows more than `grace_secs`
/// past due whose recorded copy is not on `queue`: they hold their scope's
/// direct posts until an operator replays the entry or removes the row
/// (documented in the task-queue page). Returns how many.
pub async fn report_orphans<C: GenericClient + Sync>(
    client: &C,
    queue: &str,
    grace_secs: i32,
) -> Result<usize> {
    pgmq_broker::validate_queue_name(queue).map_err(anyhow::Error::msg)?;
    let rows = client
        .query(
            &format!(
                "SELECT w.delivery_id, w.scope, \
                 (extract(epoch FROM clock_timestamp() - w.due_at))::bigint \
                 FROM {WAITING_TABLE} w \
                 WHERE w.due_at < clock_timestamp() - make_interval(secs => $1::integer) \
                 AND NOT EXISTS (SELECT 1 FROM pgmq.q_{queue} q WHERE q.msg_id = w.copy_msg_id) \
                 LIMIT 100"
            ),
            &[&grace_secs],
        )
        .await
        .context("Error looking for orphan electoral-log waiting rows")?;
    for row in &rows {
        let delivery_id: String = row.get(0);
        let scope: String = row.get(1);
        let overdue_secs: i64 = row.get(2);
        tracing::error!(
            alert = ALERT_ORPHAN_WAITING,
            delivery_id,
            scope,
            overdue_secs,
            "An electoral-log entry waits with no copy on the queue: its log's direct posts are held"
        );
    }
    Ok(rows.len())
}

/// A direct post was refused: its log has entries waiting for the board's
/// signer, which must be stored first (the log keeps its order). Not a
/// [`electoral_log::signer::SignerUnavailable`]: nothing queues it.
#[derive(Debug)]
pub struct ScopeWaiting {
    pub scope: String,
    pub waiting: i64,
}

impl std::fmt::Display for ScopeWaiting {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "the electoral log of {} has {} entries waiting (for its signer, its tenant, \
             a retry, or in a batch's hands); this entry is refused until they are stored \
             (the log keeps its order)",
            self.scope, self.waiting
        )
    }
}

impl std::error::Error for ScopeWaiting {}

/// Whether `error` is, or was caused by, a [`ScopeWaiting`].
pub fn is_scope_waiting(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| cause.is::<ScopeWaiting>())
}

/// The one decision whether a direct post to `scope` may proceed: only when
/// none of the scope's entries waits. Call it in a queue-database
/// transaction kept open until the post's append is done: it takes the
/// scope's lock ([`lock_scopes`]), which every write that makes an entry of
/// the scope wait takes too, so no entry can start waiting between this
/// check and the append (it waits for the lock and comes after the post).
pub async fn direct_post_gate<C: GenericClient + Sync>(client: &C, scope: &str) -> Result<()> {
    // Shared: direct posts of a scope don't wait for each other, only for a
    // write that makes an entry wait (exclusive). A lock not had in time
    // refuses the post (fail closed), it never hangs.
    client
        .batch_execute(&format!("SET LOCAL lock_timeout = '{GATE_LOCK_TIMEOUT}'"))
        .await
        .context("Error setting the electoral-log scope lock's timeout")?;
    // First the scope's lease (shared): not while a batch is processing events
    // of the scope (it holds it exclusive until what it could not store waits,
    // durably), so an entry admitted here never gets ahead of an event of its
    // scope that was already attempted. Then the scope lock. Same order as a
    // batch (lease, then scope lock): no cycle.
    gate_lock_shared(client, &lease_key(scope), scope, "a batch is processing it").await?;
    gate_lock_shared(
        client,
        &format!("electoral-log-queue-scope:{scope}"),
        scope,
        "an entry is starting to wait",
    )
    .await?;
    let waiting: i64 = client
        .query_one(
            &format!("SELECT count(scope) FROM {WAITING_TABLE} WHERE scope = $1"),
            &[&scope],
        )
        .await
        .context("Error reading the electoral-log entries that wait")?
        .get(0);
    if waiting > 0 {
        tracing::error!(
            scope,
            waiting,
            "A direct electoral-log post is refused: its log has entries waiting for the signer"
        );
        return Err(anyhow::Error::new(ScopeWaiting {
            scope: scope.to_string(),
            waiting,
        }));
    }
    Ok(())
}

/// What one round of putting events back did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PutBack {
    /// Events whose signer cannot sign now.
    pub signer_unavailable: usize,
    /// Events whose tenant does not exist yet.
    pub tenant_missing: usize,
    /// Events retried (bounded) after another failure.
    pub retry: usize,
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

/// How long a held event waits after its scope's head is due.
pub const HELD_LAG: Duration = Duration::from_secs(1);

/// Each waiting event of a batch as it is put back, with its deferral and its
/// wait: an event whose signer is unavailable doubles its own wait; a held
/// event is due [`HELD_LAG`] after its scope's earliest due event (one of this
/// batch's, or one already in the queue: `heads`), or after the first wait if
/// there is none.
pub fn plan(
    waiting: &[WaitingEvent],
    heads: &Waiting,
    backoff: &DeferralBackoff,
    now_ms: i64,
) -> Vec<(LogEventInput, Deferral, Duration)> {
    let mut planned = Vec::with_capacity(waiting.len());
    let mut due_by_scope: HashMap<String, i64> = HashMap::new();
    for (index, item) in waiting.iter().enumerate() {
        let order = u32::try_from(index).unwrap_or(u32::MAX);
        let (event, delay) = next_round(&item.event, order, backoff, now_ms);
        if item.reason.is_head() {
            let due = now_ms.saturating_add(i64::try_from(delay.as_millis()).unwrap_or(i64::MAX));
            let scope_due = due_by_scope.entry(scope_of(&event)).or_insert(due);
            *scope_due = (*scope_due).min(due);
        }
        let deferral = event.deferral.unwrap_or(Deferral {
            rounds: 1,
            since_ms: now_ms,
            order, bounded_since_ms: None, bounded_used_ms: 0 });
        planned.push((item.reason, event, deferral, delay));
    }
    planned
        .into_iter()
        .map(|(reason, event, deferral, delay)| {
            let delay = match reason {
                WaitReason::SignerUnavailable | WaitReason::TenantMissing | WaitReason::Retry => {
                    delay
                }
                WaitReason::Held => {
                    let scope = scope_of(&event);
                    let head = [due_by_scope.get(&scope).copied(), heads.due_ms(&scope)]
                        .into_iter()
                        .flatten()
                        .min();
                    match head {
                        Some(due_ms) => {
                            let until = u64::try_from(due_ms.saturating_sub(now_ms)).unwrap_or(0);
                            Duration::from_millis(until) + HELD_LAG
                        }
                        None => backoff.first,
                    }
                }
            };
            (event, deferral, delay)
        })
        .collect()
}

/// Put the waiting events of a batch back on `queue` (the event queue), each
/// due after its wait ([`plan`]), in the caller's transaction. An event of
/// which a copy already waits there is not put back again.
pub async fn put_back<C: GenericClient + Sync>(
    client: &C,
    queue: &str,
    waiting: &[WaitingEvent],
    heads: &Waiting,
    backoff: &DeferralBackoff,
    now_ms: i64,
) -> Result<PutBack> {
    pgmq_broker::validate_queue_name(queue).map_err(anyhow::Error::msg)?;
    let scopes: Vec<String> = waiting.iter().map(|item| scope_of(&item.event)).collect();
    lock_scopes(client, &scopes).await?;
    // The bounded budget, kept here only: a bounded round ($6 = now) opens a
    // stretch if none is open (an open one keeps its start); any other round
    // ($6 NULL) closes an open stretch, adding it to the time spent ($7 = now).
    // The total only grows: the bound can't be extended by restarts or by
    // alternating rounds, and time waiting for the signer never counts.
    let mark = format!(
        "INSERT INTO {WAITING_TABLE} AS w (delivery_id, scope, since_ms, ord, due_at, bounded_since_ms, bounded_used_ms) \
         VALUES ($1, $2, $3, $4, clock_timestamp() + make_interval(secs => $5::integer), $6, 0) \
         ON CONFLICT (delivery_id) DO UPDATE SET due_at = EXCLUDED.due_at, \
         bounded_used_ms = CASE WHEN EXCLUDED.bounded_since_ms IS NULL AND w.bounded_since_ms IS NOT NULL \
             THEN w.bounded_used_ms + GREATEST($7::bigint - w.bounded_since_ms, 0) ELSE w.bounded_used_ms END, \
         bounded_since_ms = CASE WHEN EXCLUDED.bounded_since_ms IS NULL THEN NULL \
             ELSE COALESCE(w.bounded_since_ms, EXCLUDED.bounded_since_ms) END"
    );
    // A copy already waits only if the row's own recorded message is still on
    // the queue (not a header a queue writer could forge).
    let send = format!(
        "SELECT pgmq.send($1::text, $2::jsonb, $3::jsonb, $4::integer) \
         WHERE NOT EXISTS (SELECT 1 FROM pgmq.q_{queue} q JOIN {WAITING_TABLE} w \
         ON q.msg_id = w.copy_msg_id WHERE w.delivery_id = $5::text)"
    );
    let record = format!("UPDATE {WAITING_TABLE} SET copy_msg_id = $1 WHERE delivery_id = $2");
    let mut summary = PutBack::default();
    let planned = plan(waiting, heads, backoff, now_ms);
    for (item, (event, deferral, delay)) in waiting.iter().zip(planned) {
        let delay_secs = i32::try_from(delay.as_millis().div_ceil(1000)).unwrap_or(i32::MAX);
        let delivery_id = event.delivery_id.clone().unwrap_or_default();
        let message = event_message(&event)?;
        let headers = deferral_headers(&event, &deferral, item);
        client
            .execute(
                &mark,
                &[
                    &delivery_id,
                    &scope_of(&event),
                    &deferral.since_ms,
                    &i64::from(deferral.order),
                    &delay_secs,
                    &(matches!(item.reason, WaitReason::TenantMissing | WaitReason::Retry)
                        .then_some(now_ms)),
                    &now_ms,
                ],
            )
            .await
            .context("Error marking an electoral-log entry as waiting")?;
        let sent = client
            .query(
                &send,
                &[&queue, &message, &headers, &delay_secs, &delivery_id],
            )
            .await
            .context("Error putting back an electoral-log event that waits")?;
        match item.reason {
            WaitReason::SignerUnavailable => summary.signer_unavailable += 1,
            WaitReason::TenantMissing => summary.tenant_missing += 1,
            WaitReason::Retry => summary.retry += 1,
            WaitReason::Held => summary.held += 1,
        }
        match sent.first() {
            None => summary.already_waiting += 1,
            Some(row) => {
                let msg_id: i64 = row.get(0);
                client
                    .execute(&record, &[&msg_id, &delivery_id])
                    .await
                    .context("Error recording an electoral-log entry's copy")?;
                summary.put_back += 1;
            }
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
            tenant_missing = summary.tenant_missing,
            retry = summary.retry,
            held = summary.held,
            put_back = summary.put_back,
            already_waiting = summary.already_waiting,
            waiting_in_queue = waiting_before.len(),
            oldest_waiting_secs = oldest_secs,
            next_attempt_secs,
            "Electoral-log events have waited for too long (their signer, tenant or a retry); they are kept and tried again"
        );
    } else {
        tracing::warn!(
            signer_unavailable = summary.signer_unavailable,
            tenant_missing = summary.tenant_missing,
            retry = summary.retry,
            held = summary.held,
            put_back = summary.put_back,
            already_waiting = summary.already_waiting,
            waiting_in_queue = waiting_before.len(),
            oldest_waiting_secs = oldest_secs,
            next_attempt_secs,
            "Electoral-log events wait (their signer, tenant or a retry); they are kept and tried again"
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
        // A scope's head is tried again at most every 30 s (a log's backlog
        // drains within about 30 s of its key service coming back).
        assert_eq!(delays, vec![5, 10, 20, 30, 30, 30, 30, 30, 30]);
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
    fn a_bounded_head_is_dead_lettered_only_strictly_past_its_bound() {
        let backoff = DeferralBackoff::default();
        assert_eq!(backoff.bounded, DEFAULT_BOUNDED_WAIT);
        assert_eq!(DEFAULT_BOUNDED_WAIT, Duration::from_secs(600));
        let now = 1_000_000_000;
        let d = |since: Option<i64>, used: i64| Deferral {
            rounds: 1, since_ms: 0, order: 0, bounded_since_ms: since, bounded_used_ms: used,
        };
        assert!(!backoff.bound_exceeded(None, now), "a first wait has spent nothing");
        assert!(!backoff.bound_exceeded(Some(&d(None, 0)), now));
        assert!(!backoff.bound_exceeded(Some(&d(Some(now), 0)), now));
        assert!(!backoff.bound_exceeded(Some(&d(Some(now - 600_000), 0)), now));
        assert!(backoff.bound_exceeded(Some(&d(Some(now - 600_001), 0)), now));
        // Earlier stretches count; time between them (waiting for the signer) doesn't.
        assert!(!backoff.bound_exceeded(Some(&d(Some(now - 300_000), 300_000)), now));
        assert!(backoff.bound_exceeded(Some(&d(Some(now - 300_000), 300_001)), now));
        // Outside a stretch, the time already spent still counts (alternating
        // rounds can't extend the bound).
        assert!(backoff.bound_exceeded(Some(&d(None, 600_001)), now));
        // A clock that went back counts nothing for the current stretch.
        assert!(!backoff.bound_exceeded(Some(&d(Some(now + 5_000), 0)), now));
    }

    #[test]
    fn every_head_reason_waits_on_its_own_backoff_and_holds_its_scope() {
        let backoff = DeferralBackoff::default();
        for reason in [WaitReason::SignerUnavailable, WaitReason::TenantMissing, WaitReason::Retry] {
            assert!(reason.is_head());
            let waiting = vec![
                WaitingEvent { event: event("head", "e1", None), reason, error: String::new() },
                WaitingEvent { event: event("later", "e1", None), reason: WaitReason::Held, error: String::new() },
            ];
            let planned = plan(&waiting, &Waiting::default(), &backoff, 0);
            // The head: its own first wait (5 s); the held one: 1 s after it.
            assert_eq!(planned[0].2, Duration::from_secs(5), "{reason:?}");
            assert_eq!(planned[1].2, Duration::from_secs(6), "{reason:?}");
            // A scope whose head still waits is not released by storing
            // something else of it.
            let stored: HashSet<String> = ["other".to_string()].into();
            let events = vec![event("other", "e1", None)];
            assert!(released_scopes(&events, &stored, &waiting).is_empty(), "{reason:?}");
        }
        assert!(!WaitReason::Held.is_head());
        assert_eq!(WaitReason::TenantMissing.as_str(), "tenant-missing");
        assert_eq!(WaitReason::Retry.as_str(), "retry");
    }

    #[test]
    fn a_bounded_start_comes_only_from_its_row_never_from_a_message() {
        let row = event("x", "e1", Some(Deferral { rounds: 3, since_ms: 5_000, order: 2, bounded_since_ms: None, bounded_used_ms: 0 }));
        let mut waiting = Waiting::default();
        waiting.add(scope_of(&row), WaitKey::of(&row));
        // A message claiming a later (or any) bounded start is ignored: a queue
        // writer can't move or extend the bound.
        let forged = event("x", "e1", Some(Deferral { rounds: 3, since_ms: 1, order: 0, bounded_since_ms: Some(i64::MAX), bounded_used_ms: 0 }));
        let (bound, _) = bind_to_rows(&[forged.clone()], waiting.clone());
        let deferral = bound[0].deferral.unwrap();
        assert_eq!((deferral.since_ms, deferral.order), (5_000, 2), "the row's, not the message's");
        assert_eq!(deferral.bounded_since_ms, None, "no bounded start in its row yet");
        // The row's (written by a worker at its first bounded round) is the one.
        waiting.add_bounded("x".into(), Some(9_000), 0);
        assert_eq!(bind_to_rows(&[forged], waiting).0[0].deferral.unwrap().bounded_since_ms, Some(9_000));
    }

    #[test]
    fn a_deferral_keeps_when_and_where_it_first_waited() {
        let first = Deferral::next(None, 1_000, 3);
        assert_eq!(first, Deferral { rounds: 1, since_ms: 1_000, order: 3, bounded_since_ms: None, bounded_used_ms: 0 });
        let later = Deferral::next(Some(first), 99_000, 0);
        assert_eq!(later, Deferral { rounds: 2, since_ms: 1_000, order: 3, bounded_since_ms: None, bounded_used_ms: 0 });
        let last = Deferral::next(Some(Deferral { rounds: u32::MAX, ..first }), 0, 0);
        assert_eq!(last.rounds, u32::MAX);
    }

    #[test]
    fn an_event_waits_behind_earlier_ones_of_its_scope_only() {
        let early = Deferral { rounds: 4, since_ms: 1_000, order: 0, bounded_since_ms: None, bounded_used_ms: 0 };
        let late = Deferral { rounds: 1, since_ms: 5_000, order: 0, bounded_since_ms: None, bounded_used_ms: 0 };
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
            event("held", "e1", Some(Deferral { rounds: 2, since_ms: 1_000, order: 1, bounded_since_ms: None, bounded_used_ms: 0 })),
            event("new-2", "e2", None),
            event("first", "e1", Some(Deferral { rounds: 9, since_ms: 1_000, order: 0, bounded_since_ms: None, bounded_used_ms: 0 })),
            event("later", "e1", Some(Deferral { rounds: 1, since_ms: 2_000, order: 0, bounded_since_ms: None, bounded_used_ms: 0 })),
        ];
        let order: Vec<&str> = in_wait_order(&events)
            .into_iter()
            .map(|event| event.delivery_id.as_deref().unwrap())
            .collect();
        assert_eq!(order, vec!["first", "held", "later", "new-1", "new-2"]);
    }

    #[test]
    fn the_deferral_survives_the_queue_format() {
        let deferral = Deferral { rounds: 7, since_ms: 1_234, order: 2, bounded_since_ms: None, bounded_used_ms: 0 };
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
