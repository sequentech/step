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
    absent_admin_rows, admin_changes, admin_column_problems, event_changes, merged_tenant,
    read_admin_rows, read_group_roles, role_changes, tenant_configuration_changes,
    tenant_settings_changes, trustee_changes,
};
use super::configuration_package_sections::{
    admission_problem, census_usernames, event_document, messaging_problem,
    trustee_reference_problems, EventState,
};
use crate::postgres::election_event::get_election_event_by_id_if_exist;
use crate::postgres::keycloak_realm::{realm_usernames, user_groups};
use crate::postgres::tasks_execution::get_task_by_id;
use crate::postgres::tenant::get_tenant_by_id;
use crate::postgres::tenant::{lock_tenant, update_tenant_configuration};
use crate::postgres::trustee::{get_all_trustees, insert_trustee};
use crate::services::audit_log::AdminAction;
use crate::services::audit_log::AuditActor;
use crate::services::database::get_hasura_pool;
use crate::services::database::get_keycloak_pool;
use crate::services::documents::upload_and_return_document;
use crate::services::import::import_election_event::get_election_event_schema;
use crate::services::keycloak::map_realm_data;
use crate::services::task_audit;
use crate::services::tasks_execution::{post, update_complete, update_fail};
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
use sequent_core::election_config::tenant_package::secret_field_paths;
use sequent_core::election_config::tenant_package::TenantConfiguration;
use sequent_core::election_config::tenant_package::TrusteePublic;
use sequent_core::services::keycloak::{get_event_realm, get_tenant_realm, KeycloakAdminClient};
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
    /// Trustees the package registers, so the event's key ceremony may name
    /// them before that section has run.
    pub package_trustees: BTreeSet<String>,
    /// The verified package when the delivery was signed. Its freshness
    /// against the tenant's imports is checked in every section's preview,
    /// so it is checked again right before each section is applied.
    pub verified:
        Option<std::sync::Arc<sequent_core::election_config::package_verify::VerifiedPackage>>,
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

    /// Why a signed package may not be applied now: a revision not newer
    /// than the tenant's last import, or this same package imported into an
    /// event other than the one this application imports (then the event
    /// section would be refused after the tenant sections had changed).
    async fn admission_blocker(&self) -> Result<Option<String>> {
        use sequent_core::election_config::package_verify::freshness;
        let Some(verified) = &self.context.verified else {
            return Ok(None);
        };
        let mut db: DbClient = get_hasura_pool().await.get().await?;
        let transaction = db.transaction().await?;
        let last = crate::postgres::configuration_packages::last_import_with_event(
            &transaction,
            &self.context.tenant_id,
            &verified.manifest.configuration.external_id,
        )
        .await?;
        let event_id = derived_event_id(&self.context.tenant_id, &self.context.package_sha256);
        Ok(admission_problem(
            last.as_ref().map(|(last, imported_into)| {
                (freshness(verified, Some(last)), imported_into.as_str())
            }),
            &event_id,
        ))
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

    /// Each existing administrator account's groups, from the Keycloak
    /// database the user import writes to.
    async fn existing_usernames(
        &self,
        usernames: &[String],
    ) -> Result<BTreeMap<String, BTreeSet<String>>> {
        let mut db: DbClient = get_keycloak_pool().await.get().await?;
        let transaction = db.transaction().await?;
        user_groups(
            &transaction,
            &get_tenant_realm(&self.context.tenant_id),
            usernames,
        )
        .await
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

impl PlatformTarget {
    async fn preview_section(
        &self,
        section: Section,
        files: &[Artifact],
    ) -> Result<SectionPreview> {
        match section {
            Section::TenantConfiguration => {
                let wanted = self.tenant_configuration(files)?;
                let mut preview = tenant_configuration_changes(&self.tenant().await?, &wanted);
                let raw: Value =
                    serde_json::from_slice(&one(files, "tenant configuration")?.bytes)?;
                for path in secret_field_paths(&raw, "tenant configuration") {
                    preview.blockers.push(format!(
                        "{path}: credentials never travel in a package; set them on the installed system"
                    ));
                }
                Ok(preview)
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
                let mut preview = admin_changes(&rows, &existing, &known);
                preview
                    .blockers
                    .extend(admin_column_problems(&one(files, "administrators")?.bytes)?);
                Ok(preview)
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
                let event_zip = &one(files, "election event")?.bytes;
                let event_id =
                    derived_event_id(&self.context.tenant_id, &self.context.package_sha256);
                let mut problems = Vec::new();
                let mut db: DbClient = get_hasura_pool().await.get().await?;
                let transaction = db.transaction().await?;
                // The importer's own validation of the event, before anything
                // is applied rather than when the import runs last.
                let document = String::from_utf8(event_document(event_zip)?)
                    .context("the event document is not UTF-8")?;
                match get_election_event_schema(
                    &document,
                    Some(event_id.clone()),
                    self.context.tenant_id.clone(),
                )
                .await
                {
                    Ok((schema, _)) => {
                        // The importer skips a messaging configuration its caller
                        // may not write and still reports the event imported.
                        problems.extend(messaging_problem(
                            schema.messaging_config.is_some(),
                            &self.context.event_import.messaging_config_permission,
                        ));
                        let referenced: Vec<String> = schema
                            .keys_ceremonies
                            .iter()
                            .flatten()
                            .flat_map(|ceremony| ceremony.trustee_ids.iter().cloned())
                            .collect();
                        let registered: Vec<String> =
                            get_all_trustees(&transaction, &self.context.tenant_id)
                                .await?
                                .into_iter()
                                .filter_map(|t| t.name)
                                .collect();
                        problems.extend(trustee_reference_problems(
                            &referenced,
                            &registered,
                            &self.context.package_trustees,
                        ));
                    }
                    Err(error) => problems.push(format!("the event would be refused: {error:#}")),
                }
                let census = match census_usernames(event_zip)? {
                    Ok(census) => census,
                    Err(problem) => {
                        problems.push(problem);
                        BTreeSet::new()
                    }
                };
                let exists = get_election_event_by_id_if_exist(
                    &transaction,
                    &self.context.tenant_id,
                    &event_id,
                )
                .await?
                .is_some();
                let mut keycloak: DbClient = get_keycloak_pool().await.get().await?;
                let keycloak = keycloak.transaction().await?;
                let voters = realm_usernames(
                    &keycloak,
                    &get_event_realm(&self.context.tenant_id, &event_id),
                )
                .await?;
                let state = match (exists, voters) {
                    (true, voters) => EventState::Present {
                        voters: voters.unwrap_or_default(),
                    },
                    (false, Some(_)) => EventState::RealmOnly,
                    (false, None) => EventState::Absent,
                };
                Ok(event_changes(&event_id, &state, &census, problems))
            }
        }
    }
}

#[async_trait]
impl PackageTarget for PlatformTarget {
    async fn hold_imports(&self) -> Result<Box<dyn std::any::Any + Send>> {
        let held = crate::postgres::configuration_packages::hold_imports(
            get_hasura_pool().await.as_ref(),
            &self.context.tenant_id,
        )
        .await?;
        Ok(Box::new(held))
    }

    async fn preview(&self, section: Section, files: &[Artifact]) -> Result<SectionPreview> {
        let mut preview = self.preview_section(section, files).await?;
        if let Some(blocker) = self.admission_blocker().await? {
            preview.blockers.insert(0, blocker);
        }
        Ok(preview)
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
                // Open, under the claim: its run stopped. The next preview
                // says whether its import landed, and settles the row.
                TaskState::Interrupted
            },
        )
    }

    async fn settle(&self, task_id: &str, succeeded: bool) -> Result<()> {
        let task = get_task_by_id(task_id).await?;
        let open = task.execution_status != TasksExecutionStatus::SUCCESS.to_string()
            && task.execution_status != TasksExecutionStatus::FAILED.to_string();
        if open {
            if succeeded {
                update_complete(&task, None).await?;
            } else {
                update_fail(&task, "the section did not converge").await?;
            }
        }
        Ok(())
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
                let task = self.task(task_id).await?;
                let current = write_tenant_configuration(
                    &*get_hasura_pool().await,
                    &self.context.tenant_id,
                    &wanted,
                )
                .await?;
                let changes = tenant_configuration_changes(&current, &wanted).changes;
                task_audit::record(task_audit::tenant_config(
                    self.context.audit_actor.clone(),
                    &self.context.tenant_id,
                    AdminAction::Import,
                    json!({"section": "tenant-configuration", "changes": changes}),
                )?)
                .await?;
                update_complete(&task, None).await?;
                Ok(SectionSubmission {
                    task_id: Some(task.id.clone()),
                    detail: json!({"changes": changes}),
                })
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
                // Registrations of one tenant's trustees by name are serialized,
                // so two writers cannot both find a name absent and insert it.
                transaction
                    .query_one(
                        "SELECT pg_advisory_xact_lock(hashtextextended($1, 0))",
                        &[&format!("trustee-registration:{}", self.context.tenant_id)],
                    )
                    .await?;
                let existing = get_all_trustees(&transaction, &self.context.tenant_id).await?;
                let mut names: BTreeSet<String> =
                    existing.into_iter().filter_map(|t| t.name).collect();
                let mut registered = Vec::new();
                for trustee in &wanted {
                    // Each name once, even if the file repeats it.
                    if !names.insert(trustee.name.clone()) {
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

/// Writes a package's tenant configuration over the tenant's row as it is now,
/// locked until committed, and only the four fields: keys the package does not
/// name (a legacy service-account key, the signing trust) stay. Returns the row
/// as it was. Its connection goes back to the pool before this returns: the run
/// still holds its claim, and what follows (the audit record, the task's
/// status) takes connections of its own, so keeping this one could leave a
/// small pool with none (the airgap rehearsal's hang).
pub(crate) async fn write_tenant_configuration(
    pool: &deadpool_postgres::Pool,
    tenant_id: &str,
    wanted: &TenantConfiguration,
) -> Result<sequent_core::types::hasura::core::Tenant> {
    let mut db: DbClient = pool.get().await?;
    let transaction = db.transaction().await?;
    let current = lock_tenant(&transaction, tenant_id).await?;
    let merged = merged_tenant(&current, wanted);
    update_tenant_configuration(&transaction, &merged).await?;
    transaction.commit().await?;
    Ok(current)
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
