// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::ports::electoral_log::{
    PhoneBlacklistChange, PhoneBlacklistEntryLog,
};
use crate::services::authorization::authorize;
use crate::services::dependencies::HarvestServices;
use crate::types::error_response::{ErrorCode, ErrorResponse};
use anyhow::Context;
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::hasura::core::PhoneBlacklistEntry;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use windmill::postgres::phone_blacklist as pg_phone_blacklist;

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

#[instrument(skip(claims, input, services))]
#[post("/create-phone-blacklist-entry", format = "json", data = "<input>")]
pub async fn create_phone_blacklist_entry(
    claims: JwtClaims,
    input: Json<CreatePhoneBlacklistEntryInput>,
    services: &State<HarvestServices>,
) -> Result<Json<PhoneBlacklistEntry>, (Status, String)> {
    let body = input.into_inner();
    let tenant_id = &claims.hasura_claims.tenant_id;
    let event_id = &body.election_event_id;
    let user_id = &claims.hasura_claims.user_id;

    authorize(
        &claims,
        true,
        Some(tenant_id.clone()),
        vec![Permissions::PHONE_BLACKLIST_CREATE],
    )?;

    let mut hasura_db_client: DbClient = services
        .databases
        .hasura()
        .await
        .get()
        .await
        .context("Failed to get client from the db pool")
        .map_err(|e| {
            tracing::error!("{e:?}");
            (Status::InternalServerError, format!("{e:?}"))
        })?;

    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .context("Failed to start transaction")
        .map_err(|e| {
            tracing::error!("{e:?}");
            (Status::InternalServerError, format!("{e:?}"))
        })?;

    // Insert the entry
    let entry = pg_phone_blacklist::insert_phone_blacklist_entry(
        &hasura_transaction,
        tenant_id,
        event_id,
        &body.phone_e164,
        body.reason.as_ref(),
        user_id,
    )
    .await
    .map_err(|e| {
        (
            Status::InternalServerError,
            format!("Failed to insert the entry: {e:?}"),
        )
    })?;

    // Post the electoral log
    services
        .electoral_log
        .phone_blacklist_entry(
            &hasura_transaction,
            PhoneBlacklistEntryLog {
                change: PhoneBlacklistChange::Created,
                tenant_id,
                election_event_id: event_id,
                user_id,
                username: claims.preferred_username,
                phone_e164: body.phone_e164.clone(),
            },
        )
        .await
        .context("Failed to post the electoral log message")
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

    hasura_transaction.commit().await.map_err(|e| {
        (
            Status::InternalServerError,
            format!("Failed to commit the transaction: {e:?}"),
        )
    })?;

    Ok(Json(entry))
}

#[instrument(skip(claims, input, services))]
#[post("/delete-phone-blacklist-entry", format = "json", data = "<input>")]
pub async fn delete_phone_blacklist_entry(
    claims: JwtClaims,
    input: Json<DeletePhoneBlacklistEntryInput>,
    services: &State<HarvestServices>,
) -> Result<Json<DeletePhoneBlacklistEntryOutput>, (Status, String)> {
    let body = input.into_inner();
    let tenant_id = &claims.hasura_claims.tenant_id;
    let event_id = &body.election_event_id;
    let entry_id = &body.id;
    let user_id = &claims.hasura_claims.user_id;

    authorize(
        &claims,
        true,
        Some(tenant_id.clone()),
        vec![Permissions::PHONE_BLACKLIST_DELETE],
    )?;

    let mut hasura_db_client: DbClient = services
        .databases
        .hasura()
        .await
        .get()
        .await
        .context("Failed to get client from the db pool")
        .map_err(|e| {
            tracing::error!("{e:?}");
            (Status::InternalServerError, format!("{e:?}"))
        })?;

    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .context("Failed to start transaction")
        .map_err(|e| {
            tracing::error!("{e:?}");
            (Status::InternalServerError, format!("{e:?}"))
        })?;

    // Delete the entry
    let deleted = pg_phone_blacklist::delete_phone_blacklist_entry(
        &hasura_transaction,
        tenant_id,
        event_id,
        entry_id,
    )
    .await
    .map_err(|e| {
        (
            Status::InternalServerError,
            format!("Failed to delete the entry: {e:?}"),
        )
    })?;

    // Post the electoral log
    services
        .electoral_log
        .phone_blacklist_entry(
            &hasura_transaction,
            PhoneBlacklistEntryLog {
                change: PhoneBlacklistChange::Deleted,
                tenant_id,
                election_event_id: event_id,
                user_id,
                username: claims.preferred_username,
                phone_e164: deleted.phone_e164,
            },
        )
        .await
        .context("Failed to post the electoral log message")
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

    // Commit the transaction
    hasura_transaction.commit().await.map_err(|e| {
        (
            Status::InternalServerError,
            format!("Failed to commit the transaction: {e:?}"),
        )
    })?;

    Ok(Json(DeletePhoneBlacklistEntryOutput { id: body.id }))
}
