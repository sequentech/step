// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Returning a tenant to what it was before its first configuration package,
//! so that the package and its roster can be imported again.
//!
//! What the tenant was is its baseline (`tenant_reset_baseline`), recorded
//! when the first package is uploaded. A reset runs in four steps:
//!
//! 1. claim: the tenant becomes RESETTING (it takes no new rows from then
//!    on), the refusals are checked again, and what the external cleanup
//!    needs (election events, boards, documents) is written in the task's
//!    annotations, since the next step deletes the rows that name it;
//! 2. purge and restore: the tenant's rows are deleted and the baseline's
//!    rows put back, in one transaction;
//! 3. external cleanup: electoral-log databases, protocol boards, stored
//!    files and election event realms; then the tenant realm is put back;
//! 4. finish: the audit entry, and the tenant is ACTIVE again.
//!
//! A reset that fails leaves the tenant RESETTING. It is never rolled back:
//! running it again (every step is idempotent) finishes it.

pub mod realm;

use crate::postgres::tenant_reset::{
    allow_tenant_reset, capture_rows, capture_tenant_row, claim_reset, count_tenant_rows,
    get_baseline, has_baseline, has_imported, lifecycle_state, lock_tenant, other_tenants_exist,
    purge_tenant_rows, restore_rows, restore_tenant_row, save_baseline, set_lifecycle_state,
    try_application_lock, Baseline, LifecycleState, TableCounts,
};
use crate::services::audit_log::{stage, AdminAction, AuditActor, AuditObject};
use crate::services::database::get_hasura_pool;
use crate::services::delete_election_event::{
    delete_election_event_related_documents, delete_keycloak_realm,
};
use crate::services::electoral_log_board::get_event_databases;
use crate::services::serialize_tasks_logs::append_general_log;
use crate::services::task_audit;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use realm::{
    capture_realm, check_caller, event_realms, restore_realm, RealmBaseline, RealmRestoreSummary,
};
use sequent_core::services::keycloak::{get_event_realm, get_tenant_realm, KeycloakAdminClient};
use sequent_core::services::s3;
use sequent_core::types::hasura::core::TasksExecution;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use tracing::{info, instrument, warn};
use uuid::Uuid;

/// Set to `false` to turn resets off for the whole deployment.
pub const ENABLED_ENV: &str = "TENANT_RESET_ENABLED";
/// Set to `true` to let a reset erase cast or sealed ballots, after the
/// admin types `ERASE_BALLOTS_PHRASE`.
pub const ALLOW_BALLOTS_ENV: &str = "TENANT_RESET_ALLOW_BALLOTS";
pub const ERASE_BALLOTS_PHRASE: &str = "ERASE BALLOTS";
/// The task's annotation with what the external cleanup needs.
pub const MANIFEST_ANNOTATION: &str = "reset_manifest";
pub const TASK_TYPE: &str = "RESET_TENANT";
/// A task still IN_PROGRESS after this long is taken as dead, not running.
const STALE_TASK_HOURS: i32 = 6;
/// Where uploaded configuration packages wait to be applied.
const STAGING_PREFIX: &str = "configuration-package-staging";

pub fn enabled() -> bool {
    std::env::var(ENABLED_ENV).map_or(true, |v| v.trim() != "false")
}

pub fn ballots_override_allowed() -> bool {
    std::env::var(ALLOW_BALLOTS_ENV).is_ok_and(|v| v.trim() == "true")
}

/// Why a tenant can't be reset now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Refusal {
    /// Resets are turned off in this deployment.
    Disabled,
    /// The super-admin tenant while other tenants exist: its realm holds
    /// the admins of every other tenant.
    SuperAdminTenant,
    /// The tenant has no baseline: its first package was imported before
    /// resets existed, or while it had an election event, which a baseline
    /// can't hold. What it was before is unknown.
    NoBaseline,
    /// Nothing was imported since the baseline.
    NothingToReset,
    /// Another task of the tenant is running.
    TaskRunning { task_id: String, name: String },
    /// An election event has voting open on some channel.
    VotingOpen { election_event_id: String },
    /// An election event is locked down.
    LockedDown { election_event_id: String },
    /// Ballots were cast or sealed. `overridable` when the deployment lets
    /// an admin erase them by typing `ERASE_BALLOTS_PHRASE`.
    Ballots {
        cast: u64,
        sealed: u64,
        overridable: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventSummary {
    pub id: String,
    pub name: Option<String>,
}

/// What a reset would delete, and whether it may run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Preview {
    pub enabled: bool,
    pub has_baseline: bool,
    pub baseline_captured_at: Option<String>,
    /// A previous reset did not finish: running again finishes it.
    pub resetting: bool,
    pub election_events: Vec<EventSummary>,
    /// Rows per table now.
    pub rows: TableCounts,
    pub cast_ballots: u64,
    pub sealed_ballots: u64,
    pub refusals: Vec<Refusal>,
    pub erase_ballots_phrase: &'static str,
}

/// What the external cleanup needs, read before the rows that name it are
/// deleted.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub election_event_ids: BTreeSet<String>,
    pub board_names: BTreeSet<String>,
    /// Tenant-level documents the baseline doesn't have.
    pub document_ids: BTreeSet<String>,
    /// When the first configuration package was applied, in ms since the
    /// epoch: realm users created since are the import's.
    pub created_after_ms: Option<i64>,
}

impl Manifest {
    pub fn merge(&mut self, other: Manifest) {
        self.election_event_ids.extend(other.election_event_ids);
        self.board_names.extend(other.board_names);
        self.document_ids.extend(other.document_ids);
        self.created_after_ms = match (self.created_after_ms, other.created_after_ms) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        };
    }
}

/// What the admin confirmed.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResetOptions {
    /// The admin typed `ERASE_BALLOTS_PHRASE`.
    #[serde(default)]
    pub erase_ballots: bool,
}

/// What a reset did, kept in its task's annotations.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ResetSummary {
    pub rows_deleted: TableCounts,
    pub rows_restored: TableCounts,
    pub election_events: usize,
    pub electoral_log_databases_dropped: usize,
    pub boards_deleted: usize,
    pub realms_deleted: Vec<String>,
    pub realm: RealmRestoreSummary,
    /// What the reset could not clean, for the admin to know.
    pub not_cleaned: Vec<String>,
}

async fn hasura_client() -> Result<deadpool_postgres::Object> {
    get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| anyhow!("Error getting a Hasura database client: {e}"))
}

async fn tenant_events(tx: &Transaction<'_>, tenant: &Uuid) -> Result<Vec<EventSummary>> {
    Ok(tx
        .query(
            "SELECT id::text, presentation FROM sequent_backend.election_event \
             WHERE tenant_id = $1 ORDER BY created_at, id",
            &[tenant],
        )
        .await?
        .iter()
        .map(|row| EventSummary {
            id: row.get(0),
            name: crate::services::delete_election_event::event_display_name(
                row.get::<_, Option<Value>>(1).as_ref(),
            ),
        })
        .collect())
}

/// Ballots cast in the tenant's election events, from their electoral-log
/// databases.
async fn cast_ballots(tenant_id: &str) -> Result<u64> {
    let databases = get_event_databases().await?;
    let tenant = Uuid::parse_str(tenant_id)?;
    let mut cast = 0u64;
    for event in databases.events().await? {
        if Uuid::parse_str(&event.tenant_id)? != tenant {
            continue;
        }
        let client = databases.reader_client(&event.election_event_id).await?;
        let count: i64 = client
            .query_one("SELECT count(*) FROM ballot_box_ballot", &[])
            .await
            .with_context(|| format!("Error counting the ballots of {}", event.election_event_id))?
            .get(0);
        cast += count as u64;
    }
    Ok(cast)
}

/// Whether the tenant is the deployment's super-admin tenant, whose realm
/// holds the admins of every other tenant.
pub fn is_super_admin_tenant(tenant_id: &str) -> bool {
    let tenant = Uuid::parse_str(tenant_id).ok();
    std::env::var("SUPER_ADMIN_TENANT_ID")
        .ok()
        .and_then(|id| Uuid::parse_str(&id).ok())
        .is_some_and(|id| Some(id) == tenant)
}

/// Waits until no vote of the tenant can still be on its way into a
/// ballot box. A vote is accepted under its event's activity mark, written
/// at most `ACTIVITY_MARK_INTERVAL` before it, and the sequencer trusts a
/// mark older than `ACTIVITY_CLEAR_MARGIN` to cover no vote in flight.
async fn settle_ballot_activity(tenant_id: &str) -> Result<()> {
    use electoral_log::adapters::events::{ACTIVITY_CLEAR_MARGIN, ACTIVITY_MARK_INTERVAL};
    let tenant = Uuid::parse_str(tenant_id)?;
    let settle = ACTIVITY_MARK_INTERVAL + ACTIVITY_CLEAR_MARGIN;
    let wait = crate::services::ballot_box::events_with_ballot_activity()
        .await?
        .into_iter()
        .filter(|a| Uuid::parse_str(&a.tenant_id).is_ok_and(|t| t == tenant))
        .filter_map(|a| {
            let age = a.listed_at.duration_since(a.marked_at).unwrap_or_default();
            settle.checked_sub(age)
        })
        .max();
    if let Some(wait) = wait {
        info!("Waiting {wait:?} for votes on their way into a ballot box");
        tokio::time::sleep(wait).await;
    }
    Ok(())
}

/// Why the tenant can't be reset, with `cast` ballots counted elsewhere.
/// `retry` is a reset finishing an earlier one, which finds nothing
/// imported: the earlier one deleted it. `keep_task` is the reset's own
/// task, set only when claiming: then the checks also take the locks that
/// keep a new tenant and an Apply out until the claim commits.
async fn refusals(
    tx: &Transaction<'_>,
    tenant_id: &str,
    cast: u64,
    options: &ResetOptions,
    keep_task: Option<&Uuid>,
    retry: bool,
) -> Result<(Vec<Refusal>, u64)> {
    let tenant = Uuid::parse_str(tenant_id)?;
    let claiming = keep_task.is_some();
    let mut refused = vec![];
    if !enabled() {
        refused.push(Refusal::Disabled);
    }
    // When claiming, the lock is held until the claim commits; the reset
    // state row then refuses new tenants until the reset finishes.
    if is_super_admin_tenant(tenant_id) && other_tenants_exist(tx, tenant_id, claiming).await? {
        refused.push(Refusal::SuperAdminTenant);
    }
    if !has_baseline(tx, tenant_id).await? {
        refused.push(if has_imported(tx, tenant_id).await? {
            Refusal::NoBaseline
        } else {
            Refusal::NothingToReset
        });
    } else if !retry
        && !has_imported(tx, tenant_id).await?
        && tenant_events(tx, &tenant).await?.is_empty()
    {
        refused.push(Refusal::NothingToReset);
    }

    for row in tx
        .query(
            "SELECT id::text, name FROM sequent_backend.tasks_execution \
             WHERE tenant_id = $1 AND execution_status = 'IN_PROGRESS' \
               AND created_at > now() - make_interval(hours => $2) \
               AND ($3::uuid IS NULL OR id <> $3) \
             ORDER BY created_at LIMIT 1",
            &[&tenant, &STALE_TASK_HOURS, &keep_task],
        )
        .await?
    {
        refused.push(Refusal::TaskRunning {
            task_id: row.get(0),
            name: row.get(1),
        });
    }
    // An Apply holds its lock for its whole run, however old its task is.
    if claiming && refused.is_empty() && !try_application_lock(tx, tenant_id).await? {
        let running = tx
            .query_opt(
                "SELECT id::text, name FROM sequent_backend.tasks_execution \
                 WHERE tenant_id = $1 AND execution_status = 'IN_PROGRESS' \
                   AND type = 'APPLY_CONFIGURATION_PACKAGE' \
                 ORDER BY created_at DESC LIMIT 1",
                &[&tenant],
            )
            .await?;
        refused.push(Refusal::TaskRunning {
            task_id: running.as_ref().map(|r| r.get(0)).unwrap_or_default(),
            name: running
                .as_ref()
                .map(|r| r.get(1))
                .unwrap_or_else(|| "Apply configuration package".to_string()),
        });
    }

    use crate::services::delete_election_event::{
        find_election_event_deletion_refusal, ElectionEventDeletionRefusal,
    };
    for event in tenant_events(tx, &tenant).await? {
        // Permission labels don't limit a reset, which is the whole
        // tenant's: every election's label is passed.
        let labels: Vec<String> =
            crate::postgres::election::get_elections(tx, tenant_id, &event.id)
                .await?
                .into_iter()
                .filter_map(|e| e.permission_label)
                .collect();
        match find_election_event_deletion_refusal(tx, tenant_id, &event.id, &labels).await? {
            Some(ElectionEventDeletionRefusal::VotingInProgress) => {
                refused.push(Refusal::VotingOpen {
                    election_event_id: event.id,
                })
            }
            Some(ElectionEventDeletionRefusal::LockedDown) => refused.push(Refusal::LockedDown {
                election_event_id: event.id,
            }),
            _ => {}
        }
    }

    let sealed: i64 = tx
        .query_one(
            "SELECT count(*) FROM sequent_backend.ballot_box_seal WHERE tenant_id = $1",
            &[&tenant],
        )
        .await?
        .get(0);
    let sealed = sealed as u64;
    if cast > 0 || sealed > 0 {
        let overridable = ballots_override_allowed();
        if !(overridable && options.erase_ballots) {
            refused.push(Refusal::Ballots {
                cast,
                sealed,
                overridable,
            });
        }
    }
    Ok((refused, sealed))
}

/// What a reset of the tenant would do, and why it would be refused.
#[instrument(err)]
pub async fn preview(tenant_id: &str) -> Result<Preview> {
    let cast = cast_ballots(tenant_id).await?;
    let mut client = hasura_client().await?;
    let tx = client.build_transaction().read_only(true).start().await?;
    let tenant = Uuid::parse_str(tenant_id)?;
    let resetting = lifecycle_state(&tx, tenant_id).await? == LifecycleState::Resetting;
    let (refused, sealed) = refusals(
        &tx,
        tenant_id,
        cast,
        &ResetOptions::default(),
        None,
        resetting,
    )
    .await?;
    let captured_at: Option<String> = tx
        .query_opt(
            "SELECT captured_at::text FROM sequent_backend.tenant_reset_baseline WHERE tenant_id = $1",
            &[&tenant],
        )
        .await?
        .map(|row| row.get(0));
    let preview = Preview {
        enabled: enabled(),
        has_baseline: captured_at.is_some(),
        baseline_captured_at: captured_at,
        resetting,
        election_events: tenant_events(&tx, &tenant).await?,
        rows: count_tenant_rows(&tx, tenant_id).await?,
        cast_ballots: cast,
        sealed_ballots: sealed,
        // The ballot refusal is shown, but an admin can still confirm past
        // it when the deployment allows.
        refusals: refused,
        erase_ballots_phrase: ERASE_BALLOTS_PHRASE,
    };
    tx.commit().await?;
    Ok(preview)
}

/// Whether a baseline can be recorded now: none is, nothing was imported,
/// and the tenant has no election event. The baseline holds no event data,
/// so a reset would delete an event made before it without restoring it;
/// such a tenant is refused as having no baseline instead.
async fn baseline_possible(tx: &Transaction<'_>, tenant_id: &str) -> Result<bool> {
    let tenant = Uuid::parse_str(tenant_id)?;
    Ok(!has_baseline(tx, tenant_id).await?
        && !has_imported(tx, tenant_id).await?
        && tenant_events(tx, &tenant).await?.is_empty())
}

/// Records the baseline if the tenant has none and has imported nothing:
/// called when a configuration package is uploaded and when one is applied.
/// Returns whether it recorded one.
#[instrument(err)]
pub async fn ensure_baseline(tenant_id: &str, task_id: Option<&str>) -> Result<bool> {
    {
        let mut client = hasura_client().await?;
        let tx = client.build_transaction().read_only(true).start().await?;
        let needed = baseline_possible(&tx, tenant_id).await?;
        tx.commit().await?;
        if !needed {
            return Ok(false);
        }
    }
    // Keycloak first, outside the transaction.
    let keycloak = KeycloakAdminClient::pub_new().await?;
    let realm = capture_realm(&keycloak, &get_tenant_realm(tenant_id)).await?;

    let mut client = hasura_client().await?;
    let tx = client.transaction().await?;
    // Checked again: another upload may have recorded one, or applied. The
    // tenant row lock, held until the baseline commits, waits for writers
    // already in (they hold a key-share lock on it) and keeps new ones out,
    // so an event can't commit between the check and the baseline.
    lock_tenant(&tx, tenant_id).await?;
    if !baseline_possible(&tx, tenant_id).await? {
        tx.rollback().await?;
        return Ok(false);
    }
    let baseline = Baseline {
        tenant_row: capture_tenant_row(&tx, tenant_id).await?,
        rows: capture_rows(&tx, tenant_id).await?,
        realm: serde_json::to_value(&realm)?,
    };
    let saved = save_baseline(&tx, tenant_id, &baseline, task_id).await?;
    tx.commit().await?;
    if saved {
        info!("Recorded the reset baseline of tenant {tenant_id}");
    }
    Ok(saved)
}

async fn compute_manifest(
    tx: &Transaction<'_>,
    tenant_id: &str,
    baseline: Option<&Baseline>,
) -> Result<Manifest> {
    let tenant = Uuid::parse_str(tenant_id)?;
    let strings = |rows: Vec<tokio_postgres::Row>| -> BTreeSet<String> {
        rows.iter().map(|row| row.get::<_, String>(0)).collect()
    };
    let election_event_ids = strings(
        tx.query(
            "SELECT id::text FROM sequent_backend.election_event WHERE tenant_id = $1",
            &[&tenant],
        )
        .await?,
    );
    let board_names = strings(
        tx.query(
            "SELECT name FROM sequent_backend.protocol_board WHERE tenant_id = $1",
            &[&tenant],
        )
        .await?,
    );
    let kept_documents: BTreeSet<String> = baseline
        .and_then(|b| b.rows.get("document"))
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(|r| r.get("id").and_then(Value::as_str).map(String::from))
                .collect()
        })
        .unwrap_or_default();
    let document_ids = strings(
        tx.query(
            "SELECT id::text FROM sequent_backend.document \
             WHERE tenant_id = $1 AND election_event_id IS NULL",
            &[&tenant],
        )
        .await?,
    )
    .into_iter()
    .filter(|id| !kept_documents.contains(id))
    .collect();
    let created_after_ms: Option<i64> = tx
        .query_one(
            "SELECT (extract(epoch FROM least( \
                 (SELECT min(created_at) FROM sequent_backend.tasks_execution \
                   WHERE tenant_id = $1 AND type = 'APPLY_CONFIGURATION_PACKAGE'), \
                 (SELECT min(imported_at) FROM sequent_backend.configuration_package \
                   WHERE tenant_id = $1))) * 1000)::bigint",
            &[&tenant],
        )
        .await?
        .get(0);
    Ok(Manifest {
        election_event_ids,
        board_names,
        document_ids,
        created_after_ms,
    })
}

/// The manifests of earlier resets of the tenant that did not finish: their
/// rows are gone, so what they named is only there.
async fn unfinished_manifests(tx: &Transaction<'_>, tenant: &Uuid) -> Result<Manifest> {
    let mut merged = Manifest::default();
    // The reset's own task too: a redelivery runs after the purge, so what
    // its first run recorded is no longer in the tables.
    for row in tx
        .query(
            "SELECT annotations -> $2 FROM sequent_backend.tasks_execution \
             WHERE tenant_id = $1 AND type = $3 \
               AND execution_status <> 'SUCCESS' AND annotations ? $2",
            &[tenant, &MANIFEST_ANNOTATION, &TASK_TYPE],
        )
        .await?
    {
        if let Some(value) = row.get::<_, Option<Value>>(0) {
            match serde_json::from_value::<Manifest>(value) {
                Ok(manifest) => merged.merge(manifest),
                Err(error) => warn!("Ignoring an unreadable reset manifest: {error}"),
            }
        }
    }
    Ok(merged)
}

async fn annotate_task(tx: &Transaction<'_>, task: &Uuid, key: &str, value: Value) -> Result<()> {
    tx.execute(
        "UPDATE sequent_backend.tasks_execution \
         SET annotations = coalesce(annotations, '{}'::jsonb) || jsonb_build_object($2::text, $3::jsonb) \
         WHERE id = $1",
        &[task, &key, &value],
    )
    .await
    .context("Error annotating the reset task")?;
    Ok(())
}

/// Deletes protocol boards and their messages from the board database. The
/// message bodies kept in the board service's own bucket are not reached.
async fn delete_boards(names: &BTreeSet<String>) -> Result<usize> {
    if names.is_empty() {
        return Ok(0);
    }
    let url = std::env::var("WBRAID_BOARD_DATABASE_URL")
        .context("WBRAID_BOARD_DATABASE_URL must be set")?;
    let mut options: sqlx::postgres::PgConnectOptions = url.parse()?;
    if let Ok(password) = std::env::var("WBRAID_B4_PG_PASSWORD") {
        options = options.password(&password);
    }
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect_with(options)
        .await?;
    let names: Vec<String> = names.iter().cloned().collect();
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM messages WHERE board_name = ANY($1)")
        .bind(&names)
        .execute(&mut *tx)
        .await?;
    let deleted = sqlx::query("DELETE FROM boards WHERE name = ANY($1)")
        .bind(&names)
        .execute(&mut *tx)
        .await?
        .rows_affected();
    tx.commit().await?;
    Ok(deleted as usize)
}

/// Drops the tenant's electoral-log event databases, live and retired. The
/// tenant's own log database is kept: it holds the record of the reset.
async fn drop_electoral_logs(tenant_id: &str, manifest: &Manifest) -> Result<usize> {
    let databases = get_event_databases().await?;
    let tenant = Uuid::parse_str(tenant_id)?;
    let mut events: BTreeSet<String> = manifest.election_event_ids.clone();
    for event in databases.events().await? {
        if Uuid::parse_str(&event.tenant_id)? == tenant {
            events.insert(event.election_event_id);
        }
    }
    let mut dropped = 0;
    for event in &events {
        databases.drop_event(tenant_id, event).await?;
        dropped += 1;
    }
    for retired in databases.retired_events(tenant_id).await? {
        databases.drop_retired(&retired).await?;
        dropped += 1;
    }
    Ok(dropped)
}

async fn delete_files(tenant_id: &str, manifest: &Manifest) -> Result<()> {
    for event in &manifest.election_event_ids {
        delete_election_event_related_documents(tenant_id, event).await?;
    }
    let private = s3::get_private_bucket()?;
    let public = s3::get_public_bucket()?;
    for document in &manifest.document_ids {
        let prefix = format!("tenant-{tenant_id}/document-{document}/");
        for bucket in [&private, &public] {
            s3::delete_files_from_s3(bucket.clone(), prefix.clone(), s3::S3Endpoint::Server)
                .await
                .map_err(|e| anyhow!("Error deleting document files: {e:?}"))?;
        }
    }
    s3::delete_files_from_s3(
        private,
        format!("{STAGING_PREFIX}/tenant-{tenant_id}/"),
        s3::S3Endpoint::Server,
    )
    .await
    .map_err(|e| anyhow!("Error deleting staged configuration packages: {e:?}"))?;
    Ok(())
}

/// Runs, or finishes, the reset of `tenant_id` for the task `task`.
/// `keep_user` is the Keycloak id of the admin doing it, who stays as they
/// are in the tenant realm.
#[instrument(skip(task, actor), err)]
pub async fn run_reset(
    task: &TasksExecution,
    tenant_id: &str,
    actor: Option<AuditActor>,
    keep_user: &str,
    options: &ResetOptions,
) -> Result<ResetSummary> {
    let tenant = Uuid::parse_str(tenant_id)?;
    let task_id = Uuid::parse_str(&task.id)?;
    // A redelivery of a reset that finished: the tenant may already have
    // been imported into again.
    if let Some(summary) = finished_summary(&task_id).await? {
        info!("Reset task {task_id} already finished");
        return Ok(summary);
    }
    // The admin doing it keeps their account and access: checked before
    // anything changes.
    let realm_record = {
        let mut client = hasura_client().await?;
        let tx = client.build_transaction().read_only(true).start().await?;
        let baseline = get_baseline(&tx, tenant_id).await?;
        tx.commit().await?;
        baseline.map(|b| b.realm).filter(|realm| !realm.is_null())
    };
    if let Some(realm) = realm_record {
        let realm: RealmBaseline =
            serde_json::from_value(realm).context("Unreadable realm baseline")?;
        let keycloak = KeycloakAdminClient::pub_new().await?;
        check_caller(&keycloak, &get_tenant_realm(tenant_id), &realm, keep_user).await?;
    }
    let cast = cast_ballots(tenant_id).await?;
    let mut summary = ResetSummary::default();

    // 1. Claim.
    let (manifest, retry) = {
        let mut client = hasura_client().await?;
        let tx = client.transaction().await?;
        let retry = lock_tenant(&tx, tenant_id).await? == LifecycleState::Resetting;
        let (refused, _) = refusals(&tx, tenant_id, cast, options, Some(&task_id), retry).await?;
        if !refused.is_empty() {
            tx.rollback().await?;
            return Err(anyhow!(
                "The tenant can't be reset: {}",
                serde_json::to_string(&refused)?
            ));
        }
        claim_reset(&tx, tenant_id, is_super_admin_tenant(tenant_id)).await?;
        let baseline = get_baseline(&tx, tenant_id).await?;
        let mut manifest = compute_manifest(&tx, tenant_id, baseline.as_ref()).await?;
        manifest.merge(unfinished_manifests(&tx, &tenant).await?);
        annotate_task(
            &tx,
            &task_id,
            MANIFEST_ANNOTATION,
            serde_json::to_value(&manifest)?,
        )
        .await?;
        tx.commit().await?;
        (manifest, retry)
    };
    summary.election_events = manifest.election_event_ids.len();

    // Voting was closed when the claim committed, but a vote seen open just
    // before can still be on its way into a ballot box: count again once
    // none can be.
    if !(options.erase_ballots && ballots_override_allowed()) {
        settle_ballot_activity(tenant_id).await?;
        let late = cast_ballots(tenant_id).await?;
        if late > 0 {
            if !retry {
                let mut client = hasura_client().await?;
                let tx = client.transaction().await?;
                set_lifecycle_state(&tx, tenant_id, LifecycleState::Active).await?;
                tx.commit().await?;
            }
            return Err(anyhow!(
                "The tenant can't be reset: {}",
                serde_json::to_string(&[Refusal::Ballots {
                    cast: late,
                    sealed: 0,
                    overridable: ballots_override_allowed(),
                }])?
            ));
        }
    }

    // 2. Purge and restore.
    let realm_baseline: Option<RealmBaseline> = {
        let mut client = hasura_client().await?;
        let tx = client.transaction().await?;
        allow_tenant_reset(&tx, tenant_id).await?;
        let baseline = get_baseline(&tx, tenant_id)
            .await?
            .ok_or_else(|| anyhow!("The tenant has no reset baseline"))?;
        summary.rows_deleted = purge_tenant_rows(&tx, tenant_id, Some(task_id)).await?;
        summary.rows_restored = restore_rows(&tx, tenant_id, &baseline.rows).await?;
        restore_tenant_row(&tx, tenant_id, &baseline.tenant_row).await?;
        tx.commit().await?;
        if baseline.realm.is_null() {
            None
        } else {
            Some(serde_json::from_value(baseline.realm).context("Unreadable realm baseline")?)
        }
    };

    // 3. External cleanup.
    summary.electoral_log_databases_dropped = drop_electoral_logs(tenant_id, &manifest).await?;
    summary.boards_deleted = delete_boards(&manifest.board_names).await?;
    if !manifest.board_names.is_empty() {
        summary
            .not_cleaned
            .push("protocol board message bodies in the board service's bucket".into());
    }
    delete_files(tenant_id, &manifest).await?;
    let keycloak = KeycloakAdminClient::pub_new().await?;
    let mut realms: BTreeSet<String> = event_realms(&keycloak, tenant_id)
        .await?
        .into_iter()
        .collect();
    realms.extend(
        manifest
            .election_event_ids
            .iter()
            .map(|event| get_event_realm(tenant_id, event)),
    );
    for name in &realms {
        delete_keycloak_realm(name).await?;
    }
    summary.realms_deleted = realms.into_iter().collect();
    summary.realm = restore_realm(
        &keycloak,
        &get_tenant_realm(tenant_id),
        realm_baseline.as_ref(),
        keep_user,
        manifest.created_after_ms,
    )
    .await?;

    // 4. Finish.
    let mut client = hasura_client().await?;
    let tx = client.transaction().await?;
    // The tenant is still RESETTING: the audit entry needs the reset's leave.
    allow_tenant_reset(&tx, tenant_id).await?;
    let change = task_audit::change(
        actor,
        task_audit::target(tenant_id, None)?,
        AuditObject::new(task_audit::TENANT, tenant_id, None),
        AdminAction::Reset,
    )
    .with_extra(json!({
        "task_id": task.id,
        "election_events": manifest.election_event_ids,
        "rows_deleted": summary.rows_deleted,
        "rows_restored": summary.rows_restored,
        "realms_deleted": summary.realms_deleted,
        "users_deleted": summary.realm.users_deleted,
        "erase_ballots": options.erase_ballots,
        "cast_ballots": cast,
    }));
    stage(&tx, &change).await?;
    annotate_task(
        &tx,
        &task_id,
        "reset_summary",
        serde_json::to_value(&summary)?,
    )
    .await?;
    set_lifecycle_state(&tx, tenant_id, LifecycleState::Active).await?;
    // Completed with the reset, so that a redelivery finds it done.
    let logs = serde_json::to_value(append_general_log(
        &task.logs,
        "Task completed successfully",
    ))?;
    tx.execute(
        "UPDATE sequent_backend.tasks_execution \
         SET execution_status = 'SUCCESS', end_at = now(), logs = $2 WHERE id = $1",
        &[&task_id, &logs],
    )
    .await
    .context("Error completing the reset task")?;
    tx.commit().await?;
    crate::tasks::audit_log_outbox::kick_audit_log_outbox();
    Ok(summary)
}

/// The summary of a reset task that already finished.
async fn finished_summary(task: &Uuid) -> Result<Option<ResetSummary>> {
    let client = hasura_client().await?;
    let row = client
        .query_opt(
            "SELECT execution_status, annotations -> 'reset_summary' \
             FROM sequent_backend.tasks_execution WHERE id = $1",
            &[task],
        )
        .await
        .context("Error reading the reset task")?;
    let Some(row) = row else { return Ok(None) };
    let status: Option<String> = row.get(0);
    let summary: Option<Value> = row.get(1);
    Ok(match summary {
        Some(summary) => Some(serde_json::from_value(summary).unwrap_or_default()),
        None if status.as_deref() == Some("SUCCESS") => Some(ResetSummary::default()),
        None => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifests_merge_to_the_earliest_import() {
        let mut a = Manifest {
            election_event_ids: ["e1".to_string()].into(),
            created_after_ms: Some(20),
            ..Default::default()
        };
        a.merge(Manifest {
            election_event_ids: ["e2".to_string()].into(),
            board_names: ["b".to_string()].into(),
            created_after_ms: Some(10),
            ..Default::default()
        });
        assert_eq!(a.election_event_ids.len(), 2);
        assert_eq!(a.board_names.len(), 1);
        assert_eq!(a.created_after_ms, Some(10));
        a.merge(Manifest::default());
        assert_eq!(a.created_after_ms, Some(10));
    }

    #[test]
    fn refusals_read_as_tagged_json() {
        let value = serde_json::to_value(Refusal::Ballots {
            cast: 2,
            sealed: 0,
            overridable: false,
        })
        .unwrap();
        assert_eq!(value["kind"], "ballots");
        assert_eq!(value["cast"], 2);
        assert_eq!(
            serde_json::to_value(Refusal::NoBaseline).unwrap()["kind"],
            "no_baseline"
        );
    }

    #[test]
    fn resets_are_on_unless_turned_off() {
        // Read from the environment at call time; not set in tests.
        if std::env::var(ENABLED_ENV).is_err() {
            assert!(enabled());
        }
        if std::env::var(ALLOW_BALLOTS_ENV).is_err() {
            assert!(!ballots_override_allowed());
        }
    }
}
