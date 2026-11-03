// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Completed voter imports reuse the durable audit outbox. The same row is
//! their receipt and the source of their signed electoral-log entry.

use crate::postgres::audit_log_outbox::NewAuditLogEntry;
use crate::postgres::tasks_execution::merge_task_execution_annotations;
use crate::services::audit_log::{stage_entry, AuditActor};
use crate::services::task_audit;
use anyhow::{Context, Result};
use deadpool_postgres::Transaction;
use electoral_log::messages::newtypes::AuditStatementKind;
use electoral_log::messages::statement::{StatementEventType, StatementLogType};
use sequent_core::services::uuid_validation::parse_uuid_v4;
use sequent_core::types::voter_import::VotersImportedEvidence;
use serde_json::json;

/// Serializes imports with one another and with the readiness snapshot. The
/// Hasura lock is kept through the Keycloak commit and the receipt commit.
pub async fn lock_voter_imports(tx: &Transaction<'_>, tenant: &str, event: &str) -> Result<()> {
    let lock_key = format!(
        "voter-import:{}:{}",
        parse_uuid_v4(tenant)?,
        parse_uuid_v4(event)?
    );
    tx.query_one(
        "SELECT pg_advisory_xact_lock(hashtextextended($1, 0))",
        &[&lock_key],
    )
    .await?;
    Ok(())
}

/// Newest completed import, including an uncertified one. Never select the
/// newest *valid* import: that could conceal a later unverified census.
pub async fn latest_voter_import(
    tx: &Transaction<'_>,
    tenant: &str,
    event: &str,
) -> Result<Option<VotersImportedEvidence>> {
    tx.query_opt(
        "SELECT details FROM sequent_backend.audit_log_outbox
         WHERE tenant_id = $1 AND election_event_id = $2 AND statement_kind = 'VotersImported'
         ORDER BY seq DESC LIMIT 1",
        &[&parse_uuid_v4(tenant)?, &parse_uuid_v4(event)?],
    )
    .await?
    .map(|row| serde_json::from_value(row.get(0)).context("Invalid completed voter-import receipt"))
    .transpose()
}

/// Called only after the Keycloak transaction committed. The receipt, its
/// delivery request and task completion are committed by the caller together.
pub async fn record_voter_import(
    tx: &Transaction<'_>,
    tenant: &str,
    event: &str,
    task_id: &str,
    actor: Option<&AuditActor>,
    mut evidence: VotersImportedEvidence,
) -> Result<VotersImportedEvidence> {
    let actor = actor.cloned().unwrap_or_default();
    // Actor columns support filtering, but only details_json is inside the
    // signed statement. Preserve attribution there through ID finalization.
    let mut details = serde_json::to_value(&evidence)?;
    details["actor"] = serde_json::to_value(&actor)?;
    let id = stage_entry(
        tx,
        &NewAuditLogEntry {
            target: task_audit::target(tenant, Some(event))?,
            statement_kind: AuditStatementKind::VotersImported,
            event_type: if actor.user_id.is_some() {
                StatementEventType::USER
            } else {
                StatementEventType::SYSTEM
            },
            log_type: StatementLogType::INFO,
            user_id: actor.user_id,
            username: actor.username,
            election_id: None,
            area_id: None,
            description: format!(
                "Imported {} voters",
                evidence.counts_by_area.values().sum::<u64>()
            ),
            details: details.clone(),
        },
    )
    .await?;
    evidence.import_id = id.to_string();
    details["import_id"] = json!(evidence.import_id);
    tx.execute(
        "UPDATE sequent_backend.audit_log_outbox SET details = $1 WHERE id = $2",
        &[&details, &id],
    )
    .await?;
    merge_task_execution_annotations(
        tx,
        tenant,
        task_id,
        &json!({
            "document_id": evidence.document_id,
            "voter_import_receipt": evidence,
        }),
    )
    .await?;
    Ok(evidence)
}

/// Complete the task inside the receipt's transaction. A retry can then see
/// completion before attempting to insert the same voters a second time.
pub async fn complete_import_task(tx: &Transaction<'_>, tenant: &str, task_id: &str) -> Result<()> {
    let changed = tx.execute(
        "UPDATE sequent_backend.tasks_execution SET execution_status = 'SUCCESS', end_at = now()
         WHERE tenant_id = $1 AND id = $2",
        &[&parse_uuid_v4(tenant)?, &parse_uuid_v4(task_id)?],
    ).await?;
    anyhow::ensure!(changed == 1, "Import task execution no longer exists");
    Ok(())
}

/// Only an active task may fail. A late or mismatched replay must preserve
/// successful receipts and the original reason of failed/cancelled tasks.
pub async fn fail_import_task(
    tx: &Transaction<'_>,
    tenant: &str,
    event: Option<&str>,
    task_id: &str,
    logs: serde_json::Value,
    annotations: serde_json::Value,
) -> Result<bool> {
    let event = event
        .filter(|id| !id.is_empty())
        .map(parse_uuid_v4)
        .transpose()?;
    let changed = tx
        .execute(
            "UPDATE sequent_backend.tasks_execution
         SET execution_status = 'FAILED', end_at = now(), logs = $1,
             annotations = COALESCE(annotations, '{}'::jsonb) || $2::jsonb
         WHERE tenant_id = $3 AND id = $4 AND election_event_id IS NOT DISTINCT FROM $5
             AND type = 'IMPORT_USERS' AND execution_status = 'IN_PROGRESS'",
            &[
                &logs,
                &annotations,
                &parse_uuid_v4(tenant)?,
                &parse_uuid_v4(task_id)?,
                &event,
            ],
        )
        .await?;
    Ok(changed == 1)
}
