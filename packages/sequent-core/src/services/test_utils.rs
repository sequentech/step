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
    /// Default values mirror a realistic Keycloak token:
    /// - Expires in 5 minutes (300 seconds)
    /// - Subject: "b2cc4af1-718d-490b-af36-66c957fea791"
    /// - Tenant: "90505c8a-23a9-4cdf-a26b-4e19f6a097d5"
    /// - ACR: "silver"
    /// - No special permissions (empty `allowed_roles`) - use `with_permissions()` to add
    #[instrument(level = "trace")]
    pub fn new() -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            claims: JwtClaims {
                exp: now + 300,
                iat: now,
                auth_time: Some(now - 5),
                jti: format!("onrtac:{}", uuid::Uuid::new_v4()),
                iss: "http://127.0.0.1:8090/realms/tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5"
                    .to_string(),
                aud: Some(crate::services::jwt::StringOrVec::Single(
                    "account".to_string(),
                )),
                sub: "b2cc4af1-718d-490b-af36-66c957fea791".to_string(),
                typ: "Bearer".to_string(),
                azp: "admin-portal".to_string(),
                nonce: None,
                session_state: None,
                acr: "silver".to_string(),
                allowed_origins: vec!["*".to_string()],
                realm_access: None,
                resource_access: Some(std::collections::HashMap::from([(
                    "account".to_string(),
                    JwtRolesAccess {
                        roles: vec![
                            "manage-account".to_string(),
                            "manage-account-links".to_string(),
                            "view-profile".to_string(),
                        ],
                    },
                )])),
                scope: "openid profile email".to_string(),
                sid: Some(uuid::Uuid::new_v4().to_string()),
                email_verified: true,
                hasura_claims: JwtHasuraClaims {
                    default_role: "admin-user".to_string(),
                    tenant_id: "90505c8a-23a9-4cdf-a26b-4e19f6a097d5".to_string(),
                    user_id: "b2cc4af1-718d-490b-af36-66c957fea791".to_string(),
                    area_id: None,
                    authorized_election_ids: None,
                    allowed_roles: vec![],
                    permission_labels: Some("{}".to_string()),
                },
                name: Some("Test Trustee trustee".to_string()),
                preferred_username: Some("trustee1".to_string()),
                given_name: Some("Test Trustee".to_string()),
                family_name: Some("trustee".to_string()),
                trustee: Some("trustee1".to_string()),
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

    /// Sets the authorized election IDs in `hasura_claims.authorized_election_ids`.
    #[instrument(level = "trace", skip(self))]
    pub fn with_authorized_election_ids(
        mut self,
        election_ids: &[&str],
    ) -> Self {
        self.claims.hasura_claims.authorized_election_ids =
            Some(election_ids.iter().map(|s| s.to_string()).collect());
        self
    }

    /// Sets the trustee claim (e.g., "server" for native trustees, "trustee1" for browser).
    #[instrument(level = "trace", skip(self))]
    pub fn with_trustee(mut self, trustee: Option<&str>) -> Self {
        self.claims.trustee = trustee.map(|s| s.to_string());
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

/// Default test tenant ID (matches TestTokenBuilder defaults).
pub const TEST_TENANT_ID: &str = "90505c8a-23a9-4cdf-a26b-4e19f6a097d5";

/// Default test election event ID.
pub const TEST_ELECTION_EVENT_ID: &str = "388b3eff-e583-4a56-82b7-0ad15eaa409a";

/// Default test slug.
pub const TEST_SLUG: &str = "dev";

/// Creates a board name following the format used in production.
///
/// Board name format: `{slug}tenant{tenant_chars}event{election_event_id_no_dashes}`
/// where `tenant_chars` is the first 17 characters of tenant_id with dashes removed.
///
/// # Example
/// ```ignore
/// let board = create_test_board_name("90505c8a-23a9-4cdf-a26b-4e19f6a097d5", "388b3eff-e583-4a56-82b7-0ad15eaa409a", "dev");
/// // Returns: "devtenant90505c8a23a94cdfaevent388b3effe5834a5682b70ad15eaa409a"
/// ```
#[instrument(level = "trace")]
pub fn create_test_board_name(
    tenant_id: &str,
    election_event_id: &str,
    slug: &str,
) -> String {
    let tenant: String =
        tenant_id.chars().filter(|&c| c != '-').take(17).collect();
    format!("{slug}tenant{tenant}event{election_event_id}")
        .chars()
        .filter(|&c| c != '-')
        .collect()
}

/// Creates a board name using the default test tenant and election event IDs.
///
/// Uses `TEST_TENANT_ID`, `TEST_ELECTION_EVENT_ID`, and `TEST_SLUG`.
#[instrument(level = "trace")]
pub fn create_default_test_board_name() -> String {
    create_test_board_name(TEST_TENANT_ID, TEST_ELECTION_EVENT_ID, TEST_SLUG)
}

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
        assert_eq!(claims.sub, "b2cc4af1-718d-490b-af36-66c957fea791");
        assert_eq!(
            claims.hasura_claims.tenant_id,
            "90505c8a-23a9-4cdf-a26b-4e19f6a097d5"
        );
        assert_eq!(claims.acr, "silver");
        assert_eq!(claims.trustee, Some("trustee1".to_string()));
        assert_eq!(claims.azp, "admin-portal");
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

    #[test]
    fn test_token_builder_with_authorized_election_ids() {
        let keypair = generate_test_keypair("test-kid");
        let token = TestTokenBuilder::new()
            .with_authorized_election_ids(&[
                "388b3eff-e583-4a56-82b7-0ad15eaa409a",
                "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
            ])
            .build(&keypair);

        let claims = decode_jwt(&token).expect("Failed to decode token");
        let election_ids = claims
            .hasura_claims
            .authorized_election_ids
            .expect("Should have authorized_election_ids");
        assert_eq!(election_ids.len(), 2);
        assert!(election_ids
            .contains(&"388b3eff-e583-4a56-82b7-0ad15eaa409a".to_string()));
        assert!(election_ids
            .contains(&"aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee".to_string()));
    }

    #[test]
    fn test_token_builder_with_trustee() {
        let keypair = generate_test_keypair("test-kid");

        // Test setting trustee to "server" (native trustee)
        let token = TestTokenBuilder::new()
            .with_trustee(Some("server"))
            .build(&keypair);
        let claims = decode_jwt(&token).expect("Failed to decode token");
        assert_eq!(claims.trustee, Some("server".to_string()));

        // Test clearing trustee
        let token = TestTokenBuilder::new().with_trustee(None).build(&keypair);
        let claims = decode_jwt(&token).expect("Failed to decode token");
        assert_eq!(claims.trustee, None);
    }

    #[test]
    fn test_create_test_board_name() {
        let board = create_test_board_name(
            "90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
            "388b3eff-e583-4a56-82b7-0ad15eaa409a",
            "dev",
        );
        assert_eq!(
            board,
            "devtenant90505c8a23a94cdfaevent388b3effe5834a5682b70ad15eaa409a"
        );
    }

    #[test]
    fn test_create_test_board_name_different_slug() {
        let board = create_test_board_name(
            "90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
            "388b3eff-e583-4a56-82b7-0ad15eaa409a",
            "prod",
        );
        assert!(board.starts_with("prodtenant"));
    }

    #[test]
    fn test_create_default_test_board_name() {
        let board = create_default_test_board_name();
        // Should use TEST_TENANT_ID, TEST_ELECTION_EVENT_ID, TEST_SLUG
        assert!(board.starts_with("devtenant"));
        assert!(board.contains("event"));
        // Verify it matches the expected format
        let expected = create_test_board_name(
            TEST_TENANT_ID,
            TEST_ELECTION_EVENT_ID,
            TEST_SLUG,
        );
        assert_eq!(board, expected);
    }

    #[test]
    fn test_board_name_tenant_prefix_length() {
        // The tenant prefix should always be 17 chars
        let board = create_test_board_name(
            "90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
            "388b3eff-e583-4a56-82b7-0ad15eaa409a",
            "dev",
        );
        // Extract the tenant part: after "devtenant" (9 chars) and before "event"
        let after_tenant = &board[9..]; // Skip "devtenant"
        let event_pos = after_tenant.find("event").unwrap();
        let tenant_prefix = &after_tenant[..event_pos];
        assert_eq!(tenant_prefix.len(), 17);
        assert_eq!(tenant_prefix, "90505c8a23a94cdfa");
    }
}
