// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::election_event::{get_election_event_by_id, ElectionEventDatafix};
use crate::services::cast_votes::{CastVote, CastVoteStatus};
use crate::services::database::{get_hasura_pool, get_keycloak_pool};
use crate::services::datafix;
use crate::services::datafix::types::{SoapRequest, SoapRequestResponse};
use crate::services::datafix::utils::{is_datafix_election_event, voted_via_internet};
use crate::services::users::{get_username_by_id, list_users, ListUsersFilter};
use crate::types::error::Result;
use celery::error::TaskError;
use deadpool_postgres::Client as DbClient;
use deadpool_postgres::Transaction;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::services::keycloak::KeycloakAdminClient;
use sequent_core::types::hasura::core::ElectionEvent;
use sequent_core::types::keycloak::{VOTED_CHANNEL, VOTED_CHANNEL_INTERNET_VALUE};
use std::collections::HashMap;
use tracing::{error, info, instrument};

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn process_cast_vote(cast_vote: CastVote) -> Result<()> {
    // WIP
    //.. LOCK THIS FUCNTION

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
    let voter_id = cast_vote
        .voter_id_string
        .clone()
        .ok_or("Voter id not found")?;
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
            process_cast_vote_request_to_datafix(
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

    if status != CastVoteStatus::InProgress {
        // update cast_vote status on the table.
    }

    Ok(())
}

#[instrument(skip(hasura_transaction), err)]
pub async fn process_cast_vote_request_to_datafix(
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
        // Send the request only if the voter has not voted yet via internet, because it could be a re-vote or voting multiple times for different contests
        // But we do not want to send more than one requst to Datafix, wich in turn would return HasVoted.
        let result = datafix::voterview_requests::send(
            SoapRequest::SetVoted,
            ElectionEventDatafix(election_event),
            &username,
        )
        .await;

        // TODO: Post the result in the electoral_log
        match result {
            Ok(SoapRequestResponse::Ok) => {} // Continue processing
            Ok(SoapRequestResponse::HasVotedErrorMsg) => {
                return Ok(CastVoteStatus::Discarded);
            }
            Ok(SoapRequestResponse::OtherErrorMsg(msg)) => {
                error!("Error sending request to Datafix: {msg:?}");
                return Ok(CastVoteStatus::InProgress);
            }
            Ok(SoapRequestResponse::Faultstring(msg)) => {
                error!("Error sending request to Datafix: {msg:?}");
                return Ok(CastVoteStatus::InProgress);
            }
            Err(e) => {
                error!("Error sending request to Datafix: {e:?}");
                return Ok(CastVoteStatus::InProgress);
            }
        };

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
                &realm, &voter_id, None, attributes, None, None, None, None, None, None,
            )
            .await
            .map_err(|e| {
                error!("Error editing user Internet channel: {e:?}");
            });
    }
    Ok(CastVoteStatus::Valid)
}
