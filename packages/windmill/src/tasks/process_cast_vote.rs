// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::cast_vote::{
    compare_and_set_cast_vote_status, get_cast_vote_by_id, get_voter_cast_vote_state,
    has_valid_cast_vote,
};
use crate::postgres::election_event::{get_election_event_by_id, ElectionEventDatafix};
use crate::services::cast_votes::{CastVote, CastVoteStatus};
use crate::services::database::get_hasura_pool;
use crate::services::datafix;
use crate::services::datafix::types::{SoapRequest, SoapRequestResponse};
use crate::services::datafix::utils::{
    datafix_annotations, datafix_voter_lock_key, post_operation_result_to_electoral_log,
    voted_via_internet, voted_via_not_internet_channel, DATAFIX_VOTER_LOCK_SECS,
};
use crate::services::datafix::voterview_requests::SoapSendError;
use crate::services::pg_lock::PgLock;
use crate::types::error::Result;
use celery::error::TaskError;
use chrono::Duration;
use deadpool_postgres::Client as DbClient;
use electoral_log::messages::newtypes::ExtApiRequestDirection;
use sequent_core::services::date::ISO8601;
use sequent_core::services::keycloak::{get_event_realm, KeycloakAdminClient};
use sequent_core::services::uuid_validation::parse_uuid_v4;
use sequent_core::types::hasura::core::ElectionEvent;
use sequent_core::types::keycloak::{VOTED_CHANNEL, VOTED_CHANNEL_INTERNET_VALUE};
use sequent_core::util::retry::retry_with_exponential_backoff;
use std::collections::HashMap;
use std::time::Duration as StdDuration;
use tracing::{error, info, instrument, warn};
use uuid::Uuid;

/// Processes a single Datafix vote left `in-progress` by the insert path:
/// reloads the row, skips it unless it is still `in-progress`, then takes the
/// event-wide per-voter lock and delegates the actual claim/send to
/// `process_locked_cast_vote`. The lock is always released, and its release
/// error is only surfaced after the processing result so a failed send is not
/// masked. `max_retries = 0` because the review beat, not Celery, drives retries.
#[instrument(fields(cast_vote_id = %cast_vote_id), err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn process_cast_vote(
    tenant_id: String,
    election_event_id: String,
    cast_vote_id: String,
) -> Result<()> {
    let cast_vote_id =
        Uuid::parse_str(&cast_vote_id).map_err(|err| format!("Invalid cast_vote_id: {err}"))?;
    let Some(cast_vote) = load_cast_vote(&tenant_id, &election_event_id, &cast_vote_id).await?
    else {
        info!("Cast vote no longer exists; skipping");
        return Ok(());
    };
    if cast_vote.status != CastVoteStatus::InProgress {
        info!("Cast vote is no longer in-progress; skipping");
        return Ok(());
    }

    let voter_id = cast_vote
        .voter_id_string
        .as_deref()
        .ok_or("Voter id not found")?;
    let lock = match PgLock::acquire(
        datafix_voter_lock_key(&cast_vote.tenant_id, &cast_vote.election_event_id, voter_id),
        Uuid::new_v4().to_string(),
        ISO8601::now() + Duration::seconds(DATAFIX_VOTER_LOCK_SECS),
    )
    .await
    {
        Ok(lock) => lock,
        Err(err) => {
            info!("Another Datafix voter operation owns the lock: {err}");
            return Ok(());
        }
    };

    let result =
        process_locked_cast_vote(&tenant_id, &election_event_id, &cast_vote_id, &lock).await;
    let release_result = lock.release().await;
    result?;
    release_result.map_err(|err| format!("Error releasing Datafix voter lock: {err}"))?;
    Ok(())
}

/// Runs the Datafix send while the per-voter lock is held: it re-checks for an
/// earlier `indeterminate` vote (leaving this one for the beat if found),
/// validates the event's Datafix configuration, resolves the voter, claims the
/// vote and sends `SetVoted`, transitioning the row to its terminal status.
#[instrument(skip(lock), fields(cast_vote_id = %cast_vote_id), err)]
async fn process_locked_cast_vote(
    tenant_id: &str,
    election_event_id: &str,
    cast_vote_id: &Uuid,
    lock: &PgLock,
) -> Result<()> {
    let Some(pending_cast_vote) =
        load_cast_vote(tenant_id, election_event_id, cast_vote_id).await?
    else {
        return Ok(());
    };
    if has_indeterminate_vote(&pending_cast_vote).await? {
        info!("Another cast vote for this voter requires reconciliation; leaving this vote in-progress");
        return Ok(());
    }

    let cast_vote = pending_cast_vote;
    let voter_id = cast_vote
        .voter_id_string
        .as_deref()
        .ok_or("Voter id not found")?;
    let election_event = load_election_event(&cast_vote).await?;
    datafix_annotations(&election_event)
        .map_err(|err| format!("Invalid Datafix configuration: {err}"))?
        .ok_or("Cast vote is pending but the election event is not configured for Datafix")?;

    let realm = get_event_realm(&cast_vote.tenant_id, &cast_vote.election_event_id);
    let keycloak = KeycloakAdminClient::new()
        .await
        .map_err(|err| format!("Error obtaining Keycloak client: {err:?}"))?;
    let user = keycloak
        .get_user(&realm, voter_id)
        .await
        .map_err(|err| format!("Error fetching voter from Keycloak: {err:?}"))?;
    let username = user.username.clone().ok_or("Username is None")?;
    let attributes = user.attributes.clone().unwrap_or_default();
    lock.update_expiry_for(DATAFIX_VOTER_LOCK_SECS)
        .await
        .map_err(|err| format!("Datafix voter lock was lost after Keycloak lookup: {err}"))?;

    if user.enabled != Some(true) || voted_via_not_internet_channel(&attributes) {
        let changed = transition_cast_vote(
            &cast_vote,
            CastVoteStatus::InProgress,
            CastVoteStatus::Discarded,
        )
        .await?;
        audit_operation(
            &cast_vote,
            voter_id,
            &username,
            if changed {
                "SetVoted Skipped: voter is disabled or marked via another channel".to_string()
            } else {
                "SetVoted skip ignored after concurrent resolution".to_string()
            },
        )
        .await;
        return Ok(());
    }

    let prior_valid_vote = has_prior_valid_vote(&cast_vote, voter_id).await?;
    if voted_via_internet(&attributes) || prior_valid_vote {
        let changed = transition_cast_vote(
            &cast_vote,
            CastVoteStatus::InProgress,
            CastVoteStatus::Valid,
        )
        .await?;
        if changed && !voted_via_internet(&attributes) {
            mark_voted_via_internet(&realm, voter_id).await?;
        }
        return Ok(());
    }

    let prepared = datafix::voterview_requests::prepare(
        SoapRequest::SetVoted,
        ElectionEventDatafix(election_event),
        &Some(username.clone()),
    )
    .await
    .map_err(|err| format!("Unable to prepare SetVoted before dispatch: {err}"))?;

    lock.update_expiry_for(DATAFIX_VOTER_LOCK_SECS)
        .await
        .map_err(|err| format!("Datafix voter lock was lost before SetVoted: {err}"))?;

    let Some(_) = claim_cast_vote(tenant_id, election_event_id, cast_vote_id).await? else {
        info!("Cast vote was claimed or resolved by another worker; skipping");
        return Ok(());
    };
    let template_sha256 = prepared.template_sha256().to_string();
    let result = datafix::voterview_requests::send_prepared(prepared).await;
    match &result {
        Ok(result) => info!(
            template_sha256 = %result.template_sha256,
            response = result.response.classification(),
            "Datafix API response received"
        ),
        Err(err) => info!(
            template_sha256 = %template_sha256,
            "Datafix API request failed: {err}"
        ),
    }

    match result {
        Ok(result) => {
            let operation = match result.response {
                SoapRequestResponse::Ok => {
                    let changed = transition_cast_vote(
                        &cast_vote,
                        CastVoteStatus::Indeterminate,
                        CastVoteStatus::Valid,
                    )
                    .await?;
                    if changed {
                        if let Err(err) = mark_voted_via_internet(&realm, voter_id).await {
                            error!("Could not mark the voter Internet channel: {err}");
                        }
                        format!(
                            "SetVoted Succeeded (template_sha256={})",
                            result.template_sha256
                        )
                    } else {
                        format!(
                            "SetVoted result ignored after concurrent resolution (template_sha256={})",
                            result.template_sha256
                        )
                    }
                }
                SoapRequestResponse::AlreadyVoted => {
                    let changed = transition_cast_vote(
                        &cast_vote,
                        CastVoteStatus::Indeterminate,
                        CastVoteStatus::Discarded,
                    )
                    .await?;
                    if changed {
                        format!(
                            "SetVoted Failed: voter already voted (template_sha256={})",
                            result.template_sha256
                        )
                    } else {
                        format!(
                            "SetVoted already-voted result ignored after concurrent resolution (template_sha256={})",
                            result.template_sha256
                        )
                    }
                }
                SoapRequestResponse::Rejected(_) => {
                    let changed = transition_cast_vote(
                        &cast_vote,
                        CastVoteStatus::Indeterminate,
                        CastVoteStatus::Discarded,
                    )
                    .await?;
                    if changed {
                        format!(
                            "SetVoted Rejected (template_sha256={})",
                            result.template_sha256
                        )
                    } else {
                        format!(
                            "SetVoted rejection ignored after concurrent resolution (template_sha256={})",
                            result.template_sha256
                        )
                    }
                }
                response @ (SoapRequestResponse::AlreadyNotVoted
                | SoapRequestResponse::Fault(_)) => {
                    format!(
                        "SetVoted Indeterminate: {} (template_sha256={})",
                        response.classification(),
                        result.template_sha256
                    )
                }
            };
            audit_operation(&cast_vote, voter_id, &username, operation).await;
        }
        Err(SoapSendError::NotDispatched(err)) => {
            let changed = transition_cast_vote(
                &cast_vote,
                CastVoteStatus::Indeterminate,
                CastVoteStatus::InProgress,
            )
            .await?;
            let operation = if changed {
                format!(
                    "SetVoted NotDispatched: connection-error, vote requeued (template_sha256={template_sha256})"
                )
            } else {
                format!(
                    "SetVoted connection-error ignored after concurrent resolution (template_sha256={template_sha256})"
                )
            };
            audit_operation(&cast_vote, voter_id, &username, operation).await;
            return Err(format!(
                "VoterView SetVoted was not dispatched; the vote was requeued: {err}"
            )
            .into());
        }
        Err(SoapSendError::Ambiguous(err)) => {
            let operation = format!(
                "SetVoted Indeterminate: transport-or-response-error (template_sha256={template_sha256})"
            );
            audit_operation(&cast_vote, voter_id, &username, operation).await;
            return Err(format!("VoterView SetVoted outcome is indeterminate: {err}").into());
        }
    }

    Ok(())
}

/// Returns whether the voter has any other `indeterminate` ballot in the event,
/// so processing of this vote is deferred until the earlier one is reconciled.
#[instrument(skip(cast_vote), fields(cast_vote_id = %cast_vote.id), err)]
async fn has_indeterminate_vote(cast_vote: &CastVote) -> Result<bool> {
    let voter_id = cast_vote
        .voter_id_string
        .as_deref()
        .ok_or("Voter id not found")?;
    let tenant_id = parse_uuid_v4(&cast_vote.tenant_id)?;
    let election_event_id = parse_uuid_v4(&cast_vote.election_event_id)?;
    let mut client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|err| format!("Error getting Hasura DB client: {err:?}"))?;
    let transaction = client
        .transaction()
        .await
        .map_err(|err| format!("Error starting Hasura transaction: {err:?}"))?;
    let state = get_voter_cast_vote_state(&transaction, &tenant_id, &election_event_id, voter_id)
        .await
        .map_err(|err| format!("Error checking unresolved cast votes: {err:?}"))?;
    Ok(state.has_indeterminate_vote)
}

/// Loads the cast vote by id in its own short transaction, or `None` if it no
/// longer exists.
#[instrument(fields(cast_vote_id = %cast_vote_id), err)]
async fn load_cast_vote(
    tenant_id: &str,
    election_event_id: &str,
    cast_vote_id: &Uuid,
) -> Result<Option<CastVote>> {
    let mut client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|err| format!("Error getting Hasura DB client: {err:?}"))?;
    let transaction = client
        .transaction()
        .await
        .map_err(|err| format!("Error starting Hasura transaction: {err:?}"))?;
    get_cast_vote_by_id(&transaction, tenant_id, election_event_id, cast_vote_id)
        .await
        .map_err(|err| format!("Error loading cast vote: {err:?}").into())
}

/// Atomically claims an `in-progress` vote by moving it to `indeterminate`,
/// returning the claimed row only if this worker won the compare-and-set. `None`
/// means the row is gone or was already claimed/advanced by another worker, so
/// the caller must not process it.
#[instrument(fields(cast_vote_id = %cast_vote_id), err)]
async fn claim_cast_vote(
    tenant_id: &str,
    election_event_id: &str,
    cast_vote_id: &Uuid,
) -> Result<Option<CastVote>> {
    let mut client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|err| format!("Error getting Hasura DB client: {err:?}"))?;
    let transaction = client
        .transaction()
        .await
        .map_err(|err| format!("Error starting Hasura transaction: {err:?}"))?;
    let Some(cast_vote) =
        get_cast_vote_by_id(&transaction, tenant_id, election_event_id, cast_vote_id)
            .await
            .map_err(|err| format!("Error loading cast vote: {err:?}"))?
    else {
        return Ok(None);
    };
    if cast_vote.status != CastVoteStatus::InProgress {
        return Ok(None);
    }
    let claimed = compare_and_set_cast_vote_status(
        &transaction,
        tenant_id,
        election_event_id,
        cast_vote_id,
        CastVoteStatus::InProgress,
        CastVoteStatus::Indeterminate,
    )
    .await
    .map_err(|err| format!("Error claiming cast vote: {err:?}"))?;
    transaction
        .commit()
        .await
        .map_err(|err| format!("Error committing cast vote claim: {err:?}"))?;
    Ok(claimed.then_some(cast_vote))
}

/// Loads the election event that owns the cast vote, needed for its Datafix
/// configuration and realm.
#[instrument(skip(cast_vote), fields(election_event_id = %cast_vote.election_event_id), err)]
async fn load_election_event(cast_vote: &CastVote) -> Result<ElectionEvent> {
    let mut client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|err| format!("Error getting Hasura DB client: {err:?}"))?;
    let transaction = client
        .transaction()
        .await
        .map_err(|err| format!("Error starting Hasura transaction: {err:?}"))?;
    get_election_event_by_id(
        &transaction,
        &cast_vote.tenant_id,
        &cast_vote.election_event_id,
    )
    .await
    .map_err(|err| format!("Error loading election event: {err:?}").into())
}

/// Returns whether the voter already has a `valid` vote for the event; a prior
/// valid vote means this ballot must not be counted a second time.
#[instrument(skip(cast_vote), fields(cast_vote_id = %cast_vote.id), err)]
async fn has_prior_valid_vote(cast_vote: &CastVote, voter_id: &str) -> Result<bool> {
    let mut client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|err| format!("Error getting Hasura DB client: {err:?}"))?;
    let transaction = client
        .transaction()
        .await
        .map_err(|err| format!("Error starting Hasura transaction: {err:?}"))?;
    has_valid_cast_vote(
        &transaction,
        &cast_vote.tenant_id,
        &cast_vote.election_event_id,
        voter_id,
    )
    .await
    .map_err(|err| format!("Error checking prior valid votes: {err:?}").into())
}

/// Compare-and-sets the vote from `expected` to `next` in its own transaction,
/// returning whether the row moved. A `false` result is logged (not an error):
/// it means another worker already advanced the row past `expected`.
#[instrument(skip(cast_vote), fields(cast_vote_id = %cast_vote.id), err)]
async fn transition_cast_vote(
    cast_vote: &CastVote,
    expected: CastVoteStatus,
    next: CastVoteStatus,
) -> Result<bool> {
    let cast_vote_id = Uuid::parse_str(&cast_vote.id)
        .map_err(|err| format!("Invalid cast_vote_id in stored row: {err}"))?;
    let mut client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|err| format!("Error getting Hasura DB client: {err:?}"))?;
    let transaction = client
        .transaction()
        .await
        .map_err(|err| format!("Error starting Hasura transaction: {err:?}"))?;
    let changed = compare_and_set_cast_vote_status(
        &transaction,
        &cast_vote.tenant_id,
        &cast_vote.election_event_id,
        &cast_vote_id,
        expected,
        next,
    )
    .await
    .map_err(|err| format!("Error transitioning cast vote status: {err:?}"))?;
    transaction
        .commit()
        .await
        .map_err(|err| format!("Error committing cast vote status: {err:?}"))?;
    if !changed {
        warn!("Cast vote status changed concurrently; terminal status was not overwritten");
    }
    Ok(changed)
}

/// Records the outcome of an outbound Datafix operation in the electoral log.
/// Failures are logged and swallowed so auditing never fails the vote itself.
#[instrument(skip(cast_vote), fields(cast_vote_id = %cast_vote.id))]
async fn audit_operation(cast_vote: &CastVote, voter_id: &str, username: &str, operation: String) {
    let operation = format!("cast_vote_id={}; {operation}", cast_vote.id);
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
        &cast_vote.tenant_id,
        &cast_vote.election_event_id,
        Some(voter_id),
        username,
        ExtApiRequestDirection::Outbound,
        operation,
    )
    .await
    {
        error!("Unable to record the Datafix audit entry: {err}");
    }
}

#[instrument(err)]
async fn mark_voted_via_internet(realm: &str, voter_id: &str) -> Result<()> {
    let mut attributes = HashMap::new();
    attributes.insert(
        VOTED_CHANNEL.to_string(),
        vec![VOTED_CHANNEL_INTERNET_VALUE.to_string()],
    );
    retry_with_exponential_backoff(
        || async {
            let client = KeycloakAdminClient::new()
                .await
                .map_err(|err| format!("Error obtaining Keycloak client: {err:?}"))?;
            client
                .edit_user(
                    realm,
                    voter_id,
                    None,
                    Some(attributes.clone()),
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                )
                .await
                .map_err(|err| format!("Error editing voter Internet channel: {err:?}"))
        },
        3,
        StdDuration::from_millis(500),
    )
    .await
    .map(|_| ())
    .map_err(|err| format!("Error editing voter Internet channel after retries: {err}").into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn voter_lock_is_event_wide() {
        let first = datafix_voter_lock_key("tenant", "event", "voter");
        let second = datafix_voter_lock_key("tenant", "event", "voter");
        assert_eq!(first, second);
        assert_ne!(
            first,
            datafix_voter_lock_key("tenant", "other-event", "voter")
        );
        assert_ne!(
            first,
            datafix_voter_lock_key("tenant", "event", "other-voter")
        );
    }
}
