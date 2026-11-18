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
//! - **Concurrency.** Ordering holds for one batch worker at a time per
//!   scope; with several concurrent batch workers, an event handed to a
//!   batch (no longer in the queue) is not seen by another batch, which may
//!   store a later event of its scope first. Statement times still say
//!   when each happened.
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
//!   *Availability (operators).* While a log has entries waiting for its
//!   signer (an outage of the key service), every action that must record a
//!   direct entry in that election event's log fails visibly: opening,
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
//! - **Not gated (exempt).** Appends that are not direct posts of an
//!   action keep their behaviour: the ballot box's sequencer
//!   (`ballot_box.rs`, its own durable queue, retried by its beat), the
//!   signing-log outbox (`signing/log.rs`) and the audit-log outbox
//!   (`audit_log_writer.rs`), each an outbox retried until stored; sealed
//!   ballot boxes (`ceremonies/sealed_box_ballots.rs`, a ceremony that
//!   fails and retries); a log's import from a CSV backup (`import_from_csv`,
//!   records already signed); and logs made from a signer alone
//!   (`ElectoralLog::for_*_with_signer`, `for_*_with_signing_key`: the batch
//!   task, the sequencer, tenant logs). Whether any of these must also wait
//!   behind a scope's waiting entries is an open owner question.
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
    /// When each waiting entry is due, in milliseconds, by delivery ID, with its scope.
    due_by_delivery: HashMap<String, (String, i64)>,
}

impl Waiting {
    pub fn add(&mut self, scope: String, key: WaitKey) {
        self.by_scope.entry(scope).or_default().push(key);
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
    let bound = events
        .iter()
        .map(|event| {
            let mut event = event.clone();
            let row = event.delivery_id.as_ref().and_then(|id| rows.get(id));
            event.deferral = row.map(|key| Deferral {
                rounds: event.deferral.map_or(0, |deferral| deferral.rounds),
                since_ms: key.since_ms,
                order: key.order,
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
                &[&format!("electoral-log-scope:{scope}")],
            )
            .await
            .context("Error locking an electoral-log scope")?;
    }
    Ok(())
}

/// How long a direct post waits for its scope's lock before it is refused.
pub const GATE_LOCK_TIMEOUT: &str = "5s";

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
                 (extract(epoch FROM due_at) * 1000)::bigint \
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
        .filter(|item| item.reason == WaitReason::SignerUnavailable)
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
            "the electoral log of {} has {} entries waiting for its signer; \
             this entry is refused until they are stored (the log keeps its order)",
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
    client
        .query(
            "SELECT pg_advisory_xact_lock_shared(hashtextextended($1, 0))",
            &[&format!("electoral-log-scope:{scope}")],
        )
        .await
        .context("The electoral-log scope is busy: the entry is refused")?;
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
        if item.reason == WaitReason::SignerUnavailable {
            let due = now_ms.saturating_add(i64::try_from(delay.as_millis()).unwrap_or(i64::MAX));
            let scope_due = due_by_scope.entry(scope_of(&event)).or_insert(due);
            *scope_due = (*scope_due).min(due);
        }
        let deferral = event.deferral.unwrap_or(Deferral {
            rounds: 1,
            since_ms: now_ms,
            order,
        });
        planned.push((item.reason, event, deferral, delay));
    }
    planned
        .into_iter()
        .map(|(reason, event, deferral, delay)| {
            let delay = match reason {
                WaitReason::SignerUnavailable => delay,
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
    let mark = format!(
        "INSERT INTO {WAITING_TABLE} (delivery_id, scope, since_ms, ord, due_at) \
         VALUES ($1, $2, $3, $4, clock_timestamp() + make_interval(secs => $5::integer)) \
         ON CONFLICT (delivery_id) DO UPDATE SET due_at = EXCLUDED.due_at"
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
