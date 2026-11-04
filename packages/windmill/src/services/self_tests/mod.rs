// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
//! Background TSF tests. Missing evidence, timeouts and incomplete runs fail closed.
mod checks;
mod probes;
mod protocol;
pub mod snapshot;
pub mod tsf_integrity;
use crate::services::{
    database::get_hasura_pool, election_lifecycle, electoral_log_audit_deadline::AuditDeadline,
};
use anyhow::{ensure, Context, Result};
use chrono::Utc;
use deadpool_postgres::Transaction;
use sequent_core::{
    election_lifecycle::ElectoralPhase,
    self_tests::*,
    types::hasura::{core::TasksExecution, extra::TasksExecutionStatus},
};
use serde_json::{json, Value};
use std::time::Duration;

const DEADLINE_EXCEEDED: &str = "The configured suite deadline was exceeded";
/// Polls every two seconds outlast the longest configurable run, its sweep margin
/// and a sweep tick. A run still open by then no longer holds up the next event.
const STARTUP_RUN_POLLS: u16 = 330;

pub async fn settings(tx: &Transaction<'_>, tenant: &str, event: &str) -> Result<SelfTestSettings> {
    let row=tx.query_opt("SELECT settings FROM sequent_backend.self_test_settings WHERE tenant_id=$1::text::uuid AND election_event_id=$2::text::uuid",&[&tenant,&event]).await?;
    let value: SelfTestSettings = match row {
        Some(r) => serde_json::from_value(r.try_get(0)?)?,
        None => SelfTestSettings::default(),
    };
    value.validate().map_err(anyhow::Error::msg)?;
    Ok(value)
}

/// Queuing never waits for the suite; the caller receives a durable task identity.
pub async fn enqueue(
    tenant: &str,
    event: &str,
    trigger: SelfTestTrigger,
    executed_by: &str,
) -> Result<TasksExecution> {
    AuditDeadline::database()
        .run(enqueue_inner(tenant, event, trigger, executed_by))
        .await?
}
async fn enqueue_inner(
    tenant: &str,
    event: &str,
    trigger: SelfTestTrigger,
    executed_by: &str,
) -> Result<TasksExecution> {
    let mut c = get_hasura_pool().await.get().await?;
    let tx = c.transaction().await?;
    tx.query_one(
        "SELECT pg_advisory_xact_lock(hashtextextended($1,0))",
        &[&format!("self-tests:enqueue:{tenant}:{event}")],
    )
    .await?;
    if let Some(row)=tx.query_opt("SELECT id::text FROM sequent_backend.self_test_run WHERE tenant_id=$1::text::uuid AND election_event_id=$2::text::uuid AND finished_at IS NULL",&[&tenant,&event]).await? {
        let id:String=row.try_get(0)?;tx.commit().await?;return crate::postgres::tasks_execution::get_task_by_id(&id).await;
    }
    let generation = election_lifecycle::state(&tx, tenant, event)
        .await?
        .generation;
    let task = crate::services::tasks_execution::post(
        tenant,
        Some(event),
        crate::types::tasks::ETasksExecution::RUN_SELF_TESTS,
        executed_by,
    )
    .await?;
    let run = SelfTestRun {
        run_id: task.id.clone(),
        generation,
        trigger,
        started_at: Utc::now().to_rfc3339(),
        finished_at: None,
        outcome: SelfTestOutcome::RecoverableFailure,
        checks: vec![],
    };
    tx.execute("INSERT INTO sequent_backend.self_test_run(id,tenant_id,election_event_id,generation,trigger,result) VALUES($1::text::uuid,$2::text::uuid,$3::text::uuid,$4,$5,$6)",&[&task.id,&tenant,&event,&generation,&serde_json::to_value(trigger)?.as_str().context("Invalid trigger")?,&serde_json::to_value(&run)?]).await?;
    tx.commit().await?;
    drop(c);
    let sent = crate::services::celery_app::get_celery_app()
        .await
        .send_task(crate::tasks::self_tests::run_self_tests::new(
            tenant.into(),
            event.into(),
            task.id.clone(),
        ))
        .await;
    if sent.is_err() {
        let mut failure = run;
        failure.finished_at = Some(Utc::now().to_rfc3339());
        finish_abandoned(tenant, event, &failure).await?;
        anyhow::bail!("Unable to queue self-test; no successful result was recorded");
    }
    Ok(task)
}

/// A run no worker finished. Its failure is persisted whether or not the
/// completion record can be written.
async fn finish_abandoned(tenant: &str, event: &str, run: &SelfTestRun) -> Result<()> {
    let written = AuditDeadline::database()
        .run(checks::audit_record(
            tenant,
            event,
            &run.run_id,
            "completed",
            &serde_json::to_value(run)?,
        ))
        .await;
    if !matches!(written, Ok(Ok(()))) {
        tracing::warn!(%event, run_id = %run.run_id, "Self-test completion record could not be written");
    }
    persist(tenant, event, run).await
}

async fn persist(tenant: &str, event: &str, run: &SelfTestRun) -> Result<()> {
    let mut c = get_hasura_pool().await.get().await?;
    let tx = c.transaction().await?;
    // A late worker may never replace the sweeper's durable timeout result.
    let pending=tx.query_opt("SELECT generation FROM sequent_backend.self_test_run WHERE id=$1::text::uuid AND tenant_id=$2::text::uuid AND election_event_id=$3::text::uuid AND finished_at IS NULL FOR UPDATE",&[&run.run_id,&tenant,&event]).await?;
    if pending.is_none() {
        return Ok(());
    }
    let mut run = run.clone();
    // A lifecycle change overtook this run: its pass is no permit, but no failed
    // check either. The next run of the new generation decides.
    let overtaken = run.outcome == SelfTestOutcome::Passed
        && election_lifecycle::state(&tx, tenant, event)
            .await?
            .generation
            != run.generation;
    if overtaken {
        run.outcome = SelfTestOutcome::RecoverableFailure;
        if let Some(check) = run
            .checks
            .iter_mut()
            .find(|c| c.check == SelfTestCheck::AccessControl)
        {
            check.outcome = SelfTestOutcome::RecoverableFailure;
            check.detail = "Election state changed before the result was committed".into();
        }
    }
    tx.execute("UPDATE sequent_backend.self_test_run SET finished_at=now(),outcome=$4,result=$5 WHERE id=$1::text::uuid AND tenant_id=$2::text::uuid AND election_event_id=$3::text::uuid",&[&run.run_id,&tenant,&event,&serde_json::to_value(run.outcome)?.as_str().context("Invalid outcome")?,&serde_json::to_value(&run)?]).await?;
    tx.commit().await?;
    // First commit the refusal evidence. A busy safety writer cannot discard it;
    // the access view denies immediately and the lifecycle sweep retries the notice.
    if !overtaken {
        let tx = c.transaction().await?;
        match election_lifecycle::record_self_test_outcome(&tx, tenant, event, &run).await {
            Ok(()) => tx.commit().await?,
            Err(_) => {
                tracing::warn!(%event,"Self-test safety transition deferred to lifecycle sweep");
            }
        }
    }
    crate::services::tasks_execution::update_with_annotations(
        tenant,
        &run.run_id,
        if run.outcome == SelfTestOutcome::Passed {
            TasksExecutionStatus::SUCCESS
        } else {
            TasksExecutionStatus::FAILED
        },
        json!({"self_tests":run}),
        json!({"self_tests":run}),
    )
    .await?;
    Ok(())
}

/// Findings that contradict the records end the election. Unavailable evidence
/// fails this run, and a later run can still verify it.
fn audit_verdict(
    audit: &crate::services::electoral_log_audit::AuditResult,
) -> Result<Option<String>> {
    if audit.has_verified_findings() {
        return Ok(Some("Electoral-log integrity verification failed".into()));
    }
    ensure!(
        audit.findings.is_empty(),
        "Electoral-log integrity could not be verified"
    );
    Ok(None)
}

/// A start-up run before the first approval has no board-approved values to
/// compare with. It authorises nothing: an opening requires its own trigger and
/// voters require the execution phase.
fn nothing_to_compare(trigger: SelfTestTrigger, phase: ElectoralPhase, baseline: bool) -> bool {
    trigger == SelfTestTrigger::Startup && phase == ElectoralPhase::Preparation && !baseline
}

fn classify(check: SelfTestCheck, result: Result<Option<String>>) -> (SelfTestOutcome, String) {
    match result {
        Ok(None) => (SelfTestOutcome::Passed, "Verified".into()),
        Ok(Some(reason)) => (SelfTestOutcome::UnrecoverableFailure, reason),
        Err(error) => {
            // Driver/HTTP errors may embed storage URLs or connection details.
            // Persist only a controlled diagnostic in staff-visible results.
            let detail = if error.to_string() == DEADLINE_EXCEEDED {
                DEADLINE_EXCEEDED
            } else {
                match check {
                    SelfTestCheck::BallotBox=>"Ballot-box evidence could not be read or verified",
                    SelfTestCheck::ElectionData=>"Election data differs from the board-approved baseline, or could not be verified",
                    SelfTestCheck::AuditGeneration=>"Audit write, read-back or integrity verification failed",
                    SelfTestCheck::TrustedChannel=>"A public endpoint failed the required TLS profile or certificate checks",
                    SelfTestCheck::AccessControl=>"Access-control settings differ from the approved baseline, or external request monitoring is unavailable",
                    SelfTestCheck::Clock=>"The host clock observation is unavailable, stale or not synchronized with NTS",
                }
            };
            (SelfTestOutcome::RecoverableFailure, detail.into())
        }
    }
}

/// An incomplete run never passes, and keeps an unrecoverable result it found.
fn unfinished_outcome(run: &SelfTestRun) -> SelfTestOutcome {
    if run
        .checks
        .iter()
        .any(|c| c.outcome == SelfTestOutcome::UnrecoverableFailure)
    {
        SelfTestOutcome::UnrecoverableFailure
    } else {
        SelfTestOutcome::RecoverableFailure
    }
}

fn completion_record_failed(run: &mut SelfTestRun) {
    run.outcome = unfinished_outcome(run);
    let audit = &mut run.checks[2];
    if audit.outcome != SelfTestOutcome::UnrecoverableFailure {
        audit.outcome = SelfTestOutcome::RecoverableFailure;
        audit.detail = "The completion audit record could not be verified".into();
    }
}

pub async fn execute(tenant: &str, event: &str, id: &str) -> Result<()> {
    let setup = AuditDeadline::database();
    let mut c = setup
        .run(async { get_hasura_pool().await.get().await })
        .await??;
    let lock = setup.run(c.transaction()).await??;
    let acquired: bool = setup
        .run(lock.query_one(
            "SELECT pg_try_advisory_xact_lock(hashtextextended($1,0))",
            &[&format!("self-tests:execute:{event}")],
        ))
        .await??
        .try_get(0)?;
    if !acquired {
        return Ok(());
    }
    let Some(row)=lock.query_opt("SELECT result FROM sequent_backend.self_test_run WHERE id=$1::text::uuid AND tenant_id=$2::text::uuid AND election_event_id=$3::text::uuid AND finished_at IS NULL",&[&id,&tenant,&event]).await? else{return Ok(())};
    let mut run: SelfTestRun = serde_json::from_value(row.try_get(0)?)?;
    let config = settings(&lock, tenant, event).await?;
    let baseline:bool=lock.query_one("SELECT EXISTS(SELECT 1 FROM sequent_backend.self_test_baseline WHERE tenant_id=$1::text::uuid AND election_event_id=$2::text::uuid)",&[&tenant,&event]).await?.try_get(0)?;
    let skip_comparison = nothing_to_compare(
        run.trigger,
        election_lifecycle::state(&lock, tenant, event).await?.phase,
        baseline,
    );
    let budget = AuditDeadline::after(Duration::from_secs(u64::from(config.timeout_seconds)));
    let start = budget
        .run(checks::audit_record(
            tenant,
            event,
            id,
            "started",
            &json!({"run_id":id,"trigger":run.trigger}),
        ))
        .await;
    let mut snapshot_start: Option<Value> = None;
    for check in SelfTestCheck::ALL {
        if skip_comparison
            && matches!(
                check,
                SelfTestCheck::ElectionData | SelfTestCheck::AccessControl
            )
        {
            run.checks.push(SelfTestCheckResult {
                check,
                outcome: SelfTestOutcome::Passed,
                detail: "No board-approved baseline yet; nothing to compare".into(),
            });
            continue;
        }
        let result: Result<Option<String>> = match budget
            .run(async {
                match check {
                    SelfTestCheck::BallotBox => checks::ballot_boxes(tenant, event).await,
                    SelfTestCheck::ElectionData | SelfTestCheck::AccessControl => {
                        let mut db = get_hasura_pool().await.get().await?;
                        let tx = db.transaction().await?;
                        tx.batch_execute("SET LOCAL statement_timeout='10000ms'")
                            .await?;
                        let baseline = snapshot::approved(&tx, tenant, event).await?;
                        let current = snapshot::capture(&tx, tenant, event).await?;
                        if let Some(previous) = &snapshot_start {
                            ensure!(
                                snapshot::unchanged_since(previous, &current),
                                "Configuration changed during the self-test"
                            );
                        } else {
                            snapshot_start = Some(current.clone());
                        }
                        let key = if check == SelfTestCheck::ElectionData {
                            "election_data"
                        } else {
                            "access_control"
                        };
                        ensure!(
                            baseline.get(key).is_some()
                                && baseline[key] == current[key]
                                && (check != SelfTestCheck::AccessControl
                                    || snapshot::staff_only_narrowed(&baseline, &current)),
                            "Current {key} differs from the board-approved baseline"
                        );
                        if check == SelfTestCheck::AccessControl {
                            let monitored: bool = tx
                                .query_one(
                                    "SELECT sequent_backend.attack_activity_allows_voters()",
                                    &[],
                                )
                                .await?
                                .try_get(0)?;
                            ensure!(
                                monitored,
                                "External request activity is unavailable, stale or discontinuous"
                            );
                        }
                        Ok(None)
                    }
                    SelfTestCheck::AuditGeneration => {
                        ensure!(
                            matches!(&start, Ok(Ok(()))),
                            "The self-test start audit record could not be verified"
                        );
                        checks::audit_record(
                            tenant,
                            event,
                            id,
                            "probe",
                            &json!({"run_id":id,"nonce":uuid::Uuid::new_v4().to_string()}),
                        )
                        .await?;
                        let result = crate::services::electoral_log_audit::run_electoral_log_audit(
                            tenant, event,
                        )
                        .await?;
                        audit_verdict(&result)
                    }
                    SelfTestCheck::TrustedChannel => {
                        probes::trusted_channels().await?;
                        Ok(None)
                    }
                    SelfTestCheck::Clock => {
                        probes::clock()?;
                        Ok(None)
                    }
                }
            })
            .await
        {
            Ok(v) => v,
            Err(_) => Err(anyhow::anyhow!(DEADLINE_EXCEEDED)),
        };
        if result.is_err() {
            tracing::warn!(?check, run_id = id, "Self-test check failed");
        }
        let (outcome, detail) = classify(check, result);
        run.checks.push(SelfTestCheckResult {
            check,
            outcome,
            detail,
        });
        let progress = serde_json::to_value(&run)?;
        let _=budget.run(async {
            let db=get_hasura_pool().await.get().await?;
            db.execute("UPDATE sequent_backend.self_test_run SET result=$2 WHERE id=$1::text::uuid AND finished_at IS NULL",&[&id,&progress]).await?;
            Ok::<_,anyhow::Error>(())
        }).await;
    }
    run.outcome = run.aggregate();
    if run.outcome == SelfTestOutcome::Passed {
        let stable = budget
            .run(async {
                let mut db = get_hasura_pool().await.get().await?;
                let tx = db.transaction().await?;
                // persist() refuses the pass if the generation has moved meanwhile.
                let unchanged = skip_comparison || {
                    let current = snapshot::capture(&tx, tenant, event).await?;
                    snapshot_start
                        .as_ref()
                        .is_some_and(|start| snapshot::unchanged_since(start, &current))
                };
                ensure!(unchanged, "Configuration changed during the self-test");
                Ok::<_, anyhow::Error>(())
            })
            .await;
        if !matches!(stable, Ok(Ok(()))) {
            run.outcome = SelfTestOutcome::RecoverableFailure;
            run.checks[4].outcome = SelfTestOutcome::RecoverableFailure;
            run.checks[4].detail =
                "State or configuration changed, or its final verification timed out".into();
        }
    }
    run.finished_at = Some(Utc::now().to_rfc3339());
    if AuditDeadline::database()
        .run(checks::audit_record(
            tenant,
            event,
            id,
            "completed",
            &serde_json::to_value(&run)?,
        ))
        .await
        .map_or(true, |r| r.is_err())
    {
        completion_record_failed(&mut run);
    }
    AuditDeadline::database()
        .run(persist(tenant, event, &run))
        .await??;
    lock.commit().await?;
    Ok(())
}

pub async fn periodic() -> Result<()> {
    election_lifecycle::tick().await?;
    if let Err(error) = tsf_integrity::sweep().await {
        tracing::error!("TSF integrity sweep deferred: {error:#}");
    }
    let mut c = get_hasura_pool().await.get().await?;
    let tx = c.transaction().await?;
    tx.batch_execute("SET LOCAL statement_timeout='10000ms'")
        .await?;
    let events=tx.query("SELECT e.tenant_id::text,e.id::text FROM sequent_backend.election_event e JOIN sequent_backend.election_lifecycle l ON l.tenant_id=e.tenant_id AND l.election_event_id=e.id LEFT JOIN sequent_backend.self_test_settings s ON s.tenant_id=e.tenant_id AND s.election_event_id=e.id WHERE l.phase='execution' AND l.safety='operational' AND NOT EXISTS(SELECT 1 FROM sequent_backend.self_test_run r WHERE r.tenant_id=e.tenant_id AND r.election_event_id=e.id AND (r.finished_at IS NULL OR (r.generation=l.generation AND r.finished_at>now()-make_interval(secs=>GREATEST(0,COALESCE((s.settings->>'interval_minutes')::int,15)*60-COALESCE((s.settings->>'timeout_seconds')::int,60)-30))))) ORDER BY e.id LIMIT 100",&[]).await?;
    let stale=tx.query("SELECT r.tenant_id::text,r.election_event_id::text,r.result FROM sequent_backend.self_test_run r LEFT JOIN sequent_backend.self_test_settings s USING(tenant_id,election_event_id) WHERE r.finished_at IS NULL AND r.started_at<now()-make_interval(secs=>COALESCE((s.settings->>'timeout_seconds')::int,60)+30)",&[]).await?;
    tx.commit().await?;
    drop(c);
    for row in stale {
        let mut run: SelfTestRun = serde_json::from_value(row.try_get(2)?)?;
        run.outcome = unfinished_outcome(&run);
        run.finished_at = Some(Utc::now().to_rfc3339());
        finish_abandoned(
            &row.try_get::<_, String>(0)?,
            &row.try_get::<_, String>(1)?,
            &run,
        )
        .await?;
    }
    for row in events {
        let tenant: String = row.try_get(0)?;
        let event: String = row.try_get(1)?;
        if let Err(e) = enqueue(&tenant, &event, SelfTestTrigger::Periodic, "system").await {
            tracing::error!(%event,"Self-test queue failure: {e:#}");
        }
    }
    Ok(())
}

pub async fn status(tenant: &str, event: &str) -> Result<Value> {
    let mut c = get_hasura_pool().await.get().await?;
    let tx = c.transaction().await?;
    let settings = settings(&tx, tenant, event).await?;
    let rows=tx.query("SELECT result FROM sequent_backend.self_test_run WHERE tenant_id=$1::text::uuid AND election_event_id=$2::text::uuid ORDER BY started_at DESC LIMIT 25",&[&tenant,&event]).await?;
    let runs: Vec<Value> = rows
        .iter()
        .map(|r| r.try_get(0))
        .collect::<Result<_, _>>()?;
    let baseline:bool=tx.query_one("SELECT EXISTS(SELECT 1 FROM sequent_backend.self_test_baseline WHERE tenant_id=$1::text::uuid AND election_event_id=$2::text::uuid)",&[&tenant,&event]).await?.try_get(0)?;
    let notifications:Vec<Value>=tx.query("SELECT jsonb_build_object('id',id,'kind',kind,'state',state,'created_at',created_at,'attempted_at',attempted_at,'delivered_at',delivered_at,'failure',failure) FROM sequent_backend.security_notification WHERE tenant_id=$1::text::uuid AND election_event_id=$2::text::uuid ORDER BY created_at DESC LIMIT 20",&[&tenant,&event]).await?.iter().map(|r|r.try_get(0)).collect::<Result<_,_>>()?;
    let tsf_integrity = tsf_integrity::latest(&tx, tenant, event).await?;
    Ok(
        json!({"settings":settings,"runs":runs,"baseline_present":baseline,"notifications":notifications,"tsf_integrity":tsf_integrity}),
    )
}

pub async fn update_settings(
    tenant: &str,
    event: &str,
    actor: &str,
    value: &SelfTestSettings,
) -> Result<()> {
    value.validate().map_err(anyhow::Error::msg)?;
    let mut c = get_hasura_pool().await.get().await?;
    let tx = c.transaction().await?;
    election_lifecycle::require_phase(
        &tx,
        tenant,
        event,
        sequent_core::election_lifecycle::ElectoralPhase::Preparation,
    )
    .await?;
    tx.execute("INSERT INTO sequent_backend.self_test_settings(tenant_id,election_event_id,settings) VALUES($1::text::uuid,$2::text::uuid,$3) ON CONFLICT(tenant_id,election_event_id) DO UPDATE SET settings=EXCLUDED.settings",&[&tenant,&event,&serde_json::to_value(value)?]).await?;
    // Refuse the change if its durable audit record cannot be written.
    checks::audit_record(
        tenant,
        event,
        &uuid::Uuid::new_v4().to_string(),
        "settings-changed",
        &json!({"actor": actor, "settings": value}),
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

/// A process restart invalidates old operational evidence before new tasks run.
pub async fn startup() -> Result<()> {
    election_lifecycle::register_startup("windmill").await?;
    startup_runs();
    Ok(())
}

/// Queue the start-up runs in the background. They wait for a worker, which may
/// be the caller, and failing to queue them never stops the caller from starting.
pub fn startup_runs() {
    tokio::spawn(async {
        if let Err(error) = queue_startup_runs().await {
            tracing::error!("Start-up self-tests could not be queued: {error:#}");
        }
    });
}

/// One run at a time: the stale sweep counts from the moment a run is queued, so
/// runs queued together could expire waiting for a worker and fail falsely.
async fn queue_startup_runs() -> Result<()> {
    let mut c = get_hasura_pool().await.get().await?;
    let tx = c.transaction().await?;
    // With no live worker a queued run could only expire. A worker queues these
    // runs itself when it starts.
    let worker:bool=tx.query_one("SELECT EXISTS(SELECT 1 FROM sequent_backend.lifecycle_service_lease WHERE component='windmill' AND expires_at>clock_timestamp())",&[]).await?.try_get(0)?;
    if !worker {
        return Ok(());
    }
    // An event commits together with its board and signing keys, so one with no
    // board has nothing to test yet. Past execution, an event with no baseline
    // never opened and can never pass; it would only fail at every restart.
    let events=tx.query("SELECT l.tenant_id::text,l.election_event_id::text FROM sequent_backend.election_lifecycle l JOIN sequent_backend.election_event e ON e.tenant_id=l.tenant_id AND e.id=l.election_event_id WHERE COALESCE(e.is_archived,false)=false AND e.bulletin_board_reference->>'database_name' IS NOT NULL AND (l.phase IN ('preparation','execution') OR EXISTS(SELECT 1 FROM sequent_backend.self_test_baseline b WHERE b.tenant_id=l.tenant_id AND b.election_event_id=l.election_event_id)) ORDER BY l.phase='execution' DESC,l.tenant_id,l.election_event_id",&[]).await?;
    tx.commit().await?;
    drop(c);
    for row in events {
        let tenant: String = row.try_get(0)?;
        let event: String = row.try_get(1)?;
        match enqueue(&tenant, &event, SelfTestTrigger::Startup, "system").await {
            Ok(task) => wait_for_run(&task.id).await,
            Err(error) => {
                tracing::error!(%event,"Start-up self-test could not be queued: {error:#}")
            }
        }
    }
    Ok(())
}

async fn wait_for_run(id: &str) {
    for _ in 0..STARTUP_RUN_POLLS {
        tokio::time::sleep(Duration::from_secs(2)).await;
        let finished = async {
            let db = get_hasura_pool().await.get().await?;
            let pending=db.query_opt("SELECT 1 FROM sequent_backend.self_test_run WHERE id=$1::text::uuid AND finished_at IS NULL",&[&id]).await?;
            Ok::<_, anyhow::Error>(pending.is_none())
        };
        if !matches!(finished.await, Ok(false)) {
            return;
        }
    }
}

pub async fn additional(tenant: &str, event: &str, trigger: SelfTestTrigger) -> Result<()> {
    let mut c = get_hasura_pool().await.get().await?;
    let tx = c.transaction().await?;
    let enabled = settings(&tx, tenant, event)
        .await?
        .additional_conditions
        .contains(&trigger);
    tx.commit().await?;
    drop(c);
    if enabled {
        enqueue(tenant, event, trigger, "system").await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::electoral_log_audit::AuditResult;

    fn audit(findings: usize, unverified_findings: usize) -> AuditResult {
        AuditResult {
            board: String::new(),
            tree_size: 0,
            root: String::new(),
            published_checkpoints: 0,
            findings: vec!["finding".into(); findings],
            unverified_findings,
        }
    }

    fn run() -> SelfTestRun {
        SelfTestRun {
            run_id: "run".into(),
            generation: 1,
            trigger: SelfTestTrigger::Periodic,
            started_at: String::new(),
            finished_at: None,
            outcome: SelfTestOutcome::Passed,
            checks: SelfTestCheck::ALL
                .into_iter()
                .map(|check| SelfTestCheckResult {
                    check,
                    outcome: SelfTestOutcome::Passed,
                    detail: "Verified".into(),
                })
                .collect(),
        }
    }

    #[test]
    fn verified_audit_findings_are_unrecoverable() {
        let check = SelfTestCheck::AuditGeneration;
        assert_eq!(
            classify(check, audit_verdict(&audit(0, 0))),
            (SelfTestOutcome::Passed, "Verified".to_string())
        );
        assert_eq!(
            classify(check, audit_verdict(&audit(2, 1))),
            (
                SelfTestOutcome::UnrecoverableFailure,
                "Electoral-log integrity verification failed".to_string()
            )
        );
    }

    #[test]
    fn audit_findings_that_are_not_verified_stay_recoverable() {
        assert_eq!(
            classify(SelfTestCheck::AuditGeneration, audit_verdict(&audit(2, 2))),
            (
                SelfTestOutcome::RecoverableFailure,
                "Audit write, read-back or integrity verification failed".to_string()
            )
        );
    }

    #[test]
    fn raw_errors_never_reach_the_persisted_detail() {
        for check in SelfTestCheck::ALL {
            let (outcome, detail) =
                classify(check, Err(anyhow::anyhow!("postgres://user:secret@host")));
            assert_eq!(outcome, SelfTestOutcome::RecoverableFailure);
            assert!(!detail.contains("secret"), "{detail}");
            assert_eq!(
                classify(check, Err(anyhow::anyhow!(DEADLINE_EXCEEDED))),
                (
                    SelfTestOutcome::RecoverableFailure,
                    DEADLINE_EXCEEDED.to_string()
                )
            );
        }
    }

    #[test]
    fn election_data_detail_names_the_difference_from_the_baseline() {
        assert_eq!(
            classify(SelfTestCheck::ElectionData, Err(anyhow::anyhow!("x"))).1,
            "Election data differs from the board-approved baseline, or could not be verified"
        );
    }

    #[test]
    fn only_a_startup_run_in_preparation_with_no_baseline_skips_the_comparison() {
        assert!(nothing_to_compare(
            SelfTestTrigger::Startup,
            ElectoralPhase::Preparation,
            false
        ));
        assert!(!nothing_to_compare(
            SelfTestTrigger::Startup,
            ElectoralPhase::Preparation,
            true
        ));
        for phase in [
            ElectoralPhase::Execution,
            ElectoralPhase::Evaluation,
            ElectoralPhase::PostProcessing,
        ] {
            assert!(!nothing_to_compare(SelfTestTrigger::Startup, phase, false));
        }
        for trigger in [
            SelfTestTrigger::ElectionStartTime,
            SelfTestTrigger::BeforeStart,
            SelfTestTrigger::Requested,
            SelfTestTrigger::BeforeResume,
            SelfTestTrigger::Periodic,
            SelfTestTrigger::AfterMaintenance,
            SelfTestTrigger::AfterPause,
            SelfTestTrigger::BeforeCounting,
        ] {
            assert!(!nothing_to_compare(
                trigger,
                ElectoralPhase::Preparation,
                false
            ));
        }
    }

    #[test]
    fn an_incomplete_run_keeps_the_unrecoverable_result_it_found() {
        let mut incomplete = run();
        incomplete.checks.truncate(2);
        assert_eq!(
            unfinished_outcome(&incomplete),
            SelfTestOutcome::RecoverableFailure
        );
        incomplete.checks[0].outcome = SelfTestOutcome::UnrecoverableFailure;
        assert_eq!(
            unfinished_outcome(&incomplete),
            SelfTestOutcome::UnrecoverableFailure
        );
        assert_eq!(incomplete.aggregate(), unfinished_outcome(&incomplete));
    }

    #[test]
    fn a_missing_completion_record_fails_the_run_recoverably() {
        let mut passed = run();
        completion_record_failed(&mut passed);
        assert_eq!(passed.outcome, SelfTestOutcome::RecoverableFailure);
        assert_eq!(passed.aggregate(), passed.outcome);
        assert_eq!(
            passed.checks[2].detail,
            "The completion audit record could not be verified"
        );
    }

    #[test]
    fn a_missing_completion_record_never_hides_a_verified_audit_finding() {
        let mut failed = run();
        failed.checks[2].outcome = SelfTestOutcome::UnrecoverableFailure;
        failed.checks[2].detail = "Electoral-log integrity verification failed".into();
        completion_record_failed(&mut failed);
        assert_eq!(failed.outcome, SelfTestOutcome::UnrecoverableFailure);
        assert_eq!(failed.aggregate(), failed.outcome);
        assert_eq!(
            failed.checks[2].detail,
            "Electoral-log integrity verification failed"
        );
    }

    #[test]
    fn the_checks_the_failure_paths_overwrite_keep_their_positions() {
        assert_eq!(SelfTestCheck::ALL[2], SelfTestCheck::AuditGeneration);
        assert_eq!(SelfTestCheck::ALL[4], SelfTestCheck::AccessControl);
    }
}
