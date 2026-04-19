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

/// Request body for [`create_role`].
#[derive(Deserialize, Debug)]
pub struct CreateRoleBody {
    /// Tenant identifier used to resolve the Keycloak realm.
    tenant_id: String,
    /// Role definition to create in the tenant realm.
    role: Role,
}

/// Creates a group-backed role in the tenant Keycloak realm.
///
/// # Errors
///
/// Returns [`Status::Unauthorized`] when the caller lacks permission, or
/// [`Status::InternalServerError`] when Keycloak requests fail.
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
    let kc_create = KeycloakAdminClient::new()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;
    let role =
        kc_create
            .create_role(&realm, &input.role)
            .await
            .map_err(|e| {
                event!(Level::INFO, "Error {e:?}");
                (Status::InternalServerError, format!("{e:?}"))
            })?;
    let kc_lookup = KeycloakAdminClient::new()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;
    let role_with_id = kc_lookup
        .get_role_by_name(&realm, &role)
        .await
        .map_err(|e| {
            event!(Level::INFO, "Error {e:?}");
            (Status::InternalServerError, format!("{e:?}"))
        })?;

    let kc_permissions = KeycloakAdminClient::new()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;
    if let (Some(permissions), Some(id)) =
        (role.clone().permissions, role_with_id.id)
    {
        kc_permissions
            .set_role_permissions(&realm, &id, &permissions)
            .await
            .map_err(|e| {
                event!(Level::INFO, "Error {e:?}");
                (Status::InternalServerError, format!("{e:?}"))
            })?;
    }

    Ok(Json(role))
}

/// Request body for [`get_roles`].
#[derive(Deserialize, Debug)]
pub struct GetRolesBody {
    /// Tenant identifier used to resolve the Keycloak realm.
    tenant_id: String,
    /// Optional substring filter applied to role names.
    search: Option<String>,
    /// Maximum number of roles to return.
    limit: Option<usize>,
    /// Number of roles to skip before collecting results.
    offset: Option<usize>,
}

/// Lists roles defined in the tenant Keycloak realm.
///
/// # Errors
///
/// Returns [`Status::Unauthorized`] when the caller lacks permission, or
/// [`Status::InternalServerError`] when Keycloak requests fail.
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
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;
    let (roles, count) = client
        .list_roles(&realm, input.search, input.limit, input.offset)
        .await
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;
    let count_i64 = i64::try_from(count).map_err(|_| {
        (
            Status::InternalServerError,
            "role list length does not fit in i64".to_string(),
        )
    })?;
    Ok(Json(DataList {
        items: roles,
        total: TotalAggregate {
            aggregate: Aggregate { count: count_i64 },
        },
    }))
}

#[derive(Deserialize, Debug)]
#[allow(clippy::struct_field_names)] // enable same postfix for all fields
/// Request body for listing user roles.
pub struct ListUserRolesBody {
    /// The tenant ID.
    tenant_id: String,
    /// The user ID.
    user_id: String,
    /// The election event ID.
    election_event_id: Option<String>,
}

/// Lists user roles defined in the tenant Keycloak realm.
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
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;
    let roles = client
        .list_user_roles(&realm, &input.user_id)
        .await
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;
    Ok(Json(roles))
}

#[derive(Deserialize, Debug)]
#[allow(clippy::struct_field_names)] // enable same postfix for all fields
/// Request body for setting or deleting a user role.
pub struct SetOrDeleteUserRoleBody {
    /// The tenant ID.
    tenant_id: String,
    /// The user ID.
    user_id: String,
    /// The role ID.
    role_id: String,
}

/// Sets a user role in the tenant Keycloak realm.
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
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;
    client
        .set_user_role(&realm, &input.user_id, &input.role_id)
        .await
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;
    Ok(Json(OptionalId::default()))
}

/// Deletes a user role in the tenant Keycloak realm.
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
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;
    client
        .delete_user_role(&realm, &input.user_id, &input.role_id)
        .await
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;
    Ok(Json(OptionalId::default()))
}

#[derive(Deserialize, Debug)]
#[allow(clippy::struct_field_names)] // enable same postfix for all fields
/// Request body for deleting a role.
pub struct DeleteRoleBody {
    /// The tenant ID.
    tenant_id: String,
    /// The role ID.
    role_id: String,
}

/// Deletes a role in the tenant Keycloak realm.
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
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;
    client
        .delete_role(&realm, &input.role_id)
        .await
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;
    Ok(Json(OptionalId::default()))
}
