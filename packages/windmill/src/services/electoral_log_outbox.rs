// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The transactional outbox of electoral-log entries an action records in its
//! own (Hasura) transaction.
//!
//! **Writing.** An action that must be recorded with its administrator's own
//! signature (a certificate-authority import or deletion) first takes its log
//! scope's Hasura lock H(S) exclusive ([`lock_scope`]), then signs the entry
//! (the administrator's vault key, at that moment) and writes it to
//! `sequent_backend.electoral_log_outbox`, with the announcement of the
//! administrator's key signed by the tenant log's system key; it keeps H(S)
//! until it commits. If it rolls back, neither exists; if it commits, both do.
//! Nothing depends on timing.
//!
//! **Order (P2).** A direct post to scope S ([`outbox_gate`], from
//! `append_gated`, on its own Hasura connection and transaction, never its
//! caller's) takes H(S) shared with a lock timeout, and is refused while S has
//! an outbox row neither relayed nor cancelled; only then does it take the
//! task-queue gate (Q(S) shared, and the waiting table), and it keeps both
//! until its append is done. So a direct post either comes before a certificate
//! action's entry (the action waits for it to append) or is refused until that
//! entry is stored. The relay commits the entry's waiting row before it marks
//! the outbox row relayed, so there is no moment with neither. Lock order: a
//! direct post H(S) shared, then Q(S) shared, then the board; a certificate
//! action H(S) exclusive only; the relay its own relay lock, an outbox row, then
//! Q(S) exclusive, never H; the batch task Q only. No cycle.
//!
//! **Relaying** ([`relay_electoral_log_outbox`], with beat, after the audit-log
//! outbox and independently of it): one relay at a time (a try-lock; another
//! returns at once); for each scope its oldest row only (the head), each in its
//! own transaction; a row that fails holds its scope's later rows, never
//! another scope's. For each head it forwards the announcement to the tenant's
//! log (checked: an `AdminPublicKey` statement of that tenant, user and key,
//! signed by the tenant log's system key; the board name never comes from the
//! row; idempotent by delivery ID), writes the entry's waiting row and its
//! reference (task-queue database; nothing added if already there), then marks
//! the row relayed. It reads no private key.
//!
//! **Integrity.** Every row carries a MAC (HMAC-SHA256 under the tenant's
//! pending-statement vault key) over its delivery ID and its statement,
//! written in the action's own transaction: an administrator-signed row too,
//! so a copy under another delivery ID, or a changed payload, never verifies.
//! When a row was written is the database's (a trigger sets `created_at` and
//! keeps it), and the batch task checks the statement's time against it. An
//! administrator's signature may be in one row only (a unique index): a
//! second row with the same signed statement is refused at insert.
//!
//! **Dead letters.** A row that can never be relayed as it is (unreadable, a
//! MAC that doesn't verify, a missing or wrong announcement, a statement of
//! another scope than its row) is dead-lettered at once: kept, alerted on
//! (`electoral-log-outbox-dead-letter`), and no longer heading its scope, so
//! its log's later rows go on. An operator replays it once fixed
//! ([`replay_outbox_entry`], `windmill replay-electoral-log-outbox-entry`) or
//! cancels it. A replayed entry is stored after those relayed meanwhile.
//! Any other failure (a database, the vault, the board) holds the scope and is
//! tried again, with `electoral-log-outbox-stuck` after a few attempts.
//!
//! Crash points: before the waiting row commits, the row is relayed again
//! later; after it, before the outbox row is marked, the next pass finds the
//! waiting row and adds nothing (or, the entry having been stored, writes it
//! again and the log stores nothing twice); a forwarded announcement is stored
//! once (delivery ID).

use crate::services::electoral_log_deferral::{scope_key, ScopeWaiting};
use crate::services::electoral_log_pending::{PendingStatement, PendingStatementBody};
use anyhow::{anyhow, ensure, Context, Result};
use deadpool_postgres::{Client, Transaction};
use electoral_log::messages::message::{Message, SigningData};
use electoral_log::messages::newtypes::{PublicKeyDerB64, TenantIdString};
use electoral_log::messages::statement::StatementBody;
use electoral_log::{ElectoralLogMessage, LogEntry};
use tokio_postgres::GenericClient;
use tracing::{error, info, warn};
use voting_crypto::signing::VerificationKey;
use voting_crypto::{RecordDeserialize, RecordSerialize};

/// Attempts after which a failing row is reported as an error.
const ALERT_ATTEMPTS: i32 = 5;
/// Rows one relay pass moves at most.
const RELAY_PASS_LIMIT: usize = 200;
/// Alert class of an outbox row that keeps failing to be relayed.
pub const ALERT_OUTBOX_STUCK: &str = "electoral-log-outbox-stuck";
/// How long a direct post waits for its scope's Hasura lock before it is refused.
pub const OUTBOX_GATE_LOCK_TIMEOUT: &str = "5s";
/// Alert class of an outbox row that can never be relayed as it is.
pub const ALERT_OUTBOX_DEAD_LETTER: &str = "electoral-log-outbox-dead-letter";
/// How long a certificate action waits, after it commits, for the relay.
pub const RELAY_KICK_WAIT: std::time::Duration = std::time::Duration::from_secs(5);

/// A failure no retry can mend: the row is dead-lettered, not retried.
#[derive(Debug)]
pub struct PermanentRelayFailure(pub String);

impl std::fmt::Display for PermanentRelayFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for PermanentRelayFailure {}

/// `error` as a permanent relay failure.
fn permanent(error: anyhow::Error) -> anyhow::Error {
    anyhow::Error::new(PermanentRelayFailure(format!("{error:#}")))
}

/// Whether `error` is, or was caused by, a [`PermanentRelayFailure`].
pub fn is_permanent_relay_failure(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| cause.is::<PermanentRelayFailure>())
}

/// Takes the log scope's lock H(S) in the action database's transaction:
/// exclusive for a certificate action, shared for a direct post.
pub async fn lock_scope<C: GenericClient + Sync>(client: &C, scope: &str, exclusive: bool) -> Result<()> {
    let sql = if exclusive {
        "SELECT pg_advisory_xact_lock(hashtextextended($1, 0))"
    } else {
        "SELECT pg_advisory_xact_lock_shared(hashtextextended($1, 0))"
    };
    match client
        .query(sql, &[&format!("electoral-log-scope:{scope}")])
        .await
    {
        Ok(_) => Ok(()),
        // Not had within the transaction's lock_timeout: a refusal for the
        // log's order (an action is writing to its outbox), not a failure.
        Err(error)
            if error.code() == Some(&tokio_postgres::error::SqlState::LOCK_NOT_AVAILABLE) =>
        {
            Err(anyhow::Error::new(crate::services::electoral_log_deferral::ScopeBusy {
                scope: scope.to_string(),
                what: "an action is writing to its outbox",
            }))
        }
        Err(error) => Err(anyhow::Error::new(error).context("The electoral log's scope is busy")),
    }
}

/// The direct-post gate's half in the action database: takes H(S) shared (a
/// refusal after [`OUTBOX_GATE_LOCK_TIMEOUT`]) and refuses while S has an outbox
/// row neither relayed nor cancelled. Call it in a transaction of its own, kept
/// open until the post's append is done.
pub async fn outbox_gate<C: GenericClient + Sync>(client: &C, scope: &str) -> Result<()> {
    let (tenant_id, election_event_id) = scope
        .split_once('/')
        .ok_or_else(|| anyhow!("Not a log scope: {scope}"))?;
    client
        .batch_execute(&format!("SET LOCAL lock_timeout = '{OUTBOX_GATE_LOCK_TIMEOUT}'"))
        .await
        .context("Error setting the scope lock's timeout")?;
    lock_scope(client, scope, false).await?;
    if election_event_id.is_empty() {
        return Ok(());
    }
    let waiting: i64 = client
        .query_one(
            "SELECT count(*) FROM sequent_backend.electoral_log_outbox \
             WHERE tenant_id = $1::text::uuid AND election_event_id = $2::text::uuid \
             AND relayed_at IS NULL AND cancelled_at IS NULL AND dead_lettered_at IS NULL",
            &[&tenant_id, &election_event_id],
        )
        .await
        .context("Error reading the electoral-log outbox")?
        .get(0);
    if waiting > 0 {
        return Err(anyhow::Error::new(ScopeWaiting {
            scope: scope.to_string(),
            waiting,
        }));
    }
    Ok(())
}

/// The unrelayed, uncancelled outbox rows of these scopes, for the batch task:
/// its events wait behind them. Read without a lock. (delivery ID, scope,
/// created in ms, seq.)
pub async fn unrelayed_rows<C: GenericClient + Sync>(
    client: &C,
    scopes: &[String],
) -> Result<Vec<(String, String, i64, i64)>> {
    if scopes.is_empty() {
        return Ok(vec![]);
    }
    Ok(client
        .query(
            "SELECT delivery_id, tenant_id::text || '/' || election_event_id::text, \
             (extract(epoch FROM created_at) * 1000)::bigint, seq \
             FROM sequent_backend.electoral_log_outbox \
             WHERE relayed_at IS NULL AND cancelled_at IS NULL AND dead_lettered_at IS NULL \
             AND tenant_id::text || '/' || election_event_id::text = ANY($1)",
            &[&scopes],
        )
        .await
        .context("Error reading the electoral-log outbox")?
        .iter()
        .map(|row| (row.get(0), row.get(1), row.get(2), row.get(3)))
        .collect())
}

/// The announcement of an administrator's key, signed with the tenant log's
/// system key, as `ElectoralLog::post_admin_pk` makes it: its record bytes.
pub fn admin_key_announcement(
    tenant_log_key: &voting_crypto::signing::SigningKey,
    tenant_id: &str,
    user_id: &str,
    username: Option<String>,
    public_key: &str,
) -> Result<Vec<u8>> {
    let sd = SigningData::new(tenant_log_key.clone(), "", tenant_log_key.clone());
    let message = Message::admin_public_key_message(
        TenantIdString(tenant_id.to_string()),
        Some(user_id.to_string()),
        username,
        PublicKeyDerB64(public_key.to_string()),
        &sd,
        None,
        None,
    )?;
    Ok(message.record_serialize()?)
}

/// Checks a forwarded announcement: an `AdminPublicKey` statement of exactly
/// this tenant, user and key, whose signatures verify with the tenant log's
/// system key. Returns the message.
pub fn checked_announcement(
    bytes: &[u8],
    tenant_id: &str,
    user_id: &str,
    public_key: &str,
    system_pk: &VerificationKey,
) -> Result<Message> {
    let message = Message::record_deserialize(bytes).context("The announcement is unreadable")?;
    message
        .verify(system_pk)
        .context("The announcement is not signed by the tenant log's system key")?;
    let announces = matches!(
        &message.statement.body,
        StatementBody::AdminPublicKey(tenant, Some(user), key)
            if tenant.0 == tenant_id && user == user_id && key.0 == public_key
    );
    ensure!(announces, "The announcement is not of this entry's tenant, user and key");
    Ok(message)
}

/// Writes an administrator-signed entry, and its key's announcement, to the
/// outbox in the action's own transaction, which must hold the scope's lock
/// exclusive, with its MAC under the tenant's pending-statement key (made the
/// first time, in this transaction) over its delivery ID and statement. A
/// second row of the same signed statement is refused (unique index).
/// Returns its delivery ID.
pub async fn write_outbox_entry(
    hasura_transaction: &Transaction<'_>,
    statement: &PendingStatement,
    announcement: &[u8],
) -> Result<String> {
    ensure!(
        statement.statement.admin().is_some(),
        "Only administrator-signed entries go through this outbox path"
    );
    let delivery_id = statement.origin.delivery_id();
    let payload = serde_json::to_value(statement).context("Error serializing an outbox entry")?;
    let key = crate::services::vault::get_or_create_pending_mac_key(hasura_transaction, &statement.tenant_id)
        .await?;
    let mac = crate::services::electoral_log_pending::pending_mac(&key, &delivery_id, statement)?;
    hasura_transaction
        .execute(
            "INSERT INTO sequent_backend.electoral_log_outbox \
             (delivery_id, tenant_id, election_event_id, payload, announcement, mac) \
             VALUES ($1, $2::text::uuid, $3::text::uuid, $4, $5, $6)",
            &[
                &delivery_id,
                &statement.tenant_id,
                &statement.election_event_id,
                &payload,
                &announcement,
                &mac,
            ],
        )
        .await
        .context("Error writing the electoral-log outbox entry")?;
    Ok(delivery_id)
}

/// One outbox row, as the relay reads it.
struct OutboxRow {
    delivery_id: String,
    seq: i64,
    payload: serde_json::Value,
    created_ms: i64,
    announcement: Option<Vec<u8>>,
    mac: Option<Vec<u8>>,
    /// The row's own scope (what gates its log).
    tenant_id: String,
    election_event_id: String,
}

/// Moves committed outbox rows to the electoral log, one scope head at a time;
/// see the module documentation. Returns how many were relayed. `lock_client`
/// holds the relay's lock for the pass; `client` does the work.
pub async fn relay_electoral_log_outbox(lock_client: &mut Client, client: &mut Client) -> Result<usize> {
    let lock = lock_client
        .transaction()
        .await
        .context("Error starting the outbox relay")?;
    let mine: bool = lock
        .query_one(
            "SELECT pg_try_advisory_xact_lock(hashtextextended('electoral-log-outbox-relay', 0))",
            &[],
        )
        .await
        .context("Error taking the outbox relay's lock")?
        .get(0);
    if !mine {
        // Another relay is running; it keeps the order.
        return Ok(0);
    }
    let mut relayed = 0;
    let mut held: std::collections::HashSet<String> = std::collections::HashSet::new();
    while relayed < RELAY_PASS_LIMIT {
        let heads = scope_heads(&***client).await?;
        let heads: Vec<(String, String)> = heads
            .into_iter()
            .filter(|(_, scope)| !held.contains(scope))
            .collect();
        if heads.is_empty() {
            break;
        }
        for (delivery_id, scope) in heads {
            match relay_row(client, &delivery_id).await {
                Ok(true) => relayed += 1,
                Ok(false) => {}
                // Never relayable as it is: set aside, its scope goes on.
                Err(failure) if is_permanent_relay_failure(&failure) => {
                    dead_letter(client, &delivery_id, &failure).await?;
                }
                // Refused for a log's order (its tenant log has entries waiting
                // or a batch is processing it): not a failure of the row; its
                // scope waits for the next pass, no attempt is counted.
                Err(failure) if crate::services::electoral_log_deferral::is_scope_gated(&failure) => {
                    info!(delivery_id, "The outbox relay waits for a log's order: {failure:#}");
                    held.insert(scope);
                }
                Err(failure) => {
                    held.insert(scope);
                    record_failure(client, &delivery_id, &failure).await?;
                }
            }
        }
    }
    lock.commit().await.context("Error ending the outbox relay")?;
    Ok(relayed)
}

/// The oldest unrelayed, uncancelled, not dead-lettered row of each scope,
/// oldest first.
async fn scope_heads<C: GenericClient + Sync>(client: &C) -> Result<Vec<(String, String)>> {
    let mut heads: Vec<(i64, String, String)> = client
        .query(
            "SELECT DISTINCT ON (tenant_id, election_event_id) delivery_id, seq, \
             tenant_id::text || '/' || election_event_id::text \
             FROM sequent_backend.electoral_log_outbox \
             WHERE relayed_at IS NULL AND cancelled_at IS NULL AND dead_lettered_at IS NULL \
             ORDER BY tenant_id, election_event_id, seq",
            &[],
        )
        .await
        .context("Error reading the electoral-log outbox")?
        .iter()
        .map(|row| (row.get(1), row.get(0), row.get(2)))
        .collect();
    heads.sort();
    Ok(heads
        .into_iter()
        .map(|(_, delivery_id, scope)| (delivery_id, scope))
        .collect())
}

/// Relays one row in its own transaction. `false`: it was relayed or
/// cancelled meanwhile.
async fn relay_row(client: &mut Client, delivery_id: &str) -> Result<bool> {
    let transaction = client
        .transaction()
        .await
        .context("Error starting to relay an outbox entry")?;
    let Some(row) = transaction
        .query_opt(
            "SELECT delivery_id, seq, payload, (extract(epoch FROM created_at) * 1000)::bigint, \
             announcement, mac, tenant_id::text, election_event_id::text \
             FROM sequent_backend.electoral_log_outbox \
             WHERE delivery_id = $1 AND relayed_at IS NULL AND cancelled_at IS NULL \
             AND dead_lettered_at IS NULL FOR UPDATE",
            &[&delivery_id],
        )
        .await
        .context("Error reading an outbox entry")?
    else {
        return Ok(false);
    };
    let row = OutboxRow {
        delivery_id: row.get(0),
        seq: row.get(1),
        payload: row.get(2),
        created_ms: row.get(3),
        announcement: row.get(4),
        mac: row.get(5),
        tenant_id: row.get(6),
        election_event_id: row.get(7),
    };
    let statement: PendingStatement = serde_json::from_value(row.payload.clone())
        .context("Not a pending statement")
        .map_err(permanent)?;
    // The scope the row gates is the statement's own.
    if statement.tenant_id != row.tenant_id || statement.election_event_id != row.election_event_id {
        return Err(permanent(anyhow!(
            "The outbox row's statement is of another scope than its row"
        )));
    }
    // Every row (system- or administrator-signed): its MAC under the tenant's
    // vault key, over its delivery ID and statement, or it is never relayed
    // (nor signed). A missing key is permanent too: only a restored vault
    // mends it (then the row is replayed).
    let key = crate::services::vault::pending_mac_key(&transaction, &statement.tenant_id)
        .await?
        .ok_or_else(|| permanent(anyhow!("The tenant has no pending-statement MAC key")))?;
    crate::services::electoral_log_pending::verify_pending_mac(
        &key,
        &row.delivery_id,
        &statement,
        row.mac.as_deref(),
    )
    .map_err(permanent)?;
    if statement.statement.admin().is_some() {
        forward_announcement(&statement, row.announcement.as_deref()).await?;
    }
    // The waiting row commits (task-queue database) before this row is marked.
    statement
        .write_waiting(
            &row.delivery_id,
            Some((row.created_ms, row.seq)),
            row.mac.as_deref(),
        )
        .await?;
    transaction
        .execute(
            "UPDATE sequent_backend.electoral_log_outbox \
             SET relayed_at = clock_timestamp() WHERE delivery_id = $1",
            &[&row.delivery_id],
        )
        .await
        .context("Error marking an outbox entry relayed")?;
    transaction
        .commit()
        .await
        .context("Error committing an outbox entry's relay")?;
    Ok(true)
}

/// Forwards an administrator-signed entry's key announcement to the tenant's
/// log, checked ([`checked_announcement`]); the tenant log (board and system
/// key) comes from the tenant, never from the row.
async fn forward_announcement(
    statement: &PendingStatement,
    announcement: Option<&[u8]>,
) -> Result<()> {
    let Some((user_id, public_key)) = statement.statement.admin() else {
        return Ok(());
    };
    let announcement = announcement
        .ok_or_else(|| permanent(anyhow!("An administrator-signed entry without its announcement")))?;
    // The tenant log is prepared in a short transaction of its own, committed before
    // the announcement is admitted: the relay never holds the tenant-log lock while it
    // waits at its log's gate (a batch holding the tenant scope's lease may need that
    // lock: holding it here could make them wait on each other across two databases).
    let tenant_log = {
        let mut client = crate::services::database::get_hasura_pool()
            .await
            .get()
            .await
            .context("Error connecting to the action database")?;
        let tx = client
            .transaction()
            .await
            .context("Error starting to prepare the tenant log")?;
        let tenant_log =
            crate::services::electoral_log_board::ensure_tenant_log(&tx, &statement.tenant_id)
                .await
                .context("Error getting the tenant log")?;
        tx.commit().await.context("Error keeping the tenant log")?;
        tenant_log
    };
    let system_pk = VerificationKey::from_sk(&tenant_log.system_key)?;
    let message = checked_announcement(
        announcement,
        &statement.tenant_id,
        user_id,
        public_key,
        &system_pk,
    )
    .map_err(permanent)?;
    let entry = LogEntry {
        delivery_id: crate::services::electoral_log::admin_pk_delivery_id(user_id, public_key),
        message: ElectoralLogMessage::try_from(&message).map_err(permanent)?,
    };
    // Admitted for the tenant log's order (the announcement is already signed:
    // nothing is signed here). Released before the relay makes its entry wait
    // (`write_waiting`), so the relay never holds a tenant guard while it takes
    // a scope lock exclusive. Refused: the row stays for the next pass, not a failure.
    let scope = crate::services::electoral_log_deferral::scope_key(&statement.tenant_id, "");
    let admission = crate::services::electoral_log::ScopeAdmission::admit(&scope).await?;
    let forwarded = crate::services::electoral_log_board::get_board_client()
        .await?
        .append(&tenant_log.board, &[entry])
        .await
        .context("Error forwarding the administrator's key announcement");
    admission.end().await;
    forwarded?;
    Ok(())
}

async fn record_failure(client: &mut Client, delivery_id: &str, failure: &anyhow::Error) -> Result<()> {
    let text = format!("{failure:#}");
    let attempts: i32 = client
        .query_one(
            "UPDATE sequent_backend.electoral_log_outbox \
             SET attempts = attempts + 1, last_error = $2 WHERE delivery_id = $1 \
             RETURNING attempts",
            &[&delivery_id, &text],
        )
        .await
        .context("Error recording an outbox entry's failure")?
        .get(0);
    if attempts >= ALERT_ATTEMPTS {
        error!(
            alert = ALERT_OUTBOX_STUCK,
            delivery_id,
            attempts,
            "An electoral-log outbox entry keeps failing to be relayed; its log's later entries wait: {text}"
        );
    } else {
        warn!(delivery_id, "Error relaying an electoral-log outbox entry: {text}");
    }
    Ok(())
}

/// Sets a row that can never be relayed as it is aside: kept, alerted on, no
/// longer heading its scope.
async fn dead_letter(client: &mut Client, delivery_id: &str, failure: &anyhow::Error) -> Result<()> {
    let text = format!("{failure:#}");
    client
        .execute(
            "UPDATE sequent_backend.electoral_log_outbox \
             SET dead_lettered_at = clock_timestamp(), dead_letter_reason = $2, \
             attempts = attempts + 1, last_error = $2 \
             WHERE delivery_id = $1 AND relayed_at IS NULL AND cancelled_at IS NULL",
            &[&delivery_id, &text],
        )
        .await
        .context("Error dead-lettering an outbox entry")?;
    error!(
        alert = ALERT_OUTBOX_DEAD_LETTER,
        delivery_id,
        "An electoral-log outbox entry can never be relayed as it is: dead-lettered (kept; \
         replay it once fixed, or cancel it); its log's later entries go on: {text}"
    );
    Ok(())
}

/// Operator: puts a dead-lettered outbox row back in its scope, to be relayed
/// again (after its cause was fixed: a restored vault key, say). Who and why
/// are kept in its last error. Returns whether a dead-lettered row was found.
pub async fn replay_outbox_entry<C: GenericClient + Sync>(
    client: &C,
    delivery_id: &str,
    by: &str,
    reason: &str,
) -> Result<bool> {
    ensure!(!by.trim().is_empty(), "Name who replays the entry");
    ensure!(!reason.trim().is_empty(), "Say why the entry is replayed");
    let replayed = client
        .execute(
            "UPDATE sequent_backend.electoral_log_outbox \
             SET dead_lettered_at = NULL, attempts = 0, \
             last_error = 'replayed by ' || $2 || ': ' || $3 || ' (was: ' || \
             COALESCE(dead_letter_reason, '') || ')', dead_letter_reason = NULL \
             WHERE delivery_id = $1 AND dead_lettered_at IS NOT NULL \
             AND relayed_at IS NULL AND cancelled_at IS NULL",
            &[&delivery_id, &by, &reason],
        )
        .await
        .context("Error replaying an outbox entry")?;
    if replayed > 0 {
        warn!(
            delivery_id,
            requested_by = by,
            reason,
            "A dead-lettered electoral-log outbox entry was replayed by an operator"
        );
    }
    Ok(replayed > 0)
}

/// [`replay_outbox_entry`] on the action database (the CLI).
pub async fn replay_electoral_log_outbox_entry(delivery_id: &str, by: &str, reason: &str) -> Result<bool> {
    let client = crate::services::database::get_hasura_pool()
        .await
        .get()
        .await
        .context("Error connecting to the action database")?;
    replay_outbox_entry(&**client, delivery_id, by, reason).await
}

/// After a certificate action commits: relays the outbox now, waiting at most
/// [`RELAY_KICK_WAIT`], then asks for a batch dispatch now. Best effort: beat
/// does both every few seconds anyway. While its entry is not stored, the
/// log's direct posts are refused with "the electoral log of <tenant>/<event>
/// has N entries waiting for its signer; ..." ([`ScopeWaiting`]): the gate is
/// not changed here, only the wait shortened. Returns how many rows it relayed
/// (None: it failed or timed out).
pub async fn relay_now() -> Option<usize> {
    kick_relay(
        async {
            let pool = crate::services::database::get_hasura_pool().await;
            let mut lock_client = pool.get().await.context("Error connecting to the action database")?;
            let mut work_client = pool.get().await.context("Error connecting to the action database")?;
            relay_electoral_log_outbox(&mut lock_client, &mut work_client).await
        },
        || {
            crate::tasks::electoral_log::kick_electoral_log_batch_dispatcher();
        },
        RELAY_KICK_WAIT,
    )
    .await
}

/// Runs `relay` for at most `wait`, then `dispatch`, whatever the relay did
/// (another relay may hold the lock and relay the entry itself).
async fn kick_relay<F>(relay: F, dispatch: impl FnOnce(), wait: std::time::Duration) -> Option<usize>
where
    F: std::future::Future<Output = Result<usize>>,
{
    let relayed = match tokio::time::timeout(wait, relay).await {
        Ok(Ok(relayed)) => Some(relayed),
        Ok(Err(error)) => {
            warn!("Error relaying the electoral-log outbox after an action: {error:#}");
            None
        }
        Err(_) => {
            warn!("Relaying the electoral-log outbox after an action timed out; beat relays it");
            None
        }
    };
    dispatch();
    relayed
}

/// Operator: cancels an entry that can't be signed and holds its log, wherever
/// it is: an outbox row not relayed yet (marked cancelled, with who and why)
/// and its waiting row, if relayed (removed; its reference is then
/// dead-lettered with its statement). The entry is lost for the log: recorded
/// as an error under `electoral-log-entry-cancelled`. Returns whether anything
/// was cancelled.
pub async fn cancel_electoral_log_entry(delivery_id: &str, by: &str, reason: &str) -> Result<bool> {
    ensure!(!by.trim().is_empty(), "Name who cancels the entry");
    ensure!(!reason.trim().is_empty(), "Say why the entry is cancelled");
    let client = crate::services::database::get_hasura_pool()
        .await
        .get()
        .await
        .context("Error connecting to the action database")?;
    let outbox = client
        .execute(
            "UPDATE sequent_backend.electoral_log_outbox \
             SET cancelled_at = clock_timestamp(), cancelled_by = $2, cancel_reason = $3 \
             WHERE delivery_id = $1 AND relayed_at IS NULL AND cancelled_at IS NULL",
            &[&delivery_id, &by, &reason],
        )
        .await
        .context("Error cancelling an outbox entry")?;
    if outbox > 0 {
        error!(
            alert = crate::services::electoral_log_pending::ALERT_ENTRY_CANCELLED,
            delivery_id,
            requested_by = by,
            reason,
            "An electoral-log outbox entry was cancelled by an operator"
        );
    }
    let waiting =
        crate::services::electoral_log_pending::cancel_waiting_entry(delivery_id, by, reason).await?;
    Ok(outbox > 0 || waiting)
}

/// The scope key of an outbox row, as the waiting table's.
pub fn outbox_scope(statement: &PendingStatement) -> String {
    scope_key(&statement.tenant_id, &statement.election_event_id)
}

/// The statement body kinds the outbox carries here.
pub fn is_admin_signed(statement: &PendingStatementBody) -> bool {
    statement.admin().is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use voting_crypto::signing::SigningKey;

    const TENANT: &str = "90505c8a-23a9-4cdf-a26b-4e19f6a097d5";

    #[test]
    fn an_announcement_is_forwarded_only_for_its_tenant_user_and_key_and_signer() {
        let system = SigningKey::generate().unwrap();
        let other = SigningKey::generate().unwrap();
        let system_pk = VerificationKey::from_sk(&system).unwrap();
        let bytes = admin_key_announcement(&system, TENANT, "admin-id", None, "QUJD").unwrap();
        checked_announcement(&bytes, TENANT, "admin-id", "QUJD", &system_pk).unwrap();
        // Another user, key or tenant than the entry's.
        assert!(checked_announcement(&bytes, TENANT, "other", "QUJD", &system_pk).is_err());
        assert!(checked_announcement(&bytes, TENANT, "admin-id", "WFla", &system_pk).is_err());
        assert!(checked_announcement(&bytes, "another-tenant", "admin-id", "QUJD", &system_pk).is_err());
        // Signed by another key than the tenant log's.
        let foreign = admin_key_announcement(&other, TENANT, "admin-id", None, "QUJD").unwrap();
        assert!(checked_announcement(&foreign, TENANT, "admin-id", "QUJD", &system_pk).is_err());
        assert!(checked_announcement(b"junk", TENANT, "admin-id", "QUJD", &system_pk).is_err());
    }

    #[tokio::test]
    async fn a_kick_waits_for_the_relay_at_most_its_bound_then_dispatches() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let dispatched = AtomicUsize::new(0);
        let relayed = kick_relay(async { Ok(2) }, || {
            dispatched.fetch_add(1, Ordering::SeqCst);
        }, std::time::Duration::from_secs(1))
        .await;
        assert_eq!(relayed, Some(2));
        // A relay slower than the bound: the action goes on, the batch is
        // still dispatched.
        let started = std::time::Instant::now();
        let slow = kick_relay(
            async {
                tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                Ok(1)
            },
            || {
                dispatched.fetch_add(1, Ordering::SeqCst);
            },
            std::time::Duration::from_millis(100),
        )
        .await;
        assert_eq!(slow, None);
        assert!(started.elapsed() < std::time::Duration::from_secs(5));
        let failed = kick_relay(async { Err(anyhow!("down")) }, || {
            dispatched.fetch_add(1, Ordering::SeqCst);
        }, std::time::Duration::from_secs(1))
        .await;
        assert_eq!(failed, None);
        assert_eq!(dispatched.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn permanent_failures_are_told_from_the_rest_through_context() {
        let failure = permanent(anyhow!("The pending statement's MAC doesn't verify"))
            .context("relaying");
        assert!(is_permanent_relay_failure(&failure));
        assert!(!is_permanent_relay_failure(&anyhow!("connection reset")));
    }

    #[test]
    fn the_relay_reads_no_private_key() {
        // Source check: the relay's path never reads an administrator's vault
        // key (the announcement travels signed with the entry).
        let source = include_str!("electoral_log_outbox.rs");
        let code = source.split("#[cfg(test)]").next().unwrap();
        for forbidden in ["read_admin_user_signing_key", "get_admin_user_signing_key", "admin_user_signing_key"] {
            assert!(!code.contains(forbidden), "the relay must not use {forbidden}");
        }
    }
}

/// Integration tests against an action database with the migrations applied
/// (`HASURA_TEST_DATABASE_URL`, disposable): `cargo test -p windmill -- --ignored
/// electoral_log_outbox`.
#[cfg(test)]
mod db_tests {
    use super::*;
    use crate::services::electoral_log_deferral::is_scope_waiting;
    use electoral_log::messages::newtypes::CertificateAuthEventAction;
    use std::sync::Arc;
    use std::time::Duration;
    use voting_crypto::signing::SigningKey;

    fn pool() -> Arc<deadpool_postgres::Pool> {
        let config = deadpool_postgres::Config {
            url: Some(std::env::var("HASURA_TEST_DATABASE_URL").expect("HASURA_TEST_DATABASE_URL")),
            ..Default::default()
        };
        Arc::new(
            config
                .create_pool(Some(deadpool_postgres::Runtime::Tokio1), tokio_postgres::NoTls)
                .unwrap(),
        )
    }

    /// A fresh scope and an administrator-signed entry of it.
    fn entry() -> (String, PendingStatement) {
        let tenant = uuid::Uuid::new_v4().to_string();
        let event = uuid::Uuid::new_v4().to_string();
        let statement = crate::services::electoral_log::admin_signed_certificate_event(
            &SigningKey::generate().unwrap(),
            &tenant,
            &event,
            CertificateAuthEventAction::Import,
            vec!["CN=one".into()],
            "admin-id",
            None,
        )
        .unwrap();
        (scope_key(&tenant, &event), statement)
    }

    /// Another administrator-signed entry of `statement`'s scope.
    fn another(statement: &PendingStatement) -> PendingStatement {
        crate::services::electoral_log::admin_signed_certificate_event(
            &SigningKey::generate().unwrap(),
            &statement.tenant_id,
            &statement.election_event_id,
            CertificateAuthEventAction::Import,
            vec!["CN=another".into()],
            "admin-id",
            None,
        )
        .unwrap()
    }

    /// As the action writes it (its tenant made first: the vault's MAC key
    /// references it).
    async fn write(tx: &Transaction<'_>, statement: &PendingStatement, announcement: &[u8]) -> String {
        ensure_tenant(&**tx, &statement.tenant_id).await;
        write_outbox_entry(tx, statement, announcement).await.unwrap()
    }

    async fn gate(pool: &deadpool_postgres::Pool, scope: &str) -> Result<()> {
        let mut client = pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        let result = outbox_gate(&*tx, scope).await;
        tx.rollback().await.unwrap();
        result
    }

    #[tokio::test]
    #[ignore = "requires HASURA_TEST_DATABASE_URL (a disposable migrated database) and the vault's settings"]
    async fn a_rolled_back_action_leaves_no_entry_and_posts_pass() {
        let pool = pool();
        let (scope, statement) = entry();
        let mut client = pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        lock_scope(&*tx, &scope, true).await.unwrap();
        write(&tx, &statement, b"announcement").await;
        tx.rollback().await.unwrap();
        gate(&pool, &scope).await.unwrap();
    }

    #[tokio::test]
    #[ignore = "requires HASURA_TEST_DATABASE_URL (a disposable migrated database) and the vault's settings"]
    async fn a_post_during_an_uncommitted_action_waits_then_is_refused_until_relayed_or_cancelled() {
        let pool = pool();
        let (scope, statement) = entry();
        let mut action = pool.get().await.unwrap();
        let tx = action.transaction().await.unwrap();
        lock_scope(&*tx, &scope, true).await.unwrap();
        let delivery_id = write(&tx, &statement, b"announcement").await;
        // A post now waits for the action's lock...
        let post_pool = pool.clone();
        let post_scope = scope.clone();
        let post = tokio::spawn(async move { gate(&post_pool, &post_scope).await });
        tokio::time::sleep(Duration::from_millis(500)).await;
        assert!(!post.is_finished(), "the post waits for the action");
        // ...and, the action committed, is refused: its entry isn't relayed.
        tx.commit().await.unwrap();
        let refused = post.await.unwrap().unwrap_err();
        assert!(is_scope_waiting(&refused), "{refused:#}");
        // Still refused after the commit, before the relay.
        assert!(is_scope_waiting(&gate(&pool, &scope).await.unwrap_err()));
        // Relayed (or cancelled), the outbox no longer holds the log.
        let client = pool.get().await.unwrap();
        client
            .execute(
                "UPDATE sequent_backend.electoral_log_outbox SET cancelled_at = now(), \
                 cancelled_by = 'test', cancel_reason = 'test' WHERE delivery_id = $1",
                &[&delivery_id],
            )
            .await
            .unwrap();
        gate(&pool, &scope).await.unwrap();
        let recorded: Option<String> = client
            .query_one(
                "SELECT cancelled_by FROM sequent_backend.electoral_log_outbox WHERE delivery_id = $1",
                &[&delivery_id],
            )
            .await
            .unwrap()
            .get(0);
        assert_eq!(recorded.as_deref(), Some("test"));
    }

    #[tokio::test]
    #[ignore = "requires HASURA_TEST_DATABASE_URL (a disposable migrated database) and the vault's settings"]
    async fn a_post_holding_the_scope_delays_the_action_until_it_ends() {
        let pool = pool();
        let (scope, _) = entry();
        let mut post = pool.get().await.unwrap();
        let post_tx = post.transaction().await.unwrap();
        outbox_gate(&*post_tx, &scope).await.unwrap();
        let action_pool = pool.clone();
        let action_scope = scope.clone();
        let action = tokio::spawn(async move {
            let mut client = action_pool.get().await.unwrap();
            let tx = client.transaction().await.unwrap();
            lock_scope(&*tx, &action_scope, true).await.unwrap();
            tx.rollback().await.unwrap();
        });
        tokio::time::sleep(Duration::from_millis(500)).await;
        assert!(!action.is_finished(), "the action waits for the post's append");
        post_tx.rollback().await.unwrap();
        action.await.unwrap();
        // Other scopes are never held.
        let (other, _) = entry();
        gate(&pool, &other).await.unwrap();
    }

    #[tokio::test]
    #[ignore = "requires HASURA_TEST_DATABASE_URL (a disposable migrated database) and the vault's settings"]
    async fn one_relay_at_a_time() {
        let pool = pool();
        let mut holder = pool.get().await.unwrap();
        let held = holder.transaction().await.unwrap();
        let mine: bool = held
            .query_one(
                "SELECT pg_try_advisory_xact_lock(hashtextextended('electoral-log-outbox-relay', 0))",
                &[],
            )
            .await
            .unwrap()
            .get(0);
        assert!(mine);
        let mut lock_client = pool.get().await.unwrap();
        let mut work_client = pool.get().await.unwrap();
        assert_eq!(
            relay_electoral_log_outbox(&mut lock_client, &mut work_client)
                .await
                .unwrap(),
            0,
            "a second relay returns at once"
        );
        held.rollback().await.unwrap();
    }

    #[tokio::test]
    #[ignore = "requires HASURA_TEST_DATABASE_URL (a disposable migrated database) and the vault's settings"]
    async fn the_heads_are_each_scopes_oldest_row_in_seq_order() {
        let pool = pool();
        let (first_scope, first) = entry();
        let (second_scope, second) = entry();
        let client = pool.get().await.unwrap();
        let mut c = pool.get().await.unwrap();
        let tx = c.transaction().await.unwrap();
        let a = write(&tx, &first, b"a").await;
        let b = write(&tx, &second, b"b").await;
        let _later = write(&tx, &another(&first), b"c").await;
        tx.commit().await.unwrap();
        let heads = scope_heads(&**client).await.unwrap();
        let ours: Vec<&(String, String)> = heads
            .iter()
            .filter(|(_, scope)| *scope == first_scope || *scope == second_scope)
            .collect();
        assert_eq!(
            ours,
            vec![&(a.clone(), first_scope.clone()), &(b.clone(), second_scope.clone())]
        );
        client
            .execute(
                "DELETE FROM sequent_backend.electoral_log_outbox WHERE tenant_id::text || '/' || \
                 election_event_id::text = ANY($1)",
                &[&vec![first_scope, second_scope]],
            )
            .await
            .unwrap();
    }

    // ---- End to end, with the task-queue database: the relay's crash points.
    // Needs also PGMQ_TEST_DATABASE_URL (the task-queue test database; the relay
    // reaches it through QUEUE_DB__URL, set from it) and the vault's settings
    // (SECRETS_BACKEND=EnvVarMasterSecret, MASTER_SECRET), and a tenant the
    // action database has (HASURA_TEST_TENANT_ID, by default the bundled one).

    fn tenant() -> String {
        std::env::var("HASURA_TEST_TENANT_ID")
            .unwrap_or_else(|_| "90505c8a-23a9-4cdf-a26b-4e19f6a097d5".to_string())
    }

    /// The tenant row the vault's secrets reference (a freshly migrated test
    /// database has none).
    async fn ensure_tenant<C: GenericClient + Sync>(client: &C, tenant: &str) {
        let id = uuid::Uuid::parse_str(tenant).unwrap();
        client
            .execute(
                "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2) \
                 ON CONFLICT (id) DO NOTHING",
                &[&id, &format!("electoral-log-test-{tenant}")],
            )
            .await
            .unwrap();
    }

    async fn queue_database() -> deadpool_postgres::Pool {
        let url = std::env::var("PGMQ_TEST_DATABASE_URL").expect("PGMQ_TEST_DATABASE_URL");
        if std::env::var("QUEUE_DB__URL").is_err() {
            std::env::set_var("QUEUE_DB__URL", &url);
        }
        let pool = deadpool_postgres::Config {
            url: Some(url),
            ..Default::default()
        }
        .create_pool(Some(deadpool_postgres::Runtime::Tokio1), tokio_postgres::NoTls)
        .unwrap();
        let mut client = pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        pgmq_broker::setup::setup(
            &tx,
            &pgmq_broker::setup::Installation {
                environment: "pgmq-test",
                queues: &["electoral_log_event_queue"],
                roles: None,
            },
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        pool
    }

    /// A system-signed statement of a fresh scope, written to the outbox with
    /// its MAC as `PendingStatement::queue` writes it.
    async fn queue_system(pool: &deadpool_postgres::Pool, event: &str) -> (PendingStatement, String) {
        let statement = PendingStatement::new(
            &tenant(),
            event,
            PendingStatementBody::EnrollmentProblems {
                problems: "two posts".into(),
            },
        );
        let delivery_id = statement.origin.delivery_id();
        let mut client = pool.get().await.unwrap();
        ensure_tenant(&**client, &statement.tenant_id).await;
        let tx = client.transaction().await.unwrap();
        let key = crate::services::vault::get_or_create_pending_mac_key(&tx, &statement.tenant_id)
            .await
            .unwrap();
        let mac = crate::services::electoral_log_pending::pending_mac(&key, &delivery_id, &statement)
            .unwrap();
        tx.execute(
            "INSERT INTO sequent_backend.electoral_log_outbox \
             (delivery_id, tenant_id, election_event_id, payload, mac) \
             VALUES ($1, $2::text::uuid, $3::text::uuid, $4, $5)",
            &[
                &delivery_id,
                &statement.tenant_id,
                &statement.election_event_id,
                &serde_json::to_value(&statement).unwrap(),
                &mac,
            ],
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        (statement, delivery_id)
    }

    async fn relay_once(pool: &deadpool_postgres::Pool) -> usize {
        let mut lock_client = pool.get().await.unwrap();
        let mut work_client = pool.get().await.unwrap();
        relay_electoral_log_outbox(&mut lock_client, &mut work_client)
            .await
            .unwrap()
    }

    /// (waiting rows, copies on the event queue, the row's ord) of a delivery.
    async fn waiting_state(queue: &deadpool_postgres::Pool, delivery_id: &str) -> (i64, i64, Option<i64>) {
        let client = queue.get().await.unwrap();
        let rows: i64 = client
            .query_one(
                "SELECT count(*) FROM step_queue.electoral_log_waiting WHERE delivery_id = $1",
                &[&delivery_id],
            )
            .await
            .unwrap()
            .get(0);
        let copies: i64 = client
            .query_one(
                "SELECT count(*) FROM pgmq.q_electoral_log_event_queue \
                 WHERE message->'headers'->>'id' = $1",
                &[&delivery_id],
            )
            .await
            .unwrap()
            .get(0);
        let ord: Option<i64> = client
            .query_opt(
                "SELECT ord FROM step_queue.electoral_log_waiting WHERE delivery_id = $1",
                &[&delivery_id],
            )
            .await
            .unwrap()
            .map(|row| row.get(0));
        (rows, copies, ord)
    }

    async fn relayed(pool: &deadpool_postgres::Pool, delivery_id: &str) -> bool {
        pool.get()
            .await
            .unwrap()
            .query_one(
                "SELECT relayed_at IS NOT NULL FROM sequent_backend.electoral_log_outbox WHERE delivery_id = $1",
                &[&delivery_id],
            )
            .await
            .unwrap()
            .get(0)
    }

    async fn unmark(pool: &deadpool_postgres::Pool, delivery_id: &str) {
        pool.get()
            .await
            .unwrap()
            .execute(
                "UPDATE sequent_backend.electoral_log_outbox SET relayed_at = NULL WHERE delivery_id = $1",
                &[&delivery_id],
            )
            .await
            .unwrap();
    }

    #[tokio::test]
    #[ignore = "requires HASURA_TEST_DATABASE_URL, PGMQ_TEST_DATABASE_URL and the vault's settings"]
    async fn the_relay_is_idempotent_across_its_crash_points() {
        let pool = pool();
        let queue = queue_database().await;
        let event = uuid::Uuid::new_v4().to_string();
        let (_, delivery_id) = queue_system(&pool, &event).await;
        // Crash before the waiting row: the row is simply relayed later.
        assert!(!relayed(&pool, &delivery_id).await);
        relay_once(&pool).await;
        assert!(relayed(&pool, &delivery_id).await);
        assert_eq!(waiting_state(&queue, &delivery_id).await.0, 1);
        let copies = waiting_state(&queue, &delivery_id).await.1;
        assert_eq!(copies, 1);
        // Again: nothing more.
        relay_once(&pool).await;
        assert_eq!(waiting_state(&queue, &delivery_id).await.1, copies);
        // Crash between the waiting row and the mark: the next pass adds nothing.
        unmark(&pool, &delivery_id).await;
        relay_once(&pool).await;
        assert!(relayed(&pool, &delivery_id).await);
        let (rows, again, _) = waiting_state(&queue, &delivery_id).await;
        assert_eq!((rows, again), (1, copies), "no second row, no second reference");
        // Crash after the entry was stored and its row removed, before the mark:
        // it is written again (the board stores nothing twice: see
        // a_board_stores_a_delivery_once).
        queue
            .get()
            .await
            .unwrap()
            .execute(
                "DELETE FROM step_queue.electoral_log_waiting WHERE delivery_id = $1",
                &[&delivery_id],
            )
            .await
            .unwrap();
        unmark(&pool, &delivery_id).await;
        relay_once(&pool).await;
        assert_eq!(waiting_state(&queue, &delivery_id).await.0, 1);
    }

    #[tokio::test]
    #[ignore = "requires ELECTORAL_LOG_TEST_DATABASE_URL"]
    async fn a_board_stores_a_delivery_once() {
        use electoral_log::adapters::postgres::PostgresStore;
        use electoral_log::ports::ElectoralLogStore;
        let config = std::env::var("ELECTORAL_LOG_TEST_DATABASE_URL")
            .unwrap()
            .parse()
            .unwrap();
        let store = PostgresStore::new(config).unwrap();
        store.initialize().await.unwrap();
        let board = format!("test-relay-{}", uuid::Uuid::new_v4());
        store.create_board(&board).await.unwrap();
        let key = SigningKey::generate().unwrap();
        let bytes = admin_key_announcement(&key, TENANT_FOR_BOARD, "admin-id", None, "QUJD").unwrap();
        let message = Message::record_deserialize(&bytes).unwrap();
        let entry = LogEntry {
            delivery_id: "pending:enrollment-windows:once".to_string(),
            message: ElectoralLogMessage::try_from(&message).unwrap(),
        };
        let client = electoral_log::BoardClient::new(std::sync::Arc::new(store.clone()));
        let first = client.append(&board, &[entry.clone()]).await.unwrap();
        let second = client.append(&board, &[entry.clone()]).await.unwrap();
        assert_eq!(first.len(), 1);
        assert!(second.is_empty(), "the same delivery is stored once");
        assert!(store.delivered(&board, &entry.delivery_id).await.unwrap().is_some());
    }

    const TENANT_FOR_BOARD: &str = "90505c8a-23a9-4cdf-a26b-4e19f6a097d5";

    #[tokio::test]
    #[ignore = "requires HASURA_TEST_DATABASE_URL, PGMQ_TEST_DATABASE_URL and the vault's settings"]
    async fn two_relays_keep_a_scopes_order_and_a_stuck_scope_starves_no_other() {
        let pool = pool();
        let queue = queue_database().await;
        let event = uuid::Uuid::new_v4().to_string();
        let (_, first) = queue_system(&pool, &event).await;
        let (_, second) = queue_system(&pool, &event).await;
        // Another scope whose head can never be relayed (its MAC is wrong).
        let stuck_event = uuid::Uuid::new_v4().to_string();
        let (_, stuck) = queue_system(&pool, &stuck_event).await;
        pool.get()
            .await
            .unwrap()
            .execute(
                "UPDATE sequent_backend.electoral_log_outbox SET mac = '\\x00' WHERE delivery_id = $1",
                &[&stuck],
            )
            .await
            .unwrap();
        // Two relays at once: one runs, the other returns.
        let (a, b) = tokio::join!(relay_once(&pool), relay_once(&pool));
        let _ = (a, b);
        relay_once(&pool).await;
        let (_, _, first_ord) = waiting_state(&queue, &first).await;
        let (_, _, second_ord) = waiting_state(&queue, &second).await;
        assert!(first_ord.unwrap() < second_ord.unwrap(), "seq order kept");
        // The poison row holds no other scope, is never relayed, and is
        // dead-lettered (kept).
        assert!(!relayed(&pool, &stuck).await);
        assert!(dead_lettered(&pool, &stuck).await.is_some());
        assert_eq!(waiting_state(&queue, &stuck).await.0, 0, "a bad MAC is never relayed");
    }

    /// The row's dead-letter reason, if it is dead-lettered.
    async fn dead_lettered(pool: &deadpool_postgres::Pool, delivery_id: &str) -> Option<String> {
        pool.get()
            .await
            .unwrap()
            .query_one(
                "SELECT CASE WHEN dead_lettered_at IS NULL THEN NULL ELSE dead_letter_reason END \
                 FROM sequent_backend.electoral_log_outbox WHERE delivery_id = $1",
                &[&delivery_id],
            )
            .await
            .unwrap()
            .get(0)
    }

    #[tokio::test]
    #[ignore = "requires HASURA_TEST_DATABASE_URL, PGMQ_TEST_DATABASE_URL and the vault's settings"]
    async fn a_poison_row_is_dead_lettered_its_scope_goes_on_and_it_can_be_replayed() {
        let pool = pool();
        let queue = queue_database().await;
        let event = uuid::Uuid::new_v4().to_string();
        let scope = scope_key(&tenant(), &event);
        let (poison_statement, poison) = queue_system(&pool, &event).await;
        let (_, later) = queue_system(&pool, &event).await;
        let client = pool.get().await.unwrap();
        client
            .execute(
                "UPDATE sequent_backend.electoral_log_outbox SET mac = '\\x00' WHERE delivery_id = $1",
                &[&poison],
            )
            .await
            .unwrap();
        relay_once(&pool).await;
        // The poison row no longer heads its scope: the later row is relayed.
        assert!(dead_lettered(&pool, &poison).await.unwrap().contains("MAC"));
        assert!(!relayed(&pool, &poison).await);
        assert!(relayed(&pool, &later).await);
        assert_eq!(waiting_state(&queue, &later).await.0, 1);
        gate(&pool, &scope).await.unwrap();
        // Replay without fixing it: dead-lettered again. Fixed, then replayed:
        // relayed.
        assert!(replay_outbox_entry(&**client, &poison, "op", "retry").await.unwrap());
        relay_once(&pool).await;
        assert!(dead_lettered(&pool, &poison).await.is_some());
        let mut c = pool.get().await.unwrap();
        let tx = c.transaction().await.unwrap();
        let key = crate::services::vault::pending_mac_key(&tx, &tenant()).await.unwrap().unwrap();
        tx.rollback().await.unwrap();
        let mac = crate::services::electoral_log_pending::pending_mac(&key, &poison, &poison_statement).unwrap();
        client
            .execute(
                "UPDATE sequent_backend.electoral_log_outbox SET mac = $2 WHERE delivery_id = $1",
                &[&poison, &mac],
            )
            .await
            .unwrap();
        assert!(!replay_outbox_entry(&**client, &later, "op", "not dead-lettered").await.unwrap());
        assert!(replay_outbox_entry(&**client, &poison, "op", "MAC restored").await.unwrap());
        relay_once(&pool).await;
        assert!(relayed(&pool, &poison).await);
        assert_eq!(waiting_state(&queue, &poison).await.0, 1);
        let last: Option<String> = client
            .query_one(
                "SELECT last_error FROM sequent_backend.electoral_log_outbox WHERE delivery_id = $1",
                &[&poison],
            )
            .await
            .unwrap()
            .get(0);
        assert!(last.unwrap().starts_with("replayed by op: MAC restored"));
    }

    #[tokio::test]
    #[ignore = "requires HASURA_TEST_DATABASE_URL and the vault's settings"]
    async fn a_copy_under_a_new_delivery_id_or_a_tampered_row_is_never_relayed() {
        let pool = pool();
        let event = uuid::Uuid::new_v4().to_string();
        // An administrator-signed row: a second row of the same signed
        // statement is refused at insert, whatever its delivery ID.
        let (_, statement) = entry();
        let mut c = pool.get().await.unwrap();
        let tx = c.transaction().await.unwrap();
        let original = write(&tx, &statement, b"announcement").await;
        tx.commit().await.unwrap();
        let client = pool.get().await.unwrap();
        let copy = client
            .execute(
                "INSERT INTO sequent_backend.electoral_log_outbox \
                 (delivery_id, tenant_id, election_event_id, payload, announcement, mac) \
                 SELECT 'pending:certificate-authority:' || gen_random_uuid(), tenant_id, \
                 election_event_id, payload, announcement, mac \
                 FROM sequent_backend.electoral_log_outbox WHERE delivery_id = $1",
                &[&original],
            )
            .await;
        assert!(copy.is_err(), "a replayed administrator signature is refused");
        // Its payload changed (another subject): its MAC no longer verifies.
        client
            .execute(
                "UPDATE sequent_backend.electoral_log_outbox \
                 SET payload = jsonb_set(payload, '{statement,subject_dns}', '[\"CN=two\"]') \
                 WHERE delivery_id = $1",
                &[&original],
            )
            .await
            .unwrap();
        let mut work = pool.get().await.unwrap();
        let refused = relay_row(&mut work, &original).await.unwrap_err();
        assert!(is_permanent_relay_failure(&refused), "{refused:#}");
        // A system-signed row copied under another delivery ID (its MAC binds
        // the original's): refused.
        let (_, system) = queue_system(&pool, &event).await;
        let copied = format!("pending:enrollment-windows:{}", uuid::Uuid::new_v4());
        client
            .execute(
                "INSERT INTO sequent_backend.electoral_log_outbox \
                 (delivery_id, tenant_id, election_event_id, payload, mac) \
                 SELECT $2, tenant_id, election_event_id, payload, mac \
                 FROM sequent_backend.electoral_log_outbox WHERE delivery_id = $1",
                &[&system, &copied],
            )
            .await
            .unwrap();
        let refused = relay_row(&mut work, &copied).await.unwrap_err();
        assert!(is_permanent_relay_failure(&refused), "{refused:#}");
        assert!(format!("{refused:#}").contains("MAC"));
        client
            .execute(
                "UPDATE sequent_backend.electoral_log_outbox SET cancelled_at = now(), \
                 cancelled_by = 'test', cancel_reason = 'test' WHERE delivery_id = ANY($1)",
                &[&vec![original, system, copied]],
            )
            .await
            .unwrap();
    }

    #[tokio::test]
    #[ignore = "requires HASURA_TEST_DATABASE_URL and the vault's settings"]
    async fn when_a_row_was_written_is_the_databases() {
        let pool = pool();
        let event = uuid::Uuid::new_v4().to_string();
        let (_, delivery_id) = queue_system(&pool, &event).await;
        let client = pool.get().await.unwrap();
        let fresh = "SELECT created_at > now() - interval '10 minutes' \
                     FROM sequent_backend.electoral_log_outbox WHERE delivery_id = $1";
        // An inserter's created_at is replaced with the database's time.
        let backdated = format!("pending:enrollment-windows:{}", uuid::Uuid::new_v4());
        client
            .execute(
                "INSERT INTO sequent_backend.electoral_log_outbox \
                 (delivery_id, tenant_id, election_event_id, payload, mac, created_at) \
                 SELECT $2, tenant_id, election_event_id, payload, mac, '2000-01-01' \
                 FROM sequent_backend.electoral_log_outbox WHERE delivery_id = $1",
                &[&delivery_id, &backdated],
            )
            .await
            .unwrap();
        assert!(client.query_one(fresh, &[&backdated]).await.unwrap().get::<_, bool>(0));
        // And an update can't change it.
        client
            .execute(
                "UPDATE sequent_backend.electoral_log_outbox SET created_at = '2000-01-01' \
                 WHERE delivery_id = $1",
                &[&delivery_id],
            )
            .await
            .unwrap();
        assert!(client.query_one(fresh, &[&delivery_id]).await.unwrap().get::<_, bool>(0));
        client
            .execute(
                "UPDATE sequent_backend.electoral_log_outbox SET cancelled_at = now(), \
                 cancelled_by = 'test', cancel_reason = 'test' WHERE delivery_id = ANY($1)",
                &[&vec![delivery_id, backdated]],
            )
            .await
            .unwrap();
    }

    #[tokio::test]
    #[ignore = "requires HASURA_TEST_DATABASE_URL and the vault's settings"]
    async fn a_tenants_mac_key_is_never_another_tenants() {
        let pool = pool();
        let (a, b) = (uuid::Uuid::new_v4().to_string(), uuid::Uuid::new_v4().to_string());
        assert_ne!(
            crate::services::vault::pending_mac_key_name(&a),
            crate::services::vault::pending_mac_key_name(&b)
        );
        let mut c = pool.get().await.unwrap();
        ensure_tenant(&**c, &a).await;
        ensure_tenant(&**c, &b).await;
        let tx = c.transaction().await.unwrap();
        let key_a = crate::services::vault::get_or_create_pending_mac_key(&tx, &a).await.unwrap();
        // B has none until it makes its own, and it is not A's.
        assert!(crate::services::vault::pending_mac_key(&tx, &b).await.unwrap().is_none());
        let key_b = crate::services::vault::get_or_create_pending_mac_key(&tx, &b).await.unwrap();
        assert_ne!(key_a, key_b);
        assert_eq!(crate::services::vault::pending_mac_key(&tx, &a).await.unwrap(), Some(key_a));
        tx.rollback().await.unwrap();
    }

    #[tokio::test]
    #[ignore = "requires HASURA_TEST_DATABASE_URL and the vault's settings"]
    async fn two_creators_make_one_mac_key() {
        async fn create(pool: &deadpool_postgres::Pool, tenant: &str) -> [u8; 32] {
            let mut client = pool.get().await.unwrap();
            let tx = client.transaction().await.unwrap();
            let key = crate::services::vault::get_or_create_pending_mac_key(&tx, tenant)
                .await
                .unwrap();
            tx.commit().await.unwrap();
            key
        }
        let pool = pool();
        let tenant = tenant();
        ensure_tenant(&**pool.get().await.unwrap(), &tenant).await;
        let (a, b) = tokio::join!(create(&pool, &tenant), create(&pool, &tenant));
        assert_eq!(a, b, "one key");
    }
}
