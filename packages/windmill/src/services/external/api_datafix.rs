// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::datafix_types::*;
use super::utils::*;

use crate::postgres::cast_vote::{get_voter_cast_vote_state, VoterCastVoteState};
use crate::services::database::get_hasura_pool;
use crate::services::pg_lock::PgLock;
use crate::services::users::{list_users, FilterOption, ListUsersFilter};
use anyhow::Result;
use chrono::Duration;
use deadpool_postgres::{Client as DbClient, Transaction};
use electoral_log::messages::newtypes::ExtApiRequestDirection;
use keycloak::KeycloakError;
use rocket::serde::json::Json;
use sequent_core::services::connection::DatafixClaims;
use sequent_core::services::date::ISO8601;
use sequent_core::services::keycloak::{get_event_realm, KeycloakAdminClient};
use sequent_core::services::uuid_validation::parse_uuid_v4;
use sequent_core::types::keycloak::{
    User, AREA_ID_ATTR_NAME, ATTR_RESET_VALUE, DATE_OF_BIRTH, DISABLE_COMMENT,
    DISABLE_REASON_DELETE_CALL, DISABLE_REASON_MARKVOTED_CALL, TENANT_ID_ATTR_NAME, VOTED_CHANNEL,
    VOTED_CHANNEL_INTERNET_VALUE,
};
use sequent_core::util::date_time::verify_date_format_ymd;
use std::collections::HashMap;
use std::env;
use tracing::{error, instrument, warn};
use uuid::Uuid;
/// Disable the voter, datafix users are not actually deleted but just disabled.
/// Note: voter_id in Datafix API represents the username in Keycloak/Sequent´s system.
#[instrument(skip(_hasura_transaction, keycloak_transaction))]
pub async fn disable_datafix_voter(
    _hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    tenant_id: &str,
    datafix_event_id: &str,
    username: &str,
    realm: &str,
) -> Result<Json<DatafixResponse>, JsonErrorResponse> {
    let client = KeycloakAdminClient::new().await.map_err(|e| {
        error!("Error getting KeycloakAdminClient: {e:?}");
        DatafixResponse::error(DatafixErrorCode::InternalError)
    })?;

    let user_id = get_user_id(keycloak_transaction, realm, username).await?;
    let mut hash_map = HashMap::new();
    hash_map.insert(
        DISABLE_COMMENT.to_string(),
        vec![DISABLE_REASON_DELETE_CALL.to_string()],
    );
    let attributes = Some(hash_map);

    let _user = client
        .edit_user(
            realm,
            &user_id,
            Some(false),
            attributes,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .map_err(|e| {
            error!("Error editing user: {e:?}");
            DatafixResponse::error(DatafixErrorCode::InternalError)
        })?;
    Ok(DatafixResponse::ok())
}

/// Note: voter_id in Datafix API represents the username in Keycloak/Sequent´s system.
#[instrument(skip(hasura_transaction))]
pub async fn add_datafix_voter(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    datafix_event_id: &str,
    voter_info: &VoterInformationBody,
    election_event_id: &str,
    realm: &str,
) -> Result<Json<DatafixResponse>, JsonErrorResponse> {
    let username = &voter_info.voter_id;
    let client = KeycloakAdminClient::new().await.map_err(|e| {
        error!("Error getting KeycloakAdminClient: {e:?}");
        DatafixResponse::error(DatafixErrorCode::InternalError)
    })?;

    let area = find_user_area_by_name(hasura_transaction, tenant_id, election_event_id, voter_info)
        .await?;

    // Both area and birthdate have to go into the attributes HashMap. They will be taken from there but not from the User struct.
    let mut hash_map = HashMap::new();
    hash_map.insert(
        AREA_ID_ATTR_NAME.to_string(),
        vec![area.id.clone().unwrap_or_default()],
    );
    hash_map.insert(TENANT_ID_ATTR_NAME.to_string(), vec![tenant_id.to_string()]);
    // Area is required in the input body but the birthdate is not.
    if let Some(birthdate) = voter_info.birthdate.clone() {
        verify_date_format_ymd(&birthdate).map_err(|e| {
            error!("Birthdate format is not correct: {e:?}");
            DatafixResponse::error(DatafixErrorCode::InvalidRequest)
        })?;
        hash_map.insert(DATE_OF_BIRTH.to_string(), vec![birthdate]);
    }
    let attributes = Some(hash_map);
    let user = User {
        attributes: attributes.clone(),
        enabled: Some(true),
        username: Some(username.to_string()),
        area: Some(area),
        ..User::default()
    };
    let voter_group_name = env::var("KEYCLOAK_VOTER_GROUP_NAME").map_err(|e| {
        error!("Error getting env var KEYCLOAK_VOTER_GROUP_NAME: {e:?}");
        DatafixResponse::error(DatafixErrorCode::InternalError)
    })?;
    let _user = client
        .create_user(realm, &user, attributes, Some(vec![voter_group_name]))
        .await
        .map_err(|e| {
            error!("Error creating user: {e:?}");
            create_user_error_response(&e)
        })?;
    Ok(DatafixResponse::ok())
}

/// Maps a failed Keycloak user creation to the Datafix API error contract: a
/// 409 from Keycloak means the username is already taken, so the caller gets
/// `voter-already-exists`; anything else stays an internal error.
#[instrument(skip_all)]
fn create_user_error_response(e: &anyhow::Error) -> JsonErrorResponse {
    match e.downcast_ref::<KeycloakError>() {
        Some(KeycloakError::HttpFailure { status: 409, .. }) => {
            DatafixResponse::error(DatafixErrorCode::VoterAlreadyExists)
        }
        _ => DatafixResponse::error(DatafixErrorCode::InternalError),
    }
}

/// There are 2 things that can be updated, the area and the birthdate.
/// Note: voter_id in Datafix API represents the username in Keycloak/Sequent´s system.
#[instrument(skip(hasura_transaction, keycloak_transaction))]
pub async fn update_datafix_voter(
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    tenant_id: &str,
    datafix_event_id: &str,
    voter_info: &VoterInformationBody,
    election_event_id: &str,
    realm: &str,
) -> Result<Json<DatafixResponse>, JsonErrorResponse> {
    let username = voter_info.voter_id.clone();
    let client = KeycloakAdminClient::new().await.map_err(|e| {
        error!("Error getting KeycloakAdminClient: {e:?}");
        DatafixResponse::error(DatafixErrorCode::InternalError)
    })?;

    let area = find_user_area_by_name(hasura_transaction, tenant_id, election_event_id, voter_info)
        .await?;
    // Both area and birthdate have to go into the attributes HashMap. They will be taken from there but not from the User struct.
    let mut hash_map = HashMap::new();
    hash_map.insert(
        AREA_ID_ATTR_NAME.to_string(),
        vec![area.id.unwrap_or_default()],
    );
    // Area is required in the input body but birthdate is not.
    if let Some(birthdate) = voter_info.birthdate.clone() {
        verify_date_format_ymd(&birthdate).map_err(|e| {
            error!("Birthdate format is not correct: {e:?}");
            DatafixResponse::error(DatafixErrorCode::InvalidRequest)
        })?;
        hash_map.insert(DATE_OF_BIRTH.to_string(), vec![birthdate]);
    }
    let attributes = Some(hash_map);

    let user_id = get_user_id(keycloak_transaction, realm, &username).await?;
    let _user = client
        .edit_user(
            realm,
            &user_id,
            voter_info.enabled,
            attributes,
            None,
            None,
            None,
            Some(username),
            None,
            None,
        )
        .await
        .map_err(|e| {
            error!("Error editing user: {e:?}");
            DatafixResponse::error(DatafixErrorCode::InternalError)
        })?;
    Ok(DatafixResponse::ok())
}

/// Mark a voter as having voted via a given channel
/// Also disables the voter so it cannot vote online
#[instrument(skip(_hasura_transaction, keycloak_transaction))]
pub async fn mark_as_voted_via_channel(
    _hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    tenant_id: &str,
    datafix_event_id: &str,
    voter_body: &MarkVotedBody,
    realm: &str,
) -> Result<Json<DatafixResponse>, JsonErrorResponse> {
    let username = voter_body.voter_id.clone();
    let client = KeycloakAdminClient::new().await.map_err(|e| {
        error!("Error getting KeycloakAdminClient: {e:?}");
        DatafixResponse::error(DatafixErrorCode::InternalError)
    })?;

    let mut hash_map = HashMap::new();
    hash_map.insert(VOTED_CHANNEL.to_string(), vec![voter_body.channel.clone()]);
    hash_map.insert(
        DISABLE_COMMENT.to_string(),
        vec![DISABLE_REASON_MARKVOTED_CALL.to_string()],
    );
    let attributes = Some(hash_map);

    let user_id = get_user_id(keycloak_transaction, realm, &username).await?;
    let _user = client
        .edit_user(
            realm,
            &user_id,
            Some(false), // Disable the voter
            attributes,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .map_err(|e| {
            error!("Error editing user: {e:?}");
            DatafixResponse::error(DatafixErrorCode::InternalError)
        })?;
    Ok(DatafixResponse::ok())
}

/// Unmark a voter as having voted. Re-enable only when MarkVoted was the
/// operation that disabled the account; an unrelated administrator disable
/// and its reason must survive this call.
#[instrument(skip(_hasura_transaction, keycloak_transaction))]
pub async fn unmark_voter_as_voted(
    _hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    tenant_id: &str,
    datafix_event_id: &str,
    voter_id: &str,
    realm: &str,
) -> Result<Json<DatafixResponse>, JsonErrorResponse> {
    let username = voter_id.to_string();
    let client = KeycloakAdminClient::new().await.map_err(|e| {
        error!("Error getting KeycloakAdminClient: {e:?}");
        DatafixResponse::error(DatafixErrorCode::InternalError)
    })?;

    let user_id = get_user_id(keycloak_transaction, realm, &username).await?;
    let current_user = client.get_user(realm, &user_id).await.map_err(|e| {
        error!("Error loading user before unmarking voted state: {e:?}");
        DatafixResponse::error(DatafixErrorCode::InternalError)
    })?;
    let (enabled, attributes) = plan_unmark_voter_edit(&current_user);
    let _user = client
        .edit_user(
            realm,
            &user_id,
            enabled,
            Some(attributes),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .map_err(|e| {
            error!("Error editing user: {e:?}");
            DatafixResponse::error(DatafixErrorCode::InternalError)
        })?;
    Ok(DatafixResponse::ok())
}

/// Pure state transition shared by the inbound operation's tests and kept in
/// lockstep with reconciliation's VOTED_UNMARKED planning. `voted-channel`
/// is always reset. `disable-comment` and `enabled` are owned by this
/// operation only when the current reason is MARKVOTED_CALL (or the account
/// is already enabled and merely carries a stale reason).
fn plan_unmark_voter_edit(user: &User) -> (Option<bool>, HashMap<String, Vec<String>>) {
    let enabled = user.enabled.unwrap_or(false);
    let attributes = user.attributes.as_ref();
    let disable_comment = attributes
        .and_then(|values| values.get(DISABLE_COMMENT))
        .and_then(|values| values.last())
        .map(String::as_str)
        .unwrap_or(ATTR_RESET_VALUE);
    let disabled_by_mark_voted = !enabled && disable_comment == DISABLE_REASON_MARKVOTED_CALL;

    let mut changes = HashMap::from([(
        VOTED_CHANNEL.to_string(),
        vec![ATTR_RESET_VALUE.to_string()],
    )]);
    if enabled || disabled_by_mark_voted {
        changes.insert(
            DISABLE_COMMENT.to_string(),
            vec![ATTR_RESET_VALUE.to_string()],
        );
    }

    (disabled_by_mark_voted.then_some(true), changes)
}

/// Generate a new password.
#[expect(
    clippy::too_many_arguments,
    reason = "Preserve the PIN replacement API with its separate transaction, voter, election and annotation inputs."
)]
#[instrument(skip(hasura_transaction, keycloak_transaction, datafix_annotations))]
pub async fn replace_voter_pin(
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    tenant_id: &str,
    datafix_event_id: &str,
    username: &str,
    election_event_id: &str,
    realm: &str,
    datafix_annotations: &DatafixAnnotations,
) -> Result<String, JsonErrorResponse> {
    let filter = ListUsersFilter {
        tenant_id: tenant_id.to_string(),
        election_event_id: Some(election_event_id.to_string()),
        realm: realm.to_string(),
        username: Some(FilterOption::IsEqual(username.to_string())),
        ..ListUsersFilter::default()
    };

    // If a voter is disabled, do not generate a PIN
    let user_id = match list_users(hasura_transaction, keycloak_transaction, filter).await {
        Ok((users, 1)) => {
            let user = users
                .last()
                .map(|val_ref| val_ref.to_owned())
                .unwrap_or_default();
            if !user.enabled.unwrap_or(true) {
                warn!("Cannot replace pin because the user is disabled.");
                return Err(DatafixResponse::error(DatafixErrorCode::InvalidRequest));
            }
            user.id.unwrap_or_default()
        }
        Ok((_, 0)) => {
            warn!("Error getting users by username: Not Found");
            return Err(DatafixResponse::error(DatafixErrorCode::VoterNotFound));
        }
        Ok(_) => {
            warn!("Error getting users by username: Must be only one user per username");
            return Err(DatafixResponse::error(DatafixErrorCode::InternalError));
        }
        Err(e) => {
            error!("Error looking up user: {e:?}");
            return Err(DatafixResponse::error(DatafixErrorCode::InternalError));
        }
    };

    let pin = datafix_annotations
        .password_policy
        .generate_password(username);
    let password = Some(pin.clone());

    let client = KeycloakAdminClient::new().await.map_err(|e| {
        error!("Error getting KeycloakAdminClient: {e:?}");
        DatafixResponse::error(DatafixErrorCode::InternalError)
    })?;

    let _user = client
        .edit_user(
            realm, &user_id, None, // Enable/disable
            None, // attributes
            None, None, None, None, password, None,
        )
        .await
        .map_err(|e| {
            error!("Error editing user: {e:?}");
            DatafixResponse::error(DatafixErrorCode::InternalError)
        })?;

    Ok(pin)
}

/// A held per-voter lock together with the event context resolved to build it.
///
/// Acquiring the lock already resolves the election event, its realm and the
/// Datafix annotations; returning them lets each inbound handler pass the values
/// on to the service layer instead of resolving the same event a second time.
pub struct InboundVoterLock {
    pub lock: PgLock,
    pub election_event_id: String,
    pub realm: String,
    pub datafix_annotations: DatafixAnnotations,
}

/// Acquires the event-wide per-voter advisory lock that serializes every inbound
/// and outbound Datafix operation for one voter, resolving the election event,
/// realm and Datafix annotations as a side effect so the caller need not resolve
/// them again. Returns `Conflict` when another operation already holds the lock.
#[instrument(skip_all)]
pub async fn acquire_inbound_voter_lock(
    keycloak_transaction: &Transaction<'_>,
    claims: &DatafixClaims,
    username: &str,
) -> Result<InboundVoterLock, JsonErrorResponse> {
    let mut hasura_client: DbClient = get_hasura_pool().await.get().await.map_err(|err| {
        error!("Error getting Hasura client for the inbound Datafix lock: {err}");
        DatafixResponse::error(DatafixErrorCode::InternalError)
    })?;
    let hasura_transaction = hasura_client.transaction().await.map_err(|err| {
        error!("Error starting Hasura transaction for the inbound Datafix lock: {err}");
        DatafixResponse::error(DatafixErrorCode::InternalError)
    })?;
    let (election_event_id, datafix_annotations) = get_event_id_and_datafix_annotations(
        &hasura_transaction,
        &claims.tenant_id,
        &claims.datafix_event_id,
    )
    .await?;
    drop(hasura_transaction);
    drop(hasura_client);
    let realm = get_event_realm(&claims.tenant_id, &election_event_id);
    let user_id = get_user_id(keycloak_transaction, &realm, username).await?;
    let user_id_uuid = parse_uuid_v4(&user_id)
        .map_err(|_| DatafixResponse::error(DatafixErrorCode::InternalError))?;
    let lock_key = external_voter_lock_key(&claims.tenant_id, &election_event_id, &user_id_uuid);
    let lock = PgLock::acquire(
        lock_key,
        Uuid::new_v4().to_string(),
        ISO8601::now() + Duration::seconds(DATAFIX_VOTER_LOCK_SECS),
    )
    .await
    .map_err(|err| {
        error!("Another operation is updating this Datafix voter: {err}");
        DatafixResponse::error(DatafixErrorCode::VoterOperationInProgress)
    })?;
    Ok(InboundVoterLock {
        lock,
        election_event_id,
        realm,
        datafix_annotations,
    })
}

/// Releases the per-voter lock, logging (but swallowing) a release failure so a
/// completed operation is never turned into an error by lock cleanup.
#[instrument(skip_all)]
pub async fn release_inbound_voter_lock(lock: PgLock) {
    if let Err(err) = lock.release().await {
        error!("Unable to release the inbound Datafix voter lock: {err}");
    }
}

/// Maps a non-discarded vote state to the inbound API error contract.
fn active_vote_error(state: &VoterCastVoteState) -> Option<DatafixErrorCode> {
    if state.has_unresolved_vote {
        Some(DatafixErrorCode::VoterStateUnresolved)
    } else if state.has_valid_vote {
        Some(DatafixErrorCode::VoterVotedOnline)
    } else {
        None
    }
}

/// Rejects the inbound operation while the voter has any non-discarded online
/// vote. An in-progress vote is still being reconciled and a valid vote is
/// immutable through the inbound API. Callers hold the per-voter lock, so the
/// check cannot race a vote being promoted to `valid`.
#[instrument(skip_all)]
pub async fn ensure_voter_has_no_active_vote(
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    claims: &DatafixClaims,
    username: &str,
) -> Result<(), JsonErrorResponse> {
    let (election_event_id, _) = get_event_id_and_datafix_annotations(
        hasura_transaction,
        &claims.tenant_id,
        &claims.datafix_event_id,
    )
    .await?;
    let realm = get_event_realm(&claims.tenant_id, &election_event_id);
    let user_id = get_user_id(keycloak_transaction, &realm, username).await?;
    let tenant_id = parse_uuid_v4(&claims.tenant_id)
        .map_err(|_| DatafixResponse::error(DatafixErrorCode::InternalError))?;
    let election_event_uuid = parse_uuid_v4(&election_event_id)
        .map_err(|_| DatafixResponse::error(DatafixErrorCode::InternalError))?;
    let state = get_voter_cast_vote_state(
        hasura_transaction,
        &tenant_id,
        &election_event_uuid,
        &user_id,
    )
    .await
    .map_err(|err| {
        error!(
            "Error checking for an active online vote before an inbound Datafix operation: {err}"
        );
        DatafixResponse::error(DatafixErrorCode::InternalError)
    })?;
    if let Some(error_code) = active_vote_error(&state) {
        return Err(DatafixResponse::error(error_code));
    }
    Ok(())
}

/// Refuses to re-enable a Datafix voter whose voting state is still unresolved —
/// an in-progress or valid vote, or any recorded voted channel — returning
/// `Conflict` in that case.
#[instrument(skip_all)]
pub async fn ensure_inbound_reenable_is_safe(
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    claims: &DatafixClaims,
    username: &str,
) -> Result<(), JsonErrorResponse> {
    let (election_event_id, _) = get_event_id_and_datafix_annotations(
        hasura_transaction,
        &claims.tenant_id,
        &claims.datafix_event_id,
    )
    .await?;
    let realm = get_event_realm(&claims.tenant_id, &election_event_id);
    let user_id = get_user_id(keycloak_transaction, &realm, username).await?;
    let tenant_id = parse_uuid_v4(&claims.tenant_id)
        .map_err(|_| DatafixResponse::error(DatafixErrorCode::InternalError))?;
    let election_event_uuid = parse_uuid_v4(&election_event_id)
        .map_err(|_| DatafixResponse::error(DatafixErrorCode::InternalError))?;
    let state = get_voter_cast_vote_state(
        hasura_transaction,
        &tenant_id,
        &election_event_uuid,
        &user_id,
    )
    .await
    .map_err(|err| {
        error!("Error checking unresolved votes before enabling a Datafix voter: {err}");
        DatafixResponse::error(DatafixErrorCode::InternalError)
    })?;
    let client = KeycloakAdminClient::new().await.map_err(|err| {
        error!("Error creating a Keycloak client before enabling a Datafix voter: {err}");
        DatafixResponse::error(DatafixErrorCode::InternalError)
    })?;
    let user = client.get_user(&realm, &user_id).await.map_err(|err| {
        error!("Error loading a Datafix voter before enabling it: {err}");
        DatafixResponse::error(DatafixErrorCode::InternalError)
    })?;
    let attributes = user.attributes.unwrap_or_default();
    if state.has_unresolved_vote
        || state.has_valid_vote
        || voted_via_internet(&attributes)
        || voted_via_not_internet_channel(&attributes)
    {
        return Err(DatafixResponse::error(
            DatafixErrorCode::VoterStateUnresolved,
        ));
    }
    Ok(())
}

/// Records the outcome of an inbound Datafix operation in the electoral log,
/// resolving the voter id from Keycloak when a transaction is supplied. Failures
/// are logged and swallowed so auditing never fails the operation itself.
#[instrument(skip_all)]
pub async fn audit_inbound_operation(
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: Option<&Transaction<'_>>,
    claims: &DatafixClaims,
    username: &str,
    operation_name: &str,
    succeeded: bool,
) {
    let election_event_id = match get_event_id_and_datafix_annotations(
        hasura_transaction,
        &claims.tenant_id,
        &claims.datafix_event_id,
    )
    .await
    {
        Ok((election_event_id, _)) => election_event_id,
        Err(err) => {
            error!(
                "Unable to resolve the election event for the inbound Datafix audit entry: {err:?}"
            );
            return;
        }
    };
    let user_id = if let Some(transaction) = keycloak_transaction {
        let realm = get_event_realm(&claims.tenant_id, &election_event_id);
        get_user_id(transaction, &realm, username).await.ok()
    } else {
        None
    };
    let outcome = if succeeded { "Succeeded" } else { "Failed" };

    if let Err(err) = post_operation_result_to_electoral_log(
        hasura_transaction,
        &claims.tenant_id,
        &election_event_id,
        user_id.as_deref(),
        username,
        ExtApiRequestDirection::Inbound,
        format!("{operation_name} {outcome}"),
    )
    .await
    {
        error!("Unable to record the inbound Datafix {operation_name} audit entry: {err}");
    }
}

/// Audits an inbound operation on a fresh short-lived transaction, for the paths
/// where the request transaction has already been committed or dropped.
#[instrument(skip_all)]
pub async fn audit_inbound_operation_standalone(
    claims: &DatafixClaims,
    username: &str,
    operation_name: &str,
    succeeded: bool,
) {
    let mut client: DbClient = match get_hasura_pool().await.get().await {
        Ok(client) => client,
        Err(err) => {
            error!("Unable to get a Hasura client for the inbound Datafix audit: {err}");
            return;
        }
    };
    let transaction = match client.transaction().await {
        Ok(transaction) => transaction,
        Err(err) => {
            error!("Unable to start a transaction for the inbound Datafix audit: {err}");
            return;
        }
    };
    audit_inbound_operation(
        &transaction,
        None,
        claims,
        username,
        operation_name,
        succeeded,
    )
    .await;
}

/// Whether an inbound `MarkVoted` channel is a real external channel — rejects a
/// blank value, the reset sentinel, and the Internet channel (owned by the
/// online voting path, not by an inbound mark).
#[instrument]
pub fn valid_inbound_voting_channel(channel: &str) -> bool {
    let channel = channel.trim();
    !channel.is_empty()
        && !channel.eq_ignore_ascii_case(ATTR_RESET_VALUE)
        && !channel.eq_ignore_ascii_case(VOTED_CHANNEL_INTERNET_VALUE)
}

#[cfg(test)]
mod tests {
    use super::{
        active_vote_error, create_user_error_response, plan_unmark_voter_edit,
        valid_inbound_voting_channel,
    };
    use crate::postgres::cast_vote::VoterCastVoteState;
    use crate::services::external::datafix_types::DatafixErrorCode;
    use keycloak::KeycloakError;
    use rocket::http::Status;
    use sequent_core::types::keycloak::{
        User, ATTR_RESET_VALUE, DISABLE_COMMENT, DISABLE_REASON_MARKVOTED_CALL, VOTED_CHANNEL,
    };
    use std::collections::HashMap;

    /// Builds the error `create_user` returns when Keycloak answers with the
    /// given HTTP status.
    fn keycloak_http_failure(status: u16) -> anyhow::Error {
        let err = KeycloakError::HttpFailure {
            status,
            body: None,
            text: String::new(),
        };
        let message = format!("Failed to create user in keycloak: {err:?}");
        anyhow::Error::new(err).context(message)
    }

    #[test]
    fn create_user_conflict_maps_to_voter_already_exists() {
        let response = create_user_error_response(&keycloak_http_failure(409));
        assert_eq!(response.0, Status::Conflict);
        assert_eq!(
            response.1.error_code,
            Some(DatafixErrorCode::VoterAlreadyExists)
        );
    }

    #[test]
    fn other_create_user_failures_stay_internal_errors() {
        let response = create_user_error_response(&keycloak_http_failure(504));
        assert_eq!(response.0, Status::InternalServerError);
        assert_eq!(response.1.error_code, Some(DatafixErrorCode::InternalError));

        let stringified = anyhow::anyhow!("Failed to create user in keycloak");
        let response = create_user_error_response(&stringified);
        assert_eq!(response.0, Status::InternalServerError);
        assert_eq!(response.1.error_code, Some(DatafixErrorCode::InternalError));
    }

    #[test]
    fn inbound_voting_channel_rejects_reserved_values() {
        for channel in [
            "",
            " ",
            "NONE",
            "none",
            "Internet",
            "INTERNET",
            " Internet ",
        ] {
            assert!(!valid_inbound_voting_channel(channel));
        }
        assert!(valid_inbound_voting_channel("PHONE"));
        assert!(valid_inbound_voting_channel("Paper"));
    }

    #[test]
    fn active_vote_guard_distinguishes_in_progress_and_valid_votes() {
        assert_eq!(
            active_vote_error(&VoterCastVoteState {
                has_unresolved_vote: true,
                has_valid_vote: false,
            }),
            Some(DatafixErrorCode::VoterStateUnresolved)
        );
        assert_eq!(
            active_vote_error(&VoterCastVoteState {
                has_unresolved_vote: false,
                has_valid_vote: true,
            }),
            Some(DatafixErrorCode::VoterVotedOnline)
        );
        assert_eq!(
            active_vote_error(&VoterCastVoteState {
                has_unresolved_vote: false,
                has_valid_vote: false,
            }),
            None
        );
    }

    #[test]
    fn unmark_only_reenables_accounts_disabled_by_mark_voted() {
        let mark_voted_user = User {
            enabled: Some(false),
            attributes: Some(HashMap::from([(
                DISABLE_COMMENT.to_string(),
                vec![DISABLE_REASON_MARKVOTED_CALL.to_string()],
            )])),
            ..User::default()
        };
        let (enabled, attributes) = plan_unmark_voter_edit(&mark_voted_user);
        assert_eq!(enabled, Some(true));
        assert_eq!(
            attributes.get(DISABLE_COMMENT),
            Some(&vec![ATTR_RESET_VALUE.to_string()])
        );

        let manually_disabled_user = User {
            enabled: Some(false),
            attributes: Some(HashMap::from([(
                DISABLE_COMMENT.to_string(),
                vec!["Disabled manually".to_string()],
            )])),
            ..User::default()
        };
        let (enabled, attributes) = plan_unmark_voter_edit(&manually_disabled_user);
        assert_eq!(enabled, None);
        assert!(!attributes.contains_key(DISABLE_COMMENT));
        assert_eq!(
            attributes.get(VOTED_CHANNEL),
            Some(&vec![ATTR_RESET_VALUE.to_string()])
        );
    }
}
