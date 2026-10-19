// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use crate::types::error_response::ErrorCode;
use anyhow::anyhow;
use anyhow::{Context, Result};
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::ballot::ElectionPresentation;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::services::translations::{Alias, DEFAULT_LANG};
use sequent_core::services::uuid_validation::parse_uuid_v4_field;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::{event, instrument, Level};
use windmill::postgres::election;
use windmill::postgres::election_event::get_election_event_by_id_if_exist;
use windmill::services::database::get_hasura_pool;
use windmill::services::import::import_election_event::upsert_b3_and_elog;

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateElectionInput {
    election_event_id: String,
    name: String,
    presentation: ElectionPresentation,
    description: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateElectionOutput {
    id: String,
}

/// Creates an election in an existing election event of the caller's tenant.
#[instrument(skip(claims))]
#[post("/create-election", format = "json", data = "<body>")]
pub async fn create_election(
    body: Json<CreateElectionInput>,
    claims: JwtClaims,
) -> Result<Json<CreateElectionOutput>, (Status, String)> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::ELECTION_EVENT_WRITE],
    )?;
    parse_uuid_v4_field(&body.election_event_id, "election_event_id")
        .map_err(|error| (Status::BadRequest, error.to_string()))?;

    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    get_election_event_by_id_if_exist(
        &hasura_transaction,
        &claims.hasura_claims.tenant_id,
        &body.election_event_id,
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?
    .ok_or_else(|| {
        (Status::NotFound, "Election event not found".to_string())
    })?;

    let alias = body.presentation.get_alias(DEFAULT_LANG);
    let election = election::create_election(
        &hasura_transaction,
        &claims.hasura_claims.tenant_id,
        &body.election_event_id,
        &body.name,
        &alias,
        &body.presentation,
        body.description.clone(),
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    upsert_b3_and_elog(
        &hasura_transaction,
        &claims.hasura_claims.tenant_id,
        &body.election_event_id,
        &vec![election.id.clone()],
        false,
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    hasura_transaction
        .commit()
        .await
        .with_context(|| "error comitting transaction")
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    Ok(Json(CreateElectionOutput { id: election.id }))
}

#[cfg(test)]
mod create_election_tests {
    use super::*;
    use crate::services::authorization::test_claims::admin;
    use uuid::Uuid;

    /// A malformed election event id is a client error, answered before any
    /// database access.
    #[tokio::test]
    async fn create_election_rejects_a_malformed_election_event_id() {
        let body: CreateElectionInput =
            serde_json::from_value(serde_json::json!({
                "election_event_id": "event",
                "name": "election",
                "presentation": {}
            }))
            .expect("valid request");
        let claims = admin(
            &Uuid::new_v4().to_string(),
            &[Permissions::ELECTION_EVENT_WRITE.to_string()],
        );
        assert_eq!(
            create_election(Json(body), claims)
                .await
                .err()
                .map(|error| error.0),
            Some(Status::BadRequest)
        );
    }
}
