// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use anyhow::{anyhow, Result};
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::ballot::{
    AllowTallyStatus, ElectionStatus, InitReport, VotingStatus,
};
use sequent_core::serialization::deserialize_with_path;
use sequent_core::services::jwt::decode_permission_labels;
use sequent_core::types::ceremonies::TallyExecutionStatus;
use sequent_core::types::ceremonies::TallyType;
use sequent_core::types::permissions::Permissions;
use sequent_core::{
    services::jwt::JwtClaims, types::hasura::core::TallySessionConfiguration,
};
use serde::{Deserialize, Serialize};
use tracing::{event, instrument, Level};
use windmill::postgres::election::get_elections_by_ids;
use windmill::postgres::tally_session::get_tally_session_by_id;
use windmill::services::{
    ceremonies::tally_ceremony, database::get_hasura_pool,
};

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateTallyCeremonyInput {
    election_event_id: String,
    election_ids: Vec<String>,
    configuration: Option<TallySessionConfiguration>,
    tally_type: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateTallyCeremonyOutput {
    tally_session_id: String,
}

// The main function to start a key ceremony
#[instrument(skip(claims))]
#[post("/create-tally-ceremony", format = "json", data = "<body>")]
pub async fn create_tally_ceremony(
    body: Json<CreateTallyCeremonyInput>,
    claims: JwtClaims,
) -> Result<Json<CreateTallyCeremonyOutput>, (Status, String)> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::ADMIN_CEREMONY],
    )?;
    let input = body.into_inner();
    let tenant_id: String = claims.hasura_claims.tenant_id.clone();
    let user_id = claims.clone().hasura_claims.user_id;
    let username = claims
        .clone()
        .preferred_username
        .unwrap_or(claims.name.clone().unwrap_or_else(|| user_id.clone()));
    let permission_labels = decode_permission_labels(&claims);

    let mut hasura_db_client: DbClient =
        get_hasura_pool().await.get().await.map_err(|err| {
            (
                Status::InternalServerError,
                format!("Error getting hasura db pool: {err}"),
            )
        })?;

    let hasura_transaction =
        hasura_db_client.transaction().await.map_err(|err| {
            (
                Status::InternalServerError,
                format!("Error starting hasura transaction: {err}"),
            )
        })?;

    let tally_session_id = tally_ceremony::create_tally_ceremony(
        &hasura_transaction,
        tenant_id,
        &user_id,
        input.election_event_id.clone(),
        input.election_ids,
        input.configuration,
        input.tally_type.clone(),
        &permission_labels,
        username,
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    let _commit = hasura_transaction.commit().await.map_err(|err| {
        (Status::InternalServerError, format!("Commit failed: {err}"))
    })?;
    event!(
        Level::INFO,
        "Created Tally Ceremony, type={}, electionEventId={}, tallySessionId={}",
        input.tally_type,
        input.election_event_id,
        tally_session_id,
    );

    Ok(Json(CreateTallyCeremonyOutput { tally_session_id }))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateTallyCeremonyInput {
    election_event_id: String,
    tally_session_id: String,
    status: TallyExecutionStatus,
}

#[instrument(skip(claims))]
#[post("/update-tally-ceremony", format = "json", data = "<body>")]
pub async fn update_tally_ceremony(
    body: Json<UpdateTallyCeremonyInput>,
    claims: JwtClaims,
) -> Result<Json<CreateTallyCeremonyOutput>, (Status, String)> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::ADMIN_CEREMONY],
    )?;
    let input = body.into_inner();
    let tenant_id = claims.hasura_claims.tenant_id.clone();

    let mut hasura_db_client: DbClient =
        get_hasura_pool().await.get().await.map_err(|err| {
            (
                Status::InternalServerError,
                format!("Error getting hasura db pool: {err}"),
            )
        })?;

    let hasura_transaction =
        hasura_db_client.transaction().await.map_err(|err| {
            (
                Status::InternalServerError,
                format!("Error starting hasura transaction: {err}"),
            )
        })?;

    let tally_session = get_tally_session_by_id(
        &hasura_transaction,
        &tenant_id,
        &input.election_event_id,
        &input.tally_session_id,
    )
    .await
    .map_err(|_| {
        (
            Status::InternalServerError,
            format!(
                "Could not find tally session by id {}",
                input.election_event_id
            ),
        )
    })?;
    let tally_type = tally_session
        .tally_type
        .map(|val: String| {
            TallyType::try_from(val.as_str()).unwrap_or_default()
        })
        .unwrap_or_default();

    let is_tally_allowed = get_elections_by_ids(
        &hasura_transaction,
        &tenant_id,
        &input.election_event_id,
        &tally_session.election_ids.unwrap_or(vec![]),
    )
    .await
    .map_err(|_| {
        (
            Status::InternalServerError,
            format!(
                "Could not find elections for election event {}",
                input.election_event_id
            ),
        )
    })?
    .iter()
    .all(|election| {
        if let Some(election_status) = &election.status {
            deserialize_with_path::deserialize_value::<ElectionStatus>(
                election_status.clone(),
            )
            .map(|election_status| match tally_type {
                TallyType::ELECTORAL_RESULTS => {
                    election_status.allow_tally == AllowTallyStatus::ALLOWED
                        || (election_status.allow_tally
                            == AllowTallyStatus::REQUIRES_VOTING_PERIOD_END
                            && election_status.voting_status
                                == VotingStatus::CLOSED)
                }
                TallyType::INITIALIZATION_REPORT => {
                    election_status.init_report == InitReport::ALLOWED
                }
            })
            .unwrap_or(true)
        } else {
            true
        }
    });

    if (!is_tally_allowed) {
        return Err((
            Status::InternalServerError,
            format!(
                "Tally is not allowed for election event {}.",
                input.election_event_id
            ),
        ));
    }

    tally_ceremony::update_tally_ceremony(
        &hasura_transaction,
        tenant_id,
        input.election_event_id.clone(),
        input.tally_session_id.clone(),
        input.status.clone(),
    )
    .await
    .map_err(|e| {
        (
            Status::InternalServerError,
            format!("Error with update_tally_ceremony: {:?}", e),
        )
    })?;

    Ok(Json(CreateTallyCeremonyOutput {
        tally_session_id: input.tally_session_id.clone(),
    }))
}

////////////////////////////////////////////////////////////////////////////////
/// Endpoint: /restore-private-key
////////////////////////////////////////////////////////////////////////////////

#[derive(Serialize, Deserialize, Debug)]
pub struct SetPrivateKeyInput {
    election_event_id: String,
    private_key_base64: String,
    tally_session_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SetPrivateKeyOutput {
    is_valid: bool,
}

// The main function to restore the private key
#[instrument(skip(claims))]
#[post("/restore-private-key", format = "json", data = "<body>")]
pub async fn restore_private_key(
    body: Json<SetPrivateKeyInput>,
    claims: JwtClaims,
) -> Result<Json<SetPrivateKeyOutput>, (Status, String)> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::TRUSTEE_CEREMONY],
    )?;
    let input = body.into_inner();
    let tenant_id = claims.hasura_claims.tenant_id.clone();

    let mut hasura_db_client: DbClient =
        get_hasura_pool().await.get().await.map_err(|err| {
            (
                Status::InternalServerError,
                format!("Error getting hasura db pool: {err}"),
            )
        })?;

    let hasura_transaction =
        hasura_db_client.transaction().await.map_err(|err| {
            (
                Status::InternalServerError,
                format!("Error starting hasura transaction: {err}"),
            )
        })?;
    
    let is_valid = tally_ceremony::set_private_key(
        &hasura_transaction,
        &claims,
        &tenant_id,
        &input.election_event_id,
        &input.tally_session_id,
        &input.private_key_base64,
    )
    .await
    .map_err(|e| (Status::BadRequest, format!("{:?}", e)))?;

    event!(
        Level::INFO,
        "Restoring given private key, election_event_id={}, tally_session_id={}, is_valid={}",
        input.election_event_id,
        input.tally_session_id,
        is_valid,
    );

    let _commit = hasura_transaction.commit().await.map_err(|err| {
        (Status::InternalServerError, format!("Commit failed: {err}"))
    })?;
    Ok(Json(SetPrivateKeyOutput { is_valid }))
}
