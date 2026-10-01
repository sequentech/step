// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use crate::types::error_response::{ErrorCode, ErrorResponse, JsonError};
use anyhow::{anyhow, Result};
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::ballot::VotingStatusChannel;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::Permissions;
use sequent_core::types::scheduled_event::{
    validate_scheduled_voting_channels, EventProcessors,
};
use serde::{Deserialize, Serialize};
use tracing::instrument;
use windmill::services::database::get_hasura_pool;
use windmill::services::{election_dates, election_event_dates};

#[derive(Deserialize, Debug)]
pub struct ManageElectionDatesBody {
    election_event_id: String,
    election_id: Option<String>,
    scheduled_date: Option<String>,
    event_processor: EventProcessors,
    voting_channels: Option<Vec<VotingStatusChannel>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ManageElectionDatesResponse {
    error_msg: Option<String>,
}

#[instrument(skip(claims))]
#[post("/manage-election-dates", format = "json", data = "<body>")]
pub async fn manage_election_dates(
    body: Json<ManageElectionDatesBody>,
    claims: JwtClaims,
) -> Result<Json<ManageElectionDatesResponse>, JsonError> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::SCHEDULED_EVENT_WRITE],
    )
    .map_err(|e| {
        ErrorResponse::new(
            Status::Unauthorized,
            &format!("{e:?}"),
            ErrorCode::Unauthorized,
        )
    })?;
    let input = body.into_inner();

    if input.event_processor == EventProcessors::CREATE_REPORT
        || input.event_processor == EventProcessors::SEND_TEMPLATE
    {
        return Err(ErrorResponse::new(
            Status::BadRequest,
            &format!("Invalid event_processors: {:?}", input.event_processor),
            ErrorCode::InvalidEventProcessor,
        ));
    }

    validate_voting_channels(
        &input.event_processor,
        input.voting_channels.as_deref(),
    )?;

    let mut hasura_db_client: DbClient =
        get_hasura_pool().await.get().await.map_err(|e| {
            ErrorResponse::new(
                Status::InternalServerError,
                &format!("hasura db client failed: {e:?}"),
                ErrorCode::InternalServerError,
            )
        })?;
    let hasura_transaction =
        hasura_db_client.transaction().await.map_err(|e| {
            ErrorResponse::new(
                Status::InternalServerError,
                &format!("hasura transaction failed: {e:?}"),
                ErrorCode::InternalServerError,
            )
        })?;

    match input.election_id {
        Some(id) => {
            match election_dates::manage_dates(
                &hasura_transaction,
                &claims.hasura_claims.tenant_id,
                &input.election_event_id,
                &id,
                input.scheduled_date.as_deref(),
                input.event_processor.to_string().as_str(),
                input.voting_channels.clone(),
            )
            .await
            {
                Ok(_) => (),
                Err(err) => {
                    return Ok(Json(ManageElectionDatesResponse {
                        error_msg: Some(err.to_string()),
                    }));
                }
            }
        }
        None => {
            election_event_dates::manage_dates(
                &hasura_transaction,
                &claims.hasura_claims.tenant_id,
                &input.election_event_id,
                input.scheduled_date.as_deref(),
                input.event_processor.to_string().as_str(),
                input.voting_channels.clone(),
            )
            .await
            .map_err(|e| {
                ErrorResponse::new(
                    Status::InternalServerError,
                    &format!("manage election event dates failed: {e:?}"),
                    ErrorCode::InternalServerError,
                )
            })?;
        }
    }

    let _commit = hasura_transaction.commit().await.map_err(|e| {
        ErrorResponse::new(
            Status::InternalServerError,
            &format!("commit failed: {e:?}"),
            ErrorCode::InternalServerError,
        )
    })?;

    Ok(Json(ManageElectionDatesResponse { error_msg: None }))
}

fn validate_voting_channels(
    event_processor: &EventProcessors,
    voting_channels: Option<&[VotingStatusChannel]>,
) -> Result<(), JsonError> {
    validate_scheduled_voting_channels(event_processor, voting_channels)
        .map_err(|err| {
            ErrorResponse::new(
                Status::BadRequest,
                &err.to_string(),
                ErrorCode::InvalidVotingChannels,
            )
        })
}

#[cfg(test)]
mod voting_channel_validation_tests {
    use super::*;
    use sequent_core::types::scheduled_event::ONLINE_WITH_EARLY_VOTING_START_ERROR;
    use VotingStatusChannel::{EARLY_VOTING, KIOSK, ONLINE};

    #[test]
    fn start_schedule_opening_online_and_early_voting_is_a_bad_request() {
        let response = validate_voting_channels(
            &EventProcessors::START_VOTING_PERIOD,
            Some(&[EARLY_VOTING, KIOSK, ONLINE]),
        )
        .unwrap_err();
        assert_eq!(response.0, Status::BadRequest);
        assert_eq!(response.1.message, ONLINE_WITH_EARLY_VOTING_START_ERROR);
        assert_eq!(response.1.extensions.code, "InvalidVotingChannels");
    }

    #[test]
    fn end_schedule_can_close_online_and_early_voting() {
        assert!(validate_voting_channels(
            &EventProcessors::END_VOTING_PERIOD,
            Some(&[ONLINE, EARLY_VOTING]),
        )
        .is_ok());
    }
}
