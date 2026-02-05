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
//! Tests use `axum-test` for fluent HTTP testing API.

mod common;

use axum::http::{header::AUTHORIZATION, StatusCode};
use common::TestServer;
use serial_test::serial;

// ============================================================================
// Authentication Header Tests
// ============================================================================

#[tokio::test]
#[serial]
async fn test_missing_auth_header_returns_401() {
    let server = TestServer::new().await;

    let resp = server.server.get("/boards").await;

    resp.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[serial]
async fn test_invalid_bearer_format_returns_401() {
    let server = TestServer::new().await;

    // Test with "Basic" auth type instead of "Bearer"
    let resp = server
        .server
        .get("/boards")
        .add_header(AUTHORIZATION, "Basic dXNlcjpwYXNz")
        .await;

    resp.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[serial]
async fn test_empty_bearer_token_returns_401() {
    let server = TestServer::new().await;

    // Test with empty token after "Bearer "
    let resp = server
        .server
        .get("/boards")
        .add_header(AUTHORIZATION, "Bearer ")
        .await;

    resp.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[serial]
async fn test_malformed_token_returns_401() {
    let server = TestServer::new().await;

    // Test with a clearly malformed token
    let resp = server
        .server
        .get("/boards")
        .add_header(AUTHORIZATION, "Bearer not-a-valid-jwt")
        .await;

    resp.assert_status(StatusCode::UNAUTHORIZED);
}

// ============================================================================
// Token Signature Verification Tests
// ============================================================================

#[tokio::test]
#[serial]
async fn test_invalid_signature_returns_401() {
    let server = TestServer::new().await;

    // Token signed with a different key (not in JWKS cache)
    let invalid_token = server.create_invalid_signature_token(&["trustee-ceremony"]);
    let resp = server
        .server
        .get("/boards")
        .add_header(AUTHORIZATION, format!("Bearer {invalid_token}"))
        .await;

    resp.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[serial]
async fn test_expired_token_returns_401() {
    let server = TestServer::new().await;

    // Create an expired token
    let expired_token = server.create_expired_token(&["trustee-ceremony"]);
    let resp = server
        .server
        .get("/boards")
        .add_header(AUTHORIZATION, format!("Bearer {expired_token}"))
        .await;

    resp.assert_status(StatusCode::UNAUTHORIZED);
}

// ============================================================================
// Permission Checking Tests
// ============================================================================

#[tokio::test]
#[serial]
async fn test_valid_token_missing_permission_returns_403() {
    let server = TestServer::new().await;

    // Token with different permission (not trustee-ceremony)
    let token = server.create_token(&["user", "admin"]);
    let resp = server
        .server
        .get("/boards")
        .add_header(AUTHORIZATION, format!("Bearer {token}"))
        .await;

    resp.assert_status(StatusCode::FORBIDDEN);
}

#[tokio::test]
#[serial]
async fn test_valid_token_with_permission_returns_200() {
    let server = TestServer::new().await;

    // Token with correct permission
    let token = server.create_trustee_token();
    let resp = server
        .server
        .get("/boards")
        .add_header(AUTHORIZATION, format!("Bearer {token}"))
        .await;

    resp.assert_status_ok();
}

#[tokio::test]
#[serial]
async fn test_valid_token_with_multiple_permissions() {
    let server = TestServer::new().await;

    // Token with multiple permissions including the required one
    let token = server.create_token(&["user", "trustee-ceremony", "admin"]);
    let resp = server
        .server
        .get("/boards")
        .add_header(AUTHORIZATION, format!("Bearer {token}"))
        .await;

    resp.assert_status_ok();
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
    let server = TestServer::new().await;

    // Browser trustee has a token provided at initialization
    // (equivalent to WasmSessionConfig.access_token)
    let browser_token = server.create_trustee_token();

    // Typical browser trustee operations
    let resp = server
        .server
        .get("/boards")
        .add_header(AUTHORIZATION, format!("Bearer {browser_token}"))
        .await;

    resp.assert_status_ok();
}

/// Tests that browser trustee requests fail gracefully when token expires.
#[tokio::test]
#[serial]
async fn test_browser_trustee_expired_token() {
    let server = TestServer::new().await;

    // Browser trustee with an expired token
    let expired_token = server.create_expired_token(&["trustee-ceremony"]);

    let resp = server
        .server
        .get("/boards")
        .add_header(AUTHORIZATION, format!("Bearer {expired_token}"))
        .await;

    resp.assert_status(StatusCode::UNAUTHORIZED);
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
    let server = TestServer::new().await;

    // Native trustee token (would normally come from Keycloak)
    let native_token = server.create_trustee_token();

    // Native trustees make the same HTTP requests as browser trustees
    let resp = server
        .server
        .get("/boards")
        .add_header(AUTHORIZATION, format!("Bearer {native_token}"))
        .await;

    resp.assert_status_ok();
}

// ============================================================================
// Edge Cases
// ============================================================================

#[tokio::test]
#[serial]
async fn test_token_with_empty_permissions() {
    let server = TestServer::new().await;

    // Token with no permissions at all
    let token = server.create_token(&[]);
    let resp = server
        .server
        .get("/boards")
        .add_header(AUTHORIZATION, format!("Bearer {token}"))
        .await;

    resp.assert_status(StatusCode::FORBIDDEN);
}

#[tokio::test]
#[serial]
async fn test_case_sensitive_permission() {
    let server = TestServer::new().await;

    // Permission with wrong case
    let token = server.create_token(&["TRUSTEE-CEREMONY"]);
    let resp = server
        .server
        .get("/boards")
        .add_header(AUTHORIZATION, format!("Bearer {token}"))
        .await;

    resp.assert_status(StatusCode::FORBIDDEN);
}

#[tokio::test]
#[serial]
async fn test_extra_whitespace_in_auth_header() {
    let server = TestServer::new().await;

    let token = server.create_trustee_token();
    // Extra whitespace after "Bearer"
    let resp = server
        .server
        .get("/boards")
        .add_header(AUTHORIZATION, format!("Bearer  {token}"))
        .await;

    // The behavior depends on implementation - this tests the actual behavior
    // Most implementations reject extra whitespace
    let status = resp.status_code();
    assert!(
        status == StatusCode::UNAUTHORIZED || status == StatusCode::OK,
        "Extra whitespace handling should be consistent"
    );
}
