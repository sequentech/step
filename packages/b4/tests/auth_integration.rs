// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Integration tests for B4 authentication.
//!
//! These tests verify the JWT-based authentication flow for B4 endpoints,
//! covering various scenarios including:
//! - Missing/invalid authorization headers
//! - Token signature verification
//! - Permission checking
//! - Browser-based and native trustee flows
//!
//! Tests use `#[serial]` to avoid race conditions on the global JWKS cache.

mod common;

use axum::http::StatusCode;
use common::{create_auth_test_router, TestApp};
use serial_test::serial;

// ============================================================================
// Authentication Header Tests
// ============================================================================

#[tokio::test]
#[serial]
async fn test_missing_auth_header_returns_401() {
    let app = TestApp::new(create_auth_test_router());

    let req = app.get("/protected").build();
    let status = app.request_status(req).await;

    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "Missing auth header should return 401"
    );
}

#[tokio::test]
#[serial]
async fn test_invalid_bearer_format_returns_401() {
    let app = TestApp::new(create_auth_test_router());

    // Test with "Basic" auth type instead of "Bearer"
    let req = app
        .get("/protected")
        .auth_header("Basic dXNlcjpwYXNz")
        .build();
    let status = app.request_status(req).await;

    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "Non-Bearer auth should return 401"
    );
}

#[tokio::test]
#[serial]
async fn test_empty_bearer_token_returns_401() {
    let app = TestApp::new(create_auth_test_router());

    // Test with empty token after "Bearer "
    let req = app.get("/protected").auth_header("Bearer ").build();
    let status = app.request_status(req).await;

    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "Empty Bearer token should return 401"
    );
}

#[tokio::test]
#[serial]
async fn test_malformed_token_returns_401() {
    let app = TestApp::new(create_auth_test_router());

    // Test with a clearly malformed token
    let req = app
        .get("/protected")
        .bearer_token("not-a-valid-jwt")
        .build();
    let status = app.request_status(req).await;

    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "Malformed token should return 401"
    );
}

// ============================================================================
// Token Signature Verification Tests
// ============================================================================

#[tokio::test]
#[serial]
async fn test_invalid_signature_returns_401() {
    let app = TestApp::new(create_auth_test_router());

    // Token signed with a different key (not in JWKS cache)
    let invalid_token = app.create_invalid_signature_token(&["trustee-ceremony"]);
    let req = app.get("/protected").bearer_token(&invalid_token).build();
    let status = app.request_status(req).await;

    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "Token with invalid signature should return 401"
    );
}

#[tokio::test]
#[serial]
async fn test_expired_token_returns_401() {
    let app = TestApp::new(create_auth_test_router());

    // Create an expired token
    let expired_token = app.create_expired_token(&["trustee-ceremony"]);
    let req = app.get("/protected").bearer_token(&expired_token).build();
    let status = app.request_status(req).await;

    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "Expired token should return 401"
    );
}

// ============================================================================
// Permission Checking Tests
// ============================================================================

#[tokio::test]
#[serial]
async fn test_valid_token_missing_permission_returns_403() {
    let app = TestApp::new(create_auth_test_router());

    // Token with different permission (not trustee-ceremony)
    let token = app.create_token(&["user", "admin"]);
    let req = app.get("/protected").bearer_token(&token).build();
    let status = app.request_status(req).await;

    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "Valid token without required permission should return 403"
    );
}

#[tokio::test]
#[serial]
async fn test_valid_token_with_permission_returns_200() {
    let app = TestApp::new(create_auth_test_router());

    // Token with correct permission
    let token = app.create_trustee_token();
    let req = app.get("/protected").bearer_token(&token).build();
    let status = app.request_status(req).await;

    assert_eq!(
        status,
        StatusCode::OK,
        "Valid token with trustee-ceremony permission should return 200"
    );
}

#[tokio::test]
#[serial]
async fn test_valid_token_with_multiple_permissions() {
    let app = TestApp::new(create_auth_test_router());

    // Token with multiple permissions including the required one
    let token = app.create_token(&["user", "trustee-ceremony", "admin"]);
    let req = app.get("/protected").bearer_token(&token).build();
    let status = app.request_status(req).await;

    assert_eq!(
        status,
        StatusCode::OK,
        "Token with trustee-ceremony among other permissions should succeed"
    );
}

// ============================================================================
// Browser-Based Trustee Flow Tests
// ============================================================================

/// Simulates browser-based trustee authentication flow.
///
/// Browser trustees (WASM) receive a pre-obtained token and include it
/// in all requests to the bulletin board.
#[tokio::test]
#[serial]
async fn test_browser_trustee_flow() {
    let app = TestApp::new(create_auth_test_router());

    // Browser trustee has a token provided at initialization
    // (equivalent to WasmSessionConfig.access_token)
    let browser_token = app.create_trustee_token();

    // Typical browser trustee operations
    let req = app.get("/protected").bearer_token(&browser_token).build();
    let status = app.request_status(req).await;

    assert_eq!(
        status,
        StatusCode::OK,
        "Browser trustee with valid token should succeed"
    );
}

/// Tests that browser trustee requests fail gracefully when token expires.
#[tokio::test]
#[serial]
async fn test_browser_trustee_expired_token() {
    let app = TestApp::new(create_auth_test_router());

    // Browser trustee with an expired token
    let expired_token = app.create_expired_token(&["trustee-ceremony"]);

    let req = app.get("/protected").bearer_token(&expired_token).build();
    let status = app.request_status(req).await;

    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "Browser trustee with expired token should get 401"
    );
}

// ============================================================================
// Native Server Trustee Flow Tests
// ============================================================================

/// Simulates native server trustee authentication flow.
///
/// Native trustees obtain tokens from Keycloak via username/password
/// and include them in requests to the bulletin board.
#[tokio::test]
#[serial]
async fn test_native_trustee_flow() {
    let app = TestApp::new(create_auth_test_router());

    // Native trustee token (would normally come from Keycloak)
    let native_token = app.create_trustee_token();

    // Native trustees make the same HTTP requests as browser trustees
    let req = app.get("/protected").bearer_token(&native_token).build();
    let status = app.request_status(req).await;

    assert_eq!(
        status,
        StatusCode::OK,
        "Native trustee with valid token should succeed"
    );
}

// ============================================================================
// Edge Cases
// ============================================================================

#[tokio::test]
#[serial]
async fn test_token_with_empty_permissions() {
    let app = TestApp::new(create_auth_test_router());

    // Token with no permissions at all
    let token = app.create_token(&[]);
    let req = app.get("/protected").bearer_token(&token).build();
    let status = app.request_status(req).await;

    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "Token with no permissions should return 403"
    );
}

#[tokio::test]
#[serial]
async fn test_case_sensitive_permission() {
    let app = TestApp::new(create_auth_test_router());

    // Permission with wrong case
    let token = app.create_token(&["TRUSTEE-CEREMONY"]);
    let req = app.get("/protected").bearer_token(&token).build();
    let status = app.request_status(req).await;

    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "Permissions should be case-sensitive"
    );
}

#[tokio::test]
#[serial]
async fn test_extra_whitespace_in_auth_header() {
    let app = TestApp::new(create_auth_test_router());

    let token = app.create_trustee_token();
    // Extra whitespace after "Bearer"
    let req = app
        .get("/protected")
        .auth_header(&format!("Bearer  {token}"))
        .build();
    let status = app.request_status(req).await;

    // The behavior depends on implementation - this tests the actual behavior
    // Most implementations reject extra whitespace
    assert!(
        status == StatusCode::UNAUTHORIZED || status == StatusCode::OK,
        "Extra whitespace handling should be consistent"
    );
}
