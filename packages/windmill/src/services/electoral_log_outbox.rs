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
use tracing::{error, warn};
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

/// Takes the log scope's lock H(S) in the action database's transaction:
/// exclusive for a certificate action, shared for a direct post.
pub async fn lock_scope<C: GenericClient + Sync>(client: &C, scope: &str, exclusive: bool) -> Result<()> {
    let sql = if exclusive {
        "SELECT pg_advisory_xact_lock(hashtextextended($1, 0))"
    } else {
        "SELECT pg_advisory_xact_lock_shared(hashtextextended($1, 0))"
    };
    client
        .query(sql, &[&format!("electoral-log-scope:{scope}")])
        .await
        .context("The electoral log's scope is busy")?;
    Ok(())
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
             AND relayed_at IS NULL AND cancelled_at IS NULL",
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
             WHERE relayed_at IS NULL AND cancelled_at IS NULL \
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
/// exclusive. Returns its delivery ID.
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
    hasura_transaction
        .execute(
            "INSERT INTO sequent_backend.electoral_log_outbox \
             (delivery_id, tenant_id, election_event_id, payload, announcement) \
             VALUES ($1, $2::text::uuid, $3::text::uuid, $4, $5)",
            &[
                &delivery_id,
                &statement.tenant_id,
                &statement.election_event_id,
                &payload,
                &announcement,
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

/// The oldest unrelayed, uncancelled row of each scope, oldest first.
async fn scope_heads<C: GenericClient + Sync>(client: &C) -> Result<Vec<(String, String)>> {
    let mut heads: Vec<(i64, String, String)> = client
        .query(
            "SELECT DISTINCT ON (tenant_id, election_event_id) delivery_id, seq, \
             tenant_id::text || '/' || election_event_id::text \
             FROM sequent_backend.electoral_log_outbox \
             WHERE relayed_at IS NULL AND cancelled_at IS NULL \
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
             announcement FROM sequent_backend.electoral_log_outbox \
             WHERE delivery_id = $1 AND relayed_at IS NULL AND cancelled_at IS NULL FOR UPDATE",
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
    };
    let statement: PendingStatement =
        serde_json::from_value(row.payload.clone()).context("Not a pending statement")?;
    forward_announcement(&transaction, &statement, row.announcement.as_deref()).await?;
    // The waiting row commits (task-queue database) before this row is marked.
    statement
        .write_waiting(&row.delivery_id, Some((row.created_ms, row.seq)))
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
    hasura_transaction: &Transaction<'_>,
    statement: &PendingStatement,
    announcement: Option<&[u8]>,
) -> Result<()> {
    let Some((user_id, public_key)) = statement.statement.admin() else {
        return Ok(());
    };
    let announcement =
        announcement.ok_or_else(|| anyhow!("An administrator-signed entry without its announcement"))?;
    let tenant_log =
        crate::services::electoral_log_board::ensure_tenant_log(hasura_transaction, &statement.tenant_id)
            .await
            .context("Error getting the tenant log")?;
    let system_pk = VerificationKey::from_sk(&tenant_log.system_key)?;
    let message = checked_announcement(
        announcement,
        &statement.tenant_id,
        user_id,
        public_key,
        &system_pk,
    )?;
    let entry = LogEntry {
        delivery_id: crate::services::electoral_log::admin_pk_delivery_id(user_id, public_key),
        message: ElectoralLogMessage::try_from(&message)?,
    };
    crate::services::electoral_log_board::get_board_client()
        .await?
        .append(&tenant_log.board, &[entry])
        .await
        .context("Error forwarding the administrator's key announcement")?;
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

    #[test]
    fn the_relay_reads_no_private_key() {
        // Source check: the relay's path never reads an administrator's vault
        // key (the announcement travels signed with the entry).
        let source = include_str!("electoral_log_outbox.rs");
        let code = source.split("#[cfg(test)]").next().unwrap();
        for forbidden in ["read_admin_user_signing_key", "get_admin_user_signing_key", "vault::"] {
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

    async fn gate(pool: &deadpool_postgres::Pool, scope: &str) -> Result<()> {
        let mut client = pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        let result = outbox_gate(&*tx, scope).await;
        tx.rollback().await.unwrap();
        result
    }

    #[tokio::test]
    #[ignore = "requires HASURA_TEST_DATABASE_URL pointing to a disposable migrated database"]
    async fn a_rolled_back_action_leaves_no_entry_and_posts_pass() {
        let pool = pool();
        let (scope, statement) = entry();
        let mut client = pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        lock_scope(&*tx, &scope, true).await.unwrap();
        write_outbox_entry(&tx, &statement, b"announcement").await.unwrap();
        tx.rollback().await.unwrap();
        gate(&pool, &scope).await.unwrap();
    }

    #[tokio::test]
    #[ignore = "requires HASURA_TEST_DATABASE_URL pointing to a disposable migrated database"]
    async fn a_post_during_an_uncommitted_action_waits_then_is_refused_until_relayed_or_cancelled() {
        let pool = pool();
        let (scope, statement) = entry();
        let mut action = pool.get().await.unwrap();
        let tx = action.transaction().await.unwrap();
        lock_scope(&*tx, &scope, true).await.unwrap();
        let delivery_id = write_outbox_entry(&tx, &statement, b"announcement").await.unwrap();
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
    #[ignore = "requires HASURA_TEST_DATABASE_URL pointing to a disposable migrated database"]
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
    #[ignore = "requires HASURA_TEST_DATABASE_URL pointing to a disposable migrated database"]
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
    #[ignore = "requires HASURA_TEST_DATABASE_URL pointing to a disposable migrated database"]
    async fn the_heads_are_each_scopes_oldest_row_in_seq_order() {
        let pool = pool();
        let (first_scope, first) = entry();
        let (second_scope, second) = entry();
        let client = pool.get().await.unwrap();
        let mut c = pool.get().await.unwrap();
        let tx = c.transaction().await.unwrap();
        let a = write_outbox_entry(&tx, &first, b"a").await.unwrap();
        let b = write_outbox_entry(&tx, &second, b"b").await.unwrap();
        let _later = write_outbox_entry(&tx, &first, b"c").await.unwrap();
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
}
