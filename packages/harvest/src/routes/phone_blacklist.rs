// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::authorization::authorize;
use crate::types::error_response::{ErrorCode, ErrorResponse};
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::hasura::core::PhoneBlacklistEntry;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use windmill::postgres::phone_blacklist as pg_phone_blacklist;
use windmill::services::database::get_hasura_pool;

#[derive(Serialize, Deserialize, Debug)]
pub struct CreatePhoneBlacklistEntryInput {
    election_event_id: String,
    phone_e164: String,
    reason: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DeletePhoneBlacklistEntryInput {
    id: String,
    election_event_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DeletePhoneBlacklistEntryOutput {
    id: String,
}

#[instrument(skip(claims, input))]
#[post("/create-phone-blacklist-entry", format = "json", data = "<input>")]
pub async fn create_phone_blacklist_entry(
    claims: JwtClaims,
    input: Json<CreatePhoneBlacklistEntryInput>,
) -> Result<Json<PhoneBlacklistEntry>, (Status, String)> {
    let tenant_id_str = claims.hasura_claims.tenant_id.clone();
    authorize(
        &claims,
        true,
        Some(tenant_id_str.clone()),
        vec![Permissions::PHONE_BLACKLIST_CREATE],
    )?;

    let body = input.into_inner();
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

    let entry = pg_phone_blacklist::insert_phone_blacklist_entry(
        &hasura_transaction,
        &tenant_id_str,
        &body.election_event_id,
        &body.phone_e164,
        body.reason.as_ref(),
        &claims.hasura_claims.user_id,
    )
    .await
    .map_err(|e| {
        (
            Status::InternalServerError,
            format!("Failed to insert the entry: {e:?}"),
        )
    })?;

    hasura_transaction.commit().await.map_err(|err| {
        (
            Status::InternalServerError,
            format!("Failed to commit the transaction: {e:?}"),
        )
    })?;

    Ok(Json(entry))
}

#[instrument(skip(claims, input))]
#[post("/delete-phone-blacklist-entry", format = "json", data = "<input>")]
pub async fn delete_phone_blacklist_entry(
    claims: JwtClaims,
    input: Json<DeletePhoneBlacklistEntryInput>,
) -> Result<Json<DeletePhoneBlacklistEntryOutput>, (Status, String)> {
    let tenant_id_str = claims.hasura_claims.tenant_id.clone();
    authorize(
        &claims,
        true,
        Some(tenant_id_str.clone()),
        vec![Permissions::PHONE_BLACKLIST_DELETE],
    )?;

    let body = input.into_inner();
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

    pg_phone_blacklist::delete_phone_blacklist_entry(
        &hasura_transaction,
        &tenant_id_str,
        &body.election_event_id,
        &body.id,
    )
    .await
    .map_err(|e| {
        (
            Status::InternalServerError,
            format!("Failed to delete the entry: {e:?}"),
        )
    })?;
    hasura_transaction.commit().await.map_err(|err| {
        (
            Status::InternalServerError,
            format!("Failed to commit the transaction: {e:?}"),
        )
    })?;

    Ok(Json(DeletePhoneBlacklistEntryOutput { id: body.id }))
}
