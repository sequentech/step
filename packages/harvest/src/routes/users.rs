// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::authorization::authorize;
use crate::types::resources::{
    Aggregate, DataList, OrderDirection, TotalAggregate,
};
use anyhow::{anyhow, Result};
use sequent_core::services::keycloak::{get_event_realm, get_tenant_realm};
use rocket::http::Status;
use rocket::response::Debug;
use rocket::serde::json::Json;
use sequent_core::services::jwt;
use sequent_core::services::keycloak::KeycloakAdminClient;
use sequent_core::types::keycloak::User;
use serde::{Deserialize, Serialize};
use tracing::{event, instrument, Level};

#[derive(Deserialize, Debug)]
pub struct GetUsersBody {
    tenant_id: String,
    election_event_id: Option<String>,
    search: Option<String>,
    email: Option<String>,
    limit: Option<i32>,
    offset: Option<i32>,
}

#[instrument(skip(claims))]
#[post("/get-users", format = "json", data = "<body>")]
pub async fn get_users(
    claims: jwt::JwtClaims,
    body: Json<GetUsersBody>,
) -> Result<Json<DataList<User>>, (Status, String)> {
    let input = body.into_inner();
    let required_perm: String = if input.election_event_id.is_some() {
        "read-event-users".into()
    } else {
        "read-users".into()
    };
    authorize(
        &claims,
        true,
        Some(input.tenant_id.clone()),
        vec!["read-users".into()],
    )?;
    let realm = match input.election_event_id {
        Some(election_event_id) =>  get_event_realm(&input.tenant_id, &election_event_id),
        None => get_tenant_realm(&input.tenant_id),
    };
    let client = KeycloakAdminClient::new()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    let (users, count) = client
        .list_users(&realm, input.search, input.email, input.limit, input.offset)
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    Ok(Json(DataList {
        items: users,
        total: TotalAggregate {
            aggregate: Aggregate {
                count: count as i64,
            },
        },
    }))
}
