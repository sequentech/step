// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::access::{read_permission, UserScope};
use crate::services::audit;
use crate::services::authorization::{authorize, require_ordinary_permission};
use crate::services::dependencies::HarvestServices;
use crate::services::role_permissions;

use crate::types::optional::OptionalId;
use crate::types::resources::{Aggregate, DataList, TotalAggregate};
use anyhow::Result;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use sequent_core::services::jwt;
use sequent_core::services::keycloak::validate_keycloak_path_segment;
use sequent_core::services::keycloak::validate_keycloak_scope;
use sequent_core::services::keycloak::KeycloakAdminClient;
use sequent_core::services::keycloak::{get_event_realm, get_tenant_realm};
use sequent_core::types::keycloak::Role;
use sequent_core::types::permissions::Permissions;
use serde::Deserialize;
use tracing::{event, instrument, Level};
use windmill::services::audit_log::AdminAction;

#[derive(Deserialize, Debug)]
pub struct CreateRoleBody {
    tenant_id: String,
    role: Role,
}

/// Creates a role. Sign permissions it is created with are logged (see
/// [`role_permissions`]).
#[instrument(skip(claims, services))]
#[post("/create-role", format = "json", data = "<body>")]
pub async fn create_role(
    claims: jwt::JwtClaims,
    body: Json<CreateRoleBody>,
    services: &State<HarvestServices>,
) -> Result<Json<Role>, (Status, String)> {
    let input = body.into_inner();
    validate_keycloak_scope(&input.tenant_id, None)
        .map_err(|error| (Status::BadRequest, error.to_string()))?;
    authorize(
        &claims,
        true,
        Some(input.tenant_id.clone()),
        vec![Permissions::ROLE_CREATE],
    )?;
    for permission in input.role.permissions.iter().flatten() {
        require_ordinary_permission(permission)?;
    }
    role_permissions::create_role(
        services,
        &claims,
        &input.tenant_id,
        &input.role,
    )
    .await
    .map(Json)
    .map_err(|error| {
        event!(Level::INFO, "Error {:?}", error);
        error
    })
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
    validate_keycloak_scope(&input.tenant_id, None)
        .map_err(|error| (Status::BadRequest, error.to_string()))?;
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
    validate_keycloak_scope(
        &input.tenant_id,
        input.election_event_id.as_deref(),
    )
    .map_err(|error| (Status::BadRequest, error.to_string()))?;
    authorize(
        &claims,
        true,
        Some(input.tenant_id.clone()),
        vec![
            read_permission(UserScope::of(input.election_event_id.as_deref())),
            Permissions::ROLE_READ,
        ],
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

#[instrument(skip(claims, services))]
#[post("/set-user-role", format = "json", data = "<body>")]
pub async fn set_user_role(
    claims: jwt::JwtClaims,
    body: Json<SetOrDeleteUserRoleBody>,
    services: &State<HarvestServices>,
) -> Result<Json<OptionalId>, (Status, String)> {
    change_user_role(services, &claims, body.into_inner(), AdminAction::Grant)
        .await
}

#[instrument(skip(claims, services))]
#[post("/delete-user-role", format = "json", data = "<body>")]
pub async fn delete_user_role(
    claims: jwt::JwtClaims,
    body: Json<SetOrDeleteUserRoleBody>,
    services: &State<HarvestServices>,
) -> Result<Json<OptionalId>, (Status, String)> {
    change_user_role(services, &claims, body.into_inner(), AdminAction::Revoke)
        .await
}

/// Gives a tenant user a role or takes it away, and logs it in the tenant's
/// log once Keycloak made the change. The entry names the account and the
/// role when Keycloak can be asked for them.
async fn change_user_role(
    services: &HarvestServices,
    claims: &jwt::JwtClaims,
    input: SetOrDeleteUserRoleBody,
    action: AdminAction,
) -> Result<Json<OptionalId>, (Status, String)> {
    validate_keycloak_scope(&input.tenant_id, None)
        .map_err(|error| (Status::BadRequest, error.to_string()))?;
    authorize(
        claims,
        true,
        Some(input.tenant_id.clone()),
        vec![Permissions::USER_WRITE, Permissions::ROLE_WRITE],
    )?;
    let realm = get_tenant_realm(&input.tenant_id);
    let target = audit::target(&input.tenant_id, None)?;
    let client = services
        .identity
        .client()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    let username = client
        .get_user(&realm, &input.user_id)
        .await
        .ok()
        .and_then(|user| user.username);
    let role_name = match validate_keycloak_path_segment(&input.role_id) {
        Ok(()) => client
            .client
            .realm_groups_with_group_id_get(&realm, &input.role_id)
            .await
            .ok()
            .and_then(|group| group.name),
        Err(_) => None,
    };
    match action {
        AdminAction::Revoke => {
            client
                .delete_user_role(&realm, &input.user_id, &input.role_id)
                .await
        }
        _ => {
            client
                .set_user_role(&realm, &input.user_id, &input.role_id)
                .await
        }
    }
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    audit::record(
        services,
        &audit::account_role(
            claims,
            target,
            audit::account(&target, &input.user_id, username.as_deref()),
            action,
            role_name.as_deref().unwrap_or(&input.role_id),
        ),
    )
    .await?;
    Ok(Json(Default::default()))
}

#[derive(Deserialize, Debug)]
pub struct DeleteRoleBody {
    tenant_id: String,
    role_id: String,
}

/// Deletes a role. Sign permissions it held are logged as removed (see
/// [`role_permissions`]).
#[instrument(skip(claims, services))]
#[post("/delete-role", format = "json", data = "<body>")]
pub async fn delete_role(
    claims: jwt::JwtClaims,
    body: Json<DeleteRoleBody>,
    services: &State<HarvestServices>,
) -> Result<Json<OptionalId>, (Status, String)> {
    let input = body.into_inner();
    validate_keycloak_scope(&input.tenant_id, None)
        .map_err(|error| (Status::BadRequest, error.to_string()))?;
    authorize(
        &claims,
        true,
        Some(input.tenant_id.clone()),
        vec![Permissions::ROLE_WRITE],
    )?;
    role_permissions::delete_role(
        services,
        &claims,
        &input.tenant_id,
        &input.role_id,
    )
    .await?;
    Ok(Json(Default::default()))
}
