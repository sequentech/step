// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use anyhow::Result;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use serde::{Deserialize, Serialize};
use tracing::{error, info, instrument};
use windmill::postgres::tenant::get_tenant_by_id;
use windmill::services::database::get_hasura_pool;
use windmill::services::keycloak::update_keycloak_admin_golden_authentication;

#[derive(Serialize, Deserialize, Debug)]
pub struct SetAdminAuthentication {
    pub golden_authentication: bool,
}

#[derive(Serialize)]
struct SetAdminAuthenticationOutput {
    success: bool,
    message: String,
}

#[instrument(skip(claims))]
#[post("/set-admin-authentication", format = "json", data = "<input>")]
pub async fn set_admin_authentication(
    claims: JwtClaims,
    input: Json<SetAdminAuthentication>,
) -> Result<Json<SetAdminAuthenticationOutput>, (Status, String)> {
    let body = input.into_inner();

    // Authorization check
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![],
    )
    .map_err(|err| {
        error!("Authorization failed: {:?}", err);
        (Status::Forbidden, "Authorization failed".to_string())
    })?;

    let mut hasura_db_client =
        get_hasura_pool().await.get().await.map_err(|e| {
            error!("Failed to get DB pool: {:?}", e);
            (Status::InternalServerError, format!("{:?}", e))
        })?;

    let hasura_transaction =
        hasura_db_client.transaction().await.map_err(|e| {
            error!("Failed to start transaction: {:?}", e);
            (Status::InternalServerError, format!("{:?}", e))
        })?;

    let tenant =
        get_tenant_by_id(&hasura_transaction, &claims.hasura_claims.tenant_id)
            .await
            .map_err(|e| {
                error!("Failed to fetch tenant: {:?}", e);
                (Status::InternalServerError, format!("{:?}", e))
            })?;

    // Extract or set default golden_authentication
    let prev_golden_auth = tenant.settings.as_ref().map_or(true, |settings| {
        settings
            .get("golden_authentication")
            .and_then(|v| v.as_bool())
            .unwrap_or(true)
    });

    // Update golden_authentication if it has changed
    if prev_golden_auth != body.golden_authentication {
        let enable_golden_authentication = body.golden_authentication;
        info!(
            "Updating golden authentication to: {}",
            enable_golden_authentication
        );

        update_keycloak_admin_golden_authentication(
            Some(claims.hasura_claims.tenant_id.clone()),
            enable_golden_authentication,
        )
        .await
        .map_err(|error| {
            error!("Failed to update golden authentication: {:?}", error);
            (
                Status::InternalServerError,
                format!("Error updating golden authentication: {error:?}"),
            )
        })?;
    }

    // Commit transaction
    hasura_transaction.commit().await.map_err(|e| {
        error!("Transaction commit failed: {:?}", e);
        (Status::InternalServerError, format!("{:?}", e))
    })?;

    Ok(Json(SetAdminAuthenticationOutput {
        success: true,
        message: "Authentication updated successfully".to_string(),
    }))
}
