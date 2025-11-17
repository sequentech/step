// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::authorization::authorize;

use crate::types::optional::OptionalId;
use crate::types::resources::{Aggregate, DataList, TotalAggregate};
use anyhow::Result;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt;
use sequent_core::services::keycloak::KeycloakAdminClient;
use sequent_core::services::keycloak::{get_event_realm, get_tenant_realm};
use sequent_core::types::keycloak::Role;
use sequent_core::types::permissions::Permissions;
use serde::Deserialize;
use tracing::{event, instrument, Level};

#[derive(Deserialize, Debug)]
pub struct CreateRoleBody {
    tenant_id: String,
    role: Role,
}

#[instrument(skip(claims))]
#[post("/create-role", format = "json", data = "<body>")]
pub async fn create_role(
    claims: jwt::JwtClaims,
    body: Json<CreateRoleBody>,
) -> Result<Json<Role>, (Status, String)> {
    let input = body.into_inner();
    authorize(
        &claims,
        true,
        Some(input.tenant_id.clone()),
        vec![Permissions::ROLE_READ],
    )?;
    let realm = get_tenant_realm(&input.tenant_id);
    let client = KeycloakAdminClient::new()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    let role = client.create_role(&realm, &input.role).await.map_err(|e| {
        event!(Level::INFO, "Error {:?}", e);
        (Status::InternalServerError, format!("{:?}", e))
    })?;
    //client moved to create_role so need to create new one
    let client = KeycloakAdminClient::new()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    let role_with_id =
        client.get_role_by_name(&realm, &role).await.map_err(|e| {
            event!(Level::INFO, "Error {:?}", e);
            (Status::InternalServerError, format!("{:?}", e))
        })?;

    let client = KeycloakAdminClient::new()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    match (role.clone().permissions, role_with_id.id) {
        (Some(permissions), Some(id)) => {
            client
                .set_role_permissions(&realm, &id, &permissions)
                .await
                .map_err(|e| {
                    event!(Level::INFO, "Error {:?}", e);
                    (Status::InternalServerError, format!("{:?}", e))
                })?;
        }
        _ => {}
    }

    Ok(Json(role))
}

#[derive(Deserialize, Debug)]
pub struct GetRolesBody {
    tenant_id: String,
    search: Option<String>,
    limit: Option<usize>,
    offset: Option<usize>,
}

#[instrument(skip(claims))]
#[post("/get-roles", format = "json", data = "<body>")]
pub async fn get_roles(
    claims: jwt::JwtClaims,
    body: Json<GetRolesBody>,
) -> Result<Json<DataList<Role>>, (Status, String)> {
    let input = body.into_inner();
    authorize(
        &claims,
        true,
        Some(input.tenant_id.clone()),
        vec![Permissions::ROLE_READ],
    )?;
    let realm = get_tenant_realm(&input.tenant_id);
    let client = KeycloakAdminClient::new()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    let (roles, count) = client
        .list_roles(&realm, input.search, input.limit, input.offset)
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    Ok(Json(DataList {
        items: roles,
        total: TotalAggregate {
            aggregate: Aggregate {
                count: count as i64,
            },
        },
    }))
}

#[derive(Deserialize, Debug)]
pub struct ListUserRolesBody {
    tenant_id: String,
    user_id: String,
    election_event_id: Option<String>,
}

#[instrument(skip(claims))]
#[post("/list-user-roles", format = "json", data = "<body>")]
pub async fn list_user_roles(
    claims: jwt::JwtClaims,
    body: Json<ListUserRolesBody>,
) -> Result<Json<Vec<Role>>, (Status, String)> {
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
        vec![required_perm, Permissions::ROLE_READ],
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
    let roles = client
        .list_user_roles(&realm, &input.user_id)
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    Ok(Json(roles))
}

#[derive(Deserialize, Debug)]
pub struct SetOrDeleteUserRoleBody {
    tenant_id: String,
    user_id: String,
    role_id: String,
}

#[instrument(skip(claims))]
#[post("/set-user-role", format = "json", data = "<body>")]
pub async fn set_user_role(
    claims: jwt::JwtClaims,
    body: Json<SetOrDeleteUserRoleBody>,
) -> Result<Json<OptionalId>, (Status, String)> {
    let input = body.into_inner();
    authorize(
        &claims,
        true,
        Some(input.tenant_id.clone()),
        vec![Permissions::USER_WRITE, Permissions::ROLE_WRITE],
    )?;
    let realm = get_tenant_realm(&input.tenant_id);
    let client = KeycloakAdminClient::new()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    client
        .set_user_role(&realm, &input.user_id, &input.role_id)
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    Ok(Json(Default::default()))
}

#[instrument(skip(claims))]
#[post("/delete-user-role", format = "json", data = "<body>")]
pub async fn delete_user_role(
    claims: jwt::JwtClaims,
    body: Json<SetOrDeleteUserRoleBody>,
) -> Result<Json<OptionalId>, (Status, String)> {
    let input = body.into_inner();
    authorize(
        &claims,
        true,
        Some(input.tenant_id.clone()),
        vec![Permissions::USER_WRITE, Permissions::ROLE_WRITE],
    )?;
    let realm = get_tenant_realm(&input.tenant_id);
    let client = KeycloakAdminClient::new()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    client
        .delete_user_role(&realm, &input.user_id, &input.role_id)
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    Ok(Json(Default::default()))
}

#[derive(Deserialize, Debug)]
pub struct DeleteRoleBody {
    tenant_id: String,
    role_id: String,
}

#[instrument(skip(claims))]
#[post("/delete-role", format = "json", data = "<body>")]
pub async fn delete_role(
    claims: jwt::JwtClaims,
    body: Json<DeleteRoleBody>,
) -> Result<Json<OptionalId>, (Status, String)> {
    let input = body.into_inner();
    authorize(
        &claims,
        true,
        Some(input.tenant_id.clone()),
        vec![Permissions::ROLE_WRITE],
    )?;
    let realm = get_tenant_realm(&input.tenant_id);
    let client = KeycloakAdminClient::new()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    client
        .delete_role(&realm, &input.role_id)
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    Ok(Json(Default::default()))
}
