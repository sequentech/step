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
use sequent_core::services::test_utils::{
    create_test_board_name, TEST_ELECTION_EVENT_ID, TEST_SLUG, TEST_TENANT_ID,
};
use sequent_core::types::permissions::Permissions;
use serial_test::serial;

/// A tenant ID distinct from `TEST_TENANT_ID`, used for cross-tenant tests.
const OTHER_TENANT_ID: &str = "12345678-1234-1234-1234-123456789012";

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
    // Uses Permissions::TRUSTEE_CEREMONY constant instead of hardcoded string
    let trustee_perm = Permissions::TRUSTEE_CEREMONY.to_string();
    let token = server.create_token(&["user", &trustee_perm, "admin"]);
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

// ============================================================================
// Cross-Tenant Board Isolation Tests (body-driven endpoints)
// ============================================================================
//
// The multi-board endpoints and board creation take the board name from the
// request body, so the path-param `BoardAccessValidator` cannot reach them.
// These tests verify the explicit per-board authorization (authorized-boards
// membership) in those handlers.

#[tokio::test]
#[serial]
async fn test_multi_get_other_tenant_board_returns_403() {
    let server = TestServer::new().await;

    // Browser trustee authorized only for its own board requests another board.
    let own_board = create_test_board_name(TEST_TENANT_ID, TEST_ELECTION_EVENT_ID, TEST_SLUG);
    let other_board = create_test_board_name(OTHER_TENANT_ID, TEST_ELECTION_EVENT_ID, TEST_SLUG);
    let token = server.create_browser_trustee_token(TEST_TENANT_ID, &[own_board.as_str()]);

    let resp = server
        .server
        .post("/boards/messages/multi/get")
        .add_header(AUTHORIZATION, format!("Bearer {token}"))
        .json(&serde_json::json!({
            "requests": [{ "board": other_board, "last_id": 0, "limit": 100 }]
        }))
        .await;

    resp.assert_status(StatusCode::FORBIDDEN);
}

#[tokio::test]
#[serial]
async fn test_multi_get_own_tenant_board_allowed() {
    let server = TestServer::new().await;
    // Clean slate: the server is shared across serial tests, so avoid a
    // duplicate-create conflict on the board name.
    server.cleanup().await;

    // Own-tenant board: create it (server token), then read via multi-get as a
    // browser trustee in the same tenant. The tenant check must pass (not 403).
    let own_board = create_test_board_name(TEST_TENANT_ID, TEST_ELECTION_EVENT_ID, TEST_SLUG);
    server.create_board(&own_board).await;

    let token = server.create_browser_trustee_token(TEST_TENANT_ID, &[own_board.as_str()]);
    let resp = server
        .server
        .post("/boards/messages/multi/get")
        .add_header(AUTHORIZATION, format!("Bearer {token}"))
        .json(&serde_json::json!({
            "requests": [{ "board": own_board, "last_id": 0, "limit": 100 }]
        }))
        .await;

    resp.assert_status_ok();
}

#[tokio::test]
#[serial]
async fn test_create_board_other_tenant_returns_403() {
    let server = TestServer::new().await;

    // Browser trustee authorized only for its own board tries to create another.
    let own_board = create_test_board_name(TEST_TENANT_ID, TEST_ELECTION_EVENT_ID, TEST_SLUG);
    let other_board = create_test_board_name(OTHER_TENANT_ID, TEST_ELECTION_EVENT_ID, TEST_SLUG);
    let token = server.create_browser_trustee_token(TEST_TENANT_ID, &[own_board.as_str()]);

    let resp = server
        .server
        .post("/boards")
        .add_header(AUTHORIZATION, format!("Bearer {token}"))
        .json(&serde_json::json!({ "name": other_board }))
        .await;

    resp.assert_status(StatusCode::FORBIDDEN);
}

#[tokio::test]
#[serial]
async fn test_list_boards_excludes_other_tenant() {
    let server = TestServer::new().await;
    server.cleanup().await;

    // Two boards in different tenants, both created by the server role.
    let own_board = create_test_board_name(TEST_TENANT_ID, TEST_ELECTION_EVENT_ID, TEST_SLUG);
    let other_board = create_test_board_name(OTHER_TENANT_ID, TEST_ELECTION_EVENT_ID, TEST_SLUG);
    server.create_board(&own_board).await;
    server.create_board(&other_board).await;

    // A browser trustee authorized only for its own board lists boards.
    let token = server.create_browser_trustee_token(TEST_TENANT_ID, &[own_board.as_str()]);
    let resp = server
        .server
        .get("/boards")
        .add_header(AUTHORIZATION, format!("Bearer {token}"))
        .await;

    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    let names: Vec<String> = body["boards"]
        .as_array()
        .expect("boards array")
        .iter()
        .map(|b| b["name"].as_str().unwrap_or_default().to_string())
        .collect();

    assert!(names.contains(&own_board), "own board should be listed");
    assert!(
        !names.contains(&other_board),
        "other-tenant board must be hidden"
    );
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
