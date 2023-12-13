// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::authorization::authorize;
use crate::types::optional::OptionalId;
use crate::types::resources::{Aggregate, DataList, TotalAggregate};
use anyhow::Result;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt;
use sequent_core::services::keycloak;
use sequent_core::services::keycloak::KeycloakAdminClient;
use sequent_core::services::keycloak::{get_event_realm, get_tenant_realm};
use sequent_core::types::keycloak::{User, TENANT_ID_ATTR_NAME};
use sequent_core::types::permissions::Permissions;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use tracing::instrument;
use windmill::services::users::list_users;

#[derive(Deserialize, Debug)]
pub struct DeleteUserBody {
    tenant_id: String,
    election_event_id: Option<String>,
    user_id: String,
}

#[instrument(skip(claims))]
#[post("/delete-user", format = "json", data = "<body>")]
pub async fn delete_user(
    claims: jwt::JwtClaims,
    body: Json<DeleteUserBody>,
) -> Result<Json<OptionalId>, (Status, String)> {
    let input = body.into_inner();
    let required_perm: Permissions = if input.election_event_id.is_some() {
        Permissions::VOTER_WRITE
    } else {
        Permissions::USER_WRITE
    };
    authorize(
        &claims,
        true,
        Some(input.tenant_id.clone()),
        vec![required_perm],
    )?;
    let realm = match input.election_event_id {
        Some(election_event_id) => {
            get_event_realm(&input.tenant_id, &election_event_id)
        }
        None => get_tenant_realm(&input.tenant_id),
    };
    let client = KeycloakAdminClient::new()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    client
        .delete_user(&realm, &input.user_id)
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    Ok(Json(Default::default()))
}

#[derive(Deserialize, Debug)]
pub struct DeleteUsersBody {
    tenant_id: String,
    election_event_id: Option<String>,
    user_id: Array<String>,
}

#[instrument(skip(claims))]
#[post("/delete-users", format = "json", data = "<body>")]
pub async fn delete_user(
    claims: jwt::JwtClaims,
    body: Json<DeleteUsersBody>,
) -> Result<Json<OptionalId>, (Status, String)> {
    let input = body.into_inner();
    let required_perm: Permissions = if input.election_event_id.is_some() {
        Permissions::VOTER_WRITE
    } else {
        Permissions::USER_WRITE
    };
    authorize(
        &claims,
        true,
        Some(input.tenant_id.clone()),
        vec![required_perm],
    )?;
    let realm = match input.election_event_id {
        Some(election_event_id) => {
            get_event_realm(&input.tenant_id, &election_event_id)
        }
        None => get_tenant_realm(&input.tenant_id),
    };
    let client = KeycloakAdminClient::new()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    
    input.user_ids.for_each(|id|{
        client
            .delete_user(&realm, &id)
            .await
            .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    });
        
    Ok(Json(Default::default()))
}

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
    let required_perm: Permissions = if input.election_event_id.is_some() {
        Permissions::VOTER_READ
    } else {
        Permissions::USER_READ
    };
    authorize(
        &claims,
        true,
        Some(input.tenant_id.clone()),
        vec![required_perm],
    )?;
    let auth_headers = keycloak::get_client_credentials()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    let realm = match input.election_event_id {
        Some(ref election_event_id) => {
            get_event_realm(&input.tenant_id, &election_event_id)
        }
        None => get_tenant_realm(&input.tenant_id),
    };
    let client = KeycloakAdminClient::new()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    let (users, count) = list_users(
        auth_headers.clone(),
        &client,
        input.tenant_id.clone(),
        input.election_event_id.clone(),
        &realm,
        input.search,
        input.email,
        input.limit,
        input.offset,
    )
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

#[derive(Deserialize, Debug)]
pub struct CreateUserBody {
    tenant_id: String,
    election_event_id: Option<String>,
    user: User,
}

#[instrument(skip(claims))]
#[post("/create-user", format = "json", data = "<body>")]
pub async fn create_user(
    claims: jwt::JwtClaims,
    body: Json<CreateUserBody>,
) -> Result<Json<User>, (Status, String)> {
    let input = body.into_inner();
    let required_perm: Permissions = if input.election_event_id.is_some() {
        Permissions::VOTER_CREATE
    } else {
        Permissions::USER_CREATE
    };
    authorize(
        &claims,
        true,
        Some(input.tenant_id.clone()),
        vec![required_perm],
    )?;
    let realm = match input.election_event_id {
        Some(election_event_id) => {
            get_event_realm(&input.tenant_id, &election_event_id)
        }
        None => get_tenant_realm(&input.tenant_id),
    };
    let client = KeycloakAdminClient::new()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    let user = client
        .create_user(&realm, &input.user)
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    Ok(Json(user))
}

#[derive(Deserialize, Debug)]
pub struct EditUserBody {
    tenant_id: String,
    user_id: String,
    enabled: Option<bool>,
    election_event_id: Option<String>,
    attributes: Option<HashMap<String, Value>>,
    email: Option<String>,
    first_name: Option<String>,
    last_name: Option<String>,
    username: Option<String>,
    password: Option<String>,
}

#[instrument(skip(claims))]
#[post("/edit-user", format = "json", data = "<body>")]
pub async fn edit_user(
    claims: jwt::JwtClaims,
    body: Json<EditUserBody>,
) -> Result<Json<User>, (Status, String)> {
    let input = body.into_inner();
    let required_perm: Permissions = if input.election_event_id.is_some() {
        Permissions::VOTER_WRITE
    } else {
        Permissions::USER_WRITE
    };
    authorize(
        &claims,
        true,
        Some(input.tenant_id.clone()),
        vec![required_perm],
    )?;
    let realm = match input.election_event_id {
        Some(election_event_id) => {
            get_event_realm(&input.tenant_id, &election_event_id)
        }
        None => get_tenant_realm(&input.tenant_id),
    };
    let client = KeycloakAdminClient::new()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    let new_attributes = input.attributes.clone().unwrap_or(HashMap::new());

    // maintain current user attributes and do not allow to override tenant-id
    if new_attributes.contains_key(TENANT_ID_ATTR_NAME) {
        return Err((Status::BadRequest, "Cannot change tenant-id attribute".to_string()));
    }

    let user = client
        .edit_user(
            &realm,
            &input.user_id,
            input.enabled.clone(),
            Some(new_attributes),
            input.email.clone(),
            input.first_name.clone(),
            input.last_name.clone(),
            input.username.clone(),
            input.password.clone(),
        )
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    Ok(Json(user))
}

#[derive(Deserialize, Debug)]
pub struct GetUserBody {
    tenant_id: String,
    election_event_id: Option<String>,
    user_id: String,
}

#[instrument(skip(claims))]
#[post("/get-user", format = "json", data = "<body>")]
pub async fn get_user(
    claims: jwt::JwtClaims,
    body: Json<GetUserBody>,
) -> Result<Json<User>, (Status, String)> {
    let input = body.into_inner();
    let required_perm: Permissions = if input.election_event_id.is_some() {
        Permissions::VOTER_READ
    } else {
        Permissions::USER_READ
    };
    authorize(
        &claims,
        true,
        Some(input.tenant_id.clone()),
        vec![required_perm],
    )?;
    let realm = match input.election_event_id {
        Some(election_event_id) => {
            get_event_realm(&input.tenant_id, &election_event_id)
        }
        None => get_tenant_realm(&input.tenant_id),
    };
    let client = KeycloakAdminClient::new()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    let user = client
        .get_user(&realm, &input.user_id)
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    Ok(Json(user))
}
