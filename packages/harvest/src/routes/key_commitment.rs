// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use windmill::services::database::get_hasura_pool;
use windmill::postgres::trustee_commitment;

#[derive(Serialize, Deserialize, Debug)]
pub struct RecordKeyCommitmentInput {
    pub election_event_id: String,
    pub trustee_name: String,
    pub salt_b64: String,
    pub iterations: i32,
    pub hash_b64: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RecordKeyCommitmentOutput {
    pub success: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VerifyKeyCommitmentInput {
    pub election_event_id: String,
    pub trustee_name: String,
    pub salt_b64: String,
    pub iterations: i32,
    pub hash_b64: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VerifyKeyCommitmentOutput {
    pub is_valid: bool,
}

#[instrument(skip(claims))]
#[post("/record-key-commitment", format = "json", data = "<body>")]
pub async fn record_key_commitment_route(
    claims: JwtClaims,
    body: Json<RecordKeyCommitmentInput>,
) -> Result<Json<RecordKeyCommitmentOutput>, (Status, String)> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::TRUSTEE_CEREMONY],
    )?;

    let inner = body.into_inner();
    let tenant_id = &claims.hasura_claims.tenant_id;

    let mut hasura_db_client: DbClient = get_hasura_pool().await.get().await.map_err(|err| {
        (
            Status::InternalServerError,
            format!("Error loading hasura db client: {err}"),
        )
    })?;
    let hasura_transaction = hasura_db_client.transaction().await.map_err(|err| {
        (
            Status::InternalServerError,
            format!("Error creating a transaction: {err}"),
        )
    })?;

    trustee_commitment::record_key_commitment(
        &hasura_transaction,
        tenant_id,
        &inner.trustee_name,
        &inner.election_event_id,
        &inner.salt_b64,
        inner.iterations,
        &inner.hash_b64,
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

    hasura_transaction.commit().await.map_err(|err| {
        (
            Status::InternalServerError,
            format!("Failed to commit Hasura transaction: {err}"),
        )
    })?;

    Ok(Json(RecordKeyCommitmentOutput { success: true }))
}

#[instrument(skip(claims))]
#[post("/verify-key-commitment", format = "json", data = "<body>")]
pub async fn verify_key_commitment_route(
    claims: JwtClaims,
    body: Json<VerifyKeyCommitmentInput>,
) -> Result<Json<VerifyKeyCommitmentOutput>, (Status, String)> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::TRUSTEE_CEREMONY],
    )?;

    let inner = body.into_inner();
    let tenant_id = &claims.hasura_claims.tenant_id;

    let mut hasura_db_client: DbClient = get_hasura_pool().await.get().await.map_err(|err| {
        (
            Status::InternalServerError,
            format!("Error loading hasura db client: {err}"),
        )
    })?;
    let hasura_transaction = hasura_db_client.transaction().await.map_err(|err| {
        (
            Status::InternalServerError,
            format!("Error creating a transaction: {err}"),
        )
    })?;

    let is_valid = trustee_commitment::verify_key_commitment(
        &hasura_transaction,
        tenant_id,
        &inner.trustee_name,
        &inner.election_event_id,
        &inner.salt_b64,
        inner.iterations,
        &inner.hash_b64,
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

    hasura_transaction.commit().await.map_err(|err| {
        (
            Status::InternalServerError,
            format!("Failed to commit Hasura transaction: {err}"),
        )
    })?;

    Ok(Json(VerifyKeyCommitmentOutput { is_valid }))
}

