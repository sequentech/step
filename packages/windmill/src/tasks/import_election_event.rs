// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::election_event::get_election_event_by_id_if_exist;
use crate::postgres::maintenance::vacuum_analyze_direct;
use crate::services::audit_log::{stage, AdminAction, AdminChange, AuditActor, AuditObject};
use crate::services::delete_election_event::drop_uncommitted_event_database;
use crate::services::electoral_log::ElectoralLogAdminContext;
use crate::services::import::import_users::ImportUsersPrivileges;
use crate::services::import::rejection::problems_of;
use crate::services::providers::transactions_provider::provide_hasura_transaction;
use crate::services::task_audit;
use crate::services::tasks_execution::{update_complete, update_fail, update_fail_with_problems};
use crate::tasks::audit_log_outbox::kick_audit_log_outbox;
use crate::tasks::insert_election_event::ELECTION_EVENT;
use crate::{
    services::import::import_election_event::{
        self as import_election_event_service, ImportedElectionEvent,
    },
    types::error::Result,
};
use anyhow::{anyhow, Context};
use celery::error::TaskError;
use deadpool_postgres::Transaction;
use sequent_core::services::translations::Name;
use sequent_core::types::hasura::core::{ElectionEvent, TasksExecution};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{event, info, instrument, Level};

#[derive(Deserialize, Debug, Clone, Serialize)]
pub struct ImportElectionEventBody {
    pub tenant_id: String,
    pub document_id: String,
    pub password: Option<String>,
    pub check_only: Option<bool>,
    pub sha256: Option<String>,
    #[serde(default)]
    pub may_write_secret_attributes: bool,
    #[serde(default)]
    pub secret_write_initiator: Option<ElectoralLogAdminContext>,
    /// The administrator who started the import, set by the server; the
    /// signing configuration's log entries name them.
    #[serde(default)]
    pub importer: Option<ElectoralLogAdminContext>,
    /// Whether the importer holds `messaging-config-write`, set by the
    /// server: a bundle's messaging configuration is imported only then.
    #[serde(default)]
    pub messaging_config_permission: crate::services::messaging::config::MessagingConfigPermission,
    /// What the importing user may grant through the voters file, derived by
    /// the server from that user's permissions.
    #[serde(default)]
    pub privileges: ImportUsersPrivileges,
}

/// The tenant log's entry of an imported election event: the SHA-256 of the
/// file it read, the checksum the importer declared when there was one to
/// check, and what it imported.
fn imported_entry(
    object: &ImportElectionEventBody,
    tenant_id: &str,
    election_event_id: &str,
    name: Option<&str>,
    imported: &ImportedElectionEvent,
) -> anyhow::Result<AdminChange> {
    let actor = object.importer.as_ref().map(|importer| AuditActor {
        user_id: Some(importer.user_id.clone()),
        username: importer.username.clone(),
        role: None,
    });
    let mut extra = task_audit::imported_document(
        &object.document_id,
        &imported.file_sha256,
        imported.declared_sha256.as_deref(),
    );
    extra["counts"] = serde_json::to_value(&imported.counts)?;
    Ok(task_audit::change(
        actor,
        task_audit::target(tenant_id, None)?,
        AuditObject::new(ELECTION_EVENT, election_event_id, name),
        AdminAction::Import,
    )
    .with_extra(extra))
}

/// The name the import gave the event, in its default language. The name is
/// in the event's presentation, not in a column of its own.
async fn imported_name(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> anyhow::Result<Option<String>> {
    let event = get_election_event_by_id_if_exist(hasura_transaction, tenant_id, election_event_id)
        .await
        .context("Error reading the imported election event")?;
    Ok(event.and_then(|event| imported_event_name(&event)))
}

/// An event's name for its audit entry, if its translations give one.
fn imported_event_name(event: &ElectionEvent) -> Option<String> {
    Some(event.get_name(&event.get_default_language()))
        .filter(|name| !name.is_empty() && name != "-")
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn import_election_event(
    object: ImportElectionEventBody,
    election_event_id: String,
    tenant_id: String,
    task_execution: TasksExecution,
) -> Result<()> {
    execute(object, election_event_id, tenant_id, task_execution).await
}

/// The task's work, also run in-process by the package application, which
/// must not wait on another task in its own queue.
pub async fn execute(
    object: ImportElectionEventBody,
    election_event_id: String,
    tenant_id: String,
    task_execution: TasksExecution,
) -> Result<()> {
    // What the import left out, with its reason, for the task's log.
    let notes = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let result = provide_hasura_transaction(|hasura_transaction| {
        let object = object.clone();
        let tenant_id = tenant_id.clone();
        let election_event_id = election_event_id.clone();
        let notes = notes.clone();

        Box::pin(async move {
            let imported = import_election_event_service::process_document(
                hasura_transaction,
                object.clone(),
                election_event_id.clone(),
                tenant_id.clone(),
            )
            .await?;
            let name = imported_name(hasura_transaction, &tenant_id, &election_event_id).await?;
            stage(
                hasura_transaction,
                &imported_entry(
                    &object,
                    &tenant_id,
                    &election_event_id,
                    name.as_deref(),
                    &imported,
                )?,
            )
            .await?;
            *notes.lock().unwrap_or_else(|e| e.into_inner()) = imported.notes;
            Ok(())
        })
    })
    .await;

    match &result {
        Ok(_) => {
            kick_audit_log_outbox();
            // Execute database maintenance
            info!("Performing mainteinance after election event import.");
            vacuum_analyze_direct().await?;
            let mut task_execution = task_execution.clone();
            for note in notes.lock().unwrap_or_else(|e| e.into_inner()).iter() {
                task_execution.logs = serde_json::to_value(
                    crate::services::serialize_tasks_logs::append_general_log(
                        &task_execution.logs,
                        note,
                    ),
                )
                .ok();
            }
            let _ = update_complete(&task_execution, Some(object.document_id.clone())).await;
            Ok(())
        }
        Err(error) => {
            let err_str = format!("Error processing election event document: {error:#}");
            drop_uncommitted_event_database(&tenant_id, &election_event_id).await;
            let _ = match problems_of(error) {
                Some(problems) => {
                    update_fail_with_problems(&task_execution, &err_str, &problems).await
                }
                None => update_fail(&task_execution, &err_str).await,
            };
            Err(err_str.into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::import::import_election_event::ImportedCounts;

    const TENANT_ID: &str = "4f1c6b38-9f5e-4a59-8d3b-2a7f0c1e5d61";
    const EVENT_ID: &str = "b7e2d915-3c4a-4e8f-9a61-5d0f2c8b7e43";

    fn body(json: serde_json::Value) -> ImportElectionEventBody {
        serde_json::from_value(json).unwrap()
    }

    fn imported_event(presentation: serde_json::Value) -> ElectionEvent {
        serde_json::from_value(json!({
            "id": EVENT_ID,
            "tenant_id": TENANT_ID,
            "presentation": presentation,
            "is_archived": false,
            "encryption_protocol": "RSA256",
        }))
        .unwrap()
    }

    /// The name is read from the presentation, in the default language: the
    /// table has had no name column since the names moved there.
    #[test]
    fn an_imported_events_name_is_its_default_languages_name() {
        let named = imported_event(json!({
            "language_conf": {"default_language_code": "es"},
            "i18n": {"en": {"name": "Barangay 2026"}, "es": {"name": "Barangay 2026 (es)"}},
        }));
        assert_eq!(
            imported_event_name(&named).as_deref(),
            Some("Barangay 2026 (es)")
        );
        assert_eq!(imported_event_name(&imported_event(json!({}))), None);
        assert_eq!(
            imported_event_name(&imported_event(serde_json::Value::Null)),
            None
        );
    }

    fn imported(declared_sha256: Option<&str>) -> ImportedElectionEvent {
        ImportedElectionEvent {
            notes: vec![],
            file_sha256: "ABCDEF".into(),
            declared_sha256: declared_sha256.map(str::to_string),
            counts: ImportedCounts {
                elections: 2,
                contests: 3,
                candidates: 7,
                areas: 4,
                area_contests: 5,
            },
        }
    }

    #[test]
    fn an_imported_event_is_an_entry_of_the_tenant_log_with_its_source() {
        let object = body(json!({
            "tenant_id": TENANT_ID, "document_id": "document-1", "password": null,
            "check_only": false, "sha256": "ABCDEF",
            "importer": {"user_id": "admin-id", "username": "ada",
                "authorized_election_ids": null, "area_id": null},
        }));
        let entry = imported_entry(
            &object,
            TENANT_ID,
            EVENT_ID,
            Some("Barangay 2026"),
            &imported(Some("ABCDEF")),
        )
        .unwrap();

        assert_eq!(entry.target.election_event_id, None);
        assert_eq!(
            entry.object,
            AuditObject::new(ELECTION_EVENT, EVENT_ID, Some("Barangay 2026"))
        );
        assert_eq!(entry.action, AdminAction::Import);
        assert_eq!(
            entry.actor,
            AuditActor {
                user_id: Some("admin-id".into()),
                username: Some("ada".into()),
                role: None,
            }
        );
        assert_eq!(
            entry.extra,
            json!({
                "document_id": "document-1",
                "file_sha256": "abcdef",
                "declared_sha256": "abcdef",
                "declared_checksum": "matches",
                "counts": {"elections": 2, "contests": 3, "candidates": 7, "areas": 4,
                    "area_contests": 5},
            })
        );
    }

    /// The checksum compared is the one the import still had to check: none
    /// for a signed configuration package.
    #[test]
    fn a_declared_checksum_that_differs_from_the_file_is_flagged() {
        let object = body(json!({
            "tenant_id": TENANT_ID, "document_id": "document-1", "password": null,
            "check_only": false, "sha256": "012345",
        }));
        let entry = imported_entry(
            &object,
            TENANT_ID,
            EVENT_ID,
            None,
            &imported(Some("012345")),
        )
        .unwrap();
        assert_eq!(entry.extra["declared_sha256"], "012345");
        assert_eq!(entry.extra["declared_checksum"], "mismatch");
        assert_eq!(entry.extra["file_sha256"], "abcdef");
    }

    #[test]
    fn an_import_without_importer_or_checksum_logs_neither() {
        let object = body(json!({
            "tenant_id": TENANT_ID, "document_id": "document-1", "password": null,
            "check_only": null, "sha256": "",
        }));
        let entry = imported_entry(&object, TENANT_ID, EVENT_ID, None, &imported(None)).unwrap();

        assert_eq!(entry.actor, AuditActor::default());
        assert_eq!(entry.object.name, None);
        assert_eq!(entry.extra["declared_checksum"], "undeclared");
        assert!(entry.extra.get("declared_sha256").is_none());
        assert_eq!(entry.extra["counts"]["candidates"], 7);
        assert!(imported_entry(&object, "tenant", EVENT_ID, None, &imported(None)).is_err());
    }
}
