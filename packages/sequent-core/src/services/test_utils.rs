// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Test utilities for JWT/JWKS testing.
//!
//! This module provides utilities for generating test RSA keypairs and
//! building test JWT tokens that are compatible with Keycloak token structure.
//!
//! # Key Design Decisions
//!
//! - Uses production `JwtClaims` struct directly for compile-time safety
//! - If the Keycloak token structure changes, tests will fail to compile
//! - Builder pattern allows easy customization for different test scenarios

use crate::services::jwks::JWKKey;
use crate::services::jwt::{JwtClaims, JwtHasuraClaims, JwtRolesAccess};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use pkcs1::EncodeRsaPrivateKey;
use rsa::traits::PublicKeyParts;
use rsa::{RsaPrivateKey, RsaPublicKey};

/// RSA keypair for test token signing.
///
/// Contains both the encoding key for signing tokens and the corresponding
/// JWKKey for verification.
pub struct TestKeyPair {
    pub encoding_key: EncodingKey,
    pub jwk: JWKKey,
    pub kid: String,
}

/// Generates an RSA keypair and corresponding JWKKey for testing.
///
/// # Arguments
/// * `kid` - Key ID to use in the JWK and JWT header
///
/// # Example
/// ```ignore
/// let keypair = generate_test_keypair("test-key-1");
/// let token = TestTokenBuilder::new()
///     .with_permissions(&["trustee-ceremony"])
///     .build(&keypair);
/// ```
#[instrument(level = "trace", skip_all)]
pub fn generate_test_keypair(kid: &str) -> TestKeyPair {
    let mut rng = rand::thread_rng();
    let bits = 2048;
    let private_key =
        RsaPrivateKey::new(&mut rng, bits).expect("Failed to generate RSA key");
    let public_key = RsaPublicKey::from(&private_key);

    // Extract n and e as base64url-encoded strings (no padding)
    let n = URL_SAFE_NO_PAD.encode(public_key.n().to_bytes_be());
    let e = URL_SAFE_NO_PAD.encode(public_key.e().to_bytes_be());

    let jwk = JWKKey {
        alg: "RS256".to_string(),
        kty: "RSA".to_string(),
        r#use: "sig".to_string(),
        n,
        e,
        kid: kid.to_string(),
        x5t: String::new(),
        x5c: vec![],
    };

    // Convert to PEM for jsonwebtoken
    let pem = private_key
        .to_pkcs1_pem(pkcs1::LineEnding::LF)
        .expect("Failed to encode private key to PEM");
    let encoding_key = EncodingKey::from_rsa_pem(pem.as_bytes())
        .expect("Failed to create encoding key from PEM");

    TestKeyPair {
        encoding_key,
        jwk,
        kid: kid.to_string(),
    }
}

/// Builder for test JWT tokens.
///
/// Uses the production `JwtClaims` struct directly to ensure compile-time
/// compatibility with Keycloak token structure. Any changes to the production
/// struct will cause tests to fail to compile.
///
/// # Example
/// ```ignore
/// let keypair = generate_test_keypair("test-key-1");
///
/// // Create a valid trustee token
/// let token = TestTokenBuilder::new()
///     .with_permissions(&["trustee-ceremony"])
///     .with_subject("user-123")
///     .build(&keypair);
///
/// // Create an expired token for testing expiration handling
/// let expired_token = TestTokenBuilder::new()
///     .with_permissions(&["trustee-ceremony"])
///     .expired()
///     .build(&keypair);
/// ```
pub struct TestTokenBuilder {
    claims: JwtClaims,
}

impl Default for TestTokenBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TestTokenBuilder {
    /// Creates a new builder with sensible defaults for a trustee token.
    ///
    /// Default values:
    /// - Expires in 1 hour
    /// - Subject: "test-user-id"
    /// - Tenant: "test-tenant"
    /// - No special permissions (empty `allowed_roles`)
    #[instrument(level = "trace")]
    pub fn new() -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            claims: JwtClaims {
                exp: now + 3600,
                iat: now,
                auth_time: Some(now),
                jti: uuid::Uuid::new_v4().to_string(),
                iss: "http://test-keycloak/realms/test".to_string(),
                aud: None,
                sub: "test-user-id".to_string(),
                typ: "Bearer".to_string(),
                azp: "test-client".to_string(),
                nonce: None,
                session_state: None,
                acr: "1".to_string(),
                allowed_origins: vec!["*".to_string()],
                realm_access: Some(JwtRolesAccess { roles: vec![] }),
                resource_access: None,
                scope: "openid profile email".to_string(),
                sid: None,
                email_verified: true,
                hasura_claims: JwtHasuraClaims {
                    default_role: "user".to_string(),
                    tenant_id: "test-tenant".to_string(),
                    user_id: "test-user-id".to_string(),
                    area_id: None,
                    authorized_election_ids: None,
                    allowed_roles: vec![],
                    permission_labels: None,
                },
                name: Some("Test User".to_string()),
                preferred_username: Some("testuser".to_string()),
                given_name: Some("Test".to_string()),
                family_name: Some("User".to_string()),
                trustee: None,
            },
        }
    }

    /// Sets the permissions (allowed_roles in hasura_claims).
    ///
    /// # Arguments
    /// * `perms` - Slice of permission strings (e.g., `["trustee-ceremony"]`)
    #[instrument(level = "trace", skip(self))]
    pub fn with_permissions(mut self, perms: &[&str]) -> Self {
        self.claims.hasura_claims.allowed_roles =
            perms.iter().map(|s| s.to_string()).collect();
        if let Some(first) = perms.first() {
            self.claims.hasura_claims.default_role = first.to_string();
        }
        self
    }

    /// Makes the token expired (1 hour in the past).
    #[instrument(level = "trace", skip(self))]
    pub fn expired(mut self) -> Self {
        self.claims.exp = chrono::Utc::now().timestamp() - 3600;
        self
    }

    /// Sets the subject (user ID) in both `sub` and `hasura_claims.user_id`.
    #[instrument(level = "trace", skip(self))]
    pub fn with_subject(mut self, sub: &str) -> Self {
        self.claims.sub = sub.to_string();
        self.claims.hasura_claims.user_id = sub.to_string();
        self
    }

    /// Sets the tenant ID in `hasura_claims.tenant_id`.
    #[instrument(level = "trace", skip(self))]
    pub fn with_tenant(mut self, tenant_id: &str) -> Self {
        self.claims.hasura_claims.tenant_id = tenant_id.to_string();
        self
    }

    /// Sets the issuer (iss) claim.
    #[instrument(level = "trace", skip(self))]
    pub fn with_issuer(mut self, iss: &str) -> Self {
        self.claims.iss = iss.to_string();
        self
    }

    /// Sets a custom expiration time (Unix timestamp).
    #[instrument(level = "trace", skip(self))]
    pub fn with_expiration(mut self, exp: i64) -> Self {
        self.claims.exp = exp;
        self
    }

    /// Provides access to the underlying claims for advanced customization.
    pub fn claims_mut(&mut self) -> &mut JwtClaims {
        &mut self.claims
    }

    /// Signs and builds the JWT token.
    ///
    /// # Arguments
    /// * `keypair` - The test keypair to sign with
    ///
    /// # Returns
    /// A signed JWT token string
    #[instrument(level = "trace", skip(self, keypair))]
    pub fn build(self, keypair: &TestKeyPair) -> String {
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some(keypair.kid.clone());
        header.typ = Some("JWT".to_string());
        jsonwebtoken::encode(&header, &self.claims, &keypair.encoding_key)
            .expect("Failed to encode test token")
    }
}

use tracing::instrument;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::jwt::decode_jwt;

    #[test]
    fn test_generate_keypair() {
        let keypair = generate_test_keypair("test-kid");
        assert_eq!(keypair.kid, "test-kid");
        assert_eq!(keypair.jwk.kid, "test-kid");
        assert_eq!(keypair.jwk.alg, "RS256");
        assert_eq!(keypair.jwk.kty, "RSA");
        assert!(!keypair.jwk.n.is_empty());
        assert!(!keypair.jwk.e.is_empty());
    }

    #[test]
    fn test_token_builder_default() {
        let keypair = generate_test_keypair("test-kid");
        let token = TestTokenBuilder::new().build(&keypair);

        // Token should be valid JWT format (3 parts separated by dots)
        let parts: Vec<&str> = token.split('.').collect();
        assert_eq!(parts.len(), 3);

        // Should be decodable with our decode_jwt function
        let claims = decode_jwt(&token).expect("Failed to decode token");
        assert_eq!(claims.sub, "test-user-id");
        assert_eq!(claims.hasura_claims.tenant_id, "test-tenant");
    }

    #[test]
    fn test_token_builder_with_permissions() {
        let keypair = generate_test_keypair("test-kid");
        let token = TestTokenBuilder::new()
            .with_permissions(&["trustee-ceremony", "admin-user"])
            .build(&keypair);

        let claims = decode_jwt(&token).expect("Failed to decode token");
        assert_eq!(claims.hasura_claims.allowed_roles.len(), 2);
        assert!(claims
            .hasura_claims
            .allowed_roles
            .contains(&"trustee-ceremony".to_string()));
        assert!(claims
            .hasura_claims
            .allowed_roles
            .contains(&"admin-user".to_string()));
    }

    #[test]
    fn test_token_builder_expired() {
        let keypair = generate_test_keypair("test-kid");
        let token = TestTokenBuilder::new().expired().build(&keypair);

        let claims = decode_jwt(&token).expect("Failed to decode token");
        let now = chrono::Utc::now().timestamp();
        assert!(claims.exp < now, "Token should be expired");
    }

    #[test]
    fn test_token_builder_custom_subject() {
        let keypair = generate_test_keypair("test-kid");
        let token = TestTokenBuilder::new()
            .with_subject("custom-user-123")
            .build(&keypair);

        let claims = decode_jwt(&token).expect("Failed to decode token");
        assert_eq!(claims.sub, "custom-user-123");
        assert_eq!(claims.hasura_claims.user_id, "custom-user-123");
    }
}
