// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Durable electoral phases and fail-closed recovery. A successful self-test is
//! evidence for a board action; it never authorizes recovery on its own.
use crate::postgres::scheduled_event::find_scheduled_event_by_election_event_id;
use crate::postgres::trusted_write;
use crate::services::database::get_hasura_pool;
use crate::services::electoral_log_board::get_event_store;
use anyhow::{anyhow, ensure, Context, Result};
use chrono::{DateTime, Utc};
use deadpool_postgres::Transaction;
use sequent_core::ballot::{VotingStatus, VotingStatusChannel};
use sequent_core::election_lifecycle::{policy, ElectoralPhase, LifecycleState, SafetyState};
use sequent_core::self_tests::{SelfTestOutcome, SelfTestRun, SelfTestTrigger};
use sequent_core::types::scheduled_event::generate_voting_period_dates;
use serde_json::{json, Value};

/// Maximum lifetime of a before-start/resume check. Ordinary periodic checks
/// follow the event's independently configured interval.
const OPENING_CHECK_MAX_AGE_SECONDS: i64 = 300;
const LEASE_SECONDS: i64 = 45;

pub async fn state(tx: &Transaction<'_>, tenant: &str, event: &str) -> Result<LifecycleState> {
    let row=tx.query_opt("SELECT phase,safety,generation,reason FROM sequent_backend.election_lifecycle WHERE tenant_id=$1::text::uuid AND election_event_id=$2::text::uuid", &[&tenant,&event]).await?
        .context("Election lifecycle is not initialized")?;
    Ok(LifecycleState {
        phase: serde_json::from_value(Value::String(row.try_get(0)?))?,
        safety: serde_json::from_value(Value::String(row.try_get(1)?))?,
        generation: row.try_get(2)?,
        reason: row.try_get(3)?,
    })
}

/// Every writer uses the event-before-lifecycle lock order, like status changes.
async fn lock(tx: &Transaction<'_>, tenant: &str, event: &str) -> Result<()> {
    let key = format!("{tenant}:{event}");
    let acquired: bool = tx
        .query_one(
            "SELECT pg_try_advisory_xact_lock(hashtextextended($1,848527))",
            &[&key],
        )
        .await?
        .try_get(0)?;
    ensure!(
        acquired,
        "An election operation is finishing; retry this lifecycle change"
    );
    tx.query_opt("SELECT id FROM sequent_backend.election_event WHERE tenant_id=$1::text::uuid AND id=$2::text::uuid FOR UPDATE", &[&tenant,&event]).await?.context("Election event does not exist")?;
    tx.query_opt("SELECT election_event_id FROM sequent_backend.election_lifecycle WHERE tenant_id=$1::text::uuid AND election_event_id=$2::text::uuid FOR UPDATE", &[&tenant,&event]).await?.context("Election lifecycle is not initialized")?;
    Ok(())
}

pub async fn require_phase(
    tx: &Transaction<'_>,
    tenant: &str,
    event: &str,
    expected: ElectoralPhase,
) -> Result<()> {
    let key = format!("{tenant}:{event}");
    let acquired: bool = tx
        .query_one(
            "SELECT pg_try_advisory_xact_lock_shared(hashtextextended($1,848527))",
            &[&key],
        )
        .await?
        .try_get(0)?;
    ensure!(
        acquired,
        "Election safety state is changing; retry the operation"
    );
    let current = state(tx, tenant, event).await?;
    ensure!(
        current.phase == expected,
        "This operation requires electoral phase {expected:?}; current phase is {:?}",
        current.phase
    );
    Ok(())
}

async fn services_live(tx: &Transaction<'_>) -> Result<bool> {
    Ok(tx.query_one("SELECT count(*)=2 FROM sequent_backend.lifecycle_service_lease WHERE component IN ('harvest','windmill') AND expires_at>clock_timestamp()", &[]).await?.try_get(0)?)
}

/// Hold a shared safety lock through the caller's complete ballot operation.
/// This serializes in-flight operations with entry into the secure state.
pub async fn require_ballot_access(tx: &Transaction<'_>, tenant: &str, event: &str) -> Result<()> {
    let key = format!("{tenant}:{event}");
    let acquired: bool = tx
        .query_one(
            "SELECT pg_try_advisory_xact_lock_shared(hashtextextended($1,848527))",
            &[&key],
        )
        .await?
        .try_get(0)?;
    ensure!(
        acquired,
        "Election safety state is changing; retry the operation"
    );
    let current = state(tx, tenant, event).await?;
    if current.phase == ElectoralPhase::Execution {
        ensure!(
            current.safety == SafetyState::Operational,
            "Ballot boxes are unavailable while the election is in a secure failure state"
        );
        ensure!(
            services_live(tx).await?,
            "Ballot boxes are unavailable while a required service is unavailable"
        );
    }
    Ok(())
}

/// Every authenticated voter operation, including reads with an old token,
/// must satisfy the same database-derived predicate used by GraphQL.
pub async fn require_voter_access(tx: &Transaction<'_>, tenant: &str, event: &str) -> Result<()> {
    require_ballot_access(tx, tenant, event).await?;
    let allowed:bool=tx.query_opt("SELECT voter_allowed FROM sequent_backend.election_lifecycle_access WHERE tenant_id=$1::text::uuid AND election_event_id=$2::text::uuid", &[&tenant,&event]).await?.context("Election lifecycle is not initialized")?.try_get(0)?;
    ensure!(
        allowed,
        "Voting is unavailable until election-board recovery and current self-tests complete"
    );
    Ok(())
}

pub async fn notice(
    tx: &Transaction<'_>,
    tenant: &str,
    event: &str,
    kind: &str,
    details: Value,
) -> Result<()> {
    crate::services::security_notifications::queue_notice(
        tx,
        tenant,
        event,
        kind,
        &details.to_string(),
    )
    .await?;
    tx.execute("INSERT INTO sequent_backend.lifecycle_notice(tenant_id,election_event_id,kind,details) VALUES($1::text::uuid,$2::text::uuid,$3,$4)", &[&tenant,&event,&kind,&details]).await?;
    Ok(())
}

/// A test failure changes durable safety state in the transaction recording its
/// outcome. Even a stale test failure is retained; a stale success is never a permit.
pub async fn record_self_test_outcome(
    tx: &Transaction<'_>,
    tenant: &str,
    event: &str,
    run: &SelfTestRun,
) -> Result<()> {
    lock(tx, tenant, event).await?;
    let outcome = run.aggregate();
    if outcome != SelfTestOutcome::Passed {
        let safety = if outcome == SelfTestOutcome::UnrecoverableFailure {
            SafetyState::Unrecoverable
        } else {
            SafetyState::Recoverable
        };
        enter_failure(
            tx,
            tenant,
            event,
            safety,
            "Self-test failed",
            json!({"run_id":run.run_id,"outcome":outcome}),
        )
        .await?;
    }
    Ok(())
}

pub async fn enter_failure(
    tx: &Transaction<'_>,
    tenant: &str,
    event: &str,
    safety: SafetyState,
    reason: &str,
    details: Value,
) -> Result<()> {
    ensure!(
        safety != SafetyState::Operational,
        "A failure cannot authorize operational state"
    );
    lock(tx, tenant, event).await?;
    let current = state(tx, tenant, event).await?;
    let final_safety = if current.safety == SafetyState::Unrecoverable {
        SafetyState::Unrecoverable
    } else {
        safety
    };
    let safety_text = serde_json::to_value(final_safety)?
        .as_str()
        .context("Invalid safety state")?
        .to_owned();
    trusted_write(tx).await?;
    tx.execute("UPDATE sequent_backend.election_lifecycle SET safety=$3,reason=$4,generation=generation+1 WHERE tenant_id=$1::text::uuid AND election_event_id=$2::text::uuid", &[&tenant,&event,&safety_text,&reason]).await?;
    if current.phase == ElectoralPhase::Execution {
        // Preserve the phase: the board may end execution, but no action can resume
        // an unrecoverable event. Pause every active channel without changing its dates.
        for table in ["election_event", "election"] {
            let id = if table == "election_event" {
                "id"
            } else {
                "election_event_id"
            };
            for field in [
                "voting_status",
                "kiosk_voting_status",
                "early_voting_status",
                "telephone_voting_status",
            ] {
                tx.execute(&format!("UPDATE sequent_backend.{table} SET status=jsonb_set(status,ARRAY['{field}'],'\"PAUSED\"'::jsonb) WHERE tenant_id=$1::text::uuid AND {id}=$2::text::uuid AND status->>'{field}'='OPEN'"), &[&tenant,&event]).await?;
            }
        }
    }
    notice(
        tx,
        tenant,
        event,
        "MaintenanceModeEntered",
        json!({"safety":final_safety,"reason":reason,"context":details}),
    )
    .await
}

/// Authoritative imported election dates. Per-election dates override the
/// event's dates; a missing start or end is a refusal, never "open whenever".
async fn period(
    tx: &Transaction<'_>,
    tenant: &str,
    event: &str,
    election: Option<&str>,
) -> Result<(i64, i64)> {
    let schedules = find_scheduled_event_by_election_event_id(tx, tenant, event).await?;
    let specific = generate_voting_period_dates(schedules.clone(), tenant, event, election)?;
    let common = generate_voting_period_dates(schedules, tenant, event, None)?;
    let mut start = specific.start_date.or(common.start_date);
    let mut end = specific.end_date.or(common.end_date);
    if election.is_none() && (start.is_none() || end.is_none()) {
        let ids=tx.query("SELECT id::text FROM sequent_backend.election WHERE tenant_id=$1::text::uuid AND election_event_id=$2::text::uuid AND COALESCE((voting_channels->>'online')::boolean,true)", &[&tenant,&event]).await?;
        let schedules = find_scheduled_event_by_election_event_id(tx, tenant, event).await?;
        let mut starts = Vec::new();
        let mut ends = Vec::new();
        for row in ids {
            let id: String = row.try_get(0)?;
            let dates = generate_voting_period_dates(schedules.clone(), tenant, event, Some(&id))?;
            if let Some(value) = dates.start_date {
                starts.push(DateTime::parse_from_rfc3339(&value)?);
            }
            if let Some(value) = dates.end_date {
                ends.push(DateTime::parse_from_rfc3339(&value)?);
            }
        }
        if start.is_none() {
            start = starts.into_iter().min().map(|d| d.to_rfc3339());
        }
        if end.is_none() {
            end = ends.into_iter().max().map(|d| d.to_rfc3339());
        }
    }
    let start = start.context("An approved election start time is required")?;
    let end = end.context("An approved election end time is required")?;
    Ok((
        DateTime::parse_from_rfc3339(&start)?.timestamp(),
        DateTime::parse_from_rfc3339(&end)?.timestamp(),
    ))
}

pub async fn require_opening_time(
    tx: &Transaction<'_>,
    tenant: &str,
    event: &str,
    election: &str,
) -> Result<()> {
    let (start, end) = period(tx, tenant, event, Some(election)).await?;
    let now = Utc::now().timestamp();
    ensure!(
        start <= now && now < end,
        "Election {election} cannot open outside its approved start and end times"
    );
    Ok(())
}

async fn opening_passed(
    tx: &Transaction<'_>,
    tenant: &str,
    event: &str,
    generation: i64,
    trigger: SelfTestTrigger,
) -> Result<bool> {
    let trigger = serde_json::to_value(trigger)?
        .as_str()
        .context("Invalid test trigger")?
        .to_owned();
    Ok(tx.query_one("SELECT COALESCE((SELECT outcome='passed' AND trigger=$4 AND finished_at BETWEEN clock_timestamp()-($5::bigint*interval '1 second') AND clock_timestamp() FROM sequent_backend.self_test_run WHERE tenant_id=$1::text::uuid AND election_event_id=$2::text::uuid AND generation=$3 AND finished_at IS NOT NULL ORDER BY finished_at DESC,id DESC LIMIT 1),false) AND NOT EXISTS(SELECT 1 FROM sequent_backend.self_test_run WHERE tenant_id=$1::text::uuid AND election_event_id=$2::text::uuid AND outcome='unrecoverable-failure')", &[&tenant,&event,&generation,&trigger,&OPENING_CHECK_MAX_AGE_SECONDS]).await?.try_get(0)?)
}

/// A configured before-counting check is a precondition, not a background
/// check that would race the tally. The caller retries with fresh board approval.
pub async fn require_counting_test(tx: &Transaction<'_>, tenant: &str, event: &str) -> Result<()> {
    let selected:bool=tx.query_one("SELECT COALESCE((SELECT settings->'additional_conditions' ? 'before-counting' FROM sequent_backend.self_test_settings WHERE tenant_id=$1::text::uuid AND election_event_id=$2::text::uuid),false)", &[&tenant,&event]).await?.try_get(0)?;
    if !selected {
        return Ok(());
    }
    let current = state(tx, tenant, event).await?;
    if opening_passed(
        tx,
        tenant,
        event,
        current.generation,
        SelfTestTrigger::BeforeCounting,
    )
    .await?
    {
        return Ok(());
    }
    queue_required_test(tenant, event, SelfTestTrigger::BeforeCounting);
    Err(anyhow!("The configured before-counting self-test has been requested. Wait for all six checks to pass, then authorize counting again."))
}

async fn boxes_empty(tx: &Transaction<'_>, tenant: &str, event: &str) -> Result<bool> {
    // A prior tally/readout is never compatible with the first production open,
    // even if someone emptied the intermediate rows afterwards.
    let prior:bool=tx.query_one("SELECT EXISTS(SELECT 1 FROM sequent_backend.tally_session WHERE tenant_id=$1::text::uuid AND election_event_id=$2::text::uuid AND tally_type='ELECTORAL_RESULTS')", &[&tenant,&event]).await?.try_get(0)?;
    if prior {
        return Ok(false);
    }
    let store = get_event_store(event).await?;
    let client = store.client().await?;
    // Both received-but-not-cast and cast ballots count. Looking only at the
    // valid-ballot count would miss staged or rejected ballots at first start.
    let row=client.query_one("SELECT NOT EXISTS(SELECT 1 FROM ballot_box_ballot WHERE election_event_id=$1::text::uuid) AND NOT EXISTS(SELECT 1 FROM ballot_box_received WHERE election_event_id=$1::text::uuid)", &[&event]).await?;
    Ok(row.try_get(0)?)
}

/// Called before any status change reaches its first mutation. The background
/// check is independently committed, so a refused/retried action cannot erase it.
pub async fn before_status_change(
    tx: &Transaction<'_>,
    tenant: &str,
    event: &str,
    election: Option<&str>,
    status: &VotingStatus,
    scheduled: bool,
) -> Result<()> {
    // Acquire the exclusive safety lock BEFORE closing/sealing as well as
    // opening. In-flight receive/cast transactions hold the matching shared lock.
    lock(tx, tenant, event).await?;
    let current = state(tx, tenant, event).await?;
    ensure!(
        current.phase != ElectoralPhase::PostProcessing,
        "Election execution has already been read out; status changes are forbidden"
    );
    ensure!(
        *status != VotingStatus::NOT_STARTED,
        "Election execution cannot be reset"
    );
    if *status != VotingStatus::OPEN {
        return Ok(());
    }
    let (start, end) = period(tx, tenant, event, election).await?;
    let trigger = if current.phase == ElectoralPhase::Preparation
        || (scheduled && current.safety == SafetyState::Operational)
    {
        if scheduled {
            SelfTestTrigger::ElectionStartTime
        } else {
            SelfTestTrigger::BeforeStart
        }
    } else {
        SelfTestTrigger::BeforeResume
    };
    let passed = opening_passed(tx, tenant, event, current.generation, trigger).await?;
    let empty = if current.phase == ElectoralPhase::Preparation {
        boxes_empty(tx, tenant, event).await?
    } else {
        true
    };
    let decision = policy::opening_allowed(
        &current,
        Utc::now().timestamp(),
        start,
        end,
        passed,
        empty,
        services_live(tx).await?,
    );
    match decision {
        Ok(())=>Ok(()),
        Err(sequent_core::election_lifecycle::LifecycleRefusal::SelfTestRequired)=>{
            queue_required_test(tenant,event,trigger);
            Err(anyhow!("The required self-test has been requested. Wait for all six checks to pass in Security and self-tests, then retry the board action."))
        }
        Err(reason)=>Err(anyhow!("Voting cannot open: {reason}. Review the election's lifecycle status and self-test results.")),
    }
}

/// The status transaction may own an event row whose foreign key a new test
/// needs. Queue only after yielding that transaction; never wait for the worker
/// while holding an authorization or ballot lock.
fn queue_required_test(tenant: &str, event: &str, trigger: SelfTestTrigger) {
    let tenant = tenant.to_owned();
    let event = event.to_owned();
    tokio::spawn(async move {
        let queued = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            crate::services::self_tests::enqueue(&tenant, &event, trigger, "system"),
        )
        .await;
        match queued {
            Ok(Ok(_)) => {}
            Ok(Err(error)) => {
                tracing::error!(%tenant,%event,?trigger,%error,"Could not queue required self-test; voting remains closed")
            }
            Err(_) => {
                tracing::error!(%tenant,%event,?trigger,"Timed out queueing required self-test; voting remains closed")
            }
        }
    });
}

pub fn queue_optional_test(tenant: &str, event: &str, trigger: SelfTestTrigger) {
    let tenant = tenant.to_owned();
    let event = event.to_owned();
    tokio::spawn(async move {
        if !matches!(
            tokio::time::timeout(
                std::time::Duration::from_secs(10),
                crate::services::self_tests::additional(&tenant, &event, trigger)
            )
            .await,
            Ok(Ok(()))
        ) {
            tracing::error!(%tenant,%event,?trigger,"Could not queue configured additional self-test");
        }
    });
}

/// Runs in the successful status transaction. A failed test never calls this.
pub async fn after_status_change(
    tx: &Transaction<'_>,
    tenant: &str,
    event: &str,
    status: &VotingStatus,
) -> Result<()> {
    lock(tx, tenant, event).await?;
    let current = state(tx, tenant, event).await?;
    trusted_write(tx).await?;
    match status {
        VotingStatus::OPEN => {
            ensure!(
                current.safety != SafetyState::Unrecoverable,
                "An unrecoverable election cannot resume"
            );
            tx.execute("UPDATE sequent_backend.election_lifecycle SET phase='execution',safety='operational',reason=NULL WHERE tenant_id=$1::text::uuid AND election_event_id=$2::text::uuid", &[&tenant,&event]).await?;
            if current.safety != SafetyState::Operational {
                notice(
                    tx,
                    tenant,
                    event,
                    "MaintenanceModeLeft",
                    json!({"generation":current.generation}),
                )
                .await?;
                queue_required_test(tenant, event, SelfTestTrigger::AfterMaintenance);
            }
            if current.phase != ElectoralPhase::Execution {
                notice(
                    tx,
                    tenant,
                    event,
                    "ElectoralPhaseChanged",
                    json!({"from":current.phase,"to":"execution"}),
                )
                .await?;
            }
        }
        VotingStatus::CLOSED => {
            let ended:bool=tx.query_one("SELECT NOT EXISTS(SELECT 1 FROM sequent_backend.election WHERE tenant_id=$1::text::uuid AND election_event_id=$2::text::uuid AND COALESCE((voting_channels->>'online')::boolean,true) AND COALESCE(status->>'voting_status','NOT_STARTED')<>'CLOSED')", &[&tenant,&event]).await?.try_get(0)?;
            if ended
                && matches!(
                    current.phase,
                    ElectoralPhase::Preparation | ElectoralPhase::Execution
                )
            {
                tx.execute("UPDATE sequent_backend.election_lifecycle SET phase='evaluation',generation=generation+1 WHERE tenant_id=$1::text::uuid AND election_event_id=$2::text::uuid", &[&tenant,&event]).await?;
                notice(
                    tx,
                    tenant,
                    event,
                    "ElectoralPhaseChanged",
                    json!({"from":current.phase,"to":"evaluation"}),
                )
                .await?;
            }
        }
        VotingStatus::PAUSED => {
            queue_optional_test(tenant, event, SelfTestTrigger::AfterPause);
            tx.execute("UPDATE sequent_backend.election_lifecycle SET generation=generation+1 WHERE tenant_id=$1::text::uuid AND election_event_id=$2::text::uuid", &[&tenant,&event]).await?;
        }
        VotingStatus::NOT_STARTED => {
            return Err(anyhow!(
                "Voting cannot be reset to an earlier electoral phase"
            ))
        }
    }
    Ok(())
}

pub async fn start_post_processing(tx: &Transaction<'_>, tenant: &str, event: &str) -> Result<()> {
    lock(tx, tenant, event).await?;
    let current = state(tx, tenant, event).await?;
    let complete:bool=tx.query_one("SELECT EXISTS(SELECT 1 FROM sequent_backend.election WHERE tenant_id=$1::text::uuid AND election_event_id=$2::text::uuid) AND NOT EXISTS(SELECT 1 FROM sequent_backend.election e WHERE e.tenant_id=$1::text::uuid AND e.election_event_id=$2::text::uuid AND NOT EXISTS(SELECT 1 FROM sequent_backend.tally_session t WHERE t.tenant_id=e.tenant_id AND t.election_event_id=e.election_event_id AND e.id=ANY(t.election_ids) AND t.is_execution_completed AND t.tally_type='ELECTORAL_RESULTS'))", &[&tenant,&event]).await?.try_get(0)?;
    policy::post_processing_allowed(&current, complete)?;
    trusted_write(tx).await?;
    tx.execute("UPDATE sequent_backend.election_lifecycle SET phase='post_processing',generation=generation+1 WHERE tenant_id=$1::text::uuid AND election_event_id=$2::text::uuid", &[&tenant,&event]).await?;
    notice(tx,tenant,event,"ElectoralPhaseChanged",json!({"from":"evaluation","to":"post_processing","investigation":current.safety==SafetyState::Unrecoverable})).await
}

pub async fn heartbeat(component: &str) -> Result<()> {
    ensure!(
        matches!(component, "harvest" | "windmill"),
        "Unknown lifecycle component"
    );
    let client = get_hasura_pool().await.get().await?;
    let expired:bool=client.query_one("SELECT NOT EXISTS(SELECT 1 FROM sequent_backend.lifecycle_service_lease WHERE component=$1 AND expires_at>clock_timestamp())", &[&component]).await?.try_get(0)?;
    drop(client);
    // A delayed heartbeat cannot silently revive an expired lease. Record the
    // recovery requirement first, even if the other monitor has not run yet.
    if expired {
        mark_process_stopped(component).await?;
    }
    let client = get_hasura_pool().await.get().await?;
    client.execute("INSERT INTO sequent_backend.lifecycle_service_lease(component,expires_at) VALUES($1,clock_timestamp()+($2::bigint*interval '1 second')) ON CONFLICT(component) DO UPDATE SET expires_at=EXCLUDED.expires_at", &[&component,&LEASE_SECONDS]).await?;
    Ok(())
}

pub async fn register_startup(component: &str) -> Result<()> {
    mark_process_stopped(component).await?;
    heartbeat(component).await
}

pub async fn mark_process_stopped(component: &str) -> Result<()> {
    ensure!(
        matches!(component, "harvest" | "windmill"),
        "Unknown lifecycle component"
    );
    let mut client = get_hasura_pool().await.get().await?;
    // Commit the refusal before trying per-event locks. An in-flight ballot may
    // finish, but a busy event cannot roll back shutdown's global refusal.
    client
        .execute(
            "DELETE FROM sequent_backend.lifecycle_service_lease WHERE component=$1",
            &[&component],
        )
        .await?;
    let rows=client.query("SELECT tenant_id::text,election_event_id::text FROM sequent_backend.election_lifecycle WHERE phase='execution' ORDER BY tenant_id,election_event_id", &[]).await?;
    for row in rows {
        let tx = client.transaction().await?;
        let tenant: String = row.try_get(0)?;
        let event: String = row.try_get(1)?;
        if state(&tx, &tenant, &event).await?.safety == SafetyState::Operational {
            enter_failure(
                &tx,
                &tenant,
                &event,
                SafetyState::Recoverable,
                "A required service stopped or restarted",
                json!({"component":component}),
            )
            .await?;
        }
        tx.commit().await?;
    }
    Ok(())
}

/// Bounded periodic maintenance runs independently of a client's HTTP request.
pub async fn tick() -> Result<()> {
    heartbeat("windmill").await?;
    let client = get_hasura_pool().await.get().await?;
    let rows=client.query("SELECT tenant_id::text,election_event_id::text FROM sequent_backend.election_lifecycle WHERE phase IN ('preparation','execution') ORDER BY last_monitored_at,election_event_id LIMIT 100", &[]).await?;
    drop(client);
    for row in rows {
        let tenant: String = row.try_get(0)?;
        let event: String = row.try_get(1)?;
        if let Err(error) = reconcile_scheduled_starts(&tenant, &event).await {
            tracing::info!(%tenant,%event,%error,"Scheduled first opening remains pending its checks");
        }
        if let Err(error) = monitor_event(&tenant, &event).await {
            tracing::error!(%tenant,%event,%error,"Election safety monitoring refused or could not complete; access predicates remain fail-closed");
        }
    }
    if let Err(error) = flush_notices().await {
        tracing::error!(%error,"Lifecycle audit delivery deferred; durable notices remain queued");
    }
    Ok(())
}

/// The ordinary one-shot scheduler may finish before an async start-time test.
/// Retry its unchanged, still-active schedule after that check. Never recover a
/// paused/failed election or manufacture a new authorization/schedule here.
async fn reconcile_scheduled_starts(tenant: &str, event: &str) -> Result<()> {
    use sequent_core::types::scheduled_event::EventProcessors;
    let mut client = get_hasura_pool().await.get().await?;
    let tx = client.transaction().await?;
    let current = state(&tx, tenant, event).await?;
    let failed:bool=tx.query_one("SELECT EXISTS(SELECT 1 FROM sequent_backend.self_test_run WHERE tenant_id=$1::text::uuid AND election_event_id=$2::text::uuid AND outcome IN ('recoverable-failure','unrecoverable-failure'))", &[&tenant,&event]).await?.try_get(0)?;
    if (current.phase == ElectoralPhase::Preparation && failed)
        || (current.phase == ElectoralPhase::Execution
            && current.safety != SafetyState::Operational)
        || current.safety == SafetyState::Unrecoverable
    {
        return Ok(());
    }
    let baseline:bool=tx.query_one("SELECT EXISTS(SELECT 1 FROM sequent_backend.self_test_baseline WHERE tenant_id=$1::text::uuid AND election_event_id=$2::text::uuid)", &[&tenant,&event]).await?.try_get(0)?;
    if !baseline {
        return Ok(());
    }
    let paused:bool=tx.query_one("SELECT EXISTS(SELECT 1 FROM sequent_backend.election WHERE tenant_id=$1::text::uuid AND election_event_id=$2::text::uuid AND status->>'voting_status'='PAUSED')", &[&tenant,&event]).await?.try_get(0)?;
    if paused {
        return Ok(());
    }
    let schedules = find_scheduled_event_by_election_event_id(&tx, tenant, event).await?;
    for scheduled in schedules {
        if scheduled.stopped_at.is_some()
            || scheduled.archived_at.is_some()
            || scheduled.event_processor != Some(EventProcessors::START_VOTING_PERIOD)
        {
            continue;
        }
        let Some(date) = scheduled
            .cron_config
            .as_ref()
            .and_then(|c| c.scheduled_date.as_ref())
        else {
            continue;
        };
        if DateTime::parse_from_rfc3339(date)?.timestamp() > Utc::now().timestamp() {
            continue;
        }
        let target = scheduled
            .event_payload
            .as_ref()
            .and_then(|p| p.get("election_id"))
            .and_then(Value::as_str);
        if let Some(election) = target {
            crate::tasks::manage_election_dates::manage_election_date_wrapper(
                &tx,
                tenant.into(),
                event.into(),
                scheduled.id.clone(),
                election.into(),
            )
            .await?;
        } else {
            crate::tasks::manage_election_event_date::manage_election_event_date_wrapped(
                &tx,
                tenant.into(),
                event.into(),
                scheduled.id.clone(),
            )
            .await?;
        }
    }
    tx.commit().await?;
    Ok(())
}

async fn monitor_event(tenant: &str, event: &str) -> Result<()> {
    let mut client = get_hasura_pool().await.get().await?;
    let tx = client.transaction().await?;
    let live = services_live(&tx).await?;
    let current = state(&tx, tenant, event).await?;
    let allowed:bool=tx.query_one("SELECT voter_allowed FROM sequent_backend.election_lifecycle_access WHERE tenant_id=$1::text::uuid AND election_event_id=$2::text::uuid", &[&tenant,&event]).await?.try_get(0)?;
    let irreversible:bool=tx.query_one("SELECT EXISTS(SELECT 1 FROM sequent_backend.self_test_run WHERE tenant_id=$1::text::uuid AND election_event_id=$2::text::uuid AND outcome='unrecoverable-failure')", &[&tenant,&event]).await?.try_get(0)?;
    if irreversible && current.safety != SafetyState::Unrecoverable {
        enter_failure(
            &tx,
            tenant,
            event,
            SafetyState::Unrecoverable,
            "An unrecoverable self-test failure was recorded",
            json!({}),
        )
        .await?;
    } else if current.safety == SafetyState::Operational && (!live || !allowed) {
        enter_failure(
            &tx,
            tenant,
            event,
            SafetyState::Recoverable,
            if live {
                "The required self-test failed or its validity interval expired"
            } else {
                "A required service heartbeat expired"
            },
            json!({}),
        )
        .await?;
    }
    if let Ok((_, end)) = period(&tx, tenant, event, None).await {
        if Utc::now().timestamp() >= end {
            crate::services::election_event_status::update_scheduled_event_voting_status(
                &tx,
                tenant,
                None,
                None,
                event,
                &VotingStatus::CLOSED,
                &Some(vec![VotingStatusChannel::ONLINE]),
                &Default::default(),
            )
            .await?;
            // An unstarted Post need not acquire a seal, but the event's period
            // has still ended and its phase can never remain Execution.
            if matches!(
                state(&tx, tenant, event).await?.phase,
                ElectoralPhase::Preparation | ElectoralPhase::Execution
            ) {
                lock(&tx, tenant, event).await?;
                trusted_write(&tx).await?;
                tx.execute("UPDATE sequent_backend.election_lifecycle SET phase='evaluation',generation=generation+1 WHERE tenant_id=$1::text::uuid AND election_event_id=$2::text::uuid", &[&tenant,&event]).await?;
                notice(
                    &tx,
                    tenant,
                    event,
                    "ElectoralPhaseChanged",
                    json!({"from":"execution","to":"evaluation","reason":"election period ended"}),
                )
                .await?;
            }
        }
    }
    trusted_write(&tx).await?;
    tx.execute("UPDATE sequent_backend.election_lifecycle SET last_monitored_at=clock_timestamp() WHERE tenant_id=$1::text::uuid AND election_event_id=$2::text::uuid", &[&tenant,&event]).await?;
    tx.commit().await?;
    Ok(())
}

async fn flush_notices() -> Result<()> {
    use crate::services::election_event_board::get_election_event_board;
    use crate::services::electoral_log::ElectoralLog;
    use electoral_log::messages::{message::Message, newtypes::EventIdString};
    let mut client = get_hasura_pool().await.get().await?;
    let tx = client.transaction().await?;
    let rows=tx.query("SELECT id::text,tenant_id::text,election_event_id::text,kind,details FROM sequent_backend.lifecycle_notice WHERE logged_at IS NULL ORDER BY created_at LIMIT 20 FOR UPDATE SKIP LOCKED", &[]).await?;
    for row in rows {
        let id: String = row.try_get(0)?;
        let tenant: String = row.try_get(1)?;
        let event: String = row.try_get(2)?;
        let kind: String = row.try_get(3)?;
        let details: Value = row.try_get(4)?;
        let e =
            crate::postgres::election_event::get_election_event_by_id(&tx, &tenant, &event).await?;
        let board = get_election_event_board(e.bulletin_board_reference)
            .context("Election has no audit board")?;
        let log = ElectoralLog::new(&tx, &tenant, Some(&event), &board).await?;
        let message = Message::lifecycle_security_event(
            EventIdString(event.clone()),
            &kind,
            serde_json::to_string(&json!({"notice_id":id,"details":details}))?,
            &log.sd,
        )?;
        let wire: electoral_log::ElectoralLogMessage = (&message).try_into()?;
        let delivery_id = format!("lifecycle:{id}");
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            crate::services::electoral_log_board::get_board_client()
                .await?
                .append(
                    &board,
                    &[electoral_log::LogEntry {
                        delivery_id,
                        message: wire,
                    }],
                )
                .await?;
            Ok::<_, anyhow::Error>(())
        })
        .await
        .context("Lifecycle audit write timed out")??;
        tx.execute("UPDATE sequent_backend.lifecycle_notice SET logged_at=clock_timestamp() WHERE id=$1::text::uuid", &[&id]).await?;
    }
    tx.commit().await?;
    Ok(())
}
