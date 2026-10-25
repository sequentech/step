// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Preview or apply a whole delivery to a tenant. Its record — every
//! section's preview, submission, task row and outcome — is the task row's
//! annotations, saved before and after each step.

use crate::postgres::configuration_packages;
use crate::postgres::document::get_document;
use crate::postgres::tasks_execution::get_tasks_by_annotation;
use crate::services::audit_log::AuditActor;
use crate::services::database::get_hasura_pool;
use crate::services::documents::get_document_as_temp_file;
use crate::services::import::configuration_package::tenant_setting;
use crate::services::import::configuration_package_apply::{
    plan_members, run, ApplicationRecord, ApplyMode, Outcome, ProgressStore,
};
use crate::services::import::configuration_package_sections::read_group_roles;
use crate::services::import::configuration_package_target::{ApplyContext, PlatformTarget};
use crate::services::tasks_execution::update_with_annotations;
use crate::tasks::import_election_event::ImportElectionEventBody;
use crate::types::error::Result;
use crate::types::tasks::ETasksExecution;
use anyhow::{anyhow, Context};
use async_trait::async_trait;
use celery::error::TaskError;
use deadpool_postgres::Client as DbClient;
use sequent_core::election_config::manifest::read_zip;
use sequent_core::election_config::package_contract::{Requirements, Section};
use sequent_core::election_config::package_verify::{admit_at, Admission};
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::types::hasura::extra::TasksExecutionStatus;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use tracing::instrument;

/// The annotation key a run records its package under, so a later run of the
/// same package on the same tenant finds it.
pub const PACKAGE_SHA256_KEY: &str = "packageSha256";
pub const APPLICATION_KEY: &str = "application";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApplyConfigurationPackageBody {
    pub tenant_id: String,
    pub document_id: String,
    pub mode: ApplyMode,
    /// What the delivery must contain; a required section that is absent
    /// refuses the application.
    #[serde(default)]
    pub requirements: Requirements,
    /// The event import's body as the server stamped it from the requesting
    /// administrator's token.
    pub event_import: ImportElectionEventBody,
    #[serde(default)]
    pub audit_actor: Option<AuditActor>,
    pub executed_by: String,
}

/// Saves the record on the task row, keeping it in progress until the run
/// ends.
struct TaskRowStore {
    task: TasksExecution,
}

#[async_trait]
impl ProgressStore for TaskRowStore {
    async fn save(&self, record: &ApplicationRecord) -> anyhow::Result<()> {
        let status = match record.outcome {
            Outcome::Running => TasksExecutionStatus::IN_PROGRESS,
            Outcome::Previewed | Outcome::Applied => TasksExecutionStatus::SUCCESS,
            Outcome::Refused | Outcome::Failed | Outcome::Unknown => TasksExecutionStatus::FAILED,
        };
        let logs =
            serde_json::to_value(crate::services::serialize_tasks_logs::append_general_log(
                &self.task.logs,
                &format!("{:?}", record.outcome),
            ))?;
        update_with_annotations(
            &self.task.tenant_id,
            &self.task.id,
            status,
            logs,
            json!({
                PACKAGE_SHA256_KEY: record.package_sha256,
                APPLICATION_KEY: record,
            }),
        )
        .await
    }
}

/// The newest earlier record of this package on this tenant, if any.
async fn prior_record(
    tenant_id: &str,
    package_sha256: &str,
    current_task: &str,
) -> anyhow::Result<Option<ApplicationRecord>> {
    let rows = get_tasks_by_annotation(
        tenant_id,
        &ETasksExecution::APPLY_CONFIGURATION_PACKAGE.to_string(),
        PACKAGE_SHA256_KEY,
        package_sha256,
    )
    .await?;
    let others: Vec<TasksExecution> = rows
        .into_iter()
        .filter(|row| row.id != current_task)
        .collect();
    Ok(others.into_iter().find_map(|row| {
        row.annotations
            .and_then(|a| a.get(APPLICATION_KEY).cloned())
            .and_then(|record| serde_json::from_value(record).ok())
    }))
}

async fn execute(
    body: &ApplyConfigurationPackageBody,
    task_execution: &TasksExecution,
) -> anyhow::Result<ApplicationRecord> {
    let mut db: DbClient = get_hasura_pool().await.get().await?;
    let transaction = db.transaction().await?;
    let document = get_document(&transaction, &body.tenant_id, None, &body.document_id)
        .await?
        .ok_or_else(|| anyhow!("document {} not found", body.document_id))?;
    let file = get_document_as_temp_file(&body.tenant_id, &document).await?;
    let bytes = std::fs::read(file.path()).context("could not read the package")?;
    // Verified against the tenant's signing trust. Whether the event was
    // already imported is the event section's own question.
    let setting = tenant_setting(&transaction, &body.tenant_id).await?;
    let lists = configuration_packages::revocation_lists(&transaction, &body.tenant_id).await?;
    transaction.commit().await?;
    let (members, signed_package) =
        match admit_at(&bytes, setting.as_ref(), lists, chrono::Utc::now()) {
            Ok(Admission::Unsigned) => (
                read_zip(&bytes, "package").map_err(|p| anyhow!("{p}"))?,
                None,
            ),
            Ok(Admission::Verified(verified)) => (verified.members.clone(), Some(bytes.clone())),
            Err(report) => {
                return Err(anyhow::Error::new(
                    sequent_core::election_config::Rejected::new("configuration package", report),
                ))
            }
        };
    let package_sha256 = format!("{:x}", Sha256::digest(&bytes));
    let plan = plan_members(&members, package_sha256.clone(), &body.requirements)?;
    let package_groups: BTreeSet<String> = match plan.payloads.get(&Section::RolePermissions) {
        Some(files) => files
            .iter()
            .map(|f| read_group_roles(&f.bytes))
            .collect::<anyhow::Result<Vec<_>>>()?
            .into_iter()
            .flatten()
            .map(|row| row.group)
            .collect(),
        None => BTreeSet::new(),
    };
    let target = PlatformTarget {
        context: ApplyContext {
            tenant_id: body.tenant_id.clone(),
            executed_by: body.executed_by.clone(),
            audit_actor: body.audit_actor.clone(),
            event_import: body.event_import.clone(),
            package_sha256: package_sha256.clone(),
            signed_package,
            package_groups,
        },
    };
    // The application's claim: one run of a package on a tenant at a time.
    // Held by an open transaction on a connection of its own, so it ends with
    // the run, or with the worker if that dies; an earlier submission still
    // marked in progress then cannot be running any more.
    let mut claim_db: DbClient = get_hasura_pool().await.get().await?;
    let claim = claim_db.transaction().await?;
    let claimed: bool = claim
        .query_one(
            "SELECT pg_try_advisory_xact_lock(hashtextextended($1, 0))",
            &[&format!(
                "package-application:{}:{package_sha256}",
                body.tenant_id
            )],
        )
        .await?
        .get(0);
    if !claimed {
        return Err(anyhow!(
            "another application of this package to this tenant is running; wait for it to finish"
        ));
    }
    let prior = prior_record(&body.tenant_id, &package_sha256, &task_execution.id).await?;
    let store = TaskRowStore {
        task: task_execution.clone(),
    };
    let record = run(
        &plan,
        &body.tenant_id,
        body.mode,
        prior.as_ref(),
        &target,
        &store,
    )
    .await;
    claim.rollback().await?;
    record
}

#[instrument(err, skip(body))]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn apply_configuration_package(
    body: ApplyConfigurationPackageBody,
    task_execution: TasksExecution,
) -> Result<()> {
    match execute(&body, &task_execution).await {
        Ok(record) => match record.outcome {
            Outcome::Previewed | Outcome::Applied => Ok(()),
            outcome => Err(anyhow!("configuration package {outcome:?}").into()),
        },
        Err(error) => {
            let message = format!("Error applying the configuration package: {error:#}");
            let _ = crate::services::tasks_execution::update_fail(&task_execution, &message).await;
            Err(anyhow!(message).into())
        }
    }
}
