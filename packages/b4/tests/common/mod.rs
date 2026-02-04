// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Test infrastructure for B4 authentication integration tests.
//!
//! This module provides utilities for testing B4's JWT-based authentication
//! without requiring a full database and S3 setup.

use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use sequent_core::services::jwks::test_support::setup_test_jwks_cache;
use sequent_core::services::test_utils::{generate_test_keypair, TestKeyPair, TestTokenBuilder};
use tower::ServiceExt;

/// Test application wrapper for B4 integration tests.
///
/// Provides utilities for creating test requests with various authentication
/// scenarios and verifying responses.
pub struct TestApp {
    pub router: Router,
    pub keypair: TestKeyPair,
}

impl TestApp {
    /// Creates a new TestApp with the given router.
    ///
    /// Sets up the JWKS cache with test keys for authentication.
    pub fn new(router: Router) -> Self {
        let keypair = generate_test_keypair("b4-test-key");

        // Set up the global JWKS cache with our test key
        setup_test_jwks_cache(vec![keypair.jwk.clone()]);

        Self { router, keypair }
    }

    /// Creates a valid token with the specified permissions.
    pub fn create_token(&self, permissions: &[&str]) -> String {
        TestTokenBuilder::new()
            .with_permissions(permissions)
            .build(&self.keypair)
    }

    /// Creates a valid token with TRUSTEE_CEREMONY permission.
    pub fn create_trustee_token(&self) -> String {
        self.create_token(&["trustee-ceremony"])
    }

    /// Creates an expired token.
    pub fn create_expired_token(&self, permissions: &[&str]) -> String {
        TestTokenBuilder::new()
            .with_permissions(permissions)
            .expired()
            .build(&self.keypair)
    }

    /// Creates a token signed with a different key (invalid signature).
    pub fn create_invalid_signature_token(&self, permissions: &[&str]) -> String {
        let other_keypair = generate_test_keypair("other-key");
        TestTokenBuilder::new()
            .with_permissions(permissions)
            .build(&other_keypair)
    }

    /// Sends a request to the router and returns the response status.
    pub async fn request_status(&self, req: Request<Body>) -> StatusCode {
        self.router
            .clone()
            .oneshot(req)
            .await
            .expect("Request failed")
            .status()
    }

    /// Creates a GET request builder.
    pub fn get(&self, uri: &str) -> RequestBuilder {
        RequestBuilder::new("GET", uri)
    }

    /// Creates a POST request builder.
    pub fn post(&self, uri: &str) -> RequestBuilder {
        RequestBuilder::new("POST", uri)
    }
}

/// Builder for creating test HTTP requests.
pub struct RequestBuilder {
    method: String,
    uri: String,
    auth_header: Option<String>,
    body: Option<String>,
    content_type: Option<String>,
}

impl RequestBuilder {
    pub fn new(method: &str, uri: &str) -> Self {
        Self {
            method: method.to_string(),
            uri: uri.to_string(),
            auth_header: None,
            body: None,
            content_type: None,
        }
    }

    /// Sets the Authorization header with a Bearer token.
    pub fn bearer_token(mut self, token: &str) -> Self {
        self.auth_header = Some(format!("Bearer {token}"));
        self
    }

    /// Sets a custom Authorization header value.
    pub fn auth_header(mut self, value: &str) -> Self {
        self.auth_header = Some(value.to_string());
        self
    }

    /// Sets the request body as JSON.
    pub fn json_body(mut self, body: &str) -> Self {
        self.body = Some(body.to_string());
        self.content_type = Some("application/json".to_string());
        self
    }

    /// Builds the request.
    pub fn build(self) -> Request<Body> {
        let body = self.body.map(Body::from).unwrap_or(Body::empty());

        let mut builder = Request::builder()
            .method(self.method.as_str())
            .uri(&self.uri);

        if let Some(auth) = &self.auth_header {
            builder = builder.header("authorization", auth);
        }

        if let Some(ct) = &self.content_type {
            builder = builder.header("content-type", ct);
        }

        builder.body(body).expect("Failed to build request")
    }
}

/// Helper to create a minimal test router for auth-only tests.
///
/// This router has endpoints that require authentication but minimal
/// business logic, useful for testing auth without a database.
pub fn create_auth_test_router() -> Router {
    use axum::routing::get;
    use b4::auth::{RequirePermissions, TrusteeCeremony};

    async fn protected_endpoint(
        RequirePermissions { claims, .. }: RequirePermissions<TrusteeCeremony>,
    ) -> String {
        format!("Hello, {}!", claims.sub)
    }

    Router::new().route("/protected", get(protected_endpoint))
}
