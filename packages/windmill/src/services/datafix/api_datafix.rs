// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::audit::{
    inbound_operation_log_entry, AppliedInboundOperation, InboundOperation, InboundVoterChanges,
};
use super::types::*;
use super::utils::*;

use crate::postgres::cast_vote::{get_voter_cast_vote_state, VoterCastVoteState};
use crate::services::database::get_hasura_pool;
use crate::services::pg_lock::PgLock;
use crate::services::users::{get_user_area_id, list_users, FilterOption, ListUsersFilter};
use anyhow::Result;
use chrono::Duration;
use deadpool_postgres::{Client as DbClient, Transaction};
use electoral_log::messages::newtypes::ExtApiRequestDirection;
use keycloak::KeycloakError;
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

/// Keycloak admin client for an inbound operation.
#[instrument(skip_all)]
async fn keycloak_admin_client() -> Result<KeycloakAdminClient, DatafixError> {
    KeycloakAdminClient::new().await.map_err(|e| {
        error!("Error getting KeycloakAdminClient: {e:?}");
        DatafixError::internal(format!("Error getting KeycloakAdminClient: {e}"))
    })
}

/// Maps a failed Keycloak user edit to the internal error recorded in the
/// electoral log.
#[instrument(skip_all)]
fn edit_user_error(e: anyhow::Error) -> DatafixError {
    error!("Error editing user: {e:?}");
    DatafixError::internal(format!("Error editing user: {e}"))
}

/// The request's birthdate, validated as `YYYY-MM-DD` when present. Area is
/// required in the input body but the birthdate is not.
#[instrument(skip_all)]
fn validated_birthdate(voter_info: &VoterInformationBody) -> Result<Option<String>, DatafixError> {
    let Some(birthdate) = voter_info.birthdate.clone() else {
        return Ok(None);
    };
    verify_date_format_ymd(&birthdate).map_err(|e| {
        error!("Birthdate format is not correct: {e:?}");
        DatafixError::new(
            DatafixErrorCode::InvalidRequest,
            format!("Birthdate format is not correct: {e}"),
        )
    })?;
    Ok(Some(birthdate))
}

/// The voter's recorded voted channel, `NONE` when the attribute was never
/// set (Keycloak returns the attribute as a list; the last value wins, as in
/// `voted_via_internet`).
#[instrument(skip_all)]
fn recorded_voted_channel(attributes: &HashMap<String, Vec<String>>) -> String {
    attributes
        .get(VOTED_CHANNEL)
        .and_then(|values| values.last())
        .map(String::as_str)
        .unwrap_or(ATTR_RESET_VALUE)
        .to_string()
}

/// Disable the voter, datafix users are not actually deleted but just disabled.
/// Note: voter_id in Datafix API represents the username in Keycloak/Sequent´s system.
#[instrument(skip(hasura_transaction, keycloak_transaction), err)]
pub async fn disable_datafix_voter(
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    tenant_id: &str,
    datafix_event_id: &str,
    username: &str,
    realm: &str,
) -> Result<AppliedInboundOperation, DatafixError> {
    let client = keycloak_admin_client().await?;

    let user_id = get_user_id(keycloak_transaction, realm, username).await?;
    let attributes = HashMap::from([(
        DISABLE_COMMENT.to_string(),
        vec![DISABLE_REASON_DELETE_CALL.to_string()],
    )]);

    let user = client
        .edit_user(
            realm,
            &user_id,
            Some(false),
            Some(attributes),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .map_err(edit_user_error)?;
    Ok(AppliedInboundOperation::from_user(
        &user,
        InboundVoterChanges::VoterDisabled {
            disable_comment: DISABLE_REASON_DELETE_CALL,
        },
    ))
}

/// Note: voter_id in Datafix API represents the username in Keycloak/Sequent´s system.
#[instrument(skip(hasura_transaction), err)]
pub async fn add_datafix_voter(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    datafix_event_id: &str,
    voter_info: &VoterInformationBody,
    election_event_id: &str,
    realm: &str,
) -> Result<AppliedInboundOperation, DatafixError> {
    let username = &voter_info.voter_id;
    let client = keycloak_admin_client().await?;

    let area = find_user_area_by_name(hasura_transaction, tenant_id, election_event_id, voter_info)
        .await?;
    let area_name = area.name.clone().unwrap_or_default();

    // Both area and birthdate have to go into the attributes HashMap. They will be taken from there but not from the User struct.
    let mut hash_map = HashMap::new();
    hash_map.insert(
        AREA_ID_ATTR_NAME.to_string(),
        vec![area.id.clone().unwrap_or_default()],
    );
    hash_map.insert(TENANT_ID_ATTR_NAME.to_string(), vec![tenant_id.to_string()]);
    let birthdate = validated_birthdate(voter_info)?;
    if let Some(birthdate) = &birthdate {
        hash_map.insert(DATE_OF_BIRTH.to_string(), vec![birthdate.clone()]);
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
        DatafixError::internal(format!(
            "Error getting env var KEYCLOAK_VOTER_GROUP_NAME: {e}"
        ))
    })?;
    let user = client
        .create_user(realm, &user, attributes, Some(vec![voter_group_name]))
        .await
        .map_err(|e| {
            error!("Error creating user: {e:?}");
            create_user_error(&e)
        })?;
    Ok(AppliedInboundOperation::from_user(
        &user,
        InboundVoterChanges::VoterAdded {
            area_name,
            birthdate,
        },
    ))
}

/// Maps a failed Keycloak user creation to the Datafix API error contract: a
/// 409 from Keycloak means the username is already taken, so the caller gets
/// `voter-already-exists`; anything else stays an internal error.
#[instrument(skip_all)]
fn create_user_error(e: &anyhow::Error) -> DatafixError {
    match e.downcast_ref::<KeycloakError>() {
        Some(KeycloakError::HttpFailure { status: 409, .. }) => {
            DatafixError::new(DatafixErrorCode::VoterAlreadyExists, "Voter already exists")
        }
        _ => DatafixError::internal(format!("Error creating user: {e}")),
    }
}

/// There are 2 things that can be updated, the area and the birthdate.
/// Note: voter_id in Datafix API represents the username in Keycloak/Sequent´s system.
#[instrument(skip(hasura_transaction, keycloak_transaction), err)]
pub async fn update_datafix_voter(
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    tenant_id: &str,
    datafix_event_id: &str,
    voter_info: &VoterInformationBody,
    election_event_id: &str,
    realm: &str,
) -> Result<AppliedInboundOperation, DatafixError> {
    let username = voter_info.voter_id.clone();
    let client = keycloak_admin_client().await?;

    let area = find_user_area_by_name(hasura_transaction, tenant_id, election_event_id, voter_info)
        .await?;
    let area_name = area.name.clone().unwrap_or_default();
    // Both area and birthdate have to go into the attributes HashMap. They will be taken from there but not from the User struct.
    let mut hash_map = HashMap::new();
    hash_map.insert(
        AREA_ID_ATTR_NAME.to_string(),
        vec![area.id.unwrap_or_default()],
    );
    let birthdate = validated_birthdate(voter_info)?;
    if let Some(birthdate) = &birthdate {
        hash_map.insert(DATE_OF_BIRTH.to_string(), vec![birthdate.clone()]);
    }
    let attributes = Some(hash_map);

    let user_id = get_user_id(keycloak_transaction, realm, &username).await?;
    let user = client
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
        .map_err(edit_user_error)?;
    Ok(AppliedInboundOperation::from_user(
        &user,
        InboundVoterChanges::VoterUpdated {
            area_name,
            birthdate,
            enabled: voter_info.enabled,
        },
    ))
}

/// Mark a voter as having voted via a given channel
/// Also disables the voter so it cannot vote online
#[instrument(skip(hasura_transaction, keycloak_transaction), err)]
pub async fn mark_as_voted_via_channel(
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    tenant_id: &str,
    datafix_event_id: &str,
    voter_body: &MarkVotedBody,
    realm: &str,
) -> Result<AppliedInboundOperation, DatafixError> {
    let username = voter_body.voter_id.clone();
    let client = keycloak_admin_client().await?;

    let attributes = HashMap::from([
        (VOTED_CHANNEL.to_string(), vec![voter_body.channel.clone()]),
        (
            DISABLE_COMMENT.to_string(),
            vec![DISABLE_REASON_MARKVOTED_CALL.to_string()],
        ),
    ]);

    let user_id = get_user_id(keycloak_transaction, realm, &username).await?;
    let user = client
        .edit_user(
            realm,
            &user_id,
            Some(false), // Disable the voter
            Some(attributes),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .map_err(edit_user_error)?;
    Ok(AppliedInboundOperation::from_user(
        &user,
        InboundVoterChanges::VoterMarkedVoted {
            channel: voter_body.channel.clone(),
            disable_comment: DISABLE_REASON_MARKVOTED_CALL,
        },
    ))
}

/// Unmark a voter as having voted. Re-enable only when MarkVoted was the
/// operation that disabled the account; an unrelated administrator disable
/// and its reason must survive this call.
#[instrument(skip(hasura_transaction, keycloak_transaction), err)]
pub async fn unmark_voter_as_voted(
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    tenant_id: &str,
    datafix_event_id: &str,
    voter_id: &str,
    realm: &str,
) -> Result<AppliedInboundOperation, DatafixError> {
    let username = voter_id.to_string();
    let client = keycloak_admin_client().await?;

    let user_id = get_user_id(keycloak_transaction, realm, &username).await?;
    let current_user = client.get_user(realm, &user_id).await.map_err(|e| {
        error!("Error loading user before unmarking voted state: {e:?}");
        DatafixError::internal(format!(
            "Error loading user before unmarking voted state: {e}"
        ))
    })?;
    let previous_channel =
        recorded_voted_channel(current_user.attributes.as_ref().unwrap_or(&HashMap::new()));
    let (enabled, attributes) = plan_unmark_voter_edit(&current_user);
    let disable_comment_reset = attributes.contains_key(DISABLE_COMMENT);
    let user = client
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
        .map_err(edit_user_error)?;
    Ok(AppliedInboundOperation::from_user(
        &user,
        InboundVoterChanges::VoterUnmarkedVoted {
            previous_channel,
            reenabled: enabled == Some(true),
            disable_comment_reset,
        },
    ))
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

/// A freshly generated PIN together with the applied operation recorded in the
/// electoral log; only the latter is logged, never the PIN.
pub struct ReplacedPin {
    pub pin: String,
    pub applied: AppliedInboundOperation,
}

/// Generate a new password.
#[instrument(
    skip(hasura_transaction, keycloak_transaction, datafix_annotations),
    err
)]
pub async fn replace_voter_pin(
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    tenant_id: &str,
    datafix_event_id: &str,
    username: &str,
    election_event_id: &str,
    realm: &str,
    datafix_annotations: &DatafixAnnotations,
) -> Result<ReplacedPin, DatafixError> {
    let filter = ListUsersFilter {
        tenant_id: tenant_id.to_string(),
        election_event_id: Some(election_event_id.to_string()),
        realm: realm.to_string(),
        username: Some(FilterOption::IsEqual(username.to_string())),
        ..ListUsersFilter::default()
    };

    // If a voter is disabled, do not generate a PIN
    let user = match list_users(hasura_transaction, keycloak_transaction, filter).await {
        Ok((users, 1)) => {
            let user = users
                .last()
                .map(|val_ref| val_ref.to_owned())
                .unwrap_or_default();
            if !user.enabled.unwrap_or(true) {
                warn!("Cannot replace pin because the user is disabled.");
                return Err(DatafixError::new(
                    DatafixErrorCode::InvalidRequest,
                    "Cannot replace pin because the user is disabled",
                ));
            }
            user
        }
        Ok((_, 0)) => {
            warn!("Error getting users by username: Not Found");
            return Err(DatafixError::new(
                DatafixErrorCode::VoterNotFound,
                "Voter not found",
            ));
        }
        Ok(_) => {
            warn!("Error getting users by username: Must be only one user per username");
            return Err(DatafixError::internal(
                "Multiple users found for the username",
            ));
        }
        Err(e) => {
            error!("Error looking up user: {e:?}");
            return Err(DatafixError::internal(format!(
                "Error looking up user: {e}"
            )));
        }
    };
    let user_id = user.id.clone().unwrap_or_default();

    let pin = datafix_annotations
        .password_policy
        .generate_password(username);
    let password = Some(pin.clone());

    // edit_user defaults a missing `temporary` to `true`; Datafix-issued PINs
    // should default to `false` unless the annotation says otherwise.
    let temporary = datafix_annotations
        .password_policy
        .temporary
        .unwrap_or(false);

    let client = keycloak_admin_client().await?;

    let _user = client
        .edit_user(
            realm,
            &user_id,
            None, // Enable/disable
            None, // attributes
            None,
            None,
            None,
            None,
            password,
            Some(temporary),
        )
        .await
        .map_err(edit_user_error)?;

    Ok(ReplacedPin {
        pin,
        applied: AppliedInboundOperation {
            user_id: Some(user_id),
            area_id: user.get_area_id(),
            changes: InboundVoterChanges::PinReplaced { temporary },
        },
    })
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
#[instrument(skip_all, err)]
pub async fn acquire_inbound_voter_lock(
    keycloak_transaction: &Transaction<'_>,
    claims: &DatafixClaims,
    username: &str,
) -> Result<InboundVoterLock, DatafixError> {
    let mut hasura_client: DbClient = get_hasura_pool().await.get().await.map_err(|err| {
        error!("Error getting Hasura client for the inbound Datafix lock: {err}");
        DatafixError::internal(format!(
            "Error getting Hasura client for the inbound Datafix lock: {err}"
        ))
    })?;
    let hasura_transaction = hasura_client.transaction().await.map_err(|err| {
        error!("Error starting Hasura transaction for the inbound Datafix lock: {err}");
        DatafixError::internal(format!(
            "Error starting Hasura transaction for the inbound Datafix lock: {err}"
        ))
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
        .map_err(|err| DatafixError::internal(format!("Invalid voter id: {err}")))?;
    let lock_key = datafix_voter_lock_key(&claims.tenant_id, &election_event_id, &user_id_uuid);
    let lock = PgLock::acquire(
        lock_key,
        Uuid::new_v4().to_string(),
        ISO8601::now() + Duration::seconds(DATAFIX_VOTER_LOCK_SECS),
    )
    .await
    .map_err(|err| {
        error!("Another operation is updating this Datafix voter: {err}");
        DatafixError::new(
            DatafixErrorCode::VoterOperationInProgress,
            "Another operation is updating this Datafix voter",
        )
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
fn active_vote_error(state: &VoterCastVoteState) -> Option<DatafixError> {
    if state.has_unresolved_vote {
        Some(DatafixError::new(
            DatafixErrorCode::VoterStateUnresolved,
            "The voter has an in-progress online vote",
        ))
    } else if state.has_valid_vote {
        Some(DatafixError::new(
            DatafixErrorCode::VoterVotedOnline,
            "The voter has a valid online vote",
        ))
    } else {
        None
    }
}

/// Why re-enabling the voter must be refused, if its voting state is still
/// unresolved: an in-progress or valid vote, or any recorded voted channel.
fn reenable_refusal(
    state: &VoterCastVoteState,
    attributes: &HashMap<String, Vec<String>>,
) -> Option<DatafixError> {
    let reason = if state.has_unresolved_vote {
        "it has an in-progress online vote".to_string()
    } else if state.has_valid_vote {
        "it has a valid online vote".to_string()
    } else if voted_via_internet(attributes) || voted_via_not_internet_channel(attributes) {
        format!(
            "it is recorded as having voted via {}",
            recorded_voted_channel(attributes)
        )
    } else {
        return None;
    };
    Some(DatafixError::new(
        DatafixErrorCode::VoterStateUnresolved,
        format!("Cannot re-enable the voter: {reason}"),
    ))
}

/// Rejects the inbound operation while the voter has any non-discarded online
/// vote. An in-progress vote is still being reconciled and a valid vote is
/// immutable through the inbound API. Callers hold the per-voter lock, so the
/// check cannot race a vote being promoted to `valid`.
#[instrument(skip_all, err)]
pub async fn ensure_voter_has_no_active_vote(
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    claims: &DatafixClaims,
    username: &str,
) -> Result<(), DatafixError> {
    let (election_event_id, _) = get_event_id_and_datafix_annotations(
        hasura_transaction,
        &claims.tenant_id,
        &claims.datafix_event_id,
    )
    .await?;
    let realm = get_event_realm(&claims.tenant_id, &election_event_id);
    let user_id = get_user_id(keycloak_transaction, &realm, username).await?;
    let tenant_id = parse_uuid_v4(&claims.tenant_id)
        .map_err(|err| DatafixError::internal(format!("Invalid tenant id: {err}")))?;
    let election_event_uuid = parse_uuid_v4(&election_event_id)
        .map_err(|err| DatafixError::internal(format!("Invalid election event id: {err}")))?;
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
        DatafixError::internal(format!(
            "Error checking for an active online vote before an inbound Datafix operation: {err}"
        ))
    })?;
    if let Some(err) = active_vote_error(&state) {
        return Err(err);
    }
    Ok(())
}

/// Refuses to re-enable a Datafix voter whose voting state is still unresolved —
/// an in-progress or valid vote, or any recorded voted channel — returning
/// `Conflict` in that case.
#[instrument(skip_all, err)]
pub async fn ensure_inbound_reenable_is_safe(
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    claims: &DatafixClaims,
    username: &str,
) -> Result<(), DatafixError> {
    let (election_event_id, _) = get_event_id_and_datafix_annotations(
        hasura_transaction,
        &claims.tenant_id,
        &claims.datafix_event_id,
    )
    .await?;
    let realm = get_event_realm(&claims.tenant_id, &election_event_id);
    let user_id = get_user_id(keycloak_transaction, &realm, username).await?;
    let tenant_id = parse_uuid_v4(&claims.tenant_id)
        .map_err(|err| DatafixError::internal(format!("Invalid tenant id: {err}")))?;
    let election_event_uuid = parse_uuid_v4(&election_event_id)
        .map_err(|err| DatafixError::internal(format!("Invalid election event id: {err}")))?;
    let state = get_voter_cast_vote_state(
        hasura_transaction,
        &tenant_id,
        &election_event_uuid,
        &user_id,
    )
    .await
    .map_err(|err| {
        error!("Error checking unresolved votes before enabling a Datafix voter: {err}");
        DatafixError::internal(format!(
            "Error checking unresolved votes before enabling a Datafix voter: {err}"
        ))
    })?;
    let client = keycloak_admin_client().await?;
    let user = client.get_user(&realm, &user_id).await.map_err(|err| {
        error!("Error loading a Datafix voter before enabling it: {err}");
        DatafixError::internal(format!(
            "Error loading a Datafix voter before enabling it: {err}"
        ))
    })?;
    let attributes = user.attributes.unwrap_or_default();
    if let Some(err) = reenable_refusal(&state, &attributes) {
        return Err(err);
    }
    Ok(())
}

/// The voter's Keycloak id and area for an audit entry: taken from the
/// applied operation when Keycloak returned them with the write, otherwise
/// looked up in Keycloak (the same lookup Keycloak user events use).
#[instrument(skip_all)]
async fn audited_voter(
    keycloak_transaction: Option<&Transaction<'_>>,
    realm: &str,
    username: &str,
    applied: Option<&AppliedInboundOperation>,
) -> (Option<String>, Option<String>) {
    let mut user_id = applied.and_then(|applied| applied.user_id.clone());
    let mut area_id = applied.and_then(|applied| applied.area_id.clone());
    let Some(transaction) = keycloak_transaction else {
        return (user_id, area_id);
    };
    if user_id.is_none() {
        user_id = get_user_id(transaction, realm, username).await.ok();
    }
    if area_id.is_none() {
        if let Some(user_id) = &user_id {
            area_id = match get_user_area_id(transaction, realm, user_id).await {
                Ok(area_id) => area_id,
                Err(err) => {
                    warn!("Unable to resolve the voter area for the inbound Datafix audit entry: {err}");
                    None
                }
            };
        }
    }
    (user_id, area_id)
}

/// Records the outcome of an inbound Datafix operation in the electoral log:
/// what the operation applied, or why it failed, with the voter's Keycloak id
/// and area (resolved from Keycloak when a transaction is supplied). Failures
/// are logged and swallowed so auditing never fails the operation itself.
#[instrument(skip_all, fields(operation = %operation))]
pub async fn audit_inbound_operation(
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: Option<&Transaction<'_>>,
    claims: &DatafixClaims,
    username: &str,
    operation: InboundOperation,
    outcome: Result<&AppliedInboundOperation, &DatafixError>,
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
                "Unable to resolve the election event for the inbound Datafix audit entry: {err}"
            );
            return;
        }
    };
    let realm = get_event_realm(&claims.tenant_id, &election_event_id);
    let (user_id, area_id) =
        audited_voter(keycloak_transaction, &realm, username, outcome.ok()).await;

    if let Err(err) = post_operation_result_to_electoral_log(
        hasura_transaction,
        &claims.tenant_id,
        &election_event_id,
        user_id.as_deref(),
        username,
        area_id.as_deref(),
        ExtApiRequestDirection::Inbound,
        inbound_operation_log_entry(username, operation, outcome),
    )
    .await
    {
        error!("Unable to record the inbound Datafix {operation} audit entry: {err}");
    }
}

/// Audits an inbound operation on a fresh short-lived Hasura transaction, for
/// the paths where the request transaction has not been opened yet or has
/// already been committed or dropped.
#[instrument(skip_all, fields(operation = %operation))]
pub async fn audit_inbound_operation_standalone(
    keycloak_transaction: Option<&Transaction<'_>>,
    claims: &DatafixClaims,
    username: &str,
    operation: InboundOperation,
    outcome: Result<&AppliedInboundOperation, &DatafixError>,
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
        keycloak_transaction,
        claims,
        username,
        operation,
        outcome,
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
        active_vote_error, create_user_error, plan_unmark_voter_edit, recorded_voted_channel,
        reenable_refusal, valid_inbound_voting_channel,
    };
    use crate::postgres::cast_vote::VoterCastVoteState;
    use crate::services::datafix::types::{DatafixError, DatafixErrorCode};
    use keycloak::KeycloakError;
    use sequent_core::types::keycloak::{
        User, ATTR_RESET_VALUE, DISABLE_COMMENT, DISABLE_REASON_MARKVOTED_CALL, VOTED_CHANNEL,
        VOTED_CHANNEL_INTERNET_VALUE,
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
        assert_eq!(
            create_user_error(&keycloak_http_failure(409)),
            DatafixError::new(DatafixErrorCode::VoterAlreadyExists, "Voter already exists")
        );
    }

    #[test]
    fn other_create_user_failures_stay_internal_errors() {
        let err = create_user_error(&keycloak_http_failure(504));
        assert_eq!(err.code, DatafixErrorCode::InternalError);
        assert!(err.detail.starts_with("Error creating user: "));

        let stringified = anyhow::anyhow!("Failed to create user in keycloak");
        assert_eq!(
            create_user_error(&stringified),
            DatafixError::internal("Error creating user: Failed to create user in keycloak")
        );
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
            Some(DatafixError::new(
                DatafixErrorCode::VoterStateUnresolved,
                "The voter has an in-progress online vote"
            ))
        );
        assert_eq!(
            active_vote_error(&VoterCastVoteState {
                has_unresolved_vote: false,
                has_valid_vote: true,
            }),
            Some(DatafixError::new(
                DatafixErrorCode::VoterVotedOnline,
                "The voter has a valid online vote"
            ))
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
    fn reenable_refusal_names_the_unresolved_state() {
        let resolved = VoterCastVoteState {
            has_unresolved_vote: false,
            has_valid_vote: false,
        };
        let no_channel = HashMap::new();
        let paper = HashMap::from([(VOTED_CHANNEL.to_string(), vec!["PAPER".to_string()])]);
        let internet = HashMap::from([(
            VOTED_CHANNEL.to_string(),
            vec![VOTED_CHANNEL_INTERNET_VALUE.to_string()],
        )]);
        let reset = HashMap::from([(
            VOTED_CHANNEL.to_string(),
            vec![ATTR_RESET_VALUE.to_string()],
        )]);

        assert_eq!(reenable_refusal(&resolved, &no_channel), None);
        assert_eq!(reenable_refusal(&resolved, &reset), None);
        assert_eq!(
            reenable_refusal(&resolved, &paper),
            Some(DatafixError::new(
                DatafixErrorCode::VoterStateUnresolved,
                "Cannot re-enable the voter: it is recorded as having voted via PAPER"
            ))
        );
        assert_eq!(
            reenable_refusal(&resolved, &internet).map(|err| err.detail),
            Some(
                "Cannot re-enable the voter: it is recorded as having voted via Internet"
                    .to_string()
            )
        );
        assert_eq!(
            reenable_refusal(
                &VoterCastVoteState {
                    has_unresolved_vote: true,
                    has_valid_vote: false,
                },
                &no_channel
            )
            .map(|err| err.detail),
            Some("Cannot re-enable the voter: it has an in-progress online vote".to_string())
        );
        assert_eq!(
            reenable_refusal(
                &VoterCastVoteState {
                    has_unresolved_vote: false,
                    has_valid_vote: true,
                },
                &paper
            )
            .map(|err| err.detail),
            Some("Cannot re-enable the voter: it has a valid online vote".to_string())
        );
    }

    #[test]
    fn recorded_voted_channel_defaults_to_the_reset_value() {
        assert_eq!(recorded_voted_channel(&HashMap::new()), ATTR_RESET_VALUE);
        assert_eq!(
            recorded_voted_channel(&HashMap::from([(
                VOTED_CHANNEL.to_string(),
                vec!["PHONE".to_string(), "PAPER".to_string()]
            )])),
            "PAPER"
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
