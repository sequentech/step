// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The Hasura actions of Vitrine (step-integration.md, section 2.14): the
//! booth's context and private events' forwards for voters, and the
//! Transparency tab for administrators.

use crate::services::authorization::{authorize, authorize_voter_event};
use crate::types::error_response::{ErrorCode, ErrorResponse, JsonError};
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::{Permissions, VoterPermissions};
use tracing::{error, instrument, warn};
use windmill::services::database::get_hasura_pool;
use windmill::services::vitrine::client::{Service, SidecarClient};
use windmill::services::vitrine::config::{DeploymentConfig, CONFIG_FILE_ENV};
use windmill::services::vitrine::transparency::{self, TransparencyError};
use windmill::services::vitrine::voter::{self, ForwardError};
use windmill::types::vitrine::{
    EnableTransparencyInput, GetTransparencyInput, TransparencyOutput,
    UploadTransparencyPolicyInput, VitrineForwardOutput, VitrineVoterContext,
    VitrineVoterContextInput, VitrineVoterForwardInput,
};

fn internal_error(error: impl std::fmt::Debug) -> JsonError {
    error!("Vitrine action failed: {error:?}");
    ErrorResponse::new(
        Status::InternalServerError,
        ErrorCode::InternalServerError.as_ref(),
        ErrorCode::InternalServerError,
    )
}

fn unauthorized((status, message): (Status, String)) -> JsonError {
    ErrorResponse::new(status, &message, ErrorCode::Unauthorized)
}

async fn hasura_client() -> Result<DbClient, JsonError> {
    get_hasura_pool().await.get().await.map_err(internal_error)
}

/// The sidecar's internal listener with the admin token, if the deployment
/// has one.
fn admin_sidecar() -> Option<SidecarClient> {
    SidecarClient::from_env(Service::Harvest)
        .map_err(|e| {
            warn!("No Vitrine sidecar for the Transparency tab: {e:#}")
        })
        .ok()
}

/// The sidecar's internal listener, for voters' forwards: no admin token.
fn voter_sidecar() -> Option<SidecarClient> {
    let url = std::env::var(Service::Harvest.internal_url_env()).ok()?;
    SidecarClient::new(&url, None)
        .map_err(|e| error!("{e:#}"))
        .ok()
}

/// The deployment configuration, if `HARVEST_VITRINE_CONFIG_FILE` names one.
fn deployment_config() -> Result<Option<DeploymentConfig>, JsonError> {
    match std::env::var(CONFIG_FILE_ENV) {
        Ok(path) if !path.is_empty() => {
            DeploymentConfig::load(&path).map(Some).map_err(|e| {
                error!("{e:#}");
                ErrorResponse::new(
                    Status::InternalServerError,
                    &format!("{e:#}"),
                    ErrorCode::VitrineConfigurationInvalid,
                )
            })
        }
        _ => Ok(None),
    }
}

fn transparency_error(error: TransparencyError) -> JsonError {
    match error {
        TransparencyError::Refused(why) => ErrorResponse::new(
            Status::Conflict,
            &why,
            ErrorCode::TransparencyRefused,
        ),
        TransparencyError::Sidecar(e) => {
            warn!("{e}");
            ErrorResponse::new(
                Status::BadGateway,
                &e.to_string(),
                ErrorCode::VitrineUnavailable,
            )
        }
        TransparencyError::Internal(e) => internal_error(e),
    }
}

fn forward_error(error: ForwardError) -> JsonError {
    match error {
        ForwardError::BadRequest(why) => ErrorResponse::new(
            Status::BadRequest,
            &why,
            ErrorCode::InvalidVitrineForward,
        ),
        ForwardError::Refused(why) => ErrorResponse::new(
            Status::Forbidden,
            &why,
            ErrorCode::VitrineForwardRefused,
        ),
        ForwardError::Sidecar(e) => {
            warn!("{e}");
            ErrorResponse::new(
                Status::BadGateway,
                ErrorCode::VitrineUnavailable.as_ref(),
                ErrorCode::VitrineUnavailable,
            )
        }
        ForwardError::Internal(e) => internal_error(e),
    }
}

fn authorize_transparency_tab(claims: &JwtClaims) -> Result<(), JsonError> {
    authorize(
        claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::ELECTION_EVENT_TRANSPARENCY_TAB],
    )
    .map_err(unauthorized)
}

/// What the booth must hold before it encrypts in the voter's election.
#[instrument(skip(claims))]
#[post("/vitrine/voter-context", format = "json", data = "<body>")]
pub async fn vitrine_voter_context(
    body: Json<VitrineVoterContextInput>,
    claims: JwtClaims,
) -> Result<Json<VitrineVoterContext>, JsonError> {
    let input = body.into_inner();
    let (_, election_ids) = authorize_voter_event(
        &claims,
        vec![VoterPermissions::CAST_VOTE],
        &input.election_event_id,
    )
    .map_err(unauthorized)?;
    if !election_ids.contains(&input.election_id) {
        return Err(unauthorized((
            Status::Forbidden,
            "Voter not authorized".into(),
        )));
    }
    let mut db = hasura_client().await?;
    voter::voter_context(
        &mut db,
        &claims.hasura_claims.tenant_id,
        &input.election_event_id,
        &input.election_id,
    )
    .await
    .map(Json)
    .map_err(internal_error)
}

/// A private event's receipt upgrade or opening, for one of the voter's own
/// ballots.
#[instrument(skip(claims, body))]
#[post("/vitrine/voter-forward", format = "json", data = "<body>")]
pub async fn vitrine_voter_forward(
    body: Json<VitrineVoterForwardInput>,
    claims: JwtClaims,
) -> Result<Json<VitrineForwardOutput>, JsonError> {
    let input = body.into_inner();
    authorize_voter_event(
        &claims,
        vec![VoterPermissions::CAST_VOTE],
        &input.election_event_id,
    )
    .map_err(unauthorized)?;
    let sidecar = voter_sidecar();
    let mut db = hasura_client().await?;
    voter::voter_forward(
        &mut db,
        sidecar.as_ref(),
        &claims.hasura_claims.tenant_id,
        &claims.hasura_claims.user_id,
        &input,
    )
    .await
    .map(Json)
    .map_err(forward_error)
}

#[instrument(skip(claims))]
#[post("/vitrine/transparency", format = "json", data = "<body>")]
pub async fn get_transparency(
    body: Json<GetTransparencyInput>,
    claims: JwtClaims,
) -> Result<Json<TransparencyOutput>, JsonError> {
    authorize_transparency_tab(&claims)?;
    let config = deployment_config()?;
    let sidecar = admin_sidecar();
    let mut db = hasura_client().await?;
    let tx = db.transaction().await.map_err(internal_error)?;
    let output = transparency::transparency_output(
        &tx,
        sidecar.as_ref(),
        config.as_ref(),
        &claims.hasura_claims.tenant_id,
        &body.election_event_id,
    )
    .await
    .map_err(transparency_error)?;
    tx.commit().await.map_err(internal_error)?;
    Ok(Json(output))
}

#[instrument(skip(claims))]
#[post("/vitrine/enable-transparency", format = "json", data = "<body>")]
pub async fn enable_transparency(
    body: Json<EnableTransparencyInput>,
    claims: JwtClaims,
) -> Result<Json<TransparencyOutput>, JsonError> {
    authorize_transparency_tab(&claims)?;
    let config = deployment_config()?;
    let sidecar = admin_sidecar();
    let mut db = hasura_client().await?;
    transparency::enable_transparency(
        &mut db,
        sidecar.as_ref(),
        config.as_ref(),
        &claims.hasura_claims.tenant_id,
        &body.election_event_id,
        body.mode,
    )
    .await
    .map(Json)
    .map_err(transparency_error)
}

#[instrument(skip(claims, body))]
#[post(
    "/vitrine/upload-transparency-policy",
    format = "json",
    data = "<body>"
)]
pub async fn upload_transparency_policy(
    body: Json<UploadTransparencyPolicyInput>,
    claims: JwtClaims,
) -> Result<Json<TransparencyOutput>, JsonError> {
    authorize_transparency_tab(&claims)?;
    let config = deployment_config()?;
    let sidecar = admin_sidecar();
    let mut db = hasura_client().await?;
    transparency::upload_transparency_policy(
        &mut db,
        sidecar.as_ref(),
        config.as_ref(),
        &claims.hasura_claims.tenant_id,
        &body.election_event_id,
        &body.signed_policy,
    )
    .await
    .map(Json)
    .map_err(transparency_error)
}
