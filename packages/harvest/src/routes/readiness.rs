// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! A permission-scoped readiness snapshot; failed checks never mutate events.

use crate::services::audit;
use crate::services::authorization::{
    authorize, authorize_election_permission_labels,
};
use crate::services::dependencies::HarvestServices;
use crate::types::error_response::{ErrorCode, ErrorResponse, JsonError};
use rocket::{http::Status, serde::json::Json, State};
use sequent_core::services::jwt::JwtClaims;
use sequent_core::services::uuid_validation::parse_uuid_v4;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use windmill::domain::readiness::ReadinessReport;

#[derive(Debug, Deserialize, Serialize)]
pub struct ReadinessInput {
    official_event_id: String,
    tested_event_id: String,
}

fn invalid(message: &str) -> JsonError {
    ErrorResponse::new(
        Status::BadRequest,
        message,
        ErrorCode::InvalidReadinessCheck,
    )
}

fn internal(error: impl std::fmt::Debug) -> JsonError {
    tracing::error!("Readiness check could not be completed: {error:?}");
    ErrorResponse::new(
        Status::InternalServerError,
        "Readiness check could not be completed",
        ErrorCode::InternalServerError,
    )
}

fn validate(input: &ReadinessInput) -> Result<(), JsonError> {
    for id in [&input.official_event_id, &input.tested_event_id] {
        parse_uuid_v4(id)
            .map_err(|_| invalid("Each event ID must be a UUID v4"))?;
    }
    if parse_uuid_v4(&input.official_event_id)
        .map_err(|_| invalid("Invalid official event ID"))?
        == parse_uuid_v4(&input.tested_event_id)
            .map_err(|_| invalid("Invalid tested event ID"))?
    {
        return Err(invalid("The official and tested events must differ"));
    }
    Ok(())
}

#[post("/readiness-check", format = "json", data = "<input>")]
pub async fn readiness_check(
    claims: JwtClaims,
    input: Json<ReadinessInput>,
    services: &State<HarvestServices>,
) -> Result<Json<ReadinessReport>, JsonError> {
    let input = input.into_inner();
    authorize(
        &claims,
        false,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::READINESS_CHECK],
    )
    .map_err(|(status, message)| {
        ErrorResponse::new(status, &message, ErrorCode::Unauthorized)
    })?;
    validate(&input)?;
    let tenant = &claims.hasura_claims.tenant_id;
    let mut client = services
        .databases
        .hasura()
        .await
        .get()
        .await
        .map_err(internal)?;
    let tx = client.transaction().await.map_err(internal)?;
    // Check both tenants before acquiring event locks or inspecting evidence.
    for event in [&input.official_event_id, &input.tested_event_id] {
        let row = windmill::postgres::election_event::get_election_event_by_id_if_exist(&tx, tenant, event)
            .await.map_err(internal)?;
        if row.is_none() {
            return Err(ErrorResponse::new(
                Status::NotFound,
                "Election event not found",
                ErrorCode::ElectionEventNotFound,
            ));
        }
        authorize_election_permission_labels(&tx, &claims, event, None)
            .await
            .map_err(|(status, message)| {
                ErrorResponse::new(status, &message, ErrorCode::Unauthorized)
            })?;
    }
    windmill::services::readiness::lock_events(
        &tx,
        tenant,
        &input.official_event_id,
        &input.tested_event_id,
    )
    .await
    .map_err(internal)?;
    let actor = audit::actor(&claims);
    let report = windmill::services::readiness::verify(
        &tx,
        tenant,
        &input.official_event_id,
        &input.tested_event_id,
        &actor,
    )
    .await
    .map_err(internal)?;
    // This transaction only holds observation locks. The signed event entry
    // has its own durable append and exists before a passing response returns.
    tx.commit().await.map_err(internal)?;
    Ok(Json(report))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_two_events_must_be_distinct_real_ids() {
        let official = uuid::Uuid::new_v4().to_string();
        let tested = uuid::Uuid::new_v4().to_string();
        assert!(validate(&ReadinessInput {
            official_event_id: official.clone(),
            tested_event_id: tested
        })
        .is_ok());
        assert!(validate(&ReadinessInput {
            official_event_id: official.clone(),
            tested_event_id: official
        })
        .is_err());
        assert!(validate(&ReadinessInput {
            official_event_id: "not-an-id".into(),
            tested_event_id: uuid::Uuid::new_v4().to_string()
        })
        .is_err());
    }
}
