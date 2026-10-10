// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::{
    authorize, authorize_election_permission_labels,
};
use crate::types::error_response::{ErrorCode, ErrorResponse, JsonError};
use anyhow::{anyhow, Result};
use chrono::Utc;
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
use windmill::services::election_dates::{
    self, InvalidSchedule, ScheduleInput, ScheduleWarning,
};
use windmill::services::schedule_recompute;
use windmill::services::signing::log::Actor;

#[derive(Deserialize, Debug)]
pub struct ManageElectionDatesBody {
    election_event_id: String,
    election_id: Option<String>,
    /// Older clients: the instant, with an offset.
    scheduled_date: Option<String>,
    event_processor: EventProcessors,
    voting_channels: Option<Vec<VotingStatusChannel>>,
    /// The wall time, `YYYY-MM-DDTHH:MM`, in `time_zone`.
    #[serde(default)]
    local_date_time: Option<String>,
    /// IANA zone of `local_date_time`; defaults to the row's zone.
    #[serde(default)]
    time_zone: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct ManageElectionDatesResponse {
    error_msg: Option<String>,
    /// The stored instant (RFC 3339, UTC).
    scheduled_date: Option<String>,
    warnings: Vec<ScheduleWarning>,
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

    authorize_election_permission_labels(
        &hasura_transaction,
        &claims,
        &input.election_event_id,
        input.election_id.as_ref().map(std::slice::from_ref),
    )
    .await
    .map_err(|(status, message)| {
        let code = if status == Status::Forbidden {
            ErrorCode::Unauthorized
        } else {
            ErrorCode::InternalServerError
        };
        ErrorResponse::new(status, &message, code)
    })?;

    let actor = actor(&claims);
    let schedule = ScheduleInput {
        local_date_time: input.local_date_time.clone(),
        time_zone: input.time_zone.clone(),
        scheduled_date: input.scheduled_date.clone(),
    };
    let saved = match election_dates::save_schedule(
        &hasura_transaction,
        &claims.hasura_claims.tenant_id,
        &input.election_event_id,
        input.election_id.as_deref(),
        &input.event_processor,
        &schedule,
        input.voting_channels.clone(),
        &actor,
    )
    .await
    {
        Ok(saved) => saved,
        Err(err) => {
            // A refused date is the caller's to fix; the row isn't saved.
            return match err.downcast_ref::<InvalidSchedule>() {
                Some(refusal) => Ok(Json(ManageElectionDatesResponse {
                    error_msg: Some(refusal.to_string()),
                    ..Default::default()
                })),
                None => Err(ErrorResponse::new(
                    Status::InternalServerError,
                    &format!("manage election dates failed: {err:?}"),
                    ErrorCode::InternalServerError,
                )),
            };
        }
    };

    let refresh_enrollment = matches!(
        input.event_processor,
        EventProcessors::START_ENROLLMENT_PERIOD
            | EventProcessors::END_ENROLLMENT_PERIOD
    );
    if refresh_enrollment {
        windmill::services::enrollment_windows::begin_synchronization(
            &hasura_transaction, &claims.hasura_claims.tenant_id, &input.election_event_id,
        ).await.map_err(|_| ErrorResponse::new(Status::ServiceUnavailable,
            "Enrollment synchronization could not be started; the schedule was not saved.", ErrorCode::InternalServerError))?;
    }
    let _commit = hasura_transaction.commit().await.map_err(|e| {
        ErrorResponse::new(
            Status::InternalServerError,
            &format!("commit failed: {e:?}"),
            ErrorCode::InternalServerError,
        )
    })?;

    if refresh_enrollment {
        windmill::services::enrollment_windows::complete_synchronization(
            &claims.hasura_claims.tenant_id, &input.election_event_id,
        ).await.map_err(|_| ErrorResponse::new(Status::ServiceUnavailable,
            "Schedule saved, but enrollment synchronization could not be confirmed. Correct the synchronization failure and retry the save.", ErrorCode::InternalServerError))?;
    }

    Ok(Json(ManageElectionDatesResponse {
        error_msg: None,
        scheduled_date: saved.scheduled_date,
        warnings: saved.warnings,
    }))
}

#[derive(Deserialize, Debug)]
pub struct ApplyScheduleRecomputeBody {
    election_event_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ApplyScheduleRecomputeResponse {
    updated: usize,
}

/// Applies the scheduled dates the tz database check recomputed for the
/// event (VOTE-LIFECYCLE): nothing changes before an administrator does this.
#[instrument(skip(claims))]
#[post("/apply-schedule-recompute", format = "json", data = "<body>")]
pub async fn apply_schedule_recompute(
    body: Json<ApplyScheduleRecomputeBody>,
    claims: JwtClaims,
) -> Result<Json<ApplyScheduleRecomputeResponse>, JsonError> {
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
    let internal = |e: &dyn std::fmt::Debug| {
        ErrorResponse::new(
            Status::InternalServerError,
            &format!("apply schedule recompute failed: {e:?}"),
            ErrorCode::InternalServerError,
        )
    };
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| internal(&e))?;
    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|e| internal(&e))?;
    let now = Utc::now();
    let refresh_enrollment = schedule_recompute::enrollment_changes_pending(
        &hasura_transaction,
        &claims.hasura_claims.tenant_id,
        &body.election_event_id,
        now,
    )
    .await
    .map_err(|e| internal(&e))?;
    let updated = schedule_recompute::apply(
        &hasura_transaction,
        &claims.hasura_claims.tenant_id,
        &body.election_event_id,
        &actor(&claims),
        now,
    )
    .await
    .map_err(|e| internal(&e))?;
    if refresh_enrollment && updated > 0 {
        windmill::services::enrollment_windows::begin_synchronization(
            &hasura_transaction,
            &claims.hasura_claims.tenant_id,
            &body.election_event_id,
        )
        .await
        .map_err(|e| internal(&e))?;
    }
    hasura_transaction
        .commit()
        .await
        .map_err(|e| internal(&e))?;
    if refresh_enrollment && updated > 0 {
        windmill::services::enrollment_windows::complete_synchronization(
            &claims.hasura_claims.tenant_id,
            &body.election_event_id,
        )
        .await
        .map_err(|e| internal(&e))?;
    }
    Ok(Json(ApplyScheduleRecomputeResponse { updated }))
}

fn actor(claims: &JwtClaims) -> Actor {
    Actor {
        user_id: claims.hasura_claims.user_id.clone(),
        username: claims
            .preferred_username
            .clone()
            .unwrap_or_else(|| claims.hasura_claims.user_id.clone()),
    }
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
