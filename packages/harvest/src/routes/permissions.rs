// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::authorization::{authorize, require_ordinary_permission};

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
pub struct GetPermissionsBody {
    tenant_id: String,
    search: Option<String>,
    limit: Option<usize>,
    offset: Option<usize>,
}

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
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    let (permissions, count) = client
        .list_permissions(&realm, input.search, input.limit, input.offset)
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    Ok(Json(DataList {
        items: permissions,
        total: TotalAggregate {
            aggregate: Aggregate {
                count: count as i64,
            },
        },
    }))
}

#[derive(Deserialize, Debug)]
pub struct CreatePermissionsBody {
    tenant_id: String,
    permission: Permission,
}

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
    if let Some(name) = &input.permission.name {
        require_ordinary_permission(name)?;
    }
    let realm = get_tenant_realm(&input.tenant_id);
    let client = KeycloakAdminClient::new()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    let permission = client
        .create_permission(&realm, &input.permission)
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    Ok(Json(permission))
}

#[derive(Deserialize, Debug)]
pub struct SetOrDeleteRolePermissionsBody {
    tenant_id: String,
    role_id: String,
    permission_name: String,
}

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
    require_ordinary_permission(&input.permission_name)?;
    let realm = get_tenant_realm(&input.tenant_id);
    let client = KeycloakAdminClient::new()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    client
        .set_role_permission(&realm, &input.role_id, &input.permission_name)
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    Ok(Json(Default::default()))
}

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
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    client
        .delete_role_permission(&realm, &input.role_id, &input.permission_name)
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    Ok(Json(Default::default()))
}

#[derive(Deserialize, Debug)]
pub struct DeletePermissionBody {
    tenant_id: String,
    permission_name: String,
}

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
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    client
        .delete_permission(&realm, &input.permission_name)
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;
    Ok(Json(Default::default()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const TENANT_ID: &str = "tenant";
    const RESERVED_NAMES: [&str; 5] = [
        "admin",
        "service-account",
        "datafix-account",
        "super-admin-user",
        "cli-account-admin",
    ];

    fn tenant_admin(permissions: &[Permissions]) -> jwt::JwtClaims {
        let allowed_roles: Vec<String> = permissions
            .iter()
            .map(|permission| permission.to_string())
            .collect();
        serde_json::from_value(serde_json::json!({
            "exp": 1, "iat": 0, "jti": "test", "iss": "test",
            "sub": "admin", "typ": "Bearer", "azp": "admin-portal",
            "acr": "1", "allowed-origins": [], "scope": "openid",
            "email_verified": false,
            "https://hasura.io/jwt/claims": {
                "x-hasura-default-role": "admin-user",
                "x-hasura-tenant-id": TENANT_ID,
                "x-hasura-user-id": "admin",
                "x-hasura-allowed-roles": allowed_roles
            }
        }))
        .unwrap()
    }

    #[rocket::async_test]
    async fn reserved_permission_names_are_not_created() {
        for name in RESERVED_NAMES {
            let body = Json(CreatePermissionsBody {
                tenant_id: TENANT_ID.to_string(),
                permission: Permission {
                    id: None,
                    attributes: None,
                    container_id: None,
                    description: None,
                    name: Some(name.to_string()),
                },
            });

            let (status, message) = create_permission(
                tenant_admin(&[Permissions::USER_PERMISSION_CREATE]),
                body,
            )
            .await
            .unwrap_err();

            assert_eq!(status, Status::BadRequest, "{name}");
            assert!(message.contains(name), "{message}");
        }
    }

    #[rocket::async_test]
    async fn reserved_permission_names_are_not_attached_to_a_role() {
        for name in RESERVED_NAMES {
            let body = Json(SetOrDeleteRolePermissionsBody {
                tenant_id: TENANT_ID.to_string(),
                role_id: "role-id".to_string(),
                permission_name: name.to_string(),
            });

            let (status, message) = set_role_permission(
                tenant_admin(&[
                    Permissions::USER_PERMISSION_WRITE,
                    Permissions::ROLE_WRITE,
                ]),
                body,
            )
            .await
            .unwrap_err();

            assert_eq!(status, Status::BadRequest, "{name}");
            assert!(message.contains(name), "{message}");
        }
    }
}
