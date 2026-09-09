// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::authorization::authorize;
use anyhow::{anyhow, Context};
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::hasura::core::PhoneBlacklistEntry;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use windmill::postgres::election_event::get_election_event_by_id;
use windmill::postgres::phone_blacklist as pg_phone_blacklist;
use windmill::services::database::get_hasura_pool;
use windmill::services::election_event_board::get_election_event_board;
use windmill::services::electoral_log::ElectoralLog;

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

    let mut hasura_db_client: DbClient = get_hasura_pool()
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
    async {
        let event = get_election_event_by_id(
            &hasura_transaction,
            tenant_id,
            &body.election_event_id,
        )
        .await?;
        let electoral_log = ElectoralLog::for_admin_user(
            &hasura_transaction,
            &get_election_event_board(event.bulletin_board_reference)
                .ok_or(anyhow!("missing board"))?,
            tenant_id,
            event_id,
            user_id,
            claims.preferred_username.clone(),
            None,
            None,
        )
        .await?;
        electoral_log
            .post_phone_blacklist_entry_created(
                event_id.clone(),
                body.phone_e164.clone(),
                Some(user_id.clone()),
                claims.preferred_username,
            )
            .await?;
        anyhow::Ok(())
    }
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

#[instrument(skip(claims, input))]
#[post("/delete-phone-blacklist-entry", format = "json", data = "<input>")]
pub async fn delete_phone_blacklist_entry(
    claims: JwtClaims,
    input: Json<DeletePhoneBlacklistEntryInput>,
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

    let mut hasura_db_client: DbClient = get_hasura_pool()
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
    async {
        let event =
            get_election_event_by_id(&hasura_transaction, tenant_id, event_id)
                .await?;
        let electoral_log = ElectoralLog::for_admin_user(
            &hasura_transaction,
            &get_election_event_board(event.bulletin_board_reference)
                .ok_or(anyhow!("missing board"))?,
            tenant_id,
            event_id,
            user_id,
            claims.preferred_username.clone(),
            None,
            None,
        )
        .await?;
        electoral_log
            .post_phone_blacklist_entry_deleted(
                event_id.clone(),
                deleted.phone_e164,
                Some(user_id.clone()),
                claims.preferred_username,
            )
            .await?;
        anyhow::Ok(())
    }
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
