// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::election::{get_election_by_id, get_elections, update_election_voting_status};
use crate::postgres::election_event::{
    get_all_tenant_election_events, get_election_event_by_id, update_election_event_status,
};
use crate::services::providers::transactions_provider::provide_hasura_transaction;
use anyhow::{anyhow, Result};
use deadpool_postgres::Transaction;
use sequent_core::serialization::deserialize_with_path::deserialize_value;
use sequent_core::types::hasura::core::ElectionEvent;
use tracing::{info, error, warn, instrument};
use rocket::http::Status;
use rocket::serde::json::Json;


#[derive(Serialize)]
pub struct DatafixResponse {
    pub code: u16,
    pub message: String,
}

type JsonErrorResponse = Json<DatafixResponse>;

impl DatafixResponse {
    #[instrument]
    pub fn new(status: Status) -> JsonErrorResponse {
        Json(DatafixResponse {
            code: status.code,
            message: status.reason().unwrap_or_default().to_string(),
        })
    }
}

#[instrument(err)]
async fn get_datafix_election_event_id(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    datafix_event_id: &str,
) -> Result<String, JsonErrorResponse> {
    let election_events = get_all_tenant_election_events(hasura_transaction, tenant_id)
        .await
        .map_err(|err| {
            error!("Error getting election events: {err}");
            DatafixResponse::new(Status::BadRequest)
    })?;

    info!("election_events: {:?}", election_events);
    let mut election_event_id_match = None;
    let mut itr = election_events.iter();
    while (let Some(event) = itr.next() && election_event_id_match.is_none()) {
        let annotations_datafix_id = event.annotations.map(|v| 
            v.get("DATAFIX").map(|v|  
                v.get("id").map(|v| v.to_string())
            ).flatten()
        ).flatten();
        election_event_id_match = match annotations_datafix_id {
            Some(annotations_datafix_id) if annotations_datafix_id.eq(datafix_event_id) => {
                Some(event.id.clone())
            }
            _ => None
        };
    }

    match election_event_id_match {
        Some(election_event_id) => Ok(election_event_id),
        None => {
            warn!("Datafix event id: {datafix_event_id} not found");
            return DatafixResponse::new(Status::BadRequest);
        }
    }
}


/// Disable the voter, datafix users are not actually deleted but just disabled.
#[instrument(err)]
pub async fn disable_datafix_voter(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    datafix_event_id: &str,
    voter_id: &str,
) -> Result<Json<String>, JsonErrorResponse> {

    let election_event_id = get_datafix_election_event_id(
        hasura_transaction,
        tenant_id,
        datafix_event_id,
    )
    .await?;

    let realm = get_event_realm(tenant_id, &election_event_id);
    let client = KeycloakAdminClient::new().await.map_err(|e| {
        (DatafixResponse::new(Status::InternalServerError))
    })?;

    let _user = client
        .edit_user(
            &realm,
            voter_id,
            Some(false),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .map_err(|e| {
            (DatafixResponse::new(Status::InternalServerError)) // TODO User not found should return Not found error
        })?;
    Ok(Json(DatafixResponse::new(Status::Ok)))
}