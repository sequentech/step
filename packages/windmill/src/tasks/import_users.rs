// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::document::get_document;
use crate::postgres::maintenance::vacuum_analyze_direct;
use crate::services::audit_log::{AdminAction, AdminChange, AuditActor};
use crate::services::database::get_hasura_pool;
use crate::services::documents::get_document_as_temp_file;
use crate::services::electoral_log::ElectoralLogAdminContext;
use crate::services::import::import_users::{import_users_file, ImportUsersPrivileges};
use crate::services::import::rejection::problems_in;
use crate::services::task_audit;
use crate::services::tasks_execution::*;
use crate::types::error::{Error, Result};
use anyhow::{anyhow, Context};
use celery::error::TaskError;
use deadpool_postgres::{Client as DbClient, Transaction as _};
use deadpool_postgres::{GenericClient, Transaction};
use sequent_core::election_config::import_problems;
use sequent_core::services::keycloak::get_client_credentials;
use sequent_core::services::s3;
use sequent_core::services::{keycloak, reports};
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::util::integrity_check::{integrity_check, HashFileVerifyError};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::io::Seek;
use tempfile::NamedTempFile;
use tracing::{error, info, instrument};
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
    ) -> anyhow::Result<(NamedTempFile, u8)> {
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
        Ok((temp_file, separator))
    }
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 2)]
pub async fn import_users(body: ImportUsersBody, task_execution: TasksExecution) -> Result<()> {
    let mut hasura_db_client: DbClient = match get_hasura_pool().await.get().await {
        Ok(client) => client,
        Err(err) => {
            update_fail(&task_execution, "Failed to get Hasura DB pool").await?;
            return Err(Error::String(format!(
                "Error getting Hasura DB pool: {}",
                err
            )));
        }
    };

    let hasura_transaction = match hasura_db_client.transaction().await {
        Ok(transaction) => transaction,
        Err(err) => {
            update_fail(&task_execution, "Failed to start Hasura transaction").await?;
            return Err(Error::String(format!(
                "Error starting Hasura transaction: {err}"
            )));
        }
    };

    let (mut voters_file, separator) =
        match body.get_s3_document_as_temp_file(&hasura_transaction).await {
            Ok(result) => result,
            Err(err) => {
                update_fail(
                    &task_execution,
                    "Error obtaining voters file from S3 as temp file",
                )
                .await?;
                return Err(Error::String(format!(
                    "Error obtaining voters file from S3: {err}"
                )));
            }
        };
    voters_file.rewind()?;

    match body.sha256.clone() {
        Some(hash) if !hash.is_empty() => match integrity_check(&voters_file, hash) {
            Ok(_) => {
                info!("Hash verified !");
            }
            Err(HashFileVerifyError::HashMismatch(input_hash, gen_hash)) => {
                let err_str = format!("Failed to verify the integrity: Hash of voters file: {gen_hash} does not match with the input hash: {input_hash}");
                update_fail_with_problems(
                    &task_execution,
                    &err_str,
                    &[import_problems::checksum_mismatch(&input_hash, &gen_hash)],
                )
                .await?;
                return Err(Error::String(err_str));
            }
            Err(err) => {
                let err_str = format!("Failed to verify the integrity: {err:?}");
                update_fail(&task_execution, &err_str).await?;
                return Err(err_str.into());
            }
        },
        _ => {
            info!("No hash provided, skipping integrity check");
        }
    }

    let audit_entry = match body.audit_entry(&mut voters_file, separator) {
        Ok(entry) => entry,
        Err(err) => {
            let err_str = format!("Error preparing the audit entry: {err:#}");
            update_fail(&task_execution, &err_str).await?;
            return Err(Error::String(err_str));
        }
    };

    match import_users_file(
        &hasura_transaction,
        &voters_file,
        separator,
        body.election_event_id.clone(),
        body.tenant_id,
        body.is_admin,
        body.may_write_secret_attributes,
        body.secret_write_initiator.as_ref(),
        body.privileges,
    )
    .await
    {
        Ok(_) => {
            // Execute database maintenance
            info!("Performing mainteinance after users import.");
            vacuum_analyze_direct().await?;
            ()
        }
        Err(err) => {
            match problems_in(&err) {
                Some(problems) => {
                    update_fail_with_problems(&task_execution, &err.to_string(), &problems).await?
                }
                None => update_fail(&task_execution, &err.to_string()).await?,
            }
            return Err(Error::String(format!("Error importing users file: {err}")));
        }
    }

    if let Err(err) = task_audit::record(audit_entry).await {
        let err_str = format!("Users imported, but the audit entry could not be queued: {err:#}");
        update_fail(&task_execution, &err_str).await?;
        return Err(Error::String(err_str));
    }

    update_complete(&task_execution, Some(body.document_id.clone()))
        .await
        .context("Failed to update task execution status to COMPLETED")?;

    Ok(())
}

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
