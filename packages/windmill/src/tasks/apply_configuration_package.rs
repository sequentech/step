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
use crate::services::documents::read_configuration_package;
use crate::services::import::configuration_package::tenant_setting;
use crate::services::import::configuration_package_apply::{
    plan_members, run, same_coverage, ApplicationRecord, ApplyMode, Outcome, PackagePlan,
    ProgressStore, SectionStatus, APPLICATION_FORMAT,
};
use crate::services::import::configuration_package_demo_credentials::VerifiedDemoLoginAdmission;
use crate::services::import::configuration_package_messaging::PackageMessaging;
use crate::services::import::configuration_package_sections::{read_admin_rows, read_group_roles};
use crate::services::import::configuration_package_target::{
    derived_event_id, ApplyContext, PlatformTarget,
};
use crate::services::managed_webauthn_roster::Binding;
use crate::services::tasks_execution::update_with_annotations;
use crate::tasks::import_election_event::ImportElectionEventBody;
use crate::types::error::Result;
use crate::types::tasks::ETasksExecution;
use anyhow::{anyhow, Context};
use async_trait::async_trait;
use celery::error::TaskError;
use deadpool_postgres::Client as DbClient;
use sequent_core::election_config::demo_credentials::DEMO_LOGIN_VERIFIERS_MEMBER;
use sequent_core::election_config::encrypted_delivery::{
    is_encrypted_signed_delivery, MAX_SIGNED_DELIVERY_BYTES,
};
use sequent_core::election_config::manifest::{read_zip, ManifestFormat};
use sequent_core::election_config::package_contract::{Requirements, Section};
use sequent_core::election_config::package_verify::{Admission, Freshness};
use sequent_core::election_config::portable_messaging;
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
    /// Stamped by Harvest from the tenant administrator's authenticated rights.
    #[serde(default)]
    pub portable_messaging_authorized: bool,
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

const STORE_RETRY_REQUIREMENTS: &str = "UPDATE sequent_backend.tasks_execution t
             SET annotations = COALESCE(t.annotations, '{}'::jsonb) || $3::jsonb
             WHERE t.tenant_id=$1 AND t.id=$2 AND t.type='APPLY_CONFIGURATION_PACKAGE'
               AND t.execution_status='IN_PROGRESS'
               AND NOT (COALESCE(t.annotations, '{}'::jsonb) ?| ARRAY['application','previouslyApplied'])
               AND (NOT $4 OR EXISTS (
                   SELECT 1 FROM sequent_backend.configuration_package_staging s
                    WHERE s.tenant_id=$1 AND s.document_id=$5 AND s.uploaded_by=$6
               ))";

/// Saves the record on the task row, keeping it in progress until the run
/// ends, and keeps the run's claim.
struct TaskRowStore<'a> {
    task: TasksExecution,
    claim: &'a deadpool_postgres::Transaction<'a>,
    logs: tokio::sync::Mutex<Option<serde_json::Value>>,
    retry_document: Option<RetryDocument<'a>>,
}

/// Identity of the authorized request, not the preceding operator. For
/// a protected document the hint can only be issued to its actual uploader.
struct RetryDocument<'a> {
    id: &'a str,
    actor: &'a str,
    protected: bool,
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

    async fn remember_retry_requirements(
        &self,
        digest: &str,
        requirements: &Requirements,
    ) -> anyhow::Result<()> {
        let Some(document) = &self.retry_document else {
            return Ok(()); // not an admitted, signed Admin Portal Preview
        };
        let tenant = uuid::Uuid::parse_str(&self.task.tenant_id)?;
        let task = uuid::Uuid::parse_str(&self.task.id)?;
        let document_id = uuid::Uuid::parse_str(document.id)?;
        let hint = json!({
            "packageRetry": {
                "tenantId": tenant,
                "packageSha256": digest,
                "requirements": requirements,
            }
        });
        let db = get_hasura_pool().await.get().await?;
        // Do not index this hint as an application or modify a prior receipt.
        // The tenant claim is held; match the new task and its active status
        // atomically, and recheck the stage owner without reading credentials.
        db.execute(
            STORE_RETRY_REQUIREMENTS,
            &[
                &tenant,
                &task,
                &hint,
                &document.protected,
                &document_id,
                &document.actor,
            ],
        )
        .await?;
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

/// Only called for a verified package at the current import-chain head, under
/// both tenant locks. A committed event alone says nothing about completion
/// of the other sections. A more recent unfinished Apply takes precedence.
fn completed_application(
    rows: &[TasksExecution],
    tenant_id: &str,
    plan: &PackagePlan,
    binding: Option<&Binding>,
    imported_event: &str,
) -> anyhow::Result<Option<String>> {
    if imported_event != derived_event_id(tenant_id, &plan.package_sha256)
        || !plan.coverage.is_complete()
        || !plan.payloads.contains_key(&Section::ElectionEvent)
    {
        return Ok(None);
    }
    for row in rows {
        let annotations = row.annotations.as_ref();
        let Some(value) = annotations.and_then(|a| a.get(APPLICATION_KEY)) else {
            if annotations.is_some_and(|a| a.get("previouslyApplied").is_some()) {
                continue;
            }
            return Ok(None);
        };
        let record: ApplicationRecord = serde_json::from_value(value.clone())
            .with_context(|| format!("the saved application of task {} cannot be read", row.id))?;
        if record.mode == ApplyMode::Preview {
            continue;
        }
        let messaging = plan.payloads.contains_key(&Section::MessagingAccounts);
        let complete = row.tenant_id == tenant_id
            && row.task_type == ETasksExecution::APPLY_CONFIGURATION_PACKAGE.to_string()
            && row.execution_status == TasksExecutionStatus::SUCCESS.to_string()
            && record.format == APPLICATION_FORMAT
            && record.mode == ApplyMode::Apply
            && record.outcome == Outcome::Applied
            && record.tenant_id == tenant_id
            && record.package_sha256 == plan.package_sha256
            && record.managed_roster_binding.as_ref() == binding
            && (record.managed_requirements.as_ref() == Some(&plan.requirements)
                || (record.managed_requirements.is_none()
                    && binding.is_none()
                    && !messaging
                    && record.managed_preview_operator_binding.is_none()))
            && same_coverage(&record.coverage, &plan.coverage)
            && (!messaging
                || record
                    .messaging_plan_sha256
                    .as_deref()
                    .is_some_and(valid_plan_digest))
            && record.sections.iter().all(|section| {
                section.history.in_flight.is_none()
                    && (plan.payloads.contains_key(&section.section)
                        || matches!(section.status, SectionStatus::NotIncluded))
            })
            && plan.payloads.iter().all(|(key, files)| {
                let mut matching = record.sections.iter().filter(|s| s.section == *key);
                let Some(section) = matching.next() else {
                    return false;
                };
                matching.next().is_none()
                    && section
                        .members
                        .iter()
                        .eq(files.iter().map(|file| &file.name))
                    && matches!(
                        section.status,
                        SectionStatus::Applied { .. } | SectionStatus::Unchanged { .. }
                    )
            });
        return Ok(complete.then(|| row.id.clone()));
    }
    Ok(None)
}

fn historical_outcome(mode: ApplyMode, format: ManifestFormat, browser_signed: bool) -> Outcome {
    match mode {
        ApplyMode::Preview => Outcome::Previewed,
        ApplyMode::Apply if format == ManifestFormat::V2 || browser_signed => Outcome::Refused,
        ApplyMode::Apply => Outcome::Applied,
    }
}

fn historical_annotations(
    digest: &str,
    original: &str,
    event: &str,
    mode: ApplyMode,
) -> serde_json::Value {
    json!({
        PACKAGE_SHA256_KEY: digest,
        CLAIMED_KEY: true,
        "previouslyApplied": { "taskId": original, "electionEventId": event, "mode": mode },
    })
}

/// A duplicate queue delivery can observe the terminal task while another
/// request is using the tenant. Its outcome belongs to the original delivery.
fn finished_delivery(task: &TasksExecution) -> Option<Outcome> {
    let annotations = task.annotations.as_ref()?;
    if let Some(receipt) = annotations.get("previouslyApplied") {
        return match (
            task.execution_status.as_str(),
            receipt.get("mode")?.as_str()?,
        ) {
            ("SUCCESS", "preview") => Some(Outcome::Previewed),
            ("SUCCESS", "apply") => Some(Outcome::Applied),
            ("FAILED", "apply") => Some(Outcome::Refused),
            _ => None,
        };
    }
    (task.execution_status == TasksExecutionStatus::SUCCESS.to_string()
        && annotations.pointer("/application/outcome")?.as_str()? == "applied")
        .then_some(Outcome::Applied)
}

/// Report the historical result without publishing another usable Preview or
/// submitting a section. Format 2 and security-key deliveries retain their
/// replay refusal, including the task's failure status.
async fn report_completed_application(
    tx: &deadpool_postgres::Transaction<'_>,
    target: &PlatformTarget,
    plan: &PackagePlan,
    task: &TasksExecution,
    event: &str,
    mode: ApplyMode,
) -> anyhow::Result<Option<Outcome>> {
    if task
        .annotations
        .as_ref()
        .is_some_and(|a| a.get(APPLICATION_KEY).is_some())
    {
        return Ok(None); // A redelivery must settle its own unfinished application.
    }
    let ctx = &target.context;
    let package = ctx.verified.as_ref().context("missing verified package")?;
    let rows = get_tasks_by_annotation(
        tx,
        &ctx.tenant_id,
        &ETasksExecution::APPLY_CONFIGURATION_PACKAGE.to_string(),
        PACKAGE_SHA256_KEY,
        &plan.package_sha256,
    )
    .await?;
    let Some(original) = completed_application(
        &rows,
        &ctx.tenant_id,
        plan,
        ctx.managed_roster_binding.as_ref(),
        event,
    )?
    else {
        return Ok(None);
    };
    crate::postgres::election_event::get_election_event_by_id(tx, &ctx.tenant_id, event)
        .await
        .context("the committed package destination could not be confirmed")?;
    if plan.payloads.contains_key(&Section::MessagingAccounts) {
        target.require_portable_messaging_permissions().await?;
    }
    let outcome = historical_outcome(
        mode,
        package.manifest.format,
        sequent_core::election_config::webauthn_package::is_browser_verified(package),
    );
    let message = format!("Task {original} previously applied this package to election event {event}. No sections were submitted.");
    let logs = crate::services::serialize_tasks_logs::append_general_log(&task.logs, &message);
    let status = if outcome == Outcome::Refused {
        TasksExecutionStatus::FAILED
    } else {
        TasksExecutionStatus::SUCCESS
    };
    update_with_annotations(
        &ctx.tenant_id,
        &task.id,
        status,
        serde_json::to_value(logs)?,
        historical_annotations(&plan.package_sha256, &original, event, mode),
    )
    .await?;
    Ok(Some(outcome))
}

/// A managed application names the exact successful preview it confirmed.
fn confirms_managed_preview(
    row: &TasksExecution,
    tenant_id: &str,
    package_sha256: &str,
    binding: Option<&Binding>,
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
                && record.managed_roster_binding.as_ref() == binding
                && record.managed_preview_operator_binding.as_deref() == Some(operator_binding)
                && record.managed_requirements.as_ref() == Some(&plan.requirements)
                && same_coverage(&record.coverage, &plan.coverage)
        })
}

fn valid_plan_digest(digest: &str) -> bool {
    digest.len() == 64
        && digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn messaging_preview_digest(preview: &TasksExecution) -> Option<String> {
    preview
        .annotations
        .as_ref()?
        .get(APPLICATION_KEY)?
        .get("messagingPlanSha256")?
        .as_str()
        .filter(|sha| valid_plan_digest(sha))
        .map(str::to_owned)
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

/// Check claim and status in the same update as the failure. A competing
/// delivery may have claimed or finished this task since our lock attempt.
async fn fail_delivery(
    tx: &deadpool_postgres::Transaction<'_>,
    task: &TasksExecution,
    error: &anyhow::Error,
) -> anyhow::Result<u64> {
    let logs = crate::services::serialize_tasks_logs::append_general_log(
        &task.logs,
        &format!("Error applying the configuration package: {error:#}"),
    );
    Ok(tx.execute(
        "UPDATE sequent_backend.tasks_execution SET execution_status='FAILED', end_at=now(), logs=$3
         WHERE tenant_id=$1 AND id=$2 AND type='APPLY_CONFIGURATION_PACKAGE'
           AND execution_status='IN_PROGRESS'
           AND (NOT $4 OR NOT (COALESCE(annotations, '{}'::jsonb) ? $5))",
        &[
            &uuid::Uuid::parse_str(&task.tenant_id)?,
            &uuid::Uuid::parse_str(&task.id)?,
            &serde_json::to_value(logs)?,
            &error.is::<Unclaimed>(),
            &CLAIMED_KEY,
        ],
    ).await?)
}

/// A staged document ID is not a password-handling channel. Detect the actual
/// transfer bytes (not a caller filename) before ZIP parsing for both modes.
fn require_unwrapped_package(bytes: &[u8]) -> anyhow::Result<()> {
    if is_encrypted_signed_delivery(bytes) {
        return Err(anyhow!(
            "This document is an encrypted configuration package (.ezip), not a signed ZIP. Use Upload configuration package, select the original .ezip, and enter its password there before Preview or Apply. An existing document ID cannot unlock it."
        ));
    }
    Ok(())
}

async fn execute(
    body: &ApplyConfigurationPackageBody,
    task_execution: &TasksExecution,
) -> anyhow::Result<Outcome> {
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
    // What a reset returns the tenant to, if its upload did not record it:
    // without it, this import could never be undone.
    if crate::services::tenant_reset::enabled() {
        crate::services::tenant_reset::ensure_baseline(&body.tenant_id, Some(&task_execution.id))
            .await
            .context("recording the tenant's reset baseline")
            .map_err(unclaimed)?;
    }
    let task_execution =
        get_task_by_id_with_transaction(&claim, &body.tenant_id, &task_execution.id)
            .await
            .map_err(unclaimed)?;
    // Leave a finished task's successful receipt or replay explanation intact.
    // Failed and unfinished applications still use the existing exact retry path.
    if let Some(outcome) = finished_delivery(&task_execution) {
        return Ok(outcome);
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
    let (file, protected_stage) =
        read_configuration_package(&transaction, &body.tenant_id, &document).await?;
    if body.encrypted_payload_sha256.is_some()
        && file.as_file().metadata()?.len() > MAX_SIGNED_DELIVERY_BYTES as u64
    {
        return Err(anyhow!("decrypted signed package exceeds the size limit"));
    }
    let bytes = std::fs::read(file.path()).context("could not read the package")?;
    require_unwrapped_package(&bytes)?;
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
    let imported_event = if let Some(package) = &verified {
        let last = configuration_packages::last_import_with_event(
            &transaction,
            &body.tenant_id,
            &package.manifest.configuration.external_id,
        )
        .await?;
        match crate::services::browser_package_policy::freshness(
            package,
            last.as_ref().map(|(head, _)| head),
        )
        .map_err(|problem| anyhow!("{problem}"))?
        {
            Freshness::New => None,
            Freshness::AlreadyImported => {
                let (head, event) =
                    last.context("the committed configuration revision is missing")?;
                anyhow::ensure!(
                    head.revision == package.manifest.configuration.revision,
                    "the committed configuration revision conflicts with the signed manifest"
                );
                Some(event)
            }
        }
    } else {
        None
    };
    if !protected_stage
        && members
            .iter()
            .any(|member| member.name == DEMO_LOGIN_VERIFIERS_MEMBER)
    {
        return Err(anyhow!(
            "upload this delivery using protected configuration package staging"
        ));
    }
    let demo_login = VerifiedDemoLoginAdmission::take_member(
        &mut members,
        verified.as_deref(),
        &body.tenant_id,
        &policy,
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
    let portable_accounts = plan.payloads.get(&Section::MessagingAccounts).map(|files| {
        if verified.as_ref().is_none_or(|package| {
            package.manifest.content.ballot_design_digest_algorithm
                != sequent_core::election_config::design::DesignDigestAlgorithm::V2
        }) || !plan.payloads.contains_key(&Section::ElectionEvent) {
            return Err(anyhow!("portable messaging accounts require a verified V2 package with an election event"));
        }
        if !body.portable_messaging_authorized {
            return Err(anyhow!("previewing or provisioning package messaging accounts needs tenant messaging-account-read, messaging-account-write and messaging-config-write"));
        }
        let [file] = files.as_slice() else {
            return Err(anyhow!("the signed package must carry exactly one portable messaging accounts member"));
        };
        portable_messaging::parse(&file.bytes).map_err(|_| anyhow!("invalid portable messaging accounts in the signed package"))
    }).transpose()?;
    let needs_operator = policy.managed_binding().is_some() || portable_accounts.is_some();
    let operator_binding = needs_operator.then(|| {
        body.managed_preview_operator_binding.as_deref().filter(|stamp| {
            valid_plan_digest(stamp)
        }).ok_or_else(|| anyhow!("sign in to this tenant's Admin Portal to preview or apply the signed delivery"))
    }).transpose()?;
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
    let mut target = PlatformTarget {
        event_imported_by_this_run: Default::default(),
        context: ApplyContext {
            tenant_id: body.tenant_id.clone(),
            executed_by: body.executed_by.clone(),
            audit_actor: body.audit_actor.clone(),
            event_import: body.event_import.clone(),
            package_sha256: package_sha256.clone(),
            signed_package,
            protected_document_id: protected_stage.then(|| body.document_id.clone()),
            managed_roster_binding: policy.managed_binding().cloned(),
            managed_preview_operator_binding: operator_binding.map(str::to_owned),
            package_groups,
            package_trustees,
            verified,
            demo_login,
            package_messaging: None,
        },
    };
    if let Some(event) = imported_event {
        if let Some(outcome) = report_completed_application(
            &transaction,
            &target,
            &plan,
            &task_execution,
            &event,
            body.mode,
        )
        .await?
        {
            return Ok(outcome);
        }
    }
    let mut confirmed_messaging = None;
    if let (ApplyMode::Apply, Some(operator_binding)) = (body.mode, operator_binding) {
        let refuse = || {
            anyhow!("confirm your successful preview of this package under the current receiver policy before applying")
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
            policy.managed_binding(),
            operator_binding,
            &plan,
        ) {
            return Err(refuse());
        }
        if portable_accounts.is_some() {
            confirmed_messaging = Some(messaging_preview_digest(&preview).ok_or_else(refuse)?);
        }
    }
    target.context.package_messaging = portable_accounts
        .map(|definitions| {
            // Delivered AWS key pairs live in the vault under the protected
            // staging document only; an ordinary document carries none.
            let staged_document = if protected_stage {
                Some(uuid::Uuid::parse_str(&body.document_id)?)
            } else {
                None
            };
            Ok::<_, anyhow::Error>(std::sync::Arc::new(PackageMessaging::new(
                sequent_core::services::uuid_validation::parse_uuid_v4(&body.tenant_id)?,
                package_sha256.clone(),
                definitions,
                confirmed_messaging,
                staged_document,
                // Harvest sets this from the verified JWT (audit::actor), never
                // from the request body: staged credentials need their uploader.
                body.audit_actor
                    .as_ref()
                    .and_then(|actor| actor.user_id.clone()),
            )))
        })
        .transpose()?;
    transaction.commit().await?;
    let prior = prior_record(&body.tenant_id, &package_sha256, &task_execution.id).await?;
    let store = TaskRowStore {
        task: task_execution.clone(),
        claim: &claim,
        logs: tokio::sync::Mutex::new(task_execution.logs.clone()),
        retry_document: body
            .audit_actor
            .as_ref()
            .and_then(|actor| actor.user_id.as_deref())
            .filter(|actor| {
                !actor.is_empty()
                    && body.mode == ApplyMode::Preview
                    && operator_binding.is_some()
                    && target.context.verified.is_some()
            })
            .map(|actor| RetryDocument {
                id: &body.document_id,
                actor,
                protected: protected_stage,
            }),
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
    record.map(|record| record.outcome)
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
        Ok(outcome) => match outcome {
            Outcome::Previewed | Outcome::Applied => Ok(()),
            outcome => Err(anyhow!("configuration package {outcome:?}").into()),
        },
        Err(error) => {
            let mut db: DbClient = get_hasura_pool()
                .await
                .get()
                .await
                .context("opening the task failure transaction")?;
            let tx = db.transaction().await?;
            fail_delivery(&tx, &task_execution, &error).await?;
            tx.commit().await?;
            Err(error.into())
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

    #[test]
    fn existing_document_id_rejects_encrypted_transfer_before_zip_for_preview_and_apply() {
        use sequent_core::election_config::archive::{zip, Artifact};
        use sequent_core::election_config::encrypted_delivery::{
            decrypt_signed_delivery, encrypt_signed_delivery,
        };
        let inner = zip(&[Artifact {
            name: "sample.txt".into(),
            bytes: b"demo".to_vec(),
        }])
        .unwrap();
        let wire = encrypt_signed_delivery(&inner, "synthetic test passphrase").unwrap();
        let error = require_unwrapped_package(&wire).unwrap_err().to_string();
        assert!(
            error.contains("Upload configuration package")
                && error.contains("existing document ID")
        );
        assert!(
            !error.contains("central directory") && !error.contains("synthetic test passphrase")
        );
        let unlocked = decrypt_signed_delivery(&wire, "synthetic test passphrase").unwrap();
        assert_eq!(unlocked, inner);
        require_unwrapped_package(&unlocked).unwrap(); // normal admission performs the signature check
    }

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
            messaging_plan_sha256: None,
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
            &old_row,
            "t",
            "p",
            Some(&active),
            &operator,
            &plan
        ));
        preview.managed_roster_binding = Some(active.clone());
        let current_row = row("confirmed", Some(&preview));
        assert!(!confirms_managed_preview(
            &current_row,
            "t",
            "p",
            Some(&active),
            &"c".repeat(64),
            &plan
        ));
        let mut unstamped = preview.clone();
        unstamped.managed_preview_operator_binding = None;
        assert!(!confirms_managed_preview(
            &row("old-unstamped", Some(&unstamped)),
            "t",
            "p",
            Some(&active),
            &operator,
            &plan
        ));
        assert!(confirms_managed_preview(
            &current_row,
            "t",
            "p",
            Some(&active),
            &operator,
            &plan
        ));
        plan.requirements.sections = vec![Section::Trustees];
        assert!(!confirms_managed_preview(
            &current_row,
            "t",
            "p",
            Some(&active),
            &operator,
            &plan
        ));
        plan.requirements = Requirements::default();
        assert!(!confirms_managed_preview(
            &current_row,
            "other",
            "p",
            Some(&active),
            &operator,
            &plan
        ));
        assert!(!confirms_managed_preview(
            &current_row,
            "t",
            "other",
            Some(&active),
            &operator,
            &plan
        ));
        preview.mode = ApplyMode::Apply;
        assert!(!confirms_managed_preview(
            &row("not-preview", Some(&preview)),
            "t",
            "p",
            Some(&active),
            &operator,
            &plan
        ));
    }

    #[test]
    fn the_recipient_plan_must_be_in_the_server_preview_receipt() {
        let mut preview = record(Outcome::Previewed);
        preview.mode = ApplyMode::Preview;
        let digest = "0a".repeat(32);
        preview.messaging_plan_sha256 = Some(digest.clone());
        assert_eq!(
            messaging_preview_digest(&row("preview", Some(&preview))),
            Some(digest)
        );
        for value in ["short".to_owned(), "z".repeat(64)] {
            preview.messaging_plan_sha256 = Some(value);
            assert_eq!(
                messaging_preview_digest(&row("preview", Some(&preview))),
                None
            );
        }
        preview.messaging_plan_sha256 = None;
        assert_eq!(
            messaging_preview_digest(&row("legacy-preview", Some(&preview))),
            None
        );
    }

    #[test]
    fn portable_nonmanaged_preview_is_still_bound_to_exact_operator_tenant_and_package() {
        let mut preview = record(Outcome::Previewed);
        preview.mode = ApplyMode::Preview;
        let operator = "bc".repeat(32);
        preview.managed_preview_operator_binding = Some(operator.clone());
        preview.managed_requirements = Some(Requirements::default());
        preview.messaging_plan_sha256 = Some("ad".repeat(32));
        let plan = crate::services::import::configuration_package_apply::PackagePlan {
            package_sha256: "p".into(),
            requirements: Requirements::default(),
            coverage: preview.coverage.clone(),
            payloads: BTreeMap::new(),
        };
        assert!(confirms_managed_preview(
            &row("preview", Some(&preview)),
            "t",
            "p",
            None,
            &operator,
            &plan,
        ));
        for (tenant, digest, actor) in [
            ("different", "p", operator.as_str()),
            ("t", "other-package", operator.as_str()),
            ("t", "p", "other-authorized-operator"),
        ] {
            assert!(!confirms_managed_preview(
                &row("preview", Some(&preview)),
                tenant,
                digest,
                None,
                actor,
                &plan,
            ));
        }
        preview.outcome = Outcome::Refused;
        assert!(!confirms_managed_preview(
            &row("failed-preview", Some(&preview)),
            "t",
            "p",
            None,
            &operator,
            &plan,
        ));
    }

    #[tokio::test]
    async fn retry_hint_can_only_annotate_a_new_active_task_with_the_matching_protected_uploader() {
        let pool = super::apply_test_schema::pool().await;
        let mut client = pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        let tenant = uuid::Uuid::new_v4();
        let other = uuid::Uuid::new_v4();
        for tenant in [tenant, other] {
            tx.execute(
                "INSERT INTO sequent_backend.tenant (id, slug, is_active) VALUES ($1, $2, true)",
                &[&tenant, &format!("retry-test-{tenant}")],
            )
            .await
            .unwrap();
        }
        let document: uuid::Uuid = tx
            .query_one(
                "INSERT INTO sequent_backend.document (tenant_id, name, is_public) VALUES ($1, 'test.zip', false) RETURNING id",
                &[&tenant],
            )
            .await
            .unwrap()
            .get(0);
        tx.execute(
            "INSERT INTO sequent_backend.configuration_package_staging (tenant_id, document_id, uploaded_by) VALUES ($1, $2, 'uploader')",
            &[&tenant, &document],
        )
        .await
        .unwrap();
        let task = uuid::Uuid::new_v4();
        let completed = uuid::Uuid::new_v4();
        let own_record = uuid::Uuid::new_v4();
        for (id, status, annotations) in [
            (task, "IN_PROGRESS", json!({CLAIMED_KEY: true})),
            (
                completed,
                "SUCCESS",
                json!({"previouslyApplied": {"mode": "preview"}}),
            ),
            (
                own_record,
                "IN_PROGRESS",
                json!({APPLICATION_KEY: {"outcome": "applied"}}),
            ),
        ] {
            tx.execute(
                "INSERT INTO sequent_backend.tasks_execution (id, tenant_id, name, type, execution_status, annotations, executed_by_user)
                 VALUES ($1, $2, 'retry test', 'APPLY_CONFIGURATION_PACKAGE', $3, $4, 'uploader')",
                &[&id, &tenant, &status, &annotations],
            )
            .await
            .unwrap();
        }

        let hint = json!({"packageRetry":{"sections":["election-event"],"census":"required"}});
        for (id, tenant, actor, wanted) in [
            (task, other, "uploader", 0),
            (task, tenant, "someone-else", 0),
            (completed, tenant, "uploader", 0),
            (own_record, tenant, "uploader", 0),
            (task, tenant, "uploader", 1),
        ] {
            let changed = tx
                .execute(
                    STORE_RETRY_REQUIREMENTS,
                    &[&tenant, &id, &hint, &true, &document, &actor],
                )
                .await
                .unwrap();
            assert_eq!(changed, wanted);
        }
        let annotations: serde_json::Value = tx
            .query_one(
                "SELECT annotations FROM sequent_backend.tasks_execution WHERE id = $1",
                &[&task],
            )
            .await
            .unwrap()
            .get(0);
        assert_eq!(
            annotations,
            json!({CLAIMED_KEY: true, "packageRetry": hint["packageRetry"]})
        );
        let mut hinted_task = row("hint-only", None);
        hinted_task.annotations = Some(annotations);
        assert!(choose_prior(vec![hinted_task], "other").unwrap().is_none());
        tx.rollback().await.unwrap();
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

    #[test]
    fn historical_preview_and_strict_apply_have_distinct_terminal_results() {
        for (mode, format, browser, expected, status) in [
            (
                ApplyMode::Preview,
                ManifestFormat::V2,
                true,
                Outcome::Previewed,
                "SUCCESS",
            ),
            (
                ApplyMode::Apply,
                ManifestFormat::V2,
                false,
                Outcome::Refused,
                "FAILED",
            ),
            (
                ApplyMode::Apply,
                ManifestFormat::V1,
                true,
                Outcome::Refused,
                "FAILED",
            ),
            (
                ApplyMode::Apply,
                ManifestFormat::V1,
                false,
                Outcome::Applied,
                "SUCCESS",
            ),
        ] {
            let outcome = historical_outcome(mode, format, browser);
            assert_eq!(outcome, expected);
            let mut task = row("repeat", None);
            task.execution_status = status.into();
            task.annotations = Some(historical_annotations("digest", "original", "event", mode));
            let annotations = task.annotations.as_ref().unwrap();
            assert_eq!(annotations[CLAIMED_KEY], true);
            assert!(annotations.get(APPLICATION_KEY).is_none());
            assert_eq!(finished_delivery(&task), Some(expected));
            task.execution_status = "IN_PROGRESS".into();
            assert_eq!(finished_delivery(&task), None);
        }
    }

    #[test]
    fn repeat_delivery_needs_the_committed_destination_and_a_completed_exact_apply() {
        use crate::services::import::configuration_package_apply::{
            PackagePlan, SectionHistory, SectionRecord, SectionStatus,
        };
        use crate::services::import::configuration_package_target::derived_event_id;
        let mut applied = record(Outcome::Applied);
        let binding = Binding {
            sha256: "ef".repeat(32),
            generation: 4,
            required_approvals: 2,
        };
        applied.managed_roster_binding = Some(binding.clone());
        applied.managed_requirements = Some(Requirements::default());
        applied.managed_preview_operator_binding = Some("ba".repeat(32));
        applied.messaging_plan_sha256 = Some("ab".repeat(32));
        let plan = PackagePlan {
            package_sha256: applied.package_sha256.clone(),
            requirements: Requirements::default(),
            coverage: applied.coverage.clone(),
            payloads: BTreeMap::from([
                (Section::ElectionEvent, vec![]),
                (Section::MessagingAccounts, vec![]),
            ]),
        };
        let event = derived_event_id("t", &plan.package_sha256);
        applied.sections.push(SectionRecord {
            section: Section::ElectionEvent,
            members: vec![],
            status: SectionStatus::Applied {
                submission: None,
                reconciled: true,
            },
            history: SectionHistory {
                applied: true,
                ..Default::default()
            },
        });
        // R5 recorded the reviewed messaging plan. Its submission has no installation fingerprint.
        let mut messaging = applied.sections[0].clone();
        messaging.section = Section::MessagingAccounts;
        messaging.history.submissions.push(
            crate::services::import::configuration_package_apply::SectionSubmission {
                task_id: None,
                detail: json!({"accounts": 1}),
            },
        );
        applied.sections.push(messaging);
        let completed = row("original", Some(&applied));
        for changed in [
            None,
            Some(Binding {
                required_approvals: 1,
                ..binding.clone()
            }),
            Some(Binding {
                generation: 5,
                ..binding.clone()
            }),
        ] {
            assert!(completed_application(
                std::slice::from_ref(&completed),
                "t",
                &plan,
                changed.as_ref(),
                &event
            )
            .unwrap()
            .is_none());
        }
        let mut changed_plan = plan.clone();
        changed_plan.requirements.sections.push(Section::Trustees);
        assert!(completed_application(
            std::slice::from_ref(&completed),
            "t",
            &changed_plan,
            Some(&binding),
            &event
        )
        .unwrap()
        .is_none());
        let receipt = |rows: &[TasksExecution], tenant: &str, event: &str| {
            completed_application(rows, tenant, &plan, Some(&binding), event).unwrap()
        };
        assert_eq!(
            receipt(std::slice::from_ref(&completed), "t", &event).as_deref(),
            Some("original")
        );
        for (tenant, destination) in [("other", event.as_str()), ("t", "another-event")] {
            assert_eq!(
                receipt(std::slice::from_ref(&completed), tenant, destination),
                None
            );
        }
        for outcome in [
            Outcome::Failed,
            Outcome::Running,
            Outcome::Refused,
            Outcome::Unknown,
        ] {
            let mut partial = applied.clone();
            partial.outcome = outcome;
            assert_eq!(
                receipt(
                    &[row("later", Some(&partial)), completed.clone()],
                    "t",
                    &event
                ),
                None
            );
        }
        let mut preview = applied.clone();
        preview.mode = ApplyMode::Preview;
        preview.outcome = Outcome::Previewed;
        assert_eq!(
            receipt(&[row("preview", Some(&preview))], "t", &event),
            None
        );
        let mut failed = completed.clone();
        failed.execution_status = "FAILED".into();
        assert_eq!(receipt(&[failed], "t", &event), None);
        let mut other_digest = applied.clone();
        other_digest.package_sha256 = "different bytes".into();
        assert_eq!(
            receipt(&[row("different", Some(&other_digest))], "t", &event),
            None
        );
        applied.sections[0].history.in_flight = Some("unsettled".into());
        assert_eq!(
            receipt(&[row("interrupted", Some(&applied))], "t", &event),
            None
        );
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
        let task = &found[0];
        let task_id = uuid::Uuid::parse_str(&task.id).unwrap();
        let receipt = historical_annotations("aaaa", "original", "event", ApplyMode::Preview);
        let busy = unclaimed(anyhow!("another delivery holds the claim"));
        // The same conditional UPDATE runs for a competing delivery. Neither
        // a live owner's marker nor either terminal outcome may be overwritten.
        for status in ["IN_PROGRESS", "SUCCESS", "FAILED"] {
            tx.execute("UPDATE sequent_backend.tasks_execution SET execution_status=$2, annotations=$3 WHERE id=$1",
                &[&task_id, &status, &receipt]).await.unwrap();
            assert_eq!(fail_delivery(&tx, task, &busy).await.unwrap(), 0);
            let saved = get_task_by_id_with_transaction(&tx, &task.tenant_id, &task.id)
                .await
                .unwrap();
            assert_eq!(saved.execution_status, status);
            assert_eq!(saved.annotations.as_ref(), Some(&receipt));
        }
        tx.execute("UPDATE sequent_backend.tasks_execution SET execution_status='IN_PROGRESS', annotations=NULL WHERE id=$1", &[&task_id]).await.unwrap();
        assert_eq!(fail_delivery(&tx, task, &busy).await.unwrap(), 1);
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
