// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::election_event::{get_election_event_by_id, ElectionEventDatafix};
use crate::services::datafix;
use crate::services::datafix::types::SoapRequest;
use crate::services::datafix::utils::voted_via_internet;
use crate::services::election_event_board::get_election_event_board;
use crate::services::electoral_log::ElectoralLog;
use crate::services::protocol_manager::get_protocol_manager;
use crate::services::users::{get_username_by_id, list_users, ListUsersFilter};
use crate::types::error::Result;
use crate::{
    hasura::election_event::get_election_event::GetElectionEventSequentBackendElectionEvent,
    services::database::{get_hasura_pool, get_keycloak_pool},
};
use anyhow::{anyhow, Context};
use celery::error::TaskError;
use deadpool_postgres::Client as DbClient;
use sequent_core::services::keycloak::KeycloakAdminClient;
use std::collections::HashMap;
use tracing::{event, info, instrument, Level};

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn cast_vote_actions(filter: ListUsersFilter, username: Option<String>) -> Result<()> {
    // WIP
    //..
    let voter_id: String = filter
        .user_ids
        .map(|v| v.first())
        .flatten()
        .ok_or(0)
        .map_err(|_| format!("Voter id not found"))?;

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

    let user = match list_users(&hasura_transaction, &keycloak_transaction, filter).await {
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
        let result = datafix::voterview_requests::send(
            SoapRequest::SetVoted,
            ElectionEventDatafix(election_event),
            &username,
        )
        .await;

        // TODO: Post the result in the electoral_log

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
    Ok(())
}
