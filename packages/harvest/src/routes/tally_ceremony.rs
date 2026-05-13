// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
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
use sequent_core::types::ceremonies::TallyResolution;
use sequent_core::types::ceremonies::TallyType;
use sequent_core::types::permissions::Permissions;
use sequent_core::{
    services::jwt::JwtClaims, types::hasura::core::TallySessionConfiguration,
};
use serde::{Deserialize, Serialize};
use tracing::{event, instrument, Level};
use windmill::postgres::election::get_elections_by_ids;
use windmill::postgres::tally_session::{
    get_tally_session_by_id, update_tally_session_status,
};
use windmill::services::ceremonies::tally_ceremony::{self};
use windmill::services::ceremonies::tally_resolution;
use windmill::services::database::get_hasura_pool;
use windmill::services::providers::transactions_provider::provide_hasura_transaction;

#[derive(Serialize, Deserialize, Debug)]
/// Request body for creating a tally ceremony.
pub struct CreateTallyCeremonyInput {
    /// The election event ID.
    election_event_id: String,
    /// The election IDs.
    election_ids: Vec<String>,
    /// The configuration.
    configuration: Option<TallySessionConfiguration>,
    /// The tally type.
    tally_type: String,
}

#[derive(Serialize, Deserialize, Debug)]
/// Response body for creating a tally ceremony.
pub struct CreateTallyCeremonyOutput {
    /// The tally session ID.
    tally_session_id: String,
}

/// The main function to start a key ceremony
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
    .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

    hasura_transaction.commit().await.map_err(|err| {
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
/// Request body for updating a tally ceremony.
pub struct UpdateTallyCeremonyInput {
    /// The election event ID.
    election_event_id: String,
    /// The tally session ID.
    tally_session_id: String,
    /// The status.
    status: TallyExecutionStatus,
}

/// Updates a tally ceremony.
#[instrument(skip(claims))]
#[allow(clippy::too_many_lines)]
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

    let user_id = claims.clone().hasura_claims.user_id;
    let username = claims
        .clone()
        .preferred_username
        .unwrap_or(claims.name.clone().unwrap_or_else(|| user_id.clone()));

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
        .clone()
        .tally_type
        .map(|val: String| {
            TallyType::try_from(val.as_str()).unwrap_or_default()
        })
        .unwrap_or_default();

    let is_tally_allowed = get_elections_by_ids(
        &hasura_transaction,
        &tenant_id,
        &input.election_event_id,
        &tally_session.election_ids.clone().unwrap_or(vec![]),
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
                            && (election_status.voting_status.is_closed()
                                && election_status
                                    .kiosk_voting_status
                                    .is_closed_or_never_started()
                                && election_status
                                    .early_voting_status
                                    .is_closed_or_never_started()))
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

    if !is_tally_allowed {
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
        tally_session.clone(),
        input.status.clone(),
        user_id.clone(),
        username.clone(),
    )
    .await
    .map_err(|e| {
        (
            Status::InternalServerError,
            format!("Error with update_tally_ceremony: {e:?}"),
        )
    })?;

    hasura_transaction.commit().await.map_err(|err| {
        (Status::InternalServerError, format!("Commit failed: {err}"))
    })?;

    Ok(Json(CreateTallyCeremonyOutput {
        tally_session_id: input.tally_session_id.clone(),
    }))
}

////////////////////////////////////////////////////////////////////////////////
/// Endpoint: /restore-private-key
////////////////////////////////////////////////////////////////////////////////

#[derive(Serialize, Deserialize, Debug)]
/// Request body for restoring a private key.
pub struct SetPrivateKeyInput {
    /// The election event ID.
    election_event_id: String,
    /// The private key base64.
    private_key_base64: String,
    /// The tally session ID.
    tally_session_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
/// Response body for restoring a private key.
pub struct SetPrivateKeyOutput {
    /// Whether the private key is valid.
    is_valid: bool,
}

/// The main function to restore the private key
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
    .map_err(|e| (Status::BadRequest, format!("{e:?}")))?;
    event!(
        Level::INFO,
        "Restoring given private key, election_event_id={}, tally_session_id={}, is_valid={}",
        input.election_event_id,
        input.tally_session_id,
        is_valid,
    );

    hasura_transaction.commit().await.map_err(|err| {
        (Status::InternalServerError, format!("Commit failed: {err}"))
    })?;
    Ok(Json(SetPrivateKeyOutput { is_valid }))
}

#[derive(Serialize, Deserialize, Debug)]
/// Request body for submitting a tally resolution.
pub struct SubmitTallyResolutionInput {
    /// The election event ID.
    election_event_id: String,
    /// The tally session ID.
    tally_session_id: String,
    /// The resolutions.
    resolutions: Vec<TallyResolution>,
}

#[derive(Serialize, Deserialize, Debug)]
/// Response body for submitting a tally resolution.
pub struct SubmitTallyResolutionOutput {
    /// Whether the submission was successful.
    success: bool,
    /// The tally session ID.
    tally_session_id: String,
    /// The number of resolutions submitted.
    resolved_count: usize,
}

/// Submit multiple tally resolutions for a paused tally (batch operation)
#[instrument(skip(claims))]
#[post("/submit-tally-resolution", format = "json", data = "<body>")]
pub async fn submit_tally_resolution(
    body: Json<SubmitTallyResolutionInput>,
    claims: JwtClaims,
) -> Result<Json<SubmitTallyResolutionOutput>, (Status, String)> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::TALLY_RESOLUTION_SUBMIT],
    )?;

    let input = body.into_inner();
    let tenant_id = claims.hasura_claims.tenant_id.clone();
    let user_id = claims.hasura_claims.user_id.clone();

    if input.resolutions.is_empty() {
        return Err((
            Status::BadRequest,
            "At least one resolution required".to_string(),
        ));
    }

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

    let resolved_count = tally_resolution::submit_tally_resolution(
        &hasura_transaction,
        &tenant_id,
        &input.election_event_id,
        &input.tally_session_id,
        &input.resolutions,
        &user_id,
        claims.preferred_username.clone(),
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;
    hasura_transaction.commit().await.map_err(|err| {
        (Status::InternalServerError, format!("Commit failed: {err}"))
    })?;

    event!(
        Level::INFO,
        "Batch tally resolution submission completed for tally session {}, resolved {} contest(s)",
        input.tally_session_id,
        resolved_count
    );

    Ok(Json(SubmitTallyResolutionOutput {
        success: true,
        tally_session_id: input.tally_session_id,
        resolved_count,
    }))
}
