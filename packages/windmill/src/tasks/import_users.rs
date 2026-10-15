// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::document::get_document;
use crate::postgres::maintenance::vacuum_analyze_direct;
use crate::postgres::tasks_execution::get_task_by_id_with_transaction;
use crate::postgres::voter_import::{
    complete_import_task, fail_import_task, lock_voter_imports, record_voter_import,
};
use crate::services::audit_log::stage;
use crate::services::audit_log::{AdminAction, AdminChange, AuditActor};
use crate::services::database::get_hasura_pool;
use crate::services::documents::get_document_as_temp_file;
use crate::services::electoral_log::ElectoralLogAdminContext;
use crate::services::import::import_users::{import_users_file, ImportUsersPrivileges};
use crate::services::import::rejection::{problems_in, problems_of};
use crate::services::serialize_tasks_logs::append_general_log;
use crate::services::task_audit;
use crate::services::tasks_execution::failure_annotations;
use crate::tasks::audit_log_outbox::kick_audit_log_outbox;
use crate::types::error::{Error, Result};
use crate::types::tasks::ETasksExecution;
use anyhow::{anyhow, Context};
use celery::error::TaskError;
use deadpool_postgres::Transaction;
use sequent_core::election_config::import_problems;
use sequent_core::services::uuid_validation::parse_uuid_v4;
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::types::hasura::extra::TasksExecutionStatus;
use sequent_core::types::voter_import::{FileIntegrityPolicy, VotersImportedEvidence};
use sequent_core::util::integrity_check::{integrity_check, HashFileVerifyError};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::io::Seek;
use tempfile::NamedTempFile;
use tracing::{info, instrument, warn};
#[derive(Deserialize, Debug, Clone, Serialize)]
pub struct ImportUsersBody {
    pub tenant_id: String,
    pub document_id: String,
    pub election_event_id: Option<String>,
    #[serde(default = "default_is_admin")]
    pub is_admin: bool,
    #[serde(default)]
    pub may_write_secret_attributes: bool,
    /// Who authorized writing secret attributes; recorded in the electoral
    /// log when the CSV actually contains a secret column.
    #[serde(default)]
    pub secret_write_initiator: Option<ElectoralLogAdminContext>,
    /// What the importing user may grant through the file, derived by the
    /// server from that user's permissions.
    #[serde(default)]
    pub privileges: ImportUsersPrivileges,
    pub sha256: Option<String>,
    /// The admin who asked for the import, for its audit entry. Set by the
    /// server from that user's token.
    #[serde(default)]
    pub audit_actor: Option<AuditActor>,
}

fn default_is_admin() -> bool {
    false
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ImportUsersOutput {
    pub task_execution: TasksExecution,
}

impl ImportUsersBody {
    /// The import's entry: in the election event's log for voters, in the
    /// tenant's for staff.
    fn audit_entry(&self, file: &mut NamedTempFile, separator: u8) -> anyhow::Result<AdminChange> {
        let target = task_audit::target(&self.tenant_id, self.election_event_id.as_deref())?;
        let (kind, named) = match target.election_event_id {
            Some(_) => ("voter", "voters"),
            None => ("staff_account", "staff accounts"),
        };
        let count = task_audit::csv_rows(file.as_file_mut(), separator)?;
        Ok(task_audit::bulk(
            self.audit_actor.clone(),
            target,
            kind,
            AdminAction::Import,
            format!("Imported {count} {named}"),
            json!({
                "count": count,
                "document_id": self.document_id,
                "file_sha256": task_audit::file_sha256(file.as_file_mut())?,
            }),
        ))
    }

    #[instrument(ret)]
    async fn get_s3_document_as_temp_file(
        &self,
        hasura_transaction: &Transaction<'_>,
    ) -> anyhow::Result<(NamedTempFile, u8, String)> {
        let document = get_document(
            hasura_transaction,
            self.tenant_id.as_str(),
            None,
            self.document_id.as_str(),
        )
        .await
        .with_context(|| "Error obtaining the document")?
        .ok_or(anyhow!("document not found"))?;

        let document_name = document.name.clone().unwrap_or_default();

        // Determine file type and set the appropriate separator
        let (postfix, separator) = if document_name.ends_with(".tsv") {
            (".tsv", b'\t')
        } else {
            (".csv", b',')
        };
        info!("postfix={postfix:?} separator={separator:?}");

        let temp_file = get_document_as_temp_file(self.tenant_id.as_str(), &document).await?;

        // Return the temporary file and the separator as a tuple
        Ok((temp_file, separator, document_name))
    }
}

/// Unknown policy values fail closed instead of falling back to OPTIONAL.
fn import_integrity_policy(
    presentation: Option<&serde_json::Value>,
) -> anyhow::Result<FileIntegrityPolicy> {
    match presentation
        .and_then(|value| value.get("file_integrity_policy"))
        .filter(|value| !value.is_null())
    {
        Some(value) => {
            serde_json::from_value(value.clone()).context("Invalid voter-file integrity policy")
        }
        None => Ok(FileIntegrityPolicy::default()),
    }
}

fn import_failure_problems(
    error: &anyhow::Error,
) -> Option<Vec<sequent_core::election_config::Problem>> {
    if let Some(HashFileVerifyError::HashMismatch(expected, actual)) =
        error.downcast_ref::<HashFileVerifyError>()
    {
        Some(vec![import_problems::checksum_mismatch(expected, actual)])
    } else {
        error
            .downcast_ref::<Error>()
            .and_then(problems_in)
            .or_else(|| problems_of(error))
    }
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 2)]
pub async fn import_users(body: ImportUsersBody, task_execution: TasksExecution) -> Result<()> {
    match run_import_users(&body, &task_execution).await {
        Ok(()) => Ok(()),
        Err(err) => {
            let problems = import_failure_problems(&err);
            let mut client = get_hasura_pool()
                .await
                .get()
                .await
                .context("Failed to get import failure connection")?;
            let tx = client.transaction().await?;
            let logs = serde_json::to_value(append_general_log(
                &task_execution.logs,
                &format!("Error: {err:#}"),
            ))?;
            let annotations = failure_annotations(problems.as_deref().unwrap_or_default())?;
            fail_import_task(
                &tx,
                &body.tenant_id,
                body.election_event_id.as_deref(),
                &task_execution.id,
                logs,
                annotations,
            )
            .await?;
            tx.commit().await?;
            Err(Error::String(format!(
                "Error importing users file: {err:#}"
            )))
        }
    }
}

async fn run_import_users(
    body: &ImportUsersBody,
    task_execution: &TasksExecution,
) -> anyhow::Result<()> {
    let event_id = body
        .election_event_id
        .as_deref()
        .filter(|id| !id.is_empty());
    let is_admin = event_id.is_none();
    let mut client = get_hasura_pool()
        .await
        .get()
        .await
        .context("Failed to get Hasura DB pool")?;
    let tx = client
        .transaction()
        .await
        .context("Failed to start Hasura transaction")?;
    if let Some(event) = event_id {
        lock_voter_imports(&tx, &body.tenant_id, event).await?;
    } else {
        // Staff imports also serialize retries of one task, without sharing a
        // lock with the event census.
        tx.query_one(
            "SELECT pg_advisory_xact_lock(hashtextextended($1, 0))",
            &[&format!("staff-import:{}", task_execution.id)],
        )
        .await?;
    }
    let persisted =
        get_task_by_id_with_transaction(&tx, &body.tenant_id, &task_execution.id).await?;
    anyhow::ensure!(
        persisted.task_type == ETasksExecution::IMPORT_USERS.to_string()
            && persisted.tenant_id == body.tenant_id
            && persisted
                .election_event_id
                .as_deref()
                .filter(|id| !id.is_empty())
                == event_id,
        "Import does not match the persisted task execution"
    );
    if persisted.execution_status == TasksExecutionStatus::SUCCESS.to_string() {
        anyhow::ensure!(
            persisted
                .annotations
                .as_ref()
                .and_then(|value| value.get("document_id"))
                .and_then(|value| value.as_str())
                == Some(body.document_id.as_str()),
            "Completed import document does not match this task"
        );
        kick_audit_log_outbox();
        return Ok(());
    }

    // Failed and cancelled tasks are terminal. Resubmitting creates a fresh
    // authorization/task; a late delivery must not restart a failed enqueue.
    anyhow::ensure!(
        persisted.execution_status == TasksExecutionStatus::IN_PROGRESS.to_string(),
        "Import task is no longer in progress"
    );

    let policy = if let Some(event) = event_id {
        let row = tx.query_one(
            "SELECT presentation FROM sequent_backend.election_event WHERE tenant_id = $1 AND id = $2 FOR SHARE",
            &[&parse_uuid_v4(&body.tenant_id)?, &parse_uuid_v4(event)?],
        ).await.context("Election event not found")?;
        let presentation: Option<serde_json::Value> = row.try_get(0)?;
        import_integrity_policy(presentation.as_ref())?
    } else {
        FileIntegrityPolicy::OPTIONAL
    };
    let declared = policy
        .validate_declared_sha256(body.sha256.as_deref())
        .map_err(anyhow::Error::msg)?;
    let (mut voters_file, separator, file_name) = body.get_s3_document_as_temp_file(&tx).await?;
    voters_file.rewind()?;
    if let Some(hash) = declared.as_ref() {
        integrity_check(&voters_file, hash.clone())
            .context("Failed to verify voter-file integrity")?;
    }
    let computed_sha256 = task_audit::file_sha256(voters_file.as_file_mut())?;
    let audit_entry = body.audit_entry(&mut voters_file, separator)?;
    let summary = import_users_file(
        &tx,
        &voters_file,
        separator,
        body.election_event_id.clone(),
        body.tenant_id.clone(),
        is_admin,
        body.may_write_secret_attributes,
        body.secret_write_initiator.as_ref(),
        body.privileges,
    )
    .await?;

    // Keycloak has now committed. The receipt, completion status and delivery
    // request share this Hasura transaction. A crash before this commit leaves
    // an incomplete task, so readiness cannot accept stale census evidence.
    stage(&tx, &audit_entry).await?;
    if let Some(event) = event_id {
        record_voter_import(
            &tx,
            &body.tenant_id,
            event,
            &task_execution.id,
            body.audit_actor.as_ref(),
            VotersImportedEvidence {
                import_id: String::new(),
                document_id: body.document_id.clone(),
                file_name,
                sha256: computed_sha256,
                declared_sha256: declared,
                counts_by_post: summary.counts_by_post,
                imported_at: chrono::Utc::now().to_rfc3339(),
            },
        )
        .await?;
    } else {
        crate::postgres::tasks_execution::merge_task_execution_annotations(
            &tx,
            &body.tenant_id,
            &task_execution.id,
            &json!({"document_id": body.document_id}),
        )
        .await?;
    }
    complete_import_task(&tx, &body.tenant_id, &task_execution.id).await?;
    tx.commit()
        .await
        .context("Users committed but completed-import receipt could not be committed")?;
    kick_audit_log_outbox();
    // Maintenance cannot turn an already-committed, certified import into a
    // failed task or cause the worker to insert its users again on retry.
    if let Err(err) = vacuum_analyze_direct().await {
        warn!("Users imported; deferred database maintenance failed: {err:#}");
    }
    Ok(())
}

#[cfg(test)]
#[path = "../../tests/support/schema.rs"]
mod import_test_schema;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::audit_log::{AuditObject, AuditOrigin};
    use serde_json::json;
    use std::io::Write;

    const TENANT_ID: &str = "4f1c6b38-9f5e-4a59-8d3b-2a7f0c1e5d61";
    const EVENT_ID: &str = "b7e2d915-3c4a-4e8f-9a61-5d0f2c8b7e43";
    const FILE: &[u8] = b"username,email\nada,ada@example.test\nbob,bob@example.test\n";

    fn file_sha256() -> String {
        use sha2::{Digest, Sha256};
        hex::encode(Sha256::digest(FILE))
    }

    fn body(json: serde_json::Value) -> ImportUsersBody {
        serde_json::from_value(json).unwrap()
    }

    fn file() -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(FILE).unwrap();
        file
    }

    #[tokio::test]
    async fn a_failed_import_cannot_resume_document_or_voter_processing() {
        let pool = super::import_test_schema::pool().await;
        let mut client = pool.get().await.unwrap();
        let database: String = client
            .query_one("SELECT current_database()", &[])
            .await
            .unwrap()
            .get(0);
        std::env::set_var("HASURA_DB__DBNAME", database);
        let tenant = uuid::Uuid::new_v4();
        let event = uuid::Uuid::new_v4();
        let task_id = uuid::Uuid::new_v4();
        let tx = client.transaction().await.unwrap();
        tx.execute(
            "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
            &[&tenant, &format!("failed-import-{tenant}")],
        )
        .await
        .unwrap();
        tx.execute("INSERT INTO sequent_backend.election_event (id, tenant_id, encryption_protocol) VALUES ($1, $2, 'RSA256')", &[&event, &tenant]).await.unwrap();
        tx.execute("INSERT INTO sequent_backend.tasks_execution (id, tenant_id, election_event_id, name, type, execution_status, executed_by_user) VALUES ($1, $2, $3, 'Import voters', 'IMPORT_USERS', 'FAILED', 'ada')", &[&task_id, &tenant, &event]).await.unwrap();
        let task = get_task_by_id_with_transaction(&tx, &tenant.to_string(), &task_id.to_string())
            .await
            .unwrap();
        tx.commit().await.unwrap();
        let input = body(json!({
            "tenant_id":tenant.to_string(), "election_event_id":event.to_string(),
            "document_id":uuid::Uuid::new_v4().to_string(), "sha256":"ab".repeat(32),
        }));
        let error = run_import_users(&input, &task).await.unwrap_err();
        assert_eq!(error.to_string(), "Import task is no longer in progress");
        let tx = client.transaction().await.unwrap();
        let stored =
            get_task_by_id_with_transaction(&tx, &tenant.to_string(), &task_id.to_string())
                .await
                .unwrap();
        assert_eq!(stored.execution_status, "FAILED");
        let count: i64 = tx.query_one("SELECT count(*) FROM sequent_backend.audit_log_outbox WHERE tenant_id = $1 AND statement_kind = 'VotersImported'", &[&tenant]).await.unwrap().get(0);
        assert_eq!(count, 0);
    }

    #[test]
    fn checksum_mismatch_keeps_its_structured_problem_through_context() {
        let error = anyhow::Error::new(HashFileVerifyError::HashMismatch(
            "aa".repeat(32),
            "bb".repeat(32),
        ))
        .context("Failed to verify voter-file integrity");
        let problems = import_failure_problems(&error).unwrap();
        assert_eq!(
            problems,
            vec![import_problems::checksum_mismatch(
                &"aa".repeat(32),
                &"bb".repeat(32)
            )]
        );
    }

    #[test]
    fn importer_validation_keeps_its_structured_problem_through_error_wrapping() {
        let rejection = crate::services::import::rejection::reject(
            "voters file",
            import_problems::duplicate_column("email"),
        );
        let error = anyhow::Error::new(Error::Anyhow(rejection)).context("Error importing voters");
        let problems = import_failure_problems(&error).unwrap();
        assert_eq!(problems, vec![import_problems::duplicate_column("email")]);
    }

    #[test]
    fn legacy_policy_is_optional_but_unknown_policy_does_not_disable_integrity() {
        assert_eq!(
            import_integrity_policy(None).unwrap(),
            FileIntegrityPolicy::OPTIONAL
        );
        assert_eq!(
            import_integrity_policy(Some(&json!({}))).unwrap(),
            FileIntegrityPolicy::OPTIONAL
        );
        assert_eq!(
            import_integrity_policy(Some(&json!({"file_integrity_policy": "REQUIRED_SHA256"})))
                .unwrap(),
            FileIntegrityPolicy::REQUIRED_SHA256
        );
        assert!(import_integrity_policy(Some(&json!({"file_integrity_policy": "TYPO"}))).is_err());
    }

    #[test]
    fn a_staff_import_is_one_entry_in_the_tenant_log() {
        let body = body(json!({
            "tenant_id": TENANT_ID, "document_id": "document-1", "sha256": null,
            "audit_actor": {"user_id": "admin-id", "username": "ada", "role": "admin-user"},
        }));
        let entry = body.audit_entry(&mut file(), b',').unwrap();

        assert_eq!(entry.target.election_event_id, None);
        assert_eq!(entry.object, AuditObject::new("staff_account", "*", None));
        assert_eq!(entry.action, AdminAction::Import);
        assert_eq!(entry.origin, AuditOrigin::Windmill);
        assert_eq!(entry.actor.username.as_deref(), Some("ada"));
        assert_eq!(entry.extra["count"], 2);
        assert_eq!(entry.extra["document_id"], "document-1");
        assert_eq!(entry.extra["file_sha256"], file_sha256());
        assert_eq!(entry.describe(), "Imported 2 staff accounts");
    }

    #[test]
    fn a_voter_import_goes_to_its_election_event_log() {
        let body = body(json!({
            "tenant_id": TENANT_ID, "document_id": "document-1",
            "election_event_id": EVENT_ID, "sha256": null,
        }));
        let entry = body.audit_entry(&mut file(), b',').unwrap();

        assert_eq!(
            entry.target.election_event_id.map(|id| id.to_string()),
            Some(EVENT_ID.to_string())
        );
        assert_eq!(entry.object.kind, "voter");
        assert_eq!(entry.describe(), "Imported 2 voters");
    }

    #[test]
    fn a_message_queued_before_the_actor_existed_still_reads() {
        let body = body(json!({
            "tenant_id": TENANT_ID, "document_id": "document-1", "sha256": null,
        }));
        assert_eq!(body.audit_actor, None);
        assert_eq!(
            body.audit_entry(&mut file(), b',').unwrap().actor,
            AuditActor::default()
        );
    }
}
