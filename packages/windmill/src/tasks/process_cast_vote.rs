// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::cast_vote::update_cast_vote_status;
use crate::postgres::election_event::{get_election_event_by_id, ElectionEventDatafix};
use crate::services::cast_votes::{CastVote, CastVoteStatus};
use crate::services::database::{get_hasura_pool, get_keycloak_pool};
use crate::services::datafix;
use crate::services::datafix::types::{SoapRequest, SoapRequestResponse};
use crate::services::datafix::utils::{
    is_datafix_election_event, post_operation_result_to_electoral_log, voted_via_internet,
};
use crate::services::pg_lock::PgLock;
use crate::services::users::{get_username_by_id, list_users, ListUsersFilter};
use crate::types::error::Result;
use celery::error::TaskError;
use chrono::Duration;
use deadpool_postgres::Client as DbClient;
use deadpool_postgres::Transaction;
use electoral_log::messages::newtypes::ExtApiRequestDirection;
use sequent_core::services::date::ISO8601;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::services::keycloak::KeycloakAdminClient;
use sequent_core::types::hasura::core::ElectionEvent;
use sequent_core::types::keycloak::{VOTED_CHANNEL, VOTED_CHANNEL_INTERNET_VALUE};
use std::collections::HashMap;
use tracing::{error, info, instrument};
use uuid::Uuid;

#[instrument(skip(cast_vote), err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn process_cast_vote(cast_vote: CastVote) -> Result<()> {
    info!("process_cast_vote: Processing cast vote {}", cast_vote.id);
    let voter_id = cast_vote
        .voter_id_string
        .clone()
        .ok_or("Voter id not found")?;
    let election_id = cast_vote
        .election_id
        .clone()
        .ok_or("Election id not found")?;
    let Ok(lock) = PgLock::acquire(
        format!("process_cast_vote-{election_id}-{voter_id}"),
        Uuid::new_v4().to_string(),
        ISO8601::now() + Duration::seconds(120),
    )
    .await
    else {
        info!("Skipping: process_cast_vote for election id {election_id} and voter id {voter_id}");
        return Ok(());
    };

    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| format!("Error getting hasura db client {e:?}"))?;

    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|e| format!("Error getting hasura_transaction {e:?}"))?;

    let mut keycloak_db_client: DbClient = get_keycloak_pool()
        .await
        .get()
        .await
        .map_err(|e| format!("Error getting keycloak_db_client {e:?}"))?;

    let keycloak_transaction = keycloak_db_client
        .transaction()
        .await
        .map_err(|e| format!("Error getting keycloak_transaction {e:?}"))?;

    let tenant_id = cast_vote.tenant_id.clone();
    let election_event_id = cast_vote.election_event_id.clone();
    let realm = get_event_realm(&tenant_id, &election_event_id);
    let username = get_username_by_id(&keycloak_transaction, &realm, &voter_id)
        .await
        .map_err(|e| format!("Error get_username_by_id {e:?}"))?;

    let election_event =
        get_election_event_by_id(&hasura_transaction, &tenant_id, &election_event_id)
            .await
            .map_err(|e| format!("Error getting election event {e:?}"))?;

    // In the future we should have an enum for all special election event types.
    let status = match is_datafix_election_event(&election_event) {
        true => {
            process_soap_request_to_datafix(
                &hasura_transaction,
                &keycloak_transaction,
                &realm,
                election_event,
                &cast_vote,
                &username,
            )
            .await?
        }
        false => CastVoteStatus::Valid,
    };

    let cast_vote_id = Uuid::parse_str(&cast_vote.id)
        .map_err(|err| format!("Error parsing cast_vote_id: {err:?}"))?;

    if status != CastVoteStatus::InProgress {
        update_cast_vote_status(&hasura_transaction, &cast_vote_id, status)
            .await
            .map_err(|e| format!("Error updating cast vote status {e:?}"))?;
    }

    let _commit = hasura_transaction
        .commit()
        .await
        .map_err(|e| format!("process_cast_vote: Commit failed {e:?}"));

    lock.release().await?;
    Ok(())
}

#[instrument(skip(hasura_transaction, keycloak_transaction, election_event), err)]
pub async fn process_soap_request_to_datafix(
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    realm: &str,
    election_event: ElectionEvent,
    cast_vote: &CastVote,
    username: &Option<String>,
) -> Result<CastVoteStatus> {
    let area_id = cast_vote.area_id.clone().ok_or("Area id not found")?;
    let voter_id = cast_vote
        .voter_id_string
        .clone()
        .ok_or("Voter id not found")?;
    let filter = ListUsersFilter {
        tenant_id: cast_vote.tenant_id.clone(),
        election_event_id: Some(cast_vote.election_event_id.clone()),
        realm: realm.to_string(),
        user_ids: Some(vec![voter_id.clone()]),
        area_id: Some(area_id),
        ..ListUsersFilter::default()
    };

    let user = match list_users(hasura_transaction, keycloak_transaction, filter).await {
        Ok((users, 1)) => users
            .last()
            .map(|val_ref| val_ref.to_owned())
            .unwrap_or_default(),
        Ok(_) => {
            return Err(format!("Multiple users found with id {voter_id}").into());
        }
        Err(e) => {
            return Err(format!("Voter not found with id {voter_id}, error: {e:?}").into());
        }
    };
    let attributes = user.attributes.clone().unwrap_or_default();
    if !voted_via_internet(&attributes) {
        // Send the request only if the voter has not voted yet via internet, because it could be a re-vote or voting multiple times for different elections.
        // But we do not want to send more than one request which would return HasVoted.
        let result = datafix::voterview_requests::send(
            SoapRequest::SetVoted,
            ElectionEventDatafix(election_event),
            username,
        )
        .await;

        let req_type = SoapRequest::SetVoted;
        let (result, operation) = match result {
            Ok(SoapRequestResponse::Ok) => {
                let client = KeycloakAdminClient::new()
                    .await
                    .map_err(|e| format!("Error obtaining keycloak admin client {e:?}"))?;

                // Set the attribute to avoid sending it again on the next vote.
                let mut hash_map = HashMap::new();
                hash_map.insert(
                    VOTED_CHANNEL.to_string(),
                    vec![VOTED_CHANNEL_INTERNET_VALUE.to_string()],
                );
                let attributes = Some(hash_map);
                let _user = client
                    .edit_user(
                        realm, &voter_id, None, attributes, None, None, None, None, None, None,
                    )
                    .await
                    .map_err(|e| {
                        error!("Error editing user Internet channel: {e:?}");
                    });
                (Ok(CastVoteStatus::Valid), format!("{req_type} Succeded"))
            }
            Ok(SoapRequestResponse::HasVotedErrorMsg) => {
                (Ok(CastVoteStatus::Discarded), format!("{req_type} Failed"))
            }
            Ok(SoapRequestResponse::OtherErrorMsg(msg)) => {
                error!("Error sending request to Datafix: {msg:?}");
                (Ok(CastVoteStatus::InProgress), format!("{req_type} Failed"))
            }
            Ok(SoapRequestResponse::Faultstring(msg)) => {
                error!("Error sending request to Datafix: {msg:?}");
                (Ok(CastVoteStatus::InProgress), format!("{req_type} Failed"))
            }
            Err(e) => {
                error!("Error sending request to Datafix: {e:?}");
                (Ok(CastVoteStatus::InProgress), format!("{req_type} Failed"))
            }
        };

        let username_str = username.as_deref().ok_or("Username is None")?;
        post_operation_result_to_electoral_log(
            hasura_transaction,
            &cast_vote.tenant_id,
            &cast_vote.election_event_id,
            &voter_id,
            username_str,
            ExtApiRequestDirection::Outbound,
            operation,
        )
        .await;

        result
    } else {
        Ok(CastVoteStatus::Valid)
    }
}
