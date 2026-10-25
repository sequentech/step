// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The platform side of applying a delivery: reads the target's state for the
//! previews and runs the existing importers for the submissions, each with
//! its own task row, so every section is recorded the way its standalone
//! import is.

use super::configuration_package_apply::{
    PackageTarget, SectionPreview, SectionSubmission, TaskState,
};
use super::configuration_package_sections::{
    absent_admin_rows, admin_changes, event_changes, merged_tenant, read_admin_rows,
    read_group_roles, role_changes, tenant_configuration_changes, tenant_settings_changes,
    trustee_changes,
};
use crate::postgres::election_event::get_election_event_by_id_if_exist;
use crate::postgres::tasks_execution::get_task_by_id;
use crate::postgres::tenant::get_tenant_by_id;
use crate::postgres::trustee::{get_all_trustees, insert_trustee};
use crate::services::audit_log::AuditActor;
use crate::services::database::get_hasura_pool;
use crate::services::documents::upload_and_return_document;
use crate::services::export::export_tenant::tenant_csv;
use crate::services::keycloak::map_realm_data;
use crate::services::tasks_execution::{post, update_fail};
use crate::tasks::import_election_event::ImportElectionEventBody;
use crate::tasks::import_tenant_config::ImportOptions;
use crate::tasks::import_users::ImportUsersBody;
use crate::types::tasks::ETasksExecution;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use deadpool_postgres::Client as DbClient;
use sequent_core::election_config::archive::Artifact;
use sequent_core::election_config::manifest::read_zip;
use sequent_core::election_config::package_contract::Section;
use sequent_core::election_config::tenant_package::TenantConfiguration;
use sequent_core::election_config::tenant_package::TrusteePublic;
use sequent_core::services::keycloak::{get_tenant_realm, KeycloakAdminClient};
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::types::hasura::extra::TasksExecutionStatus;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;

/// Who is applying and with what rights: set by the server from the
/// requesting administrator's token, never from the package.
#[derive(Debug, Clone)]
pub struct ApplyContext {
    pub tenant_id: String,
    pub executed_by: String,
    pub audit_actor: Option<AuditActor>,
    /// The event import's body without a document: importer, privileges,
    /// messaging permission, secret-attribute rights.
    pub event_import: ImportElectionEventBody,
    pub package_sha256: String,
    /// The whole signed package when the delivery was signed: the event
    /// importer admits it itself, so the tenant's signing policy applies to
    /// the event exactly as it does to a standalone import.
    pub signed_package: Option<Vec<u8>>,
    /// Groups the package's roles section defines, so administrators may name
    /// them before that section has run.
    pub package_groups: BTreeSet<String>,
}

/// The event's id on this tenant: derived from the tenant and the package, so
/// applying the same package again targets the same event instead of
/// importing a second one.
pub fn derived_event_id(tenant_id: &str, package_sha256: &str) -> String {
    let digest = Sha256::digest(
        format!("sequent.package-application/1\n{tenant_id}\n{package_sha256}").as_bytes(),
    );
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    uuid::Builder::from_random_bytes(bytes)
        .into_uuid()
        .to_string()
}

pub struct PlatformTarget {
    pub context: ApplyContext,
}

fn one<'a>(files: &'a [Artifact], what: &str) -> Result<&'a Artifact> {
    match files {
        [file] => Ok(file),
        _ => Err(anyhow!(
            "the package carries {} {what} files; exactly one is applied",
            files.len()
        )),
    }
}

impl PlatformTarget {
    async fn realm(&self) -> Result<keycloak::types::RealmRepresentation> {
        let client = KeycloakAdminClient::new().await?;
        let pub_client = KeycloakAdminClient::pub_new().await?;
        client
            .get_realm(&pub_client, &get_tenant_realm(&self.context.tenant_id))
            .await
            .map_err(|e| anyhow!("Error reading the tenant realm: {e:?}"))
    }

    async fn group_roles(
        &self,
        realm: &keycloak::types::RealmRepresentation,
        wanted: &BTreeSet<String>,
    ) -> Result<(BTreeMap<String, BTreeSet<String>>, BTreeSet<String>)> {
        let client = KeycloakAdminClient::new().await?;
        let pub_client = KeycloakAdminClient::pub_new().await?;
        let (_, groups, roles) = map_realm_data(realm);
        let mut current = BTreeMap::new();
        for group in groups {
            let (Some(name), Some(id)) = (group.name.clone(), group.id.clone()) else {
                continue;
            };
            // Only the groups the package lists are compared or changed.
            if !wanted.contains(&name) {
                current.insert(name, BTreeSet::new());
                continue;
            }
            let assigned = client
                .get_group_assigned_roles(&self.context.tenant_id, &id, &pub_client)
                .await
                .map_err(|e| anyhow!("Error reading roles of group {name}: {e}"))?;
            current.insert(name, assigned.into_iter().filter_map(|r| r.name).collect());
        }
        let realm_roles = roles.into_iter().filter_map(|r| r.name).collect();
        Ok((current, realm_roles))
    }

    async fn tenant(&self) -> Result<sequent_core::types::hasura::core::Tenant> {
        let mut db: DbClient = get_hasura_pool().await.get().await?;
        let transaction = db.transaction().await?;
        get_tenant_by_id(&transaction, &self.context.tenant_id).await
    }

    fn tenant_configuration(&self, files: &[Artifact]) -> Result<TenantConfiguration> {
        serde_json::from_slice(&one(files, "tenant configuration")?.bytes)
            .context("the tenant configuration is not the package's tenant record")
    }

    async fn existing_usernames(&self, usernames: &[String]) -> Result<BTreeSet<String>> {
        let client = KeycloakAdminClient::new().await?;
        let realm = get_tenant_realm(&self.context.tenant_id);
        let mut existing = BTreeSet::new();
        for username in usernames {
            if client.username_exists(&realm, username).await? {
                existing.insert(username.to_lowercase());
            }
        }
        Ok(existing)
    }

    fn event_document(&self, files: &[Artifact]) -> Result<Vec<u8>> {
        Ok(match &self.context.signed_package {
            Some(package) => package.clone(),
            None => one(files, "election event")?.bytes.clone(),
        })
    }

    /// Upload one file as a document for its import.
    async fn stage(&self, name: &str, media_type: &str, bytes: &[u8]) -> Result<String> {
        let mut file = tempfile::NamedTempFile::new()?;
        file.write_all(bytes)?;
        file.flush()?;
        let path = file
            .path()
            .to_str()
            .ok_or_else(|| anyhow!("temporary path is not UTF-8"))?
            .to_string();
        let mut db: DbClient = get_hasura_pool().await.get().await?;
        let transaction = db.transaction().await?;
        let document = upload_and_return_document(
            &transaction,
            &path,
            bytes.len() as u64,
            media_type,
            &self.context.tenant_id,
            None,
            name,
            None,
            false,
        )
        .await?;
        transaction.commit().await?;
        Ok(document.id)
    }

    async fn task(&self, task_id: Option<&str>) -> Result<TasksExecution> {
        let id = task_id.ok_or_else(|| anyhow!("the section was not begun"))?;
        get_task_by_id(id).await
    }

    /// A section's import counts as submitted only when its task row says it
    /// succeeded; anything else is the task's own failure message.
    async fn terminal(
        &self,
        task: &TasksExecution,
        ran: crate::types::error::Result<()>,
    ) -> Result<SectionSubmission> {
        let row = get_task_by_id(&task.id).await?;
        let status = row.execution_status.clone();
        match ran {
            Ok(()) if status == TasksExecutionStatus::SUCCESS.to_string() => {
                Ok(SectionSubmission {
                    task_id: Some(task.id.clone()),
                    detail: json!({"status": status}),
                })
            }
            Ok(()) => Err(anyhow!("task {} ended as {status}", task.id)),
            Err(error) => Err(anyhow!("task {} failed: {error:?}", task.id)),
        }
    }

    fn zip_one(name: &str, bytes: &[u8]) -> Result<Vec<u8>> {
        let mut out = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        out.start_file(name, zip::write::SimpleFileOptions::default())?;
        out.write_all(bytes)?;
        Ok(out.finish()?.into_inner())
    }

    async fn import_tenant_config(
        &self,
        file: &Artifact,
        options: ImportOptions,
        task_id: Option<&str>,
    ) -> Result<SectionSubmission> {
        let zipped = Self::zip_one(&file.name, &file.bytes)?;
        let document_id = self
            .stage(
                "configuration-package-tenant.zip",
                "application/zip",
                &zipped,
            )
            .await?;
        let task = self.task(task_id).await?;
        let ran = crate::tasks::import_tenant_config::execute(
            options,
            self.context.tenant_id.clone(),
            document_id,
            None,
            task.clone(),
            self.context.audit_actor.clone(),
        )
        .await;
        self.terminal(&task, ran).await
    }
}

#[async_trait]
impl PackageTarget for PlatformTarget {
    async fn preview(&self, section: Section, files: &[Artifact]) -> Result<SectionPreview> {
        match section {
            Section::TenantConfiguration => {
                let wanted = self.tenant_configuration(files)?;
                Ok(tenant_configuration_changes(&self.tenant().await?, &wanted))
            }
            Section::TenantSettings => {
                let wanted: Value = serde_json::from_slice(&one(files, "tenant settings")?.bytes)
                    .context("tenant settings are not JSON")?;
                let realm = self.realm().await?;
                let localization = realm
                    .localization_texts
                    .as_ref()
                    .map(serde_json::to_value)
                    .transpose()?;
                Ok(tenant_settings_changes(
                    realm.display_name.as_deref(),
                    localization.as_ref(),
                    &wanted,
                ))
            }
            Section::RolePermissions => {
                let wanted = read_group_roles(&one(files, "roles")?.bytes)?;
                let names = wanted.iter().map(|r| r.group.clone()).collect();
                let realm = self.realm().await?;
                let (current, realm_roles) = self.group_roles(&realm, &names).await?;
                let listed: BTreeMap<String, BTreeSet<String>> = current
                    .into_iter()
                    .filter(|(name, _)| names.contains(name))
                    .collect();
                Ok(role_changes(&wanted, &listed, &realm_roles))
            }
            Section::AdminAccounts => {
                let rows = read_admin_rows(&one(files, "administrators")?.bytes)?;
                let usernames: Vec<String> = rows.iter().map(|r| r.username.clone()).collect();
                let existing = self.existing_usernames(&usernames).await?;
                let realm = self.realm().await?;
                let (_, groups, _) = map_realm_data(&realm);
                let mut known: BTreeSet<String> =
                    groups.into_iter().filter_map(|g| g.name).collect();
                known.extend(self.context.package_groups.iter().cloned());
                Ok(admin_changes(&rows, &existing, &known))
            }
            Section::Trustees => {
                let wanted: Vec<TrusteePublic> =
                    serde_json::from_slice(&one(files, "trustees")?.bytes)
                        .context("trustees are not the public trustee list")?;
                let mut db: DbClient = get_hasura_pool().await.get().await?;
                let transaction = db.transaction().await?;
                let existing = get_all_trustees(&transaction, &self.context.tenant_id).await?;
                Ok(trustee_changes(&wanted, &existing))
            }
            Section::ElectionEvent => {
                // Validated by the import itself; the preview answers whether
                // this package's event is already on the tenant.
                self.event_document(files)?;
                let event_id =
                    derived_event_id(&self.context.tenant_id, &self.context.package_sha256);
                let mut db: DbClient = get_hasura_pool().await.get().await?;
                let transaction = db.transaction().await?;
                let exists = get_election_event_by_id_if_exist(
                    &transaction,
                    &self.context.tenant_id,
                    &event_id,
                )
                .await?
                .is_some();
                Ok(event_changes(&event_id, exists, vec![]))
            }
        }
    }

    async fn begin(&self, section: Section) -> Result<Option<String>> {
        let task_type = match section {
            Section::TenantConfiguration | Section::TenantSettings | Section::RolePermissions => {
                ETasksExecution::IMPORT_TENANT_CONFIG
            }
            Section::AdminAccounts => ETasksExecution::IMPORT_USERS,
            Section::ElectionEvent => ETasksExecution::IMPORT_ELECTION_EVENT,
            // One transaction in this process: nothing outlives the run.
            Section::Trustees => return Ok(None),
        };
        let task = post(
            &self.context.tenant_id,
            None,
            task_type,
            &self.context.executed_by,
        )
        .await?;
        Ok(Some(task.id))
    }

    /// The caller holds the application's claim, and every import runs in the
    /// claiming run's own process, so a row still in progress belongs to a run
    /// that stopped: its import cannot continue. It is closed as interrupted.
    async fn submission_state(&self, task_id: &str) -> Result<TaskState> {
        let task = get_task_by_id(task_id).await?;
        if task.tenant_id != self.context.tenant_id {
            return Err(anyhow!("task {task_id} belongs to another tenant"));
        }
        Ok(
            if task.execution_status == TasksExecutionStatus::SUCCESS.to_string() {
                TaskState::Succeeded
            } else if task.execution_status == TasksExecutionStatus::FAILED.to_string() {
                TaskState::Failed
            } else {
                update_fail(
                    &task,
                    "interrupted: the package application that ran it stopped",
                )
                .await?;
                TaskState::Failed
            },
        )
    }

    async fn apply(
        &self,
        section: Section,
        files: &[Artifact],
        task_id: Option<&str>,
    ) -> Result<SectionSubmission> {
        match section {
            Section::TenantConfiguration => {
                let wanted = self.tenant_configuration(files)?;
                let merged = merged_tenant(&self.tenant().await?, &wanted);
                let file = Artifact {
                    name: "tenant_configuration.csv".to_string(),
                    bytes: tenant_csv(merged)?,
                };
                self.import_tenant_config(
                    &file,
                    ImportOptions {
                        include_tenant: Some(true),
                        include_keycloak: Some(false),
                        include_roles: Some(false),
                    },
                    task_id,
                )
                .await
            }
            Section::TenantSettings => {
                self.import_tenant_config(
                    one(files, "tenant settings")?,
                    ImportOptions {
                        include_tenant: Some(false),
                        include_keycloak: Some(true),
                        include_roles: Some(false),
                    },
                    task_id,
                )
                .await
            }
            Section::RolePermissions => {
                self.import_tenant_config(
                    one(files, "roles")?,
                    ImportOptions {
                        include_tenant: Some(false),
                        include_keycloak: Some(false),
                        include_roles: Some(true),
                    },
                    task_id,
                )
                .await
            }
            Section::AdminAccounts => {
                let file = one(files, "administrators")?;
                let rows = read_admin_rows(&file.bytes)?;
                let usernames: Vec<String> = rows.iter().map(|r| r.username.clone()).collect();
                let existing = self.existing_usernames(&usernames).await?;
                let absent = absent_admin_rows(&file.bytes, &existing)?;
                let document_id = self.stage("admin_users.csv", "text/csv", &absent).await?;
                let task = self.task(task_id).await?;
                let ran = crate::tasks::import_users::execute(
                    ImportUsersBody {
                        tenant_id: self.context.tenant_id.clone(),
                        document_id,
                        election_event_id: None,
                        is_admin: true,
                        may_write_secret_attributes: false,
                        secret_write_initiator: None,
                        privileges: self.context.event_import.privileges,
                        sha256: None,
                        audit_actor: self.context.audit_actor.clone(),
                    },
                    task.clone(),
                )
                .await;
                self.terminal(&task, ran).await
            }
            Section::Trustees => {
                let wanted: Vec<TrusteePublic> =
                    serde_json::from_slice(&one(files, "trustees")?.bytes)?;
                let mut db: DbClient = get_hasura_pool().await.get().await?;
                let transaction = db.transaction().await?;
                let existing = get_all_trustees(&transaction, &self.context.tenant_id).await?;
                let mut registered = Vec::new();
                for trustee in &wanted {
                    if existing
                        .iter()
                        .any(|t| t.name.as_deref() == Some(trustee.name.as_str()))
                    {
                        continue;
                    }
                    let id = insert_trustee(
                        &transaction,
                        &self.context.tenant_id,
                        &trustee.name,
                        &trustee.signing_public_key,
                        &trustee.share_encryption_public_key,
                    )
                    .await?;
                    registered.push(json!({"name": trustee.name, "id": id}));
                }
                // One transaction: either every absent trustee is registered
                // or none is.
                transaction.commit().await?;
                Ok(SectionSubmission {
                    task_id: None,
                    detail: json!({"registered": registered}),
                })
            }
            Section::ElectionEvent => {
                let document = self.event_document(files)?;
                let document_id = self
                    .stage(
                        "configuration-package-event.zip",
                        "application/zip",
                        &document,
                    )
                    .await?;
                let task = self.task(task_id).await?;
                let event_id =
                    derived_event_id(&self.context.tenant_id, &self.context.package_sha256);
                let mut body = self.context.event_import.clone();
                body.tenant_id = self.context.tenant_id.clone();
                body.document_id = document_id;
                body.check_only = Some(false);
                body.password = None;
                body.sha256 = None;
                let ran = crate::tasks::import_election_event::execute(
                    body,
                    event_id.clone(),
                    self.context.tenant_id.clone(),
                    task.clone(),
                )
                .await;
                let mut submission = self.terminal(&task, ran).await?;
                submission.detail["electionEventId"] = json!(event_id);
                Ok(submission)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_event_id_is_stable_per_tenant_and_package_and_a_valid_v4() {
        let a = derived_event_id("t1", "aaaa");
        assert_eq!(a, derived_event_id("t1", "aaaa"));
        assert_ne!(a, derived_event_id("t2", "aaaa"));
        assert_ne!(a, derived_event_id("t1", "bbbb"));
        assert!(sequent_core::services::uuid_validation::parse_uuid_v4(&a).is_ok());
    }
}
