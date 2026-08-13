// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::authorization::authorize;
use anyhow::Result;
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::ballot::SupportMaterialsPolicy;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use windmill::postgres::election_event::get_election_event_by_id;
use windmill::services::database::get_hasura_pool;
use windmill::services::support_materials::{
    acknowledge_support_materials, get_support_materials_acknowledgment,
};

#[derive(Serialize, Deserialize, Debug)]
pub struct AcknowledgeSupportMaterialsInput {
    pub election_event_id: String,
    pub document_ids: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AcknowledgeSupportMaterialsOutput {
    pub election_event_id: String,
    pub document_ids: Vec<String>,
}

/// Records that the calling voter has read and acknowledged the Election
/// Event's Support Materials.
#[instrument(skip_all)]
#[post("/acknowledge-support-materials", format = "json", data = "<body>")]
pub async fn acknowledge_support_materials_route(
    body: Json<AcknowledgeSupportMaterialsInput>,
    claims: JwtClaims,
) -> Result<Json<AcknowledgeSupportMaterialsOutput>, (Status, String)> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::DOCUMENT_DOWNLOAD],
    )?;

    let input = body.into_inner();
    let tenant_id = claims.hasura_claims.tenant_id.clone();
    let voter_id = claims.hasura_claims.user_id.clone();

    if claims.hasura_claims.election_event_id.as_deref()
        != Some(input.election_event_id.as_str())
    {
        return Err((
            Status::Forbidden,
            "The election event does not match the voter's session".to_string(),
        ));
    }

    let mut hasura_db_client: DbClient =
        get_hasura_pool().await.get().await.map_err(|e| {
            (
                Status::InternalServerError,
                format!("Error acquiring hasura db client from pool {:?}", e),
            )
        })?;
    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    let election_event = get_election_event_by_id(
        &hasura_transaction,
        &tenant_id,
        &input.election_event_id,
    )
    .await
    .map_err(|e| {
        (
            Status::NotFound,
            format!("Election event not found: {:?}", e),
        )
    })?;

    hasura_transaction
        .commit()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    if election_event.effective_support_materials_policy()
        == SupportMaterialsPolicy::Off
    {
        return Err((
            Status::BadRequest,
            "Support materials are disabled for this election event"
                .to_string(),
        ));
    }

    let realm = get_event_realm(&tenant_id, &input.election_event_id);
    acknowledge_support_materials(
        &realm,
        &voter_id,
        input.document_ids.clone(),
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    Ok(Json(AcknowledgeSupportMaterialsOutput {
        election_event_id: input.election_event_id,
        document_ids: input.document_ids,
    }))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GetSupportMaterialsAcknowledgmentInput {
    pub election_event_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GetSupportMaterialsAcknowledgmentOutput {
    pub document_ids: Vec<String>,
}

/// Returns the Support Material document ids the calling voter has already
/// acknowledged for this Election Event (empty if none). Used by the Voting
/// Portal to skip the Ballot list gate when Support Materials Policy is
/// Mandatory for Voting and the voter already acknowledged in a previous
/// session.
#[instrument(skip_all)]
#[post(
    "/get-support-materials-acknowledgment",
    format = "json",
    data = "<body>"
)]
pub async fn get_support_materials_acknowledgment_route(
    body: Json<GetSupportMaterialsAcknowledgmentInput>,
    claims: JwtClaims,
) -> Result<Json<GetSupportMaterialsAcknowledgmentOutput>, (Status, String)> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::DOCUMENT_DOWNLOAD],
    )?;

    let input = body.into_inner();
    let tenant_id = claims.hasura_claims.tenant_id.clone();
    let voter_id = claims.hasura_claims.user_id.clone();

    if claims.hasura_claims.election_event_id.as_deref()
        != Some(input.election_event_id.as_str())
    {
        return Err((
            Status::Forbidden,
            "The election event does not match the voter's session".to_string(),
        ));
    }

    let realm = get_event_realm(&tenant_id, &input.election_event_id);
    let document_ids = get_support_materials_acknowledgment(&realm, &voter_id)
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    Ok(Json(GetSupportMaterialsAcknowledgmentOutput {
        document_ids,
    }))
}
