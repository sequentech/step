// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::audit_log::{AdminAction, AuditActor};
use crate::services::import::import_tenant_config::{
    import_tenant_config_zip, ImportedTenantConfig,
};
use crate::services::providers::transactions_provider::provide_hasura_transaction;
use crate::services::task_audit;
use crate::services::tasks_execution::{update_complete, update_fail};
use crate::{
    services::import::import_election_event::{self as import_election_event_service},
    types::error::Result,
};
use anyhow::{anyhow, Context};
use celery::error::TaskError;
use sequent_core::types::hasura::core::TasksExecution;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::{event, info, instrument, Level};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ImportOptions {
    pub include_tenant: Option<bool>,
    pub include_keycloak: Option<bool>,
    pub include_roles: Option<bool>,
}

/// What the tenant log's entry of an import says: the document, the SHA-256
/// of its file and the checksum declared, the sections asked for and the
/// sections imported.
fn imported_extra(
    object: &ImportOptions,
    document_id: &str,
    declared_sha256: Option<&str>,
    imported: &ImportedTenantConfig,
) -> Value {
    let mut extra =
        task_audit::imported_document(document_id, &imported.file_sha256, declared_sha256);
    extra["included"] = json!({
        "tenant": object.include_tenant.unwrap_or(false),
        "keycloak": object.include_keycloak.unwrap_or(false),
        "roles": object.include_roles.unwrap_or(false),
    });
    extra["sections"] = json!(imported.sections);
    extra["counts"] = json!({"sections": imported.sections.len()});
    extra
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn import_tenant_config(
    object: ImportOptions,
    tenant_id: String,
    document_id: String,
    sha256: Option<String>,
    task_execution: TasksExecution,
    audit_actor: Option<AuditActor>,
) -> Result<()> {
    let task_execution_clone = task_execution.clone();

    let object = object.clone();
    let tenant_id = tenant_id.clone();
    let task_execution = task_execution_clone.clone();

    let imported =
        match import_tenant_config_zip(object.clone(), &tenant_id, &document_id, sha256.clone())
            .await
        {
            Ok(imported) => imported,
            Err(err) => {
                update_fail(&task_execution, &err.to_string()).await?;
                return Err(
                    anyhow!("Error process tenant configuration documents: {:?}", err).into(),
                );
            }
        };

    let extra = imported_extra(&object, &document_id, sha256.as_deref(), &imported);
    let recorded =
        match task_audit::tenant_config(audit_actor, &tenant_id, AdminAction::Import, extra) {
            Ok(entry) => task_audit::record(entry).await,
            Err(err) => Err(err),
        };
    if let Err(err) = recorded {
        let err_str = format!(
            "Tenant configuration imported, but the audit entry could not be queued: {err:#}"
        );
        update_fail(&task_execution, &err_str).await?;
        return Err(anyhow!(err_str).into());
    }

    update_complete(&task_execution, Some(document_id.to_string()))
        .await
        .context("Failed to update task execution status to COMPLETED")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::import::import_tenant_config::TenantConfigSection;

    fn options(tenant: bool, keycloak: bool, roles: bool) -> ImportOptions {
        ImportOptions {
            include_tenant: Some(tenant),
            include_keycloak: Some(keycloak),
            include_roles: Some(roles),
        }
    }

    #[test]
    fn an_imported_configuration_names_its_file_and_the_sections_imported() {
        let imported = ImportedTenantConfig {
            file_sha256: "ABCD".into(),
            sections: vec![TenantConfigSection::Tenant, TenantConfigSection::Keycloak],
        };
        let extra = imported_extra(
            &options(true, true, true),
            "document-1",
            Some("abcd"),
            &imported,
        );
        assert_eq!(
            extra,
            json!({
                "document_id": "document-1",
                "file_sha256": "abcd",
                "declared_sha256": "abcd",
                "declared_checksum": "matches",
                "included": {"tenant": true, "keycloak": true, "roles": true},
                "sections": ["tenant", "keycloak"],
                "counts": {"sections": 2},
            })
        );
    }

    #[test]
    fn an_import_without_a_checksum_or_sections_says_so() {
        let options = ImportOptions {
            include_tenant: None,
            include_keycloak: Some(false),
            include_roles: None,
        };
        let imported = ImportedTenantConfig {
            file_sha256: "abcd".into(),
            sections: vec![],
        };
        let extra = imported_extra(&options, "document-1", None, &imported);
        assert_eq!(extra["declared_checksum"], "undeclared");
        assert_eq!(
            extra["included"],
            json!({"tenant": false, "keycloak": false, "roles": false})
        );
        assert_eq!(extra["sections"], json!([]));
        assert_eq!(extra["counts"]["sections"], 0);
    }
}
