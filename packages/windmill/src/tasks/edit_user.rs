// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::cast_vote::{
    finalize_voter_release, get_voter_cast_vote_state, mark_voter_release_pending,
    quarantine_valid_cast_votes, restore_quarantined_cast_votes, DatafixPendingOperation,
    VoterCastVoteState,
};
use crate::postgres::election_event::{get_election_event_by_id, ElectionEventDatafix};
use crate::services::database::get_hasura_pool;
use crate::services::datafix;
use crate::services::datafix::types::{SoapRequest, SoapRequestResponse, SoapRequestResult};
use crate::services::datafix::utils::{
    datafix_voter_lock_key, post_operation_result_to_electoral_log, voted_via_internet,
    voted_via_not_internet_channel, DATAFIX_VOTER_LOCK_SECS,
};
use crate::services::pg_lock::PgLock;
use crate::services::tasks_execution::{update_complete, update_fail};
use crate::types::error::{Error, Result};
use anyhow::Context;
use celery::error::TaskError;
use chrono::Duration;
use deadpool_postgres::Client as DbClient;
use electoral_log::messages::newtypes::ExtApiRequestDirection;
use sequent_core::services::date::ISO8601;
use sequent_core::services::keycloak::{get_event_realm, KeycloakAdminClient};
use sequent_core::services::uuid_validation::parse_uuid_v4;
use sequent_core::types::hasura::core::{ElectionEvent, TasksExecution};
use sequent_core::types::keycloak::{
    User, ATTR_RESET_VALUE, DISABLE_COMMENT, DISABLE_REASON_SET_NOT_VOTED_PENDING, VOTED_CHANNEL,
    VOTED_CHANNEL_INTERNET_VALUE,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{error, info, instrument};
use uuid::Uuid;

/// Input for the `edit_user` task. Mirrors the fields the `/edit-user` route
/// forwards to Keycloak. Only launched for a Datafix election event, so
/// `election_event_id` is always present.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EditUserTaskBody {
    pub tenant_id: String,
    pub user_id: String,
    pub election_event_id: String,
    pub enabled: Option<bool>,
    pub attributes: HashMap<String, Vec<String>>,
    pub email: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub temporary: Option<bool>,
}

/// Response of the `/edit-user` route. For Datafix election events the edit is
/// deferred to the `edit_user` task and `task_execution` is returned (`user` is
/// `None`) so the admin portal can track the VoterView-coupled release in a task
/// widget; otherwise the voter is edited synchronously and returned in `user`
/// with `task_execution` set to `None`.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EditUserOutput {
    pub user: Option<User>,
    pub task_execution: Option<TasksExecution>,
}

/// Edits a Datafix voter asynchronously so the admin's Save is not blocked by
/// the (potentially slow, retried) VoterView round-trip. The release logic runs
/// in [`apply_datafix_voter_edit`]; its outcome — success or a human-readable
/// reason — is recorded on `task_execution`, which backs the operator's task
/// widget. `max_retries = 0`: a retry would re-issue the non-idempotent
/// `SetNotVoted` request, so a failure is terminal and left for the operator to
/// resolve (a repeated save resumes the release via the pending marker).
#[instrument(
    skip_all,
    fields(
        tenant_id = %body.tenant_id,
        election_event_id = %body.election_event_id,
        user_id = %body.user_id
    ),
    err
)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn edit_user(body: EditUserTaskBody, task_execution: TasksExecution) -> Result<()> {
    match apply_datafix_voter_edit(&body).await {
        Ok(()) => {
            update_complete(&task_execution, None)
                .await
                .context("Failed to update the edit-user task execution status to COMPLETED")?;
            Ok(())
        }
        Err(message) => {
            update_fail(&task_execution, &message).await.ok();
            Err(Error::String(message))
        }
    }
}

/// A save that flips an enabled voter to disabled, which triggers the
/// quarantine → `SetNotVoted` → discard release path.
#[instrument]
fn is_disable_transition(current: Option<bool>, requested: Option<bool>) -> bool {
    current == Some(true) && requested == Some(false)
}

/// A save that flips a disabled voter back to enabled; only allowed once any
/// pending release has cleared.
#[instrument]
fn is_reenable_transition(current: Option<bool>, requested: Option<bool>) -> bool {
    current == Some(false) && requested == Some(true)
}

/// A save that keeps a disabled voter disabled; may need to resume an
/// interrupted release.
#[instrument]
fn is_repeated_disable(current: Option<bool>, requested: Option<bool>) -> bool {
    current == Some(false) && requested == Some(false)
}

/// Whether a `SetNotVoted` response means the external system now agrees the
/// voter has not voted, so the quarantined ballots can be discarded.
#[instrument]
fn set_not_voted_converged(response: &SoapRequestResponse) -> bool {
    matches!(
        response,
        SoapRequestResponse::Ok | SoapRequestResponse::AlreadyNotVoted
    )
}

/// Whether the voter carries the durable marker left by an interrupted release,
/// meaning a `SetNotVoted` is still owed before the voter may be re-enabled.
#[instrument(skip_all)]
fn has_pending_voter_release(attributes: &HashMap<String, Vec<String>>) -> bool {
    matches!(
        attributes
            .get(DISABLE_COMMENT)
            .and_then(|values| values.last()),
        Some(value) if value == DISABLE_REASON_SET_NOT_VOTED_PENDING
    )
}

/// Context shared by every phase of a Datafix voter edit: the request body plus
/// the realm and per-voter lock the whole flow operates under.
struct DatafixEditCtx<'a> {
    body: &'a EditUserTaskBody,
    realm: String,
    lock: &'a PgLock,
}

/// The release actions a Datafix voter edit must take, derived from the
/// enabled-flag transition, the voter's cast-vote state and its current
/// Keycloak attributes.
#[derive(Debug)]
struct VoterReleasePlan {
    /// The edit disables the voter (or repeats a disable that still owes a
    /// release), so its valid ballots must be quarantined for `SetNotVoted`.
    release_attempt: bool,
    /// The release targets a voter who voted online, so the edit must keep the
    /// Internet voted-channel marker.
    stamp_internet_channel: bool,
    /// The voter's state owes a `SetNotVoted` even if the quarantine finds no
    /// ballots: indeterminate or pending votes, or the durable voter markers.
    owes_set_not_voted: bool,
}

/// Derives the [`VoterReleasePlan`] for an edit and enforces the state guards:
/// a re-enable is refused while the voting state is unresolved, and a release
/// is refused for a voter who voted through another channel.
#[instrument(skip(current_attributes), err, ret)]
fn plan_voter_release(
    current_enabled: Option<bool>,
    requested_enabled: Option<bool>,
    cast_vote_state: &VoterCastVoteState,
    current_attributes: &HashMap<String, Vec<String>>,
) -> std::result::Result<VoterReleasePlan, String> {
    let disable_transition = is_disable_transition(current_enabled, requested_enabled);
    let reenable_transition = is_reenable_transition(current_enabled, requested_enabled);
    let repeated_disable = is_repeated_disable(current_enabled, requested_enabled);
    let retry_release = repeated_disable
        && (cast_vote_state.has_unresolved_vote
            || cast_vote_state.has_valid_vote
            || cast_vote_state.has_pending_release
            || voted_via_internet(current_attributes)
            || has_pending_voter_release(current_attributes));
    let release_attempt = disable_transition || retry_release;

    if reenable_transition
        && (cast_vote_state.has_unresolved_vote
            || cast_vote_state.has_valid_vote
            || voted_via_internet(current_attributes)
            || voted_via_not_internet_channel(current_attributes)
            || has_pending_voter_release(current_attributes))
    {
        return Err(
            "Cannot re-enable a voter while its Datafix voting state is unresolved".to_string(),
        );
    }
    if release_attempt && voted_via_not_internet_channel(current_attributes) {
        return Err(
            "Cannot release a voter recorded as having voted through another channel".to_string(),
        );
    }

    Ok(VoterReleasePlan {
        release_attempt,
        stamp_internet_channel: release_attempt && voted_via_internet(current_attributes),
        owes_set_not_voted: cast_vote_state.has_indeterminate_vote
            || cast_vote_state.has_pending_release
            || voted_via_internet(current_attributes)
            || has_pending_voter_release(current_attributes),
    })
}

/// Rejects edits to the fields an admin may not change on a Datafix voter: the
/// username (the VoterView identifier) and the voted channel. The disable
/// reason, unlike the voted channel, is an admin-facing note: the system sets
/// it automatically on release, but an admin may freely override it at any
/// time.
#[instrument(skip_all, err)]
fn validate_datafix_immutable_fields(
    body: &EditUserTaskBody,
    current_user: &User,
    current_attributes: &HashMap<String, Vec<String>>,
) -> std::result::Result<(), String> {
    if body.username.is_some() && body.username.as_ref() != current_user.username.as_ref() {
        return Err("A Datafix voter identifier cannot be changed in the admin portal".to_string());
    }
    if let Some(requested_channel) = body.attributes.get(VOTED_CHANNEL) {
        if current_attributes.get(VOTED_CHANNEL) != Some(requested_channel) {
            return Err("The Datafix voting channel cannot be edited directly".to_string());
        }
    }
    Ok(())
}

/// Loads the election event the edit targets, needed for its Datafix
/// configuration when rendering `SetNotVoted`.
#[instrument(err)]
async fn load_election_event(
    tenant_id: &str,
    election_event_id: &str,
) -> anyhow::Result<ElectionEvent> {
    let mut client: DbClient = get_hasura_pool().await.get().await?;
    let transaction = client.transaction().await?;
    get_election_event_by_id(&transaction, tenant_id, election_event_id).await
}

/// Single-round snapshot of the voter's cast-vote states, used to decide whether
/// a disable needs a release and whether a re-enable is safe.
#[instrument(err, ret)]
async fn voter_cast_vote_state(
    tenant_id: &str,
    election_event_id: &str,
    voter_id: &str,
) -> anyhow::Result<VoterCastVoteState> {
    let tenant_id = parse_uuid_v4(tenant_id)?;
    let election_event_id = parse_uuid_v4(election_event_id)?;
    let mut client: DbClient = get_hasura_pool().await.get().await?;
    let transaction = client.transaction().await?;
    get_voter_cast_vote_state(&transaction, &tenant_id, &election_event_id, voter_id).await
}

/// Moves the voter's `valid` ballots to `indeterminate` (tagged `set-not-voted`)
/// and commits, returning the affected ids so they can be restored if the
/// release is later abandoned.
#[instrument(err)]
async fn quarantine_voter_cast_votes(
    tenant_id: &str,
    election_event_id: &str,
    voter_id: &str,
) -> anyhow::Result<Vec<Uuid>> {
    let tenant_id = parse_uuid_v4(tenant_id)?;
    let election_event_id = parse_uuid_v4(election_event_id)?;
    let mut client: DbClient = get_hasura_pool().await.get().await?;
    let transaction = client.transaction().await?;
    let cast_vote_ids = quarantine_valid_cast_votes(
        &transaction,
        &tenant_id,
        &election_event_id,
        voter_id,
        DatafixPendingOperation::SetNotVoted,
    )
    .await?;
    transaction.commit().await?;
    Ok(cast_vote_ids)
}

/// Reverses a quarantine by moving the given ballots back to `valid`, used when
/// the release is ambiguous and must be undone. A no-op for an empty id list.
#[instrument(err)]
async fn restore_voter_cast_votes(
    tenant_id: &str,
    election_event_id: &str,
    cast_vote_ids: &[Uuid],
) -> anyhow::Result<()> {
    if cast_vote_ids.is_empty() {
        return Ok(());
    }
    let tenant_id = parse_uuid_v4(tenant_id)?;
    let election_event_id = parse_uuid_v4(election_event_id)?;
    let mut client: DbClient = get_hasura_pool().await.get().await?;
    let transaction = client.transaction().await?;
    restore_quarantined_cast_votes(
        &transaction,
        &tenant_id,
        &election_event_id,
        cast_vote_ids,
        DatafixPendingOperation::SetNotVoted,
    )
    .await?;
    transaction.commit().await?;
    Ok(())
}

/// Renews the per-voter lock and restores the quarantined ballots, returning a
/// human-readable reason on failure. Used to unwind a release that could not be
/// dispatched, so the lock is confirmed still held before touching the votes.
#[instrument(skip(ctx), err)]
async fn restore_pre_dispatch_cast_votes(
    ctx: &DatafixEditCtx<'_>,
    cast_vote_ids: &[Uuid],
) -> std::result::Result<(), String> {
    ctx.lock
        .update_expiry_for(DATAFIX_VOTER_LOCK_SECS)
        .await
        .map_err(|err| format!("The Datafix voter lock was lost before vote restoration: {err}"))?;
    restore_voter_cast_votes(
        &ctx.body.tenant_id,
        &ctx.body.election_event_id,
        cast_vote_ids,
    )
    .await
    .map_err(|err| format!("Unable to restore pre-dispatch cast votes: {err:?}"))
}

/// Durably records that the voter's quarantined ballots are awaiting a
/// `SetNotVoted`, so a repeated save resumes the release instead of restarting
/// it.
#[instrument(err)]
async fn mark_voter_cast_votes_release_pending(
    tenant_id: &str,
    election_event_id: &str,
    voter_id: &str,
) -> anyhow::Result<()> {
    let tenant_id = parse_uuid_v4(tenant_id)?;
    let election_event_id = parse_uuid_v4(election_event_id)?;
    let mut client: DbClient = get_hasura_pool().await.get().await?;
    let transaction = client.transaction().await?;
    mark_voter_release_pending(&transaction, &tenant_id, &election_event_id, voter_id).await?;
    transaction.commit().await?;
    Ok(())
}

/// Discards the voter's ballots and clears the pending marker once `SetNotVoted`
/// has converged, completing the release.
#[instrument(err)]
async fn discard_released_voter_cast_votes(
    tenant_id: &str,
    election_event_id: &str,
    voter_id: &str,
) -> anyhow::Result<()> {
    let tenant_id = parse_uuid_v4(tenant_id)?;
    let election_event_id = parse_uuid_v4(election_event_id)?;
    let mut client: DbClient = get_hasura_pool().await.get().await?;
    let transaction = client.transaction().await?;
    finalize_voter_release(&transaction, &tenant_id, &election_event_id, voter_id).await?;
    transaction.commit().await?;
    Ok(())
}

/// Resets the voted-channel and disable-comment attributes on re-enable so a
/// previously released voter starts clean; a no-op when neither marker is set.
#[instrument(skip(user), err)]
async fn clear_voter_release_markers(
    realm: &str,
    voter_id: &str,
    user: User,
) -> anyhow::Result<User> {
    let attributes = user.attributes.clone().unwrap_or_default();
    if !voted_via_internet(&attributes) && !has_pending_voter_release(&attributes) {
        return Ok(user);
    }

    let attributes = HashMap::from([
        (
            VOTED_CHANNEL.to_string(),
            vec![ATTR_RESET_VALUE.to_string()],
        ),
        (
            DISABLE_COMMENT.to_string(),
            vec![ATTR_RESET_VALUE.to_string()],
        ),
    ]);
    KeycloakAdminClient::new()
        .await?
        .edit_user(
            realm,
            voter_id,
            None,
            Some(attributes),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
}

/// Records the outcome of a disabled-voter release in the electoral log.
/// Failures are logged and swallowed so auditing never fails the user edit.
#[instrument(skip(ctx))]
async fn audit_datafix_user_operation(ctx: &DatafixEditCtx<'_>, username: &str, operation: String) {
    let Ok(mut client) = get_hasura_pool().await.get().await else {
        error!("Unable to get a DB connection for the Datafix audit entry");
        return;
    };
    let Ok(transaction) = client.transaction().await else {
        error!("Unable to start a transaction for the Datafix audit entry");
        return;
    };
    if let Err(err) = post_operation_result_to_electoral_log(
        &transaction,
        &ctx.body.tenant_id,
        &ctx.body.election_event_id,
        Some(ctx.body.user_id.as_str()),
        username,
        ExtApiRequestDirection::Outbound,
        operation,
    )
    .await
    {
        error!("Unable to record the Datafix audit entry: {err}");
    }
}

/// Applies the edit in Keycloak; on failure the outcome is indeterminate, so
/// the quarantined ballots are restored before reporting the error.
#[instrument(skip_all, err)]
async fn edit_keycloak_user_with_rollback(
    ctx: &DatafixEditCtx<'_>,
    client: KeycloakAdminClient,
    new_attributes: HashMap<String, Vec<String>>,
    quarantined_cast_vote_ids: &[Uuid],
) -> std::result::Result<User, String> {
    let body = ctx.body;
    match client
        .edit_user(
            &ctx.realm,
            &body.user_id,
            body.enabled,
            Some(new_attributes),
            body.email.clone(),
            body.first_name.clone(),
            body.last_name.clone(),
            body.username.clone(),
            body.password.clone(),
            body.temporary,
        )
        .await
    {
        Ok(user) => Ok(user),
        Err(err) => {
            restore_pre_dispatch_cast_votes(ctx, quarantined_cast_vote_ids).await?;
            Err(format!(
                "The Keycloak update outcome is indeterminate: {err:?}"
            ))
        }
    }
}

/// Durably marks the release as pending, renews the per-voter lock and sends
/// `SetNotVoted` to VoterView, returning the voter's username along with the
/// response. The quarantine is unwound while the request has not been
/// dispatched; once sent, an error means the VoterView state is indeterminate
/// and is audited instead.
#[instrument(skip(ctx, election_event), err)]
async fn dispatch_set_not_voted(
    ctx: &DatafixEditCtx<'_>,
    election_event: ElectionEvent,
    current_username: Option<String>,
    quarantined_cast_vote_ids: &[Uuid],
) -> std::result::Result<(String, SoapRequestResult), String> {
    let body = ctx.body;
    if let Err(err) = mark_voter_cast_votes_release_pending(
        &body.tenant_id,
        &body.election_event_id,
        &body.user_id,
    )
    .await
    {
        restore_pre_dispatch_cast_votes(ctx, quarantined_cast_vote_ids).await?;
        return Err(format!(
            "Error recording the pending voter release: {err:?}"
        ));
    }

    ctx.lock
        .update_expiry_for(DATAFIX_VOTER_LOCK_SECS)
        .await
        .map_err(|err| format!("The Datafix voter lock was lost before SetNotVoted: {err}"))?;

    let username = match current_username {
        Some(username) => username,
        None => {
            restore_pre_dispatch_cast_votes(ctx, quarantined_cast_vote_ids).await?;
            return Err("Datafix voter has no username".to_string());
        }
    };
    let prepared = match datafix::voterview_requests::prepare(
        SoapRequest::SetNotVoted,
        ElectionEventDatafix(election_event),
        &Some(username.clone()),
    )
    .await
    {
        Ok(response) => response,
        Err(err) => {
            error!("Unable to prepare SetNotVoted: {err}");
            restore_pre_dispatch_cast_votes(ctx, quarantined_cast_vote_ids).await?;
            audit_datafix_user_operation(
                ctx,
                &username,
                "SetNotVoted NotDispatched: pre-dispatch-error".to_string(),
            )
            .await;
            return Err(
                "Voter was disabled, but SetNotVoted could not be prepared and requires a safe retry"
                    .to_string(),
            );
        }
    };
    let template_sha256 = prepared.template_sha256().to_string();
    match datafix::voterview_requests::send_prepared(prepared).await {
        Ok(response) => Ok((username, response)),
        Err(err) => {
            error!("SetNotVoted transport or response error: {err}");
            audit_datafix_user_operation(
                ctx,
                &username,
                format!(
                    "SetNotVoted Indeterminate: transport-or-response-error (template_sha256={template_sha256})"
                ),
            )
            .await;
            Err(
                "Voter was disabled, but its VoterView state is indeterminate and requires reconciliation"
                    .to_string(),
            )
        }
    }
}

/// Settles a dispatched `SetNotVoted`: restores the ballots on an explicit
/// rejection, audits the outcome, and on convergence clears the voter's
/// release markers and discards the released ballots.
#[instrument(skip(ctx, user, response), err)]
async fn settle_set_not_voted_outcome(
    ctx: &DatafixEditCtx<'_>,
    user: User,
    username: &str,
    response: SoapRequestResult,
    quarantined_cast_vote_ids: &[Uuid],
) -> std::result::Result<(), String> {
    let body = ctx.body;
    if matches!(&response.response, SoapRequestResponse::Rejected(_)) {
        ctx.lock
            .update_expiry_for(DATAFIX_VOTER_LOCK_SECS)
            .await
            .map_err(|err| {
                format!("The Datafix voter lock was lost after SetNotVoted rejection: {err}")
            })?;
        restore_voter_cast_votes(
            &body.tenant_id,
            &body.election_event_id,
            quarantined_cast_vote_ids,
        )
        .await
        .map_err(|err| format!("Unable to restore votes after SetNotVoted rejection: {err:?}"))?;
    }

    let operation = if set_not_voted_converged(&response.response) {
        format!(
            "SetNotVoted Succeeded (template_sha256={})",
            response.template_sha256
        )
    } else {
        format!(
            "SetNotVoted Indeterminate: {} (template_sha256={})",
            response.response.classification(),
            response.template_sha256
        )
    };
    audit_datafix_user_operation(ctx, username, operation).await;

    if !set_not_voted_converged(&response.response) {
        return Err(format!(
            "Voter was disabled, but VoterView did not accept SetNotVoted ({}) and reconciliation is required",
            response.response.classification()
        ));
    }

    ctx.lock
        .update_expiry_for(DATAFIX_VOTER_LOCK_SECS)
        .await
        .map_err(|err| format!("The Datafix voter lock was lost after SetNotVoted: {err}"))?;

    let _user = clear_voter_release_markers(&ctx.realm, &body.user_id, user)
        .await
        .map_err(|err| {
            format!("VoterView accepted SetNotVoted, but the Internet voting marker could not be cleared: {err:?}")
        })?;
    ctx.lock
        .update_expiry_for(DATAFIX_VOTER_LOCK_SECS)
        .await
        .map_err(|err| {
            format!("The Datafix voter lock was lost before finalizing SetNotVoted: {err}")
        })?;
    discard_released_voter_cast_votes(&body.tenant_id, &body.election_event_id, &body.user_id)
        .await
        .map_err(|err| {
            format!(
                "VoterView accepted SetNotVoted, but cast votes could not be discarded: {err:?}"
            )
        })?;

    Ok(())
}

/// Runs the voter edit while the per-voter lock is held: validates the edit,
/// plans the release, quarantines the ballots, applies the Keycloak edit and,
/// when a `SetNotVoted` is owed, dispatches it and settles its outcome.
#[instrument(skip(ctx, election_event), err)]
async fn run_datafix_voter_edit(
    ctx: &DatafixEditCtx<'_>,
    election_event: ElectionEvent,
) -> std::result::Result<(), String> {
    let body = ctx.body;
    let client = KeycloakAdminClient::new()
        .await
        .map_err(|err| format!("{err:?}"))?;
    let current_user = client
        .get_user(&ctx.realm, &body.user_id)
        .await
        .map_err(|err| format!("{err:?}"))?;
    let current_attributes = current_user.attributes.clone().unwrap_or_default();
    validate_datafix_immutable_fields(body, &current_user, &current_attributes)?;

    let needs_cast_vote_state = is_disable_transition(current_user.enabled, body.enabled)
        || is_reenable_transition(current_user.enabled, body.enabled)
        || is_repeated_disable(current_user.enabled, body.enabled);
    let cast_vote_state = if needs_cast_vote_state {
        voter_cast_vote_state(&body.tenant_id, &body.election_event_id, &body.user_id)
            .await
            .map_err(|err| format!("Error checking unresolved cast votes: {err:?}"))?
    } else {
        VoterCastVoteState {
            has_unresolved_vote: false,
            has_indeterminate_vote: false,
            has_pending_release: false,
            has_valid_vote: false,
        }
    };
    let plan = plan_voter_release(
        current_user.enabled,
        body.enabled,
        &cast_vote_state,
        &current_attributes,
    )?;

    let mut new_attributes = body.attributes.clone();
    if plan.stamp_internet_channel {
        new_attributes.insert(
            VOTED_CHANNEL.to_string(),
            vec![VOTED_CHANNEL_INTERNET_VALUE.to_string()],
        );
    }
    info!("Voter edit plan: {plan:?}, cast-vote state: {cast_vote_state:?}");
    let quarantined_cast_vote_ids = if plan.release_attempt {
        quarantine_voter_cast_votes(&body.tenant_id, &body.election_event_id, &body.user_id)
            .await
            .map_err(|err| format!("Error quarantining cast votes: {err:?}"))?
    } else {
        Vec::new()
    };
    let should_send_set_not_voted =
        plan.release_attempt && (!quarantined_cast_vote_ids.is_empty() || plan.owes_set_not_voted);
    if should_send_set_not_voted {
        new_attributes.insert(
            DISABLE_COMMENT.to_string(),
            vec![DISABLE_REASON_SET_NOT_VOTED_PENDING.to_string()],
        );
    }

    let user =
        edit_keycloak_user_with_rollback(ctx, client, new_attributes, &quarantined_cast_vote_ids)
            .await?;

    if !should_send_set_not_voted {
        if plan.release_attempt && cast_vote_state.has_unresolved_vote {
            ctx.lock.update_expiry_for(DATAFIX_VOTER_LOCK_SECS).await.map_err(|err| {
                format!("The Datafix voter lock was lost before discarding undispatched votes: {err}")
            })?;
            discard_released_voter_cast_votes(
                &body.tenant_id,
                &body.election_event_id,
                &body.user_id,
            )
            .await
            .map_err(|err| format!("Error discarding undispatched Datafix votes: {err:?}"))?;
        }
        return Ok(());
    }

    let (username, response) = dispatch_set_not_voted(
        ctx,
        election_event,
        current_user.username.clone(),
        &quarantined_cast_vote_ids,
    )
    .await?;
    settle_set_not_voted_outcome(ctx, user, &username, response, &quarantined_cast_vote_ids).await
}

/// Applies an admin edit to a Datafix voter under the per-voter lock. On a
/// disable of a voter who already voted online it quarantines the ballots, sends
/// `SetNotVoted`, and either discards them on convergence or restores them
/// (leaving a durable pending marker) on an ambiguous outcome; a re-enable is
/// refused while a release is still pending. Returns `Ok(())` on success or a
/// human-readable failure reason recorded on the task widget.
#[instrument(skip(body), err)]
async fn apply_datafix_voter_edit(body: &EditUserTaskBody) -> std::result::Result<(), String> {
    let realm = get_event_realm(&body.tenant_id, &body.election_event_id);
    let election_event = load_election_event(&body.tenant_id, &body.election_event_id)
        .await
        .map_err(|err| format!("Error loading election event: {err:?}"))?;

    let lock = PgLock::acquire(
        datafix_voter_lock_key(&body.tenant_id, &body.election_event_id, &body.user_id),
        Uuid::new_v4().to_string(),
        ISO8601::now() + Duration::seconds(DATAFIX_VOTER_LOCK_SECS),
    )
    .await
    .map_err(|err| format!("Another operation is updating this voter: {err}"))?;

    let ctx = DatafixEditCtx {
        body,
        realm,
        lock: &lock,
    };
    let result = run_datafix_voter_edit(&ctx, election_event).await;

    if let Err(err) = lock.release().await {
        if result.is_ok() {
            return Err(format!("Error releasing the Datafix voter lock: {err}"));
        }
        error!("Error releasing the Datafix voter lock: {err}");
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disable_requires_an_enabled_to_disabled_transition() {
        assert!(is_disable_transition(Some(true), Some(false)));
        assert!(!is_disable_transition(Some(false), Some(false)));
        assert!(!is_disable_transition(Some(true), None));
        assert!(!is_disable_transition(None, Some(false)));
    }

    #[test]
    fn reenable_requires_a_disabled_to_enabled_transition() {
        assert!(is_reenable_transition(Some(false), Some(true)));
        assert!(!is_reenable_transition(Some(true), Some(true)));
        assert!(!is_reenable_transition(Some(false), None));
        assert!(!is_reenable_transition(None, Some(true)));
    }

    #[test]
    fn already_not_voted_is_a_converged_release() {
        assert!(set_not_voted_converged(&SoapRequestResponse::Ok));
        assert!(set_not_voted_converged(
            &SoapRequestResponse::AlreadyNotVoted
        ));
        assert!(!set_not_voted_converged(&SoapRequestResponse::Rejected(
            "rejected".to_string()
        )));
    }

    #[test]
    fn pending_release_marker_is_explicit() {
        let pending = HashMap::from([(
            DISABLE_COMMENT.to_string(),
            vec![DISABLE_REASON_SET_NOT_VOTED_PENDING.to_string()],
        )]);
        assert!(has_pending_voter_release(&pending));

        let reset = HashMap::from([(
            DISABLE_COMMENT.to_string(),
            vec![ATTR_RESET_VALUE.to_string()],
        )]);
        assert!(!has_pending_voter_release(&reset));
    }

    #[test]
    fn repeated_disable_keeps_a_disabled_voter_disabled() {
        assert!(is_repeated_disable(Some(false), Some(false)));
        assert!(!is_repeated_disable(Some(true), Some(false)));
        assert!(!is_repeated_disable(Some(false), None));
        assert!(!is_repeated_disable(None, Some(false)));
    }

    fn no_cast_votes() -> VoterCastVoteState {
        VoterCastVoteState {
            has_unresolved_vote: false,
            has_indeterminate_vote: false,
            has_pending_release: false,
            has_valid_vote: false,
        }
    }

    fn internet_voter() -> HashMap<String, Vec<String>> {
        HashMap::from([(
            VOTED_CHANNEL.to_string(),
            vec![VOTED_CHANNEL_INTERNET_VALUE.to_string()],
        )])
    }

    fn pending_release_voter() -> HashMap<String, Vec<String>> {
        HashMap::from([(
            DISABLE_COMMENT.to_string(),
            vec![DISABLE_REASON_SET_NOT_VOTED_PENDING.to_string()],
        )])
    }

    #[test]
    fn disabling_a_clean_voter_releases_without_owing_set_not_voted() {
        let plan =
            plan_voter_release(Some(true), Some(false), &no_cast_votes(), &HashMap::new()).unwrap();
        assert!(plan.release_attempt);
        assert!(!plan.stamp_internet_channel);
        assert!(!plan.owes_set_not_voted);
    }

    #[test]
    fn disabling_an_internet_voter_stamps_the_channel_and_owes_set_not_voted() {
        let plan = plan_voter_release(Some(true), Some(false), &no_cast_votes(), &internet_voter())
            .unwrap();
        assert!(plan.release_attempt);
        assert!(plan.stamp_internet_channel);
        assert!(plan.owes_set_not_voted);
    }

    #[test]
    fn a_repeated_disable_with_a_pending_marker_retries_the_release() {
        let plan = plan_voter_release(
            Some(false),
            Some(false),
            &no_cast_votes(),
            &pending_release_voter(),
        )
        .unwrap();
        assert!(plan.release_attempt);
        assert!(plan.owes_set_not_voted);
    }

    #[test]
    fn a_repeated_disable_of_a_clean_voter_plans_no_release() {
        let plan = plan_voter_release(Some(false), Some(false), &no_cast_votes(), &HashMap::new())
            .unwrap();
        assert!(!plan.release_attempt);
    }

    #[test]
    fn reenabling_is_refused_while_a_release_is_pending() {
        assert!(plan_voter_release(
            Some(false),
            Some(true),
            &no_cast_votes(),
            &pending_release_voter()
        )
        .is_err());
    }

    #[test]
    fn reenabling_is_refused_while_valid_votes_exist() {
        let state = VoterCastVoteState {
            has_valid_vote: true,
            ..no_cast_votes()
        };
        assert!(plan_voter_release(Some(false), Some(true), &state, &HashMap::new()).is_err());
    }

    #[test]
    fn reenabling_a_clean_voter_plans_no_release() {
        let plan =
            plan_voter_release(Some(false), Some(true), &no_cast_votes(), &HashMap::new()).unwrap();
        assert!(!plan.release_attempt);
    }

    #[test]
    fn releasing_an_other_channel_voter_is_refused() {
        let attributes = HashMap::from([(VOTED_CHANNEL.to_string(), vec!["PAPER".to_string()])]);
        assert!(
            plan_voter_release(Some(true), Some(false), &no_cast_votes(), &attributes).is_err()
        );
    }

    fn edit_body(
        username: Option<&str>,
        attributes: HashMap<String, Vec<String>>,
    ) -> EditUserTaskBody {
        EditUserTaskBody {
            tenant_id: String::new(),
            user_id: String::new(),
            election_event_id: String::new(),
            enabled: None,
            attributes,
            email: None,
            first_name: None,
            last_name: None,
            username: username.map(str::to_string),
            password: None,
            temporary: None,
        }
    }

    #[test]
    fn the_voter_identifier_is_immutable() {
        let current_user = User {
            username: Some("voter1".to_string()),
            ..User::default()
        };
        let renamed = edit_body(Some("voter2"), HashMap::new());
        assert!(
            validate_datafix_immutable_fields(&renamed, &current_user, &HashMap::new()).is_err()
        );
        let unchanged = edit_body(Some("voter1"), HashMap::new());
        assert!(
            validate_datafix_immutable_fields(&unchanged, &current_user, &HashMap::new()).is_ok()
        );
    }

    #[test]
    fn the_voted_channel_is_immutable() {
        let current_user = User::default();
        let channel_edit = edit_body(None, internet_voter());
        assert!(
            validate_datafix_immutable_fields(&channel_edit, &current_user, &HashMap::new())
                .is_err()
        );
        let echoed = edit_body(None, internet_voter());
        assert!(
            validate_datafix_immutable_fields(&echoed, &current_user, &internet_voter()).is_ok()
        );
    }

    #[test]
    fn the_disable_reason_can_be_edited_by_an_admin() {
        let current_user = User::default();
        let reason_edit = edit_body(None, pending_release_voter());
        assert!(
            validate_datafix_immutable_fields(&reason_edit, &current_user, &HashMap::new()).is_ok()
        );
    }
}
