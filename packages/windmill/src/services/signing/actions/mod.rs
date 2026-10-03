// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The protected actions the existing routes start: opening and closing
//! voting, initializing it, approving a configuration version, approving
//! a voter, generating the election returns and other reports, and
//! transmitting the results.
//!
//! A route asks its gate before it acts. With the action's rule
//! `NotRequired` the route runs as it always did; otherwise it answers the
//! signing request. Each of these actions reaches outside the database (the
//! bulletin board, Keycloak, messages, object storage), so its executor only
//! records a task in the approve transaction ([`DispatchedExecutor`]).
//!
//! The task, sent after the commit, runs in three steps:
//!
//! 1. [`claim`]: claims the execution for the task and gets its token (the
//!    claim time); a copy that can't claim does nothing.
//! 2. The effect, in its own transaction without the event's signing lock:
//!    it locks the request and needs it still claimed with the same token,
//!    checks that everything it needs is as signed before its first call
//!    outside the database, then acts, and keeps its result on the request.
//! 3. The report, under the event's signing lock: `finish_dispatched`, and
//!    what executing the action cancels.
//!
//! A refusal before acting (the state is not what was signed) fails the
//! request at once. A failure before the first call outside the database
//! leaves the request completed for the sweeper to try again. A failure
//! after one fails the request (`effect-partial`, logged as an error): a
//! person decides what to do.

pub mod configuration;
pub mod eml;
pub mod initialize;
pub mod reports;
pub mod transmission;
pub mod voter;
pub mod voting;

use super::executors::{ExecutionOutcome, PostCommit, PostCommitTask, SigningExecutor};
use super::guard::{effective_rule, guard, GuardOutcome, GuardRequest};
use super::requests::{claim_dispatched, finish_dispatched, DispatchedResult};
use super::{SigningCaller, SigningError, SigningResult};
use crate::postgres::signing::{
    get_signing_post, get_signing_request, list_signing_approvals, lock_signing_event,
    lock_signing_request, SigningApprovalRow, SigningRequestRow,
};
use crate::postgres::signing_actions::{
    insert_signed_action_task, set_signed_action_task_status, set_signing_effect_result,
};
use crate::services::consolidation::signed_transmission_package::StoredPackages;
use crate::services::signing::pdf::S3RevisionStore;
use crate::tasks::signing_log_outbox::kick_signing_log_outbox;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deadpool_postgres::{Client, Transaction};
use sequent_core::signing::{SigningAction, SigningRequestStatus};
use sequent_core::types::hasura::extra::TasksExecutionStatus;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fmt;
use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing::{error, instrument, warn};
use uuid::Uuid;

/// The actions this module runs.
pub const ACTIONS: [SigningAction; 8] = [
    SigningAction::InitializeVoting,
    SigningAction::OpenVoting,
    SigningAction::CloseVoting,
    SigningAction::ApproveConfiguration,
    SigningAction::ApproveVoter,
    SigningAction::TransmitResults,
    SigningAction::GenerateElectionReturns,
    SigningAction::GenerateReports,
];

/// The code of a failure after a call outside the database.
pub const EFFECT_PARTIAL: &str = "effect-partial";

/// The tenant and event of a route's input; `None` when they are not UUIDs,
/// so the route fails as it always did.
pub fn event_ids(tenant_id: &str, election_event_id: &str) -> Option<(Uuid, Uuid)> {
    Some((
        Uuid::parse_str(tenant_id).ok()?,
        Uuid::parse_str(election_event_id).ok()?,
    ))
}

/// Whether `action` can run now: `Proceed` when its rule needs no
/// signatures, else the signing request for what `build` describes.
///
/// The rule is read without a lock, and `build` (which may be costly) runs
/// only when signatures are needed. Call it before the route takes any
/// other lock: the guard takes the event's signing lock. For a Post-scoped
/// request, the caller must reach the Post.
pub async fn gate<F, Fut>(
    hasura_transaction: &Transaction<'_>,
    caller: &SigningCaller,
    action: SigningAction,
    tenant_id: Uuid,
    election_event_id: Uuid,
    build: F,
) -> SigningResult<GuardOutcome>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = SigningResult<GuardRequest>>,
{
    if !is_required(hasura_transaction, tenant_id, election_event_id, action).await? {
        return Ok(GuardOutcome::Proceed);
    }
    let request = build().await?;
    if let Some(election_id) = request.scope.election_id {
        let post = get_signing_post(
            hasura_transaction,
            tenant_id,
            election_event_id,
            election_id,
        )
        .await?
        .ok_or_else(|| SigningError::NotFound("There is no such Post.".into()))?;
        if !caller.reaches(post.permission_label.as_deref()) {
            return Err(SigningError::Forbidden(
                "You can't start this action for this Post.".into(),
            ));
        }
    }
    guard(hasura_transaction, caller, &request).await
}

/// Whether `action`'s rule needs signatures in the event.
pub async fn is_required(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    action: SigningAction,
) -> Result<bool> {
    Ok(
        effective_rule(hasura_transaction, tenant_id, election_event_id, action)
            .await?
            .is_required(),
    )
}

/// Reads an action's subject from a request.
pub fn subject_of<T: for<'de> Deserialize<'de>>(request: &SigningRequestRow) -> Result<T> {
    serde_json::from_value(request.subject.clone())
        .with_context(|| format!("Error reading the subject of {}", request.code))
}

/// The task that runs a signed action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedActionTask {
    pub tenant_id: Uuid,
    pub election_event_id: Uuid,
    pub request_id: Uuid,
    pub task_execution_id: Uuid,
}

impl SignedActionTask {
    /// The task from its Celery arguments.
    pub fn parse(
        tenant_id: &str,
        election_event_id: &str,
        request_id: &str,
        task_execution_id: &str,
    ) -> Result<Self> {
        let uuid = |text: &str, name: &str| {
            Uuid::parse_str(text).map_err(|_| anyhow!("{name} is not a UUID: {text}"))
        };
        Ok(SignedActionTask {
            tenant_id: uuid(tenant_id, "tenant_id")?,
            election_event_id: uuid(election_event_id, "election_event_id")?,
            request_id: uuid(request_id, "request_id")?,
            task_execution_id: uuid(task_execution_id, "task_execution_id")?,
        })
    }
}

/// Sends the task of a signed action, after the approve transaction
/// commits.
pub trait SignedActionDispatcher: Send + Sync {
    fn task(&self, task: SignedActionTask) -> PostCommit;
}

/// Sends `run_signed_action` to the workers.
#[derive(Debug, Clone, Copy, Default)]
pub struct CeleryDispatcher;

struct CeleryTask(SignedActionTask);

#[async_trait]
impl PostCommitTask for CeleryTask {
    async fn send(&self) -> Result<()> {
        let task = self.0;
        crate::services::celery_app::get_celery_app()
            .await
            .send_task(crate::tasks::run_signed_action::run_signed_action::new(
                task.tenant_id.to_string(),
                task.election_event_id.to_string(),
                task.request_id.to_string(),
                task.task_execution_id.to_string(),
            ))
            .await
            .map_err(|error| anyhow!("Error sending the signed action's task: {error:?}"))?;
        Ok(())
    }
}

impl SignedActionDispatcher for CeleryDispatcher {
    fn task(&self, task: SignedActionTask) -> PostCommit {
        PostCommit::SendTask(Box::new(CeleryTask(task)))
    }
}

/// The executor of an action that reaches outside the database: it records
/// the task in the approve transaction and leaves the effect to it.
pub struct DispatchedExecutor {
    action: SigningAction,
    dispatcher: Arc<dyn SignedActionDispatcher>,
}

impl DispatchedExecutor {
    pub fn new(action: SigningAction, dispatcher: Arc<dyn SignedActionDispatcher>) -> Self {
        DispatchedExecutor { action, dispatcher }
    }
}

#[async_trait]
impl SigningExecutor for DispatchedExecutor {
    fn action(&self) -> SigningAction {
        self.action
    }

    async fn execute(
        &self,
        hasura_transaction: &Transaction<'_>,
        request: &SigningRequestRow,
        _approvals: &[SigningApprovalRow],
    ) -> Result<ExecutionOutcome> {
        let task_execution_id = insert_signed_action_task(
            hasura_transaction,
            request.tenant_id,
            request.election_event_id,
            &request.requested_by_username,
            &json!({
                "signing_request_id": request.id,
                "action": request.action,
                "code": request.code,
            }),
        )
        .await?;
        Ok(ExecutionOutcome::Dispatched {
            task_execution_id,
            task: self.dispatcher.task(SignedActionTask {
                tenant_id: request.tenant_id,
                election_event_id: request.election_event_id,
                request_id: request.id,
                task_execution_id,
            }),
        })
    }

    async fn redispatch(&self, request: &SigningRequestRow) -> Result<Option<PostCommit>> {
        let task_execution_id = request
            .task_execution_id
            .ok_or_else(|| anyhow!("signing request {} has no task", request.id))?;
        Ok(Some(self.dispatcher.task(SignedActionTask {
            tenant_id: request.tenant_id,
            election_event_id: request.election_event_id,
            request_id: request.id,
            task_execution_id,
        })))
    }
}

/// The executors of [`ACTIONS`].
pub fn executors(dispatcher: Arc<dyn SignedActionDispatcher>) -> Vec<Arc<dyn SigningExecutor>> {
    ACTIONS
        .iter()
        .map(|action| {
            Arc::new(DispatchedExecutor::new(*action, dispatcher.clone()))
                as Arc<dyn SigningExecutor>
        })
        .collect()
}

/// Why a signed action stopped before acting: what it needs is not as it
/// was signed. It is not tried again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectRefused {
    /// A stable code, kept as the request's result.
    pub code: String,
    pub message: String,
}

impl fmt::Display for EffectRefused {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for EffectRefused {}

/// A refusal before acting; see [`EffectRefused`].
pub fn refuse(code: &str, message: impl Into<String>) -> anyhow::Error {
    anyhow::Error::new(EffectRefused {
        code: code.to_owned(),
        message: message.into(),
    })
}

/// The point of no return of an effect: whether it has called outside the
/// database. An effect marks it right before its first such call.
#[derive(Debug, Default)]
pub struct EffectProgress {
    outside: AtomicBool,
}

impl EffectProgress {
    /// Marks that the effect is about to call outside the database.
    pub fn reach_outside(&self) {
        self.outside.store(true, Ordering::SeqCst);
    }

    pub fn reached_outside(&self) -> bool {
        self.outside.load(Ordering::SeqCst)
    }
}

/// What a signed action does once it runs, in the effect's transaction.
#[async_trait]
pub trait SignedActionEffects: Send + Sync {
    /// Checks that everything is as signed, then runs the action of
    /// `request`; its answer is kept as the request's result. It calls
    /// [`EffectProgress::reach_outside`] before its first call outside the
    /// database, and answers [`refuse`] for a state that changed.
    async fn run(
        &self,
        hasura_transaction: &Transaction<'_>,
        request: &SigningRequestRow,
        approvals: &[SigningApprovalRow],
        progress: &EffectProgress,
    ) -> Result<Value>;
}

/// Today's services.
#[derive(Clone)]
pub struct ProductionEffects {
    pub voters: Arc<dyn voter::VoterApprover>,
}

impl Default for ProductionEffects {
    fn default() -> Self {
        ProductionEffects {
            voters: Arc::new(voter::KeycloakVoterApprover),
        }
    }
}

#[async_trait]
impl SignedActionEffects for ProductionEffects {
    async fn run(
        &self,
        hasura_transaction: &Transaction<'_>,
        request: &SigningRequestRow,
        approvals: &[SigningApprovalRow],
        progress: &EffectProgress,
    ) -> Result<Value> {
        match request.action {
            SigningAction::OpenVoting | SigningAction::CloseVoting => {
                voting::change_status(hasura_transaction, request, progress).await
            }
            SigningAction::InitializeVoting => {
                initialize::create_report_tally(hasura_transaction, request, progress).await
            }
            SigningAction::ApproveConfiguration => {
                configuration::publish(hasura_transaction, request, progress).await
            }
            SigningAction::ApproveVoter => {
                voter::approve(
                    hasura_transaction,
                    request,
                    approvals,
                    self.voters.as_ref(),
                    progress,
                )
                .await
            }
            SigningAction::TransmitResults => {
                transmission::sign_package(
                    hasura_transaction,
                    &StoredPackages,
                    request,
                    approvals,
                    progress,
                )
                .await
            }
            SigningAction::GenerateElectionReturns | SigningAction::GenerateReports => {
                reports::release_report(
                    hasura_transaction,
                    &S3RevisionStore,
                    &reports::StoredReports,
                    request,
                    progress,
                )
                .await
            }
            other => Err(refuse(
                "no-effect",
                format!("{other} is not run by a signed action task"),
            )),
        }
    }
}

/// The seal record's hook for VOTE-FREEZE. The close
/// (`update_election_status`) has already sealed, in this same transaction;
/// the hook hands the closing record (its signatures) to those seals and
/// answers what the record shows of them, one summary per country seal. It
/// must not seal: it runs only when a signed close ran, and every close
/// seals. Like an executor, it writes only to the database; anything that
/// leaves it (a signed log entry) is posted after commit.
#[async_trait]
pub trait SealRecordSink: Send + Sync {
    /// Feeds `record` to the seals the close made; what the record shows of
    /// each.
    async fn feed(
        &self,
        hasura_transaction: &Transaction<'_>,
        record: &voting::SealRecord,
    ) -> Result<Vec<voting::SealSummary>>;
}

/// No seal yet (until VOTE-FREEZE): the record stands on the closing
/// signatures.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoSeal;

#[async_trait]
impl SealRecordSink for NoSeal {
    async fn feed(
        &self,
        _hasura_transaction: &Transaction<'_>,
        _record: &voting::SealRecord,
    ) -> Result<Vec<voting::SealSummary>> {
        Ok(vec![])
    }
}

/// What a run of the task did.
#[derive(Debug, Clone, PartialEq)]
pub enum RunOutcome {
    /// Another copy holds the execution, or it ran already.
    NotClaimed,
    Executed(Value),
    /// The effect refused or failed after acting: the request failed with
    /// this code.
    Failed(String),
    /// The effect failed before acting: the request stays completed for the
    /// sweeper.
    Retry(String),
}

/// Runs the effect of `request` and the parts every action shares: when
/// voting closed, the seal record. The close comes first and the seal hook
/// after it, in the same transaction, so the hook sees the close's seals.
async fn run_effect(
    hasura_transaction: &Transaction<'_>,
    effects: &dyn SignedActionEffects,
    seal: &dyn SealRecordSink,
    request: &SigningRequestRow,
    approvals: &[SigningApprovalRow],
    progress: &EffectProgress,
) -> Result<Value> {
    let result = effects
        .run(hasura_transaction, request, approvals, progress)
        .await?;
    if request.action != SigningAction::CloseVoting || !voting::transitioned(&result) {
        return Ok(result);
    }
    let closed_at: DateTime<Utc> = hasura_transaction
        .query_one("SELECT clock_timestamp()", &[])
        .await?
        .try_get(0)?;
    let mut record = voting::SealRecord::new(request, approvals, &result, closed_at);
    record.seals = seal.feed(hasura_transaction, &record).await?;
    Ok(serde_json::to_value(&record)?)
}

/// Claims the execution of `task`: its token (the claim time), or `None`
/// when another copy holds it or it ran. The task row is in progress again,
/// so the sweeper doesn't take a running retry for a failed one.
#[instrument(skip(client), err)]
pub async fn claim(client: &mut Client, task: &SignedActionTask) -> Result<Option<DateTime<Utc>>> {
    let transaction = client.transaction().await?;
    if !claim_dispatched(
        &transaction,
        task.tenant_id,
        task.election_event_id,
        task.request_id,
        task.task_execution_id,
    )
    .await?
    {
        transaction.commit().await?;
        return Ok(None);
    }
    let token = get_signing_request(
        &transaction,
        task.tenant_id,
        task.election_event_id,
        task.request_id,
    )
    .await?
    .and_then(|request| request.execution_started_at);
    set_signed_action_task_status(
        &transaction,
        task.tenant_id,
        task.task_execution_id,
        TasksExecutionStatus::IN_PROGRESS,
    )
    .await?;
    transaction.commit().await?;
    Ok(token)
}

/// Whether `request` is still the execution `task` claimed with `token`.
fn holds(request: &SigningRequestRow, task: &SignedActionTask, token: DateTime<Utc>) -> bool {
    request.status == SigningRequestStatus::Completed
        && request.executed_at.is_none()
        && request.task_execution_id == Some(task.task_execution_id)
        && request.execution_started_at == Some(token)
}

/// What the task does after [`claim`]; see the module documentation.
#[instrument(skip(client, effects, seal), err)]
pub async fn run_claimed(
    client: &mut Client,
    effects: &dyn SignedActionEffects,
    seal: &dyn SealRecordSink,
    task: &SignedActionTask,
    token: DateTime<Utc>,
) -> Result<RunOutcome> {
    let SignedActionTask {
        tenant_id,
        election_event_id,
        request_id,
        task_execution_id,
    } = *task;

    // The effect: the request's row lock, not the event's signing lock.
    let transaction = client.transaction().await?;
    let Some(request) =
        lock_signing_request(&transaction, tenant_id, election_event_id, request_id).await?
    else {
        transaction.rollback().await?;
        return Ok(RunOutcome::NotClaimed);
    };
    if !holds(&request, task, token) {
        transaction.rollback().await?;
        return Ok(RunOutcome::NotClaimed);
    }
    let result =
        match request.execution_result.clone() {
            // The effect committed before; only its report is missing.
            Some(result) => {
                transaction.rollback().await?;
                result
            }
            None => {
                let approvals =
                    list_signing_approvals(&transaction, tenant_id, election_event_id, request_id)
                        .await?;
                let progress = EffectProgress::default();
                let committed =
                    match run_effect(&transaction, effects, seal, &request, &approvals, &progress)
                        .await
                    {
                        Ok(result) => match set_signing_effect_result(
                            &transaction,
                            tenant_id,
                            election_event_id,
                            request_id,
                            &result,
                        )
                        .await
                        {
                            Ok(()) => transaction
                                .commit()
                                .await
                                .map(|_| result)
                                .map_err(anyhow::Error::from),
                            Err(error) => {
                                let _ = transaction.rollback().await;
                                Err(error)
                            }
                        },
                        Err(error) => {
                            let _ = transaction.rollback().await;
                            Err(error)
                        }
                    };
                match committed {
                    Ok(result) => result,
                    Err(error) => return settle_failure(client, task, error, &progress).await,
                }
            }
        };

    // The report, under the event's signing lock.
    let transaction = client.transaction().await?;
    lock_signing_event(&transaction, tenant_id, election_event_id).await?;
    if !finish_dispatched(
        &transaction,
        tenant_id,
        election_event_id,
        request_id,
        task_execution_id,
        DispatchedResult::Executed(Some(result.clone())),
    )
    .await?
    {
        transaction.rollback().await?;
        return Ok(RunOutcome::NotClaimed);
    }
    if matches!(
        request.action,
        SigningAction::OpenVoting | SigningAction::CloseVoting
    ) {
        voting::cancel_opposite(&transaction, &request).await?;
    }
    set_signed_action_task_status(
        &transaction,
        tenant_id,
        task_execution_id,
        TasksExecutionStatus::SUCCESS,
    )
    .await?;
    transaction.commit().await?;
    kick_signing_log_outbox();
    Ok(RunOutcome::Executed(result))
}

/// A failed effect: a refusal, or a failure after acting, fails the request;
/// a failure before acting leaves it for the sweeper.
async fn settle_failure(
    client: &mut Client,
    task: &SignedActionTask,
    error: anyhow::Error,
    progress: &EffectProgress,
) -> Result<RunOutcome> {
    let code = match error.downcast_ref::<EffectRefused>() {
        Some(refused) => Some(refused.code.clone()),
        None if progress.reached_outside() => Some(EFFECT_PARTIAL.to_owned()),
        None => None,
    };
    let transaction = client.transaction().await?;
    let Some(code) = code else {
        warn!(request_id = %task.request_id, "the signed action failed before acting: {error:#}");
        set_signed_action_task_status(
            &transaction,
            task.tenant_id,
            task.task_execution_id,
            TasksExecutionStatus::FAILED,
        )
        .await?;
        transaction.commit().await?;
        return Ok(RunOutcome::Retry(format!("{error:#}")));
    };
    if code == EFFECT_PARTIAL {
        error!(request_id = %task.request_id, "the signed action failed after acting: {error:#}");
    } else {
        warn!(request_id = %task.request_id, "the signed action was refused: {error:#}");
    }
    lock_signing_event(&transaction, task.tenant_id, task.election_event_id).await?;
    finish_dispatched(
        &transaction,
        task.tenant_id,
        task.election_event_id,
        task.request_id,
        task.task_execution_id,
        DispatchedResult::Failed { code: code.clone() },
    )
    .await?;
    set_signed_action_task_status(
        &transaction,
        task.tenant_id,
        task.task_execution_id,
        TasksExecutionStatus::FAILED,
    )
    .await?;
    transaction.commit().await?;
    kick_signing_log_outbox();
    Ok(RunOutcome::Failed(code))
}

/// What the task of a dispatched execution does: [`claim`], then
/// [`run_claimed`].
pub async fn run_dispatched(
    client: &mut Client,
    effects: &dyn SignedActionEffects,
    seal: &dyn SealRecordSink,
    task: &SignedActionTask,
) -> Result<RunOutcome> {
    match claim(client, task).await? {
        Some(token) => run_claimed(client, effects, seal, task, token).await,
        None => Ok(RunOutcome::NotClaimed),
    }
}

#[cfg(test)]
#[path = "actions_tests.rs"]
mod actions_tests;
