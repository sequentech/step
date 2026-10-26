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
use sequent_core::election_config::package_verify::{admit_at, Admission, ConfigurationSigning};
use sequent_core::election_config::tenant_package::TrusteePublic;
use sequent_core::types::hasura::core::{Document, TasksExecution};
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

/// What a run reads before it decides anything: the package's document row,
/// and the tenant's signing trust and revocation lists, against which the
/// package is verified (its freshness against the tenant's imports is the
/// target's to check, before each section).
struct Inputs {
    document: Document,
    setting: Option<ConfigurationSigning>,
    revocation_lists: Vec<Vec<u8>>,
}

/// Reads the [`Inputs`] on one connection and gives it back to the pool before
/// returning. The run keeps only its claim's connection after this; holding
/// this one too, while every later step takes another, left a run waiting
/// forever on a pool sized from a small CPU quota.
async fn read_inputs(
    pool: &deadpool_postgres::Pool,
    tenant_id: &str,
    document_id: &str,
) -> anyhow::Result<Inputs> {
    let mut db: DbClient = pool.get().await?;
    let transaction = db.transaction().await?;
    let document = get_document(&transaction, tenant_id, None, document_id)
        .await?
        .ok_or_else(|| anyhow!("document {document_id} not found"))?;
    let setting = tenant_setting(&transaction, tenant_id).await?;
    let revocation_lists =
        configuration_packages::revocation_lists(&transaction, tenant_id).await?;
    transaction.commit().await?;
    Ok(Inputs {
        document,
        setting,
        revocation_lists,
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
            &[&format!("package-application:{}", body.tenant_id)],
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

    let inputs = read_inputs(
        &*get_hasura_pool().await,
        &body.tenant_id,
        &body.document_id,
    )
    .await?;
    let file = get_document_as_temp_file(&body.tenant_id, &inputs.document).await?;
    let bytes = std::fs::read(file.path()).context("could not read the package")?;
    let (setting, lists) = (inputs.setting, inputs.revocation_lists);
    let (members, signed_package, verified) =
        match admit_at(&bytes, setting.as_ref(), lists, chrono::Utc::now()) {
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
        context: ApplyContext {
            tenant_id: body.tenant_id.clone(),
            executed_by: body.executed_by.clone(),
            audit_actor: body.audit_actor.clone(),
            event_import: body.event_import.clone(),
            package_sha256: package_sha256.clone(),
            signed_package,
            package_groups,
            package_trustees,
            verified,
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

        // Dropping it closes the connection, and the server releases the lock.
        drop(held);
        let mut free = false;
        for _ in 0..50 {
            if import_lock_is_free(&pool, &tenant).await {
                free = true;
                break;
            }
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
    /// tests/support/schema.rs); deletes the rows it commits.
    ///
    /// The rehearsal of the airgap release found a run waiting forever: its
    /// worker's pool held two connections (sized from a 0.5 CPU quota), the run
    /// kept the inputs' connection besides its claim, and asked for a third.
    #[tokio::test]
    async fn reading_the_inputs_gives_the_connection_back_before_the_next_step() {
        let pool = super::apply_test_schema::pool().await;
        pool.resize(2);
        let (tenant, document) = (uuid::Uuid::new_v4(), uuid::Uuid::new_v4());
        {
            let setup = pool.get().await.unwrap();
            setup
                .execute(
                    "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
                    &[&tenant, &format!("inputs-{tenant}")],
                )
                .await
                .unwrap();
            setup
                .execute(
                    "INSERT INTO sequent_backend.document (id, tenant_id, name) VALUES ($1, $2, 'package.zip')",
                    &[&document, &tenant],
                )
                .await
                .unwrap();
        }
        // The claim holds one of the two connections for the whole run.
        let claim = pool.get().await.unwrap();
        let inputs = read_inputs(&pool, &tenant.to_string(), &document.to_string())
            .await
            .unwrap();
        assert_eq!(inputs.document.id, document.to_string());
        assert!(inputs.setting.is_none());
        assert!(inputs.revocation_lists.is_empty());
        let next = tokio::time::timeout(std::time::Duration::from_secs(5), pool.get())
            .await
            .expect("the inputs' connection was not given back: the run would wait forever")
            .unwrap();
        drop(next);
        claim
            .execute(
                "DELETE FROM sequent_backend.document WHERE id = $1",
                &[&document],
            )
            .await
            .unwrap();
        claim
            .execute(
                "DELETE FROM sequent_backend.tenant WHERE id = $1",
                &[&tenant],
            )
            .await
            .unwrap();
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
