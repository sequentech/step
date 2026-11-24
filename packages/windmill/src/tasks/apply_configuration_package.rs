// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Preview or apply a whole delivery to a tenant. Its record — every
//! section's preview, submission, task row and outcome — is the task row's
//! annotations, saved before and after each step.

use crate::postgres::configuration_packages;
use crate::postgres::document::get_document;
use crate::postgres::tasks_execution::{get_task_by_id_with_transaction, get_tasks_by_annotation};
use crate::services::audit_log::AuditActor;
use crate::services::browser_package_policy::{admit_with_policy, BrowserPolicy};
use crate::services::database::get_hasura_pool;
use crate::services::documents::get_document_as_temp_file;
use crate::services::import::configuration_package::tenant_setting;
use crate::services::import::configuration_package_apply::{
    plan_members, run, ApplicationRecord, ApplyMode, Outcome, ProgressStore,
};
use crate::services::import::configuration_package_demo_credentials::VerifiedDemoLoginAdmission;
use crate::services::import::configuration_package_sections::{read_admin_rows, read_group_roles};
use crate::services::import::configuration_package_target::{ApplyContext, PlatformTarget};
use crate::services::managed_webauthn_roster::Binding;
use crate::services::tasks_execution::update_with_annotations;
use crate::tasks::import_election_event::ImportElectionEventBody;
use crate::types::error::Result;
use crate::types::tasks::ETasksExecution;
use anyhow::{anyhow, Context};
use async_trait::async_trait;
use celery::error::TaskError;
use deadpool_postgres::Client as DbClient;
use sequent_core::election_config::encrypted_delivery::MAX_SIGNED_DELIVERY_BYTES;
use sequent_core::election_config::manifest::read_zip;
use sequent_core::election_config::package_contract::{Requirements, Section};
use sequent_core::election_config::package_verify::Admission;
use sequent_core::election_config::tenant_package::TrusteePublic;
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
/// Set on the task row as soon as its run holds the claim, before anything
/// else: a redelivery that finds it knows the row is a live run's.
pub const CLAIMED_KEY: &str = "claimed";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApplyConfigurationPackageBody {
    pub tenant_id: String,
    pub document_id: String,
    pub mode: ApplyMode,
    /// What the operator previewed. Required for portal-managed Apply.
    #[serde(default)]
    pub preview_task_id: Option<String>,
    /// Present for password-encrypted delivery; only these exact authenticated,
    /// signed inner bytes may be previewed or applied.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encrypted_payload_sha256: Option<String>,
    /// Server-stamped preview-specific digest of the verified operator.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub managed_preview_operator_binding: Option<String>,
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
/// ends, and keeps the run's claim.
struct TaskRowStore<'a> {
    task: TasksExecution,
    claim: &'a deadpool_postgres::Transaction<'a>,
    logs: tokio::sync::Mutex<Option<serde_json::Value>>,
}

#[async_trait]
impl ProgressStore for TaskRowStore<'_> {
    async fn save(&self, record: &ApplicationRecord) -> anyhow::Result<()> {
        let status = match record.outcome {
            Outcome::Running => TasksExecutionStatus::IN_PROGRESS,
            Outcome::Previewed | Outcome::Applied => TasksExecutionStatus::SUCCESS,
            Outcome::Refused | Outcome::Failed | Outcome::Unknown => TasksExecutionStatus::FAILED,
        };
        let mut logs = self.logs.lock().await;
        let updated =
            serde_json::to_value(crate::services::serialize_tasks_logs::append_general_log(
                &logs,
                &format!("{:?}", record.outcome),
            ))?;
        update_with_annotations(
            &self.task.tenant_id,
            &self.task.id,
            status,
            updated.clone(),
            json!({
                PACKAGE_SHA256_KEY: record.package_sha256,
                CLAIMED_KEY: true,
                APPLICATION_KEY: record,
            }),
        )
        .await?;
        *logs = Some(updated);
        Ok(())
    }

    /// The claim is the open transaction's lock: if its connection is gone,
    /// so is the claim, and the run must not decide anything more.
    async fn hold(&self) -> anyhow::Result<()> {
        self.claim
            .query_one("SELECT 1", &[])
            .await
            .map(|_| ())
            .context("the application lost its claim; stopping without further changes")
    }
}

/// The record a run resumes from: the current task's own when it has saved
/// one (a redelivery), otherwise the newest record of this package on this
/// tenant. A record that cannot be read stops the run rather than falling
/// back to an older one.
async fn prior_record(
    tenant_id: &str,
    package_sha256: &str,
    own_task: &str,
) -> anyhow::Result<Option<ApplicationRecord>> {
    let mut db: DbClient = get_hasura_pool().await.get().await?;
    let transaction = db.transaction().await?;
    let rows = get_tasks_by_annotation(
        &transaction,
        tenant_id,
        &ETasksExecution::APPLY_CONFIGURATION_PACKAGE.to_string(),
        PACKAGE_SHA256_KEY,
        package_sha256,
    )
    .await?;
    choose_prior(rows, own_task)
}

/// The record to resume from among the package's task rows, newest first:
/// the own row's when it has one, else the newest that has one.
fn choose_prior(
    rows: Vec<TasksExecution>,
    own_task: &str,
) -> anyhow::Result<Option<ApplicationRecord>> {
    let with_record: Vec<(String, serde_json::Value)> = rows
        .into_iter()
        .filter_map(|row| {
            let record = row.annotations.as_ref()?.get(APPLICATION_KEY)?.clone();
            Some((row.id, record))
        })
        .collect();
    let chosen = with_record
        .iter()
        .find(|(id, _)| id == own_task)
        .or_else(|| with_record.first());
    let Some((id, record)) = chosen else {
        return Ok(None);
    };
    serde_json::from_value(record.clone())
        .map(Some)
        .with_context(|| format!("the saved application of task {id} cannot be read"))
}

/// A managed application names the exact successful preview it confirmed.
fn confirms_managed_preview(
    row: &TasksExecution,
    tenant_id: &str,
    package_sha256: &str,
    binding: &Binding,
    operator_binding: &str,
    plan: &crate::services::import::configuration_package_apply::PackagePlan,
) -> bool {
    if row.tenant_id != tenant_id
        || row.task_type != ETasksExecution::APPLY_CONFIGURATION_PACKAGE.to_string()
        || row.execution_status != TasksExecutionStatus::SUCCESS.to_string()
    {
        return false;
    }
    row.annotations
        .as_ref()
        .and_then(|annotations| annotations.get(APPLICATION_KEY))
        .and_then(|value| serde_json::from_value::<ApplicationRecord>(value.clone()).ok())
        .is_some_and(|record| {
            record.format
                == crate::services::import::configuration_package_apply::APPLICATION_FORMAT
                && record.tenant_id == tenant_id
                && record.package_sha256 == package_sha256
                && record.mode == ApplyMode::Preview
                && record.outcome == Outcome::Previewed
                && record.managed_roster_binding.as_ref() == Some(binding)
                && record.managed_preview_operator_binding.as_deref() == Some(operator_binding)
                && record.managed_requirements.as_ref() == Some(&plan.requirements)
                && record.coverage == plan.coverage
        })
}

/// A run that stopped before it held the claim: the row may be a live run's.
#[derive(Debug)]
struct Unclaimed(anyhow::Error);

impl std::fmt::Display for Unclaimed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#}", self.0)
    }
}

impl std::error::Error for Unclaimed {}

fn unclaimed(error: impl Into<anyhow::Error>) -> anyhow::Error {
    anyhow::Error::new(Unclaimed(error.into()))
}

async fn execute(
    body: &ApplyConfigurationPackageBody,
    task_execution: &TasksExecution,
) -> anyhow::Result<ApplicationRecord> {
    // The application's claim first: one package application per tenant at a
    // time, whatever the package. Held by an open transaction on a connection
    // of its own, so it ends with the run, or with the worker if that dies;
    // while it is held, an earlier submission still marked in progress cannot
    // be running any more. Checked again before every submission. Anything
    // that fails before it is held leaves the task row to its owner.
    let mut claim_db: DbClient = get_hasura_pool().await.get().await.map_err(unclaimed)?;
    let claim = claim_db.transaction().await.map_err(unclaimed)?;
    let claimed: bool = claim
        .query_one(
            "SELECT pg_try_advisory_xact_lock(hashtextextended($1, 0))",
            &[&configuration_packages::application_lock_key(
                &body.tenant_id,
            )],
        )
        .await
        .map_err(unclaimed)?
        .get(0);
    if !claimed {
        return Err(unclaimed(anyhow!(
            "another configuration package application on this tenant is running; wait for it to finish"
        )));
    }
    let mut annotations = task_execution
        .annotations
        .clone()
        .unwrap_or_else(|| json!({}));
    annotations[CLAIMED_KEY] = json!(true);
    update_with_annotations(
        &task_execution.tenant_id,
        &task_execution.id,
        TasksExecutionStatus::IN_PROGRESS,
        task_execution.logs.clone().unwrap_or(json!([])),
        annotations,
    )
    .await?;

    let mut db: DbClient = get_hasura_pool().await.get().await?;
    let transaction = db.transaction().await?;
    let document = get_document(&transaction, &body.tenant_id, None, &body.document_id)
        .await?
        .ok_or_else(|| anyhow!("document {} not found", body.document_id))?;
    if body.encrypted_payload_sha256.is_some()
        && document
            .size
            .is_some_and(|size| size > MAX_SIGNED_DELIVERY_BYTES as i64)
    {
        return Err(anyhow!("decrypted signed package exceeds the size limit"));
    }
    let file = get_document_as_temp_file(&body.tenant_id, &document).await?;
    if body.encrypted_payload_sha256.is_some()
        && file.as_file().metadata()?.len() > MAX_SIGNED_DELIVERY_BYTES as u64
    {
        return Err(anyhow!("decrypted signed package exceeds the size limit"));
    }
    let bytes = std::fs::read(file.path()).context("could not read the package")?;
    // Verified against the tenant's signing trust. Its freshness against the
    // tenant's imports is the target's to check, before each section.
    configuration_packages::lock_imports(&transaction, &body.tenant_id).await?;
    let package_sha256 = format!("{:x}", Sha256::digest(&bytes));
    if let Some(expected) = body.encrypted_payload_sha256.as_deref() {
        if !package_sha256.eq_ignore_ascii_case(expected) {
            return Err(anyhow!(
                "configuration package bytes do not match the authenticated encrypted delivery"
            ));
        }
    }
    let policy = BrowserPolicy::load_with_db(&transaction, &body.tenant_id).await;
    let setting = tenant_setting(&transaction, &body.tenant_id).await?;
    let lists = configuration_packages::revocation_lists(&transaction, &body.tenant_id).await?;
    let (mut members, signed_package, verified) =
        // Revalidated against the tenant's current policy immediately
        // before any section applies: a policy changed since preview refuses.
        match admit_with_policy(&bytes, setting.as_ref(), &body.tenant_id, lists, chrono::Utc::now(), &policy) {
            Ok(Admission::Unsigned) if body.encrypted_payload_sha256.is_some() => {
                return Err(anyhow!("an encrypted configuration package must be signed"));
            }
            Ok(Admission::Unsigned) => (
                read_zip(&bytes, "package").map_err(|p| anyhow!("{p}"))?,
                None,
                None,
            ),
            Ok(Admission::Verified(verified)) => (
                verified.members.clone(),
                Some(bytes.clone()),
                Some(std::sync::Arc::new(*verified)),
            ),
            Err(report) => {
                return Err(anyhow::Error::new(
                    sequent_core::election_config::Rejected::new("configuration package", report),
                ))
            }
        };
    if let Some(package) = &verified {
        let last = configuration_packages::last_import(
            &transaction,
            &body.tenant_id,
            &package.manifest.configuration.external_id,
        )
        .await?;
        crate::services::browser_package_policy::freshness(package, last.as_ref())
            .map_err(|problem| anyhow!("{problem}"))?;
    }
    let demo_login = VerifiedDemoLoginAdmission::take_member(
        &mut members,
        verified.as_deref(),
        &body.tenant_id,
        body.event_import.privileges,
    )?
    .map(std::sync::Arc::new);
    let plan = plan_members(&members, package_sha256.clone(), &body.requirements)?;
    if let Some(admitted) = &demo_login {
        let roster = plan
            .payloads
            .get(&Section::AdminAccounts)
            .and_then(|files| files.first())
            .ok_or_else(|| {
                anyhow!("signed demo verifiers require the signed administrator roster")
            })?;
        admitted.verify_roster(&read_admin_rows(&roster.bytes)?)?;
    }
    let operator_binding = policy.managed_binding().map(|_| {
        body.managed_preview_operator_binding.as_deref().filter(|stamp| {
            stamp.len() == 64 && stamp.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        }).ok_or_else(|| anyhow!("sign in to this tenant's Admin Portal to preview or apply under the managed roster"))
    }).transpose()?;
    if let (ApplyMode::Apply, Some(binding), Some(operator_binding)) =
        (body.mode, policy.managed_binding(), operator_binding)
    {
        let refuse = || {
            anyhow!("confirm your successful preview of this package under the current receiver roster before applying")
        };
        let preview_id = body.preview_task_id.as_deref().ok_or_else(refuse)?;
        let preview = get_task_by_id_with_transaction(&transaction, &body.tenant_id, preview_id)
            .await
            .map_err(|_| refuse())?;
        let canonical_tenant =
            sequent_core::services::uuid_validation::parse_uuid_v4(&body.tenant_id)?.to_string();
        if !confirms_managed_preview(
            &preview,
            &canonical_tenant,
            &package_sha256,
            binding,
            operator_binding,
            &plan,
        ) {
            return Err(refuse());
        }
    }
    transaction.commit().await?;
    // The event importer atomically persists the authoritative chain anchor.
    // Without that section this application could mutate tenant state without
    // ever recording the browser manifest, allowing a later replay.
    if verified.as_ref().is_some_and(|package| {
        sequent_core::election_config::webauthn_package::is_browser_verified(package)
    }) && !plan.payloads.contains_key(&Section::ElectionEvent)
    {
        return Err(anyhow!("a security-key package must include an applicable election event so its revision is recorded"));
    }
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
    let package_trustees: BTreeSet<String> = match plan.payloads.get(&Section::Trustees) {
        Some(files) => files
            .iter()
            .map(|f| serde_json::from_slice::<Vec<TrusteePublic>>(&f.bytes))
            .collect::<std::result::Result<Vec<_>, _>>()
            .context("the trustees file is not the public trustee list")?
            .into_iter()
            .flatten()
            .map(|t| t.name)
            .collect(),
        None => BTreeSet::new(),
    };
    let target = PlatformTarget {
        event_imported_by_this_run: Default::default(),
        context: ApplyContext {
            tenant_id: body.tenant_id.clone(),
            executed_by: body.executed_by.clone(),
            audit_actor: body.audit_actor.clone(),
            event_import: body.event_import.clone(),
            package_sha256: package_sha256.clone(),
            signed_package,
            managed_roster_binding: policy.managed_binding().cloned(),
            managed_preview_operator_binding: operator_binding.map(str::to_owned),
            package_groups,
            package_trustees,
            verified,
            demo_login,
        },
    };
    let prior = prior_record(&body.tenant_id, &package_sha256, &task_execution.id).await?;
    let store = TaskRowStore {
        task: task_execution.clone(),
        claim: &claim,
        logs: tokio::sync::Mutex::new(task_execution.logs.clone()),
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
    drop(store);
    claim.rollback().await?;
    record
}

#[instrument(err, skip(body))]
#[wrap_map_err::wrap_map_err(TaskError)]
// Never retried by the queue: a run that stops is resumed by an operator
// running it again, which settles what the last one left first.
#[celery::task(max_retries = 0)]
pub async fn apply_configuration_package(
    body: ApplyConfigurationPackageBody,
    task_execution: TasksExecution,
) -> Result<()> {
    match execute(&body, &task_execution).await {
        Ok(record) => match record.outcome {
            Outcome::Previewed | Outcome::Applied => Ok(()),
            outcome => Err(anyhow!("configuration package {outcome:?}").into()),
        },
        Err(error) if error.is::<Unclaimed>() => {
            // A duplicate delivery of a run that holds the claim must leave
            // that run's row alone; a new request that met another run, or
            // could not even try, is refused on its own row.
            let live = crate::postgres::tasks_execution::get_task_by_id(&task_execution.id)
                .await
                .ok()
                .and_then(|row| row.annotations)
                .is_some_and(|a| a.get(CLAIMED_KEY).is_some());
            if !live {
                let _ = crate::services::tasks_execution::update_fail(
                    &task_execution,
                    &error.to_string(),
                )
                .await;
            }
            Err(anyhow!("{error}").into())
        }
        Err(error) => {
            let message = format!("Error applying the configuration package: {error:#}");
            let _ = crate::services::tasks_execution::update_fail(&task_execution, &message).await;
            Err(anyhow!(message).into())
        }
    }
}

#[cfg(test)]
#[path = "../../tests/support/schema.rs"]
mod apply_test_schema;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::postgres::trustee::{get_all_trustees, insert_trustee};
    use crate::services::import::configuration_package_apply::{ApplyMode, Outcome};
    use sequent_core::election_config::package_contract::Coverage;
    use std::collections::BTreeMap;

    fn row(id: &str, record: Option<&ApplicationRecord>) -> TasksExecution {
        serde_json::from_value(json!({
            "id": id, "tenant_id": "t", "name": "Apply", "task_type": "APPLY_CONFIGURATION_PACKAGE",
            "execution_status": "SUCCESS", "executed_by_user": "ada",
            "created_at": "2026-10-25T00:00:00Z",
            "annotations": record.map(|r| json!({APPLICATION_KEY: r})),
        }))
        .unwrap()
    }

    fn record(outcome: Outcome) -> ApplicationRecord {
        ApplicationRecord {
            format: "sequent.package-application/1".into(),
            tenant_id: "t".into(),
            package_sha256: "p".into(),
            managed_roster_binding: None,
            managed_preview_operator_binding: None,
            managed_requirements: None,
            mode: ApplyMode::Apply,
            coverage: Coverage {
                members: vec![],
                sections: BTreeMap::new(),
                census: sequent_core::election_config::package_contract::SectionState::NotIncluded,
            },
            sections: vec![],
            outcome,
        }
    }

    #[test]
    fn a_managed_apply_requires_the_exact_successful_preview_not_a_newer_unconfirmed_one() {
        let active = Binding {
            sha256: "a".repeat(64),
            generation: 2,
            required_approvals: 2,
        };
        let mut preview = record(Outcome::Previewed);
        preview.mode = ApplyMode::Preview;
        preview.managed_requirements = Some(Requirements::default());
        let operator = "b".repeat(64);
        preview.managed_preview_operator_binding = Some(operator.clone());
        let mut plan = crate::services::import::configuration_package_apply::PackagePlan {
            package_sha256: "p".into(),
            requirements: Requirements::default(),
            coverage: preview.coverage.clone(),
            payloads: BTreeMap::new(),
        };
        preview.managed_roster_binding = Some(Binding {
            generation: 1,
            ..active.clone()
        });
        let old_row = row("old", Some(&preview));
        assert!(!confirms_managed_preview(
            &old_row, "t", "p", &active, &operator, &plan
        ));
        preview.managed_roster_binding = Some(active.clone());
        let current_row = row("confirmed", Some(&preview));
        assert!(!confirms_managed_preview(
            &current_row,
            "t",
            "p",
            &active,
            &"c".repeat(64),
            &plan
        ));
        let mut unstamped = preview.clone();
        unstamped.managed_preview_operator_binding = None;
        assert!(!confirms_managed_preview(
            &row("old-unstamped", Some(&unstamped)),
            "t",
            "p",
            &active,
            &operator,
            &plan
        ));
        assert!(confirms_managed_preview(
            &current_row,
            "t",
            "p",
            &active,
            &operator,
            &plan
        ));
        plan.requirements.sections = vec![Section::Trustees];
        assert!(!confirms_managed_preview(
            &current_row,
            "t",
            "p",
            &active,
            &operator,
            &plan
        ));
        plan.requirements = Requirements::default();
        assert!(!confirms_managed_preview(
            &current_row,
            "other",
            "p",
            &active,
            &operator,
            &plan
        ));
        assert!(!confirms_managed_preview(
            &current_row,
            "t",
            "other",
            &active,
            &operator,
            &plan
        ));
        preview.mode = ApplyMode::Apply;
        assert!(!confirms_managed_preview(
            &row("not-preview", Some(&preview)),
            "t",
            "p",
            &active,
            &operator,
            &plan
        ));
    }

    #[test]
    fn a_run_resumes_from_its_own_record_before_a_newer_one() {
        let own = record(Outcome::Unknown);
        let newer = record(Outcome::Applied);
        // Newest first, as the lookup orders them.
        let rows = vec![
            row("newer", Some(&newer)),
            row("own", Some(&own)),
            row("fresh", None),
        ];
        assert_eq!(
            choose_prior(rows.clone(), "own").unwrap().unwrap().outcome,
            Outcome::Unknown
        );
        // A new run (its row has no record yet) takes the newest record.
        assert_eq!(
            choose_prior(rows, "fresh").unwrap().unwrap().outcome,
            Outcome::Applied
        );
        assert!(choose_prior(vec![row("fresh", None)], "fresh")
            .unwrap()
            .is_none());
    }

    #[test]
    fn an_unreadable_record_stops_the_run() {
        let mut broken = row("own", None);
        broken.annotations = Some(json!({APPLICATION_KEY: {"format": 1}}));
        assert!(choose_prior(vec![broken], "own").is_err());
    }

    /// Whether another connection could take the tenant's import lock now.
    async fn import_lock_is_free(pool: &deadpool_postgres::Pool, tenant: &str) -> bool {
        let key = crate::postgres::configuration_packages::import_lock_key(tenant).unwrap();
        let mut client = pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        let free: bool = tx
            .query_one(
                "SELECT pg_try_advisory_xact_lock(hashtextextended($1, 0))",
                &[&key],
            )
            .await
            .unwrap()
            .get(0);
        tx.rollback().await.unwrap();
        free
    }

    /// Needs the PostgreSQL server `HASURA_DB__*` names.
    #[tokio::test]
    async fn holding_imports_blocks_other_connections_until_dropped_and_never_returns_to_the_pool()
    {
        use crate::postgres::configuration_packages::hold_imports;
        let pool = super::apply_test_schema::pool().await;
        let tenant = uuid::Uuid::new_v4().to_string();
        assert!(import_lock_is_free(&pool, &tenant).await);

        let size = pool.status().size;
        let held = hold_imports(&pool, &tenant).await.unwrap();
        // Its connection left the pool: nothing else can be handed it, held.
        assert_eq!(pool.status().size, size.saturating_sub(1));
        assert!(!import_lock_is_free(&pool, &tenant).await);
        // Another tenant's imports are not held back.
        assert!(import_lock_is_free(&pool, &uuid::Uuid::new_v4().to_string()).await);

        // Probe once while held, then drop it: closing the connection releases
        // the lock asynchronously. This exercises the retry path regardless of
        // how quickly PostgreSQL observes the closed connection.
        let mut held = Some(held);
        let mut free = false;
        for _ in 0..50 {
            if import_lock_is_free(&pool, &tenant).await {
                assert!(held.is_none(), "the lock was released before its hold");
                free = true;
                break;
            }
            drop(held.take());
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        assert!(free, "the lock outlived its hold");

        // A refused hold takes no connection at all.
        let size = pool.status().size;
        assert!(hold_imports(&pool, "not-a-uuid").await.is_err());
        assert_eq!(pool.status().size, size);
    }

    /// Needs the PostgreSQL server `HASURA_DB__*` names; rolls back.
    #[tokio::test]
    async fn the_tenant_record_update_writes_only_its_four_fields_under_a_lock() {
        use crate::postgres::tenant::{lock_tenant, update_tenant_configuration};
        let pool = super::apply_test_schema::pool().await;
        let mut client = pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        let tenant = uuid::Uuid::new_v4();
        crate::postgres::tenant::allow_configuration_signing_change(&tx, &tenant.to_string())
            .await
            .unwrap();
        tx.execute(
            "INSERT INTO sequent_backend.tenant (id, slug, is_active, settings, test) VALUES ($1, $2, true, $3, 1)",
            &[&tenant, &format!("keep-{tenant}"), &json!({"configuration_signing": {"policy": "required"}, "gapi_key": {"k": "v"}})],
        )
        .await
        .unwrap();
        let mut current = lock_tenant(&tx, &tenant.to_string()).await.unwrap();
        current.slug = "ignored".into();
        current.is_active = false;
        current.test = Some(9);
        current.settings = Some(
            json!({"configuration_signing": {"policy": "required"}, "gapi_key": {"k": "v"}, "i18n": {}}),
        );
        current.voting_channels = Some(json!({"online": true}));
        update_tenant_configuration(&tx, &current).await.unwrap();
        let row = tx
            .query_one("SELECT slug, is_active, test, settings, voting_channels FROM sequent_backend.tenant WHERE id = $1", &[&tenant])
            .await
            .unwrap();
        assert_eq!(row.get::<_, String>(0), format!("keep-{tenant}"));
        assert!(row.get::<_, bool>(1));
        assert_eq!(row.get::<_, Option<i32>>(2), Some(1));
        assert_eq!(
            row.get::<_, serde_json::Value>(3)["gapi_key"],
            json!({"k": "v"})
        );
        assert_eq!(row.get::<_, serde_json::Value>(4), json!({"online": true}));
        tx.rollback().await.unwrap();
    }

    /// Needs the PostgreSQL server `HASURA_DB__*` names (see
    /// tests/support/schema.rs); rolls back everything it writes.
    #[tokio::test]
    async fn earlier_runs_and_registered_trustees_are_read_back_per_tenant() {
        let pool = super::apply_test_schema::pool().await;
        let mut client = pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        let (tenant, other) = (uuid::Uuid::new_v4(), uuid::Uuid::new_v4());
        for id in [tenant, other] {
            tx.execute(
                "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
                &[&id, &format!("apply-{id}")],
            )
            .await
            .unwrap();
        }
        let kind = ETasksExecution::APPLY_CONFIGURATION_PACKAGE.to_string();
        for (id, owner, sha) in [
            (uuid::Uuid::new_v4(), tenant, "aaaa"),
            (uuid::Uuid::new_v4(), tenant, "bbbb"),
            (uuid::Uuid::new_v4(), other, "aaaa"),
        ] {
            tx.execute(
                "INSERT INTO sequent_backend.tasks_execution (id, tenant_id, name, type, execution_status, executed_by_user, annotations) VALUES ($1, $2, 'Apply', $3, 'SUCCESS', 'ada', $4)",
                &[&id, &owner, &kind, &serde_json::json!({PACKAGE_SHA256_KEY: sha})],
            )
            .await
            .unwrap();
        }
        let found =
            get_tasks_by_annotation(&tx, &tenant.to_string(), &kind, PACKAGE_SHA256_KEY, "aaaa")
                .await
                .unwrap();
        assert_eq!(found.len(), 1, "only this tenant's runs of this package");
        assert_eq!(found[0].tenant_id, tenant.to_string());

        let id = insert_trustee(&tx, &tenant.to_string(), "t1", "c2lnbg==", "c2hhcmU=")
            .await
            .unwrap();
        let trustees = get_all_trustees(&tx, &tenant.to_string()).await.unwrap();
        assert_eq!(trustees.len(), 1);
        assert_eq!(trustees[0].id, id);
        assert_eq!(trustees[0].public_key.as_deref(), Some("c2lnbg=="));
        assert_eq!(
            trustees[0].share_encryption_public_key.as_deref(),
            Some("c2hhcmU=")
        );
        assert!(get_all_trustees(&tx, &other.to_string())
            .await
            .unwrap()
            .is_empty());
        tx.rollback().await.unwrap();
    }
}
