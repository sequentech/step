// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
};
use sequent_core::services::jwt::{decode_jwt, JwtClaims};
use sequent_core::types::permissions::Permissions;
use tracing::{error, info, instrument, warn};

/// JWT Claims extractor for Axum
/// Usage: Add `JwtAuth(claims): JwtAuth` to handler parameters
pub struct JwtAuth(pub JwtClaims);

#[async_trait]
impl<S> FromRequestParts<S> for JwtAuth
where
    S: Send + Sync,
{
    type Rejection = StatusCode;

    #[instrument(skip(parts, _state))]
    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Extract Authorization header
        let auth_header = parts
            .headers
            .get("authorization")
            .and_then(|h| h.to_str().ok())
            .ok_or_else(|| {
                tracing::error!("Missing or invalid Authorization header");
                StatusCode::UNAUTHORIZED
            })?;

        // Extract Bearer token
        let token = auth_header.strip_prefix("Bearer ").ok_or_else(|| {
            tracing::error!("Authorization header missing 'Bearer ' prefix");
            StatusCode::UNAUTHORIZED
        })?;

        // Decode and validate JWT
        let claims = decode_jwt(token).map_err(|e| {
            tracing::error!("Failed to decode JWT: {}", e);
            StatusCode::UNAUTHORIZED
        })?;

        Ok(JwtAuth(claims))
    }
}

/// Role-based authorization extractor
/// Usage: `RequireRole { role: "trustee", claims }: RequireRole`
pub struct RequireRole {
    pub role: String,
    pub claims: JwtClaims,
}

#[async_trait]
impl<S> FromRequestParts<S> for RequireRole
where
    S: Send + Sync,
{
    type Rejection = StatusCode;

    #[instrument(skip(parts, state))]
    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // First extract JWT claims
        let JwtAuth(claims) = JwtAuth::from_request_parts(parts, state).await?;

        // Initially Claude thought it will be configure this way: "trustee" role in claims.realm_access.roles nor b4 in claims.resource_access hashmap.
        // This is not the case.
        // However the requirements below are ussually be true if the user is logged in as a trustee:
        let has_trustee_role = claims.trustee.is_some()
            && claims
                .hasura_claims
                .allowed_roles
                .contains(&Permissions::TRUSTEE_WRITE.to_string())
            && claims
                .hasura_claims
                .allowed_roles
                .contains(&Permissions::TRUSTEE_READ.to_string())
            && claims
                .hasura_claims
                .allowed_roles
                .contains(&Permissions::TRUSTEE_CEREMONY.to_string());
        let user_id = &claims.sub;
        if !has_trustee_role {
            tracing::warn!("User {user_id} does not have trustee role {claims:?}",);
            return Err(StatusCode::FORBIDDEN);
        }

        Ok(RequireRole {
            role: "trustee".to_string(),
            claims,
        })
    }
}

/// Helper function to verify board access for a user
pub fn verify_board_access(claims: &JwtClaims, _board_name: &str) -> Result<(), StatusCode> {
    // Check if user has trustee role or board-specific permissions
    let has_access = claims
        .realm_access
        .as_ref()
        .map(|ra| ra.roles.contains(&"trustee".to_string()))
        .unwrap_or(false)
        || claims
            .resource_access
            .as_ref()
            .map(|ra| {
                ra.get("b4")
                    .map(|access| access.roles.contains(&"trustee".to_string()))
                    .unwrap_or(false)
            })
            .unwrap_or(false);

    if !has_access {
        return Err(StatusCode::FORBIDDEN);
    }

    Ok(())
}
