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
use sequent_core::services::keycloak::get_tenant_realm;
use sequent_core::services::keycloak::KeycloakAdminClient;
use sequent_core::types::keycloak::Permission;
use sequent_core::types::permissions::Permissions;
use serde::Deserialize;
use tracing::instrument;

#[derive(Deserialize, Debug)]
/// Request body for getting permissions.
pub struct GetPermissionsBody {
    /// The tenant ID.
    tenant_id: String,
    /// The search query.
    search: Option<String>,
    /// The limit.
    limit: Option<usize>,
    /// The offset.
    offset: Option<usize>,
}

/// Get permissions.
#[instrument(skip(claims))]
#[post("/get-permissions", format = "json", data = "<body>")]
pub async fn get_permissions(
    claims: jwt::JwtClaims,
    body: Json<GetPermissionsBody>,
) -> Result<Json<DataList<Permission>>, (Status, String)> {
    let input = body.into_inner();
    authorize(
        &claims,
        true,
        Some(input.tenant_id.clone()),
        vec![Permissions::USER_PERMISSION_READ],
    )?;
    let realm = get_tenant_realm(&input.tenant_id);
    let client = KeycloakAdminClient::new()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;
    let (permissions, count) = client
        .list_permissions(&realm, input.search, input.limit, input.offset)
        .await
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;
    let count_i64 = i64::try_from(count).map_err(|_| {
        (
            Status::InternalServerError,
            "permission list length does not fit in i64".to_string(),
        )
    })?;
    Ok(Json(DataList {
        items: permissions,
        total: TotalAggregate {
            aggregate: Aggregate { count: count_i64 },
        },
    }))
}

#[derive(Deserialize, Debug)]
/// Request body for creating a permission.
pub struct CreatePermissionsBody {
    /// The tenant ID.
    tenant_id: String,
    /// The permission.
    permission: Permission,
}

/// Create permission in Keycloak realm.
#[instrument(skip(claims))]
#[post("/create-permission", format = "json", data = "<body>")]
pub async fn create_permission(
    claims: jwt::JwtClaims,
    body: Json<CreatePermissionsBody>,
) -> Result<Json<Permission>, (Status, String)> {
    let input = body.into_inner();
    authorize(
        &claims,
        true,
        Some(input.tenant_id.clone()),
        vec![Permissions::USER_PERMISSION_CREATE],
    )?;
    let realm = get_tenant_realm(&input.tenant_id);
    let client = KeycloakAdminClient::new()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;
    let permission = client
        .create_permission(&realm, &input.permission)
        .await
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;
    Ok(Json(permission))
}

#[derive(Deserialize, Debug)]
/// Request body for setting or deleting a role permission.
pub struct SetOrDeleteRolePermissionsBody {
    /// The tenant ID.
    tenant_id: String,
    /// The role ID.
    role_id: String,
    /// The permission name.
    permission_name: String,
}

/// Set role permission in Keycloak realm.
#[instrument(skip(claims))]
#[post("/set-role-permission", format = "json", data = "<body>")]
pub async fn set_role_permission(
    claims: jwt::JwtClaims,
    body: Json<SetOrDeleteRolePermissionsBody>,
) -> Result<Json<OptionalId>, (Status, String)> {
    let input = body.into_inner();
    authorize(
        &claims,
        true,
        Some(input.tenant_id.clone()),
        vec![Permissions::USER_PERMISSION_WRITE, Permissions::ROLE_WRITE],
    )?;
    let realm = get_tenant_realm(&input.tenant_id);
    let client = KeycloakAdminClient::new()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;
    client
        .set_role_permission(&realm, &input.role_id, &input.permission_name)
        .await
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;
    Ok(Json(OptionalId::default()))
}

/// Delete role permission in Keycloak realm.
#[instrument(skip(claims))]
#[post("/delete-role-permission", format = "json", data = "<body>")]
pub async fn delete_role_permission(
    claims: jwt::JwtClaims,
    body: Json<SetOrDeleteRolePermissionsBody>,
) -> Result<Json<OptionalId>, (Status, String)> {
    let input = body.into_inner();
    authorize(
        &claims,
        true,
        Some(input.tenant_id.clone()),
        vec![Permissions::USER_PERMISSION_WRITE, Permissions::ROLE_WRITE],
    )?;
    let realm = get_tenant_realm(&input.tenant_id);
    let client = KeycloakAdminClient::new()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;
    client
        .delete_role_permission(&realm, &input.role_id, &input.permission_name)
        .await
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;
    Ok(Json(OptionalId::default()))
}

#[derive(Deserialize, Debug)]
/// Request body for deleting a permission.
pub struct DeletePermissionBody {
    /// The tenant ID.
    tenant_id: String,
    /// The permission name.
    permission_name: String,
}

/// Delete permission in Keycloak realm.
#[instrument(skip(claims))]
#[post("/delete-permission", format = "json", data = "<body>")]
pub async fn delete_permission(
    claims: jwt::JwtClaims,
    body: Json<DeletePermissionBody>,
) -> Result<Json<OptionalId>, (Status, String)> {
    let input = body.into_inner();
    authorize(
        &claims,
        true,
        Some(input.tenant_id.clone()),
        vec![Permissions::USER_PERMISSION_WRITE],
    )?;
    let realm = get_tenant_realm(&input.tenant_id);
    let client = KeycloakAdminClient::new()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;
    client
        .delete_permission(&realm, &input.permission_name)
        .await
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;
    Ok(Json(OptionalId::default()))
}
