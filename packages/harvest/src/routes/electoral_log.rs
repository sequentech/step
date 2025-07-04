// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use crate::types::resources::TotalAggregate;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Client as DbClient;
use electoral_log::client::types::*;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::types::permissions::Permissions;
use tracing::{info, instrument};
use windmill::services::database::get_keycloak_pool;
use windmill::services::electoral_log::{
    count_electoral_log, list_electoral_log as windmill_list_electoral_log,
    ElectoralLogRow,
};
use windmill::services::users::get_users_by_username;
use windmill::types::resources::DataList;

#[instrument(skip(claims))]
#[post("/immudb/electoral-log", format = "json", data = "<body>")]
pub async fn list_electoral_log(
    body: Json<GetElectoralLogBody>,
    claims: JwtClaims,
) -> Result<Json<DataList<ElectoralLogRow>>, (Status, String)> {
    let mut input = body.into_inner();
    authorize(
        &claims,
        true,
        Some(input.tenant_id.clone()),
        vec![Permissions::LOGS_READ],
    )?;

    // If there is username but no user_id in the filter, fill the user_id to
    // inprove performance.
    if let Some(filter) = &mut input.filter {
        if let (Some(username), None) = (
            filter.get(&OrderField::Username),
            filter.get(&OrderField::UserId),
        ) {
            match get_user_id(
                &input.tenant_id,
                &input.election_event_id,
                username,
            )
            .await
            {
                Ok(Some(user_id)) => {
                    filter.insert(OrderField::UserId, user_id);
                }
                Ok(None) => {
                    return Ok(Json(DataList::default()));
                }
                Err(e) => {
                    return Err((Status::InternalServerError, e.to_string()));
                }
            }
        }
    }

    let (data_res, count_res) = tokio::join!(
        windmill_list_electoral_log(input.clone()),
        count_electoral_log(input)
    );

    let mut data = data_res.map_err(|e| {
        (
            Status::InternalServerError,
            format!("Eror listing electoral log: {e:?}"),
        )
    })?;
    data.total.aggregate.count = count_res.map_err(|e| {
        (
            Status::InternalServerError,
            format!("Error counting electoral log: {e:?}"),
        )
    })?;

    Ok(Json(data))
}

/// Get user id by username
#[instrument(err)]
pub async fn get_user_id(
    tenant_id: &str,
    election_event_id: &str,
    username: &str,
) -> Result<Option<String>> {
    let realm = get_event_realm(tenant_id, election_event_id);
    let mut keycloak_db_client: DbClient = get_keycloak_pool()
        .await
        .get()
        .await
        .map_err(|e| anyhow!("Error getting keycloak client: {e:?}"))?;

    let keycloak_transaction = keycloak_db_client
        .transaction()
        .await
        .map_err(|e| anyhow!("Error getting keycloak transaction: {e:?}"))?;

    let user_ids =
        get_users_by_username(&keycloak_transaction, &realm, username)
            .await
            .map_err(|e| anyhow!("Error getting users by username: {e:?}"))?;

    match user_ids.len() {
        0 => {
            info!("Could not get users by username: Not Found");
            return Ok(None);
        }
        1 => Ok(Some(user_ids[0].clone())),
        _ => {
            return Err(anyhow!(
                "Error getting users by username: Multiple users Found"
            ));
        }
    }
}
