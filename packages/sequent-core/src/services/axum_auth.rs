// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Axum authentication extractors for JWT-based authorization.
//!
//! This module provides generic extractors for Axum handlers that validate
//! JWT tokens and check permissions. Services implement the `PermissionSet`
//! trait to define their specific permission requirements.
//!
//! # Example
//!
//! ```ignore
//! use sequent_core::services::axum_auth::{PermissionSet, RequirePermissions};
//! use sequent_core::types::permissions::Permissions;
//!
//! // Define a permission set for your service
//! pub struct TrusteeCeremony;
//!
//! impl PermissionSet for TrusteeCeremony {
//!     fn required_permissions() -> &'static [Permissions] {
//!         &[Permissions::TRUSTEE_CEREMONY]
//!     }
//! }
//!
//! // Use in handler
//! async fn my_handler(
//!     RequirePermissions { claims, .. }: RequirePermissions<TrusteeCeremony>,
//! ) -> impl IntoResponse {
//!     // Handler code
//! }
//! ```

use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
};
use tracing::instrument;

use crate::services::jwks::{get_global_jwks_cache, verify_token_signature};
use crate::services::jwt::{decode_jwt, JwtClaims};
use crate::types::permissions::Permissions;

/// JWT Claims extractor for Axum
///
/// Extracts and validates JWT from the Authorization header.
/// Usage: Add `JwtAuth(claims): JwtAuth` to handler parameters
pub struct JwtAuth(pub JwtClaims);

#[async_trait]
impl<S> FromRequestParts<S> for JwtAuth
where
    S: Send + Sync,
{
    type Rejection = StatusCode;

    #[instrument(level = "trace", skip(parts, _state))]
    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
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

        // Get JWKS from cache and verify token signature
        let jwks_cache = get_global_jwks_cache();
        let keys = jwks_cache.get_jwks_cached().await.map_err(|e| {
            tracing::error!("Failed to get JWKS from cache: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        verify_token_signature(token, &keys).map_err(|e| {
            tracing::error!("JWT signature verification failed: {e}");
            StatusCode::UNAUTHORIZED
        })?;

        // Decode and validate JWT claims
        let claims = decode_jwt(token).map_err(|e| {
            tracing::error!("Failed to decode JWT: {e}");
            StatusCode::UNAUTHORIZED
        })?;

        // Validate token expiration
        let now = chrono::Utc::now().timestamp();
        if claims.exp < now {
            tracing::error!(
                "JWT token has expired (exp: {}, now: {now})",
                claims.exp
            );
            return Err(StatusCode::UNAUTHORIZED);
        }

        Ok(JwtAuth(claims))
    }
}

/// Trait for defining required permission sets
///
/// Implement this trait in your service to create custom permission
/// requirements for handlers. Each implementation defines which
/// permissions are required for access.
pub trait PermissionSet: Send + Sync {
    /// Returns the list of permissions required by this set
    fn required_permissions() -> &'static [Permissions];
}

/// Generic permission-based authorization extractor
///
/// Validates JWT and checks that the user has all permissions defined
/// by the `PermissionSet` implementation.
///
/// Usage: `RequirePermissions<YourPermissionSet> { claims, .. }: RequirePermissions<YourPermissionSet>`
pub struct RequirePermissions<P: PermissionSet> {
    pub claims: JwtClaims,
    _marker: std::marker::PhantomData<P>,
}

#[async_trait]
impl<S, P> FromRequestParts<S> for RequirePermissions<P>
where
    S: Send + Sync,
    P: PermissionSet,
{
    type Rejection = StatusCode;

    #[instrument(level = "trace", skip(parts, state))]
    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        // First extract JWT claims
        let JwtAuth(claims) = JwtAuth::from_request_parts(parts, state).await?;
        let user_id = &claims.sub;
        let required = P::required_permissions();

        // Check all required permissions
        for permission in required {
            let permission_str = permission.to_string();
            if !claims.hasura_claims.allowed_roles.contains(&permission_str) {
                tracing::warn!("User {user_id} missing required permission: {permission_str}");
                return Err(StatusCode::FORBIDDEN);
            }
        }

        Ok(RequirePermissions {
            claims,
            _marker: std::marker::PhantomData,
        })
    }
}
