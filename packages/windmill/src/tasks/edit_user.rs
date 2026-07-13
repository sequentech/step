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
use crate::services::datafix::types::{SoapRequest, SoapRequestResponse};
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
use tracing::{error, instrument};
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
#[instrument(err)]
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
fn is_disable_transition(current: Option<bool>, requested: Option<bool>) -> bool {
    current == Some(true) && requested == Some(false)
}

/// A save that flips a disabled voter back to enabled; only allowed once any
/// pending release has cleared.
fn is_reenable_transition(current: Option<bool>, requested: Option<bool>) -> bool {
    current == Some(false) && requested == Some(true)
}

/// Whether a `SetNotVoted` response means the external system now agrees the
/// voter has not voted, so the quarantined ballots can be discarded.
fn set_not_voted_converged(response: &SoapRequestResponse) -> bool {
    matches!(
        response,
        SoapRequestResponse::Ok | SoapRequestResponse::AlreadyNotVoted
    )
}

/// Whether the voter carries the durable marker left by an interrupted release,
/// meaning a `SetNotVoted` is still owed before the voter may be re-enabled.
fn has_pending_voter_release(attributes: &HashMap<String, Vec<String>>) -> bool {
    matches!(
        attributes
            .get(DISABLE_COMMENT)
            .and_then(|values| values.last()),
        Some(value) if value == DISABLE_REASON_SET_NOT_VOTED_PENDING
    )
}

/// Loads the election event the edit targets, needed for its Datafix
/// configuration when rendering `SetNotVoted`.
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
async fn restore_pre_dispatch_cast_votes(
    lock: &PgLock,
    tenant_id: &str,
    election_event_id: &str,
    cast_vote_ids: &[Uuid],
) -> std::result::Result<(), String> {
    lock.update_expiry_for(DATAFIX_VOTER_LOCK_SECS)
        .await
        .map_err(|err| format!("The Datafix voter lock was lost before vote restoration: {err}"))?;
    restore_voter_cast_votes(tenant_id, election_event_id, cast_vote_ids)
        .await
        .map_err(|err| format!("Unable to restore pre-dispatch cast votes: {err:?}"))
}

/// Durably records that the voter's quarantined ballots are awaiting a
/// `SetNotVoted`, so a repeated save resumes the release instead of restarting
/// it.
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
        (VOTED_CHANNEL.to_string(), vec![ATTR_RESET_VALUE.to_string()]),
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
async fn audit_datafix_user_operation(
    tenant_id: &str,
    election_event_id: &str,
    user_id: &str,
    username: &str,
    operation: String,
) {
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
        tenant_id,
        election_event_id,
        Some(user_id),
        username,
        ExtApiRequestDirection::Outbound,
        operation,
    )
    .await
    {
        error!("Unable to record the Datafix audit entry: {err}");
    }
}

/// Applies an admin edit to a Datafix voter under the per-voter lock. On a
/// disable of a voter who already voted online it quarantines the ballots, sends
/// `SetNotVoted`, and either discards them on convergence or restores them
/// (leaving a durable pending marker) on an ambiguous outcome; a re-enable is
/// refused while a release is still pending. Returns `Ok(())` on success or a
/// human-readable failure reason recorded on the task widget.
async fn apply_datafix_voter_edit(body: &EditUserTaskBody) -> std::result::Result<(), String> {
    let realm = get_event_realm(&body.tenant_id, &body.election_event_id);
    let election_event_id = body.election_event_id.as_str();
    let election_event = load_election_event(&body.tenant_id, election_event_id)
        .await
        .map_err(|err| format!("Error loading election event: {err:?}"))?;
    let mut new_attributes = body.attributes.clone();

    let lock = PgLock::acquire(
        datafix_voter_lock_key(&body.tenant_id, election_event_id, &body.user_id),
        Uuid::new_v4().to_string(),
        ISO8601::now() + Duration::seconds(DATAFIX_VOTER_LOCK_SECS),
    )
    .await
    .map_err(|err| format!("Another operation is updating this voter: {err}"))?;

    let result = async {
        let client = KeycloakAdminClient::new()
            .await
            .map_err(|err| format!("{err:?}"))?;
        let current_user = client
            .get_user(&realm, &body.user_id)
            .await
            .map_err(|err| format!("{err:?}"))?;
        let current_attributes = current_user.attributes.clone().unwrap_or_default();
        let disable_transition = is_disable_transition(current_user.enabled, body.enabled);
        let reenable_transition = is_reenable_transition(current_user.enabled, body.enabled);
        let repeated_disable = current_user.enabled == Some(false) && body.enabled == Some(false);
        if body.username.is_some() && body.username.as_ref() != current_user.username.as_ref() {
            return Err(
                "A Datafix voter identifier cannot be changed in the admin portal".to_string(),
            );
        }
        if let Some(requested_channel) = new_attributes.get(VOTED_CHANNEL) {
            if current_attributes.get(VOTED_CHANNEL) != Some(requested_channel) {
                return Err("The Datafix voting channel cannot be edited directly".to_string());
            }
        }
        if let Some(requested_comment) = new_attributes.get(DISABLE_COMMENT) {
            if current_attributes.get(DISABLE_COMMENT) != Some(requested_comment) {
                return Err("The Datafix disable reason cannot be edited directly".to_string());
            }
        }
        let cast_vote_state = if disable_transition || reenable_transition || repeated_disable {
            voter_cast_vote_state(&body.tenant_id, election_event_id, &body.user_id)
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
        let retry_release = repeated_disable
            && (cast_vote_state.has_unresolved_vote
                || cast_vote_state.has_valid_vote
                || cast_vote_state.has_pending_release
                || voted_via_internet(&current_attributes)
                || has_pending_voter_release(&current_attributes));
        let release_attempt = disable_transition || retry_release;

        if release_attempt && voted_via_internet(&current_attributes) {
            new_attributes.insert(
                VOTED_CHANNEL.to_string(),
                vec![VOTED_CHANNEL_INTERNET_VALUE.to_string()],
            );
        }

        if reenable_transition
            && (cast_vote_state.has_unresolved_vote
                || cast_vote_state.has_valid_vote
                || voted_via_internet(&current_attributes)
                || voted_via_not_internet_channel(&current_attributes)
                || has_pending_voter_release(&current_attributes))
        {
            return Err(
                "Cannot re-enable a voter while its Datafix voting state is unresolved".to_string(),
            );
        }
        if repeated_disable
            && !retry_release
            && (cast_vote_state.has_unresolved_vote
                || voted_via_internet(&current_attributes)
                || has_pending_voter_release(&current_attributes))
        {
            return Err(
                "The voter is disabled, but its Datafix voting state still requires reconciliation"
                    .to_string(),
            );
        }
        if release_attempt && voted_via_not_internet_channel(&current_attributes) {
            return Err(
                "Cannot release a voter recorded as having voted through another channel"
                    .to_string(),
            );
        }

        let quarantined_cast_vote_ids = if release_attempt {
            quarantine_voter_cast_votes(&body.tenant_id, election_event_id, &body.user_id)
                .await
                .map_err(|err| format!("Error quarantining cast votes: {err:?}"))?
        } else {
            Vec::new()
        };
        let should_send_set_not_voted = release_attempt
            && (!quarantined_cast_vote_ids.is_empty()
                || cast_vote_state.has_indeterminate_vote
                || cast_vote_state.has_pending_release
                || voted_via_internet(&current_attributes)
                || has_pending_voter_release(&current_attributes));
        if should_send_set_not_voted {
            new_attributes.insert(
                DISABLE_COMMENT.to_string(),
                vec![DISABLE_REASON_SET_NOT_VOTED_PENDING.to_string()],
            );
        }

        let user = match client
            .edit_user(
                &realm,
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
            Ok(user) => user,
            Err(err) => {
                restore_pre_dispatch_cast_votes(
                    &lock,
                    &body.tenant_id,
                    election_event_id,
                    &quarantined_cast_vote_ids,
                )
                .await?;
                return Err(format!("The Keycloak update outcome is indeterminate: {err:?}"));
            }
        };

        if !should_send_set_not_voted {
            if release_attempt && cast_vote_state.has_unresolved_vote {
                lock.update_expiry_for(DATAFIX_VOTER_LOCK_SECS).await.map_err(|err| {
                    format!("The Datafix voter lock was lost before discarding undispatched votes: {err}")
                })?;
                discard_released_voter_cast_votes(&body.tenant_id, election_event_id, &body.user_id)
                    .await
                    .map_err(|err| format!("Error discarding undispatched Datafix votes: {err:?}"))?;
            }
            return Ok(());
        }

        if let Err(err) = mark_voter_cast_votes_release_pending(
            &body.tenant_id,
            election_event_id,
            &body.user_id,
        )
        .await
        {
            restore_pre_dispatch_cast_votes(
                &lock,
                &body.tenant_id,
                election_event_id,
                &quarantined_cast_vote_ids,
            )
            .await?;
            return Err(format!("Error recording the pending voter release: {err:?}"));
        }

        lock.update_expiry_for(DATAFIX_VOTER_LOCK_SECS)
            .await
            .map_err(|err| format!("The Datafix voter lock was lost before SetNotVoted: {err}"))?;

        let username = match current_user.username.clone() {
            Some(username) => username,
            None => {
                restore_pre_dispatch_cast_votes(
                    &lock,
                    &body.tenant_id,
                    election_event_id,
                    &quarantined_cast_vote_ids,
                )
                .await?;
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
                restore_pre_dispatch_cast_votes(
                    &lock,
                    &body.tenant_id,
                    election_event_id,
                    &quarantined_cast_vote_ids,
                )
                .await?;
                audit_datafix_user_operation(
                    &body.tenant_id,
                    election_event_id,
                    &body.user_id,
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
        let response = match datafix::voterview_requests::send_prepared(prepared).await {
            Ok(response) => response,
            Err(err) => {
                error!("SetNotVoted transport or response error: {err}");
                audit_datafix_user_operation(
                    &body.tenant_id,
                    election_event_id,
                    &body.user_id,
                    &username,
                    format!(
                        "SetNotVoted Indeterminate: transport-or-response-error (template_sha256={template_sha256})"
                    ),
                )
                .await;
                return Err(
                    "Voter was disabled, but its VoterView state is indeterminate and requires reconciliation"
                        .to_string(),
                );
            }
        };

        if matches!(&response.response, SoapRequestResponse::Rejected(_)) {
            lock.update_expiry_for(DATAFIX_VOTER_LOCK_SECS).await.map_err(|err| {
                format!("The Datafix voter lock was lost after SetNotVoted rejection: {err}")
            })?;
            restore_voter_cast_votes(&body.tenant_id, election_event_id, &quarantined_cast_vote_ids)
                .await
                .map_err(|err| {
                    format!("Unable to restore votes after SetNotVoted rejection: {err:?}")
                })?;
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
        audit_datafix_user_operation(
            &body.tenant_id,
            election_event_id,
            &body.user_id,
            &username,
            operation,
        )
        .await;

        if !set_not_voted_converged(&response.response) {
            return Err(format!(
                "Voter was disabled, but VoterView did not accept SetNotVoted ({}) and reconciliation is required",
                response.response.classification()
            ));
        }

        lock.update_expiry_for(DATAFIX_VOTER_LOCK_SECS)
            .await
            .map_err(|err| format!("The Datafix voter lock was lost after SetNotVoted: {err}"))?;

        let _user = clear_voter_release_markers(&realm, &body.user_id, user)
            .await
            .map_err(|err| {
                format!("VoterView accepted SetNotVoted, but the Internet voting marker could not be cleared: {err:?}")
            })?;
        lock.update_expiry_for(DATAFIX_VOTER_LOCK_SECS).await.map_err(|err| {
            format!("The Datafix voter lock was lost before finalizing SetNotVoted: {err}")
        })?;
        discard_released_voter_cast_votes(&body.tenant_id, election_event_id, &body.user_id)
            .await
            .map_err(|err| {
                format!("VoterView accepted SetNotVoted, but cast votes could not be discarded: {err:?}")
            })?;

        Ok(())
    }
    .await;

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
}
