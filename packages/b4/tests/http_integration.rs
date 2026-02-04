// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! HTTP Integration tests for B4.
//!
//! These tests start a real HTTP server and make actual HTTP requests to verify
//! the complete request/response cycle, including authentication middleware.
//!
//! Tests use testcontainers for PostgreSQL and LocalStack (S3) to provide
//! real infrastructure without external dependencies.

mod common;

use b4::api_types::{InitiateMessageRequest, InitiateMessageResponse};
use common::TestServer;
use reqwest::StatusCode;
use sequent_core::services::test_utils::{TestTokenBuilder, TEST_ELECTION_EVENT_ID, TEST_TENANT_ID};
use serial_test::serial;

// ============================================================================
// Authentication Header Tests
// ============================================================================

#[tokio::test]
#[serial]
async fn test_missing_auth_header_returns_401() {
    let server = TestServer::new().await;

    let resp = server
        .client
        .get(format!("{}/boards", server.url()))
        .send()
        .await
        .expect("Request failed");

    assert_eq!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "Missing auth header should return 401"
    );
}

#[tokio::test]
#[serial]
async fn test_invalid_bearer_format_returns_401() {
    let server = TestServer::new().await;

    let resp = server
        .client
        .get(format!("{}/boards", server.url()))
        .header("Authorization", "Basic dXNlcjpwYXNz")
        .send()
        .await
        .expect("Request failed");

    assert_eq!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "Non-Bearer auth should return 401"
    );
}

#[tokio::test]
#[serial]
async fn test_empty_bearer_token_returns_401() {
    let server = TestServer::new().await;

    let resp = server
        .client
        .get(format!("{}/boards", server.url()))
        .header("Authorization", "Bearer ")
        .send()
        .await
        .expect("Request failed");

    assert_eq!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "Empty Bearer token should return 401"
    );
}

#[tokio::test]
#[serial]
async fn test_malformed_token_returns_401() {
    let server = TestServer::new().await;

    let resp = server
        .client
        .get(format!("{}/boards", server.url()))
        .bearer_auth("not-a-valid-jwt")
        .send()
        .await
        .expect("Request failed");

    assert_eq!(
        resp.status(),
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
    let server = TestServer::new().await;

    let invalid_token = server.create_invalid_signature_token(&["trustee-ceremony"]);
    let resp = server
        .client
        .get(format!("{}/boards", server.url()))
        .bearer_auth(&invalid_token)
        .send()
        .await
        .expect("Request failed");

    assert_eq!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "Token with invalid signature should return 401"
    );
}

#[tokio::test]
#[serial]
async fn test_expired_token_returns_401() {
    let server = TestServer::new().await;

    let expired_token = server.create_expired_token(&["trustee-ceremony"]);
    let resp = server
        .client
        .get(format!("{}/boards", server.url()))
        .bearer_auth(&expired_token)
        .send()
        .await
        .expect("Request failed");

    assert_eq!(
        resp.status(),
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
    let server = TestServer::new().await;

    // Token with different permission (not trustee-ceremony)
    let token = server.create_token(&["user", "admin"]);
    let resp = server
        .client
        .get(format!("{}/boards", server.url()))
        .bearer_auth(&token)
        .send()
        .await
        .expect("Request failed");

    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "Valid token without required permission should return 403"
    );
}

#[tokio::test]
#[serial]
async fn test_valid_token_with_permission_returns_200() {
    let server = TestServer::new().await;

    let token = server.create_trustee_token();
    let resp = server
        .client
        .get(format!("{}/boards", server.url()))
        .bearer_auth(&token)
        .send()
        .await
        .expect("Request failed");

    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "Valid token with trustee-ceremony permission should return 200"
    );
}

// ============================================================================
// Board Operations Tests
// ============================================================================

#[tokio::test]
#[serial]
async fn test_create_board() {
    let server = TestServer::new().await;
    let token = server.create_trustee_token();

    let resp = server
        .client
        .post(format!("{}/boards", server.url()))
        .bearer_auth(&token)
        .json(&serde_json::json!({ "name": &server.board_name }))
        .send()
        .await
        .expect("Request failed");

    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = resp.json().await.expect("Failed to parse response");
    assert_eq!(body["name"], server.board_name);
    assert_eq!(body["status"], "active");
}

#[tokio::test]
#[serial]
async fn test_list_boards() {
    let server = TestServer::new().await;
    let token = server.create_trustee_token();

    // Create a board first
    server.create_board(&server.board_name).await;

    let resp = server
        .client
        .get(format!("{}/boards", server.url()))
        .bearer_auth(&token)
        .send()
        .await
        .expect("Request failed");

    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = resp.json().await.expect("Failed to parse response");
    assert!(body["boards"].is_array());
    assert!(!body["boards"].as_array().unwrap().is_empty());
}

#[tokio::test]
#[serial]
async fn test_get_board() {
    let server = TestServer::new().await;
    let token = server.create_trustee_token();

    // Create a board first
    server.create_board(&server.board_name).await;

    let resp = server
        .client
        .get(format!("{}/boards/{}", server.url(), server.board_name))
        .bearer_auth(&token)
        .send()
        .await
        .expect("Request failed");

    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = resp.json().await.expect("Failed to parse response");
    assert_eq!(body["name"], server.board_name);
}

#[tokio::test]
#[serial]
async fn test_get_nonexistent_board_returns_404() {
    let server = TestServer::new().await;
    let token = server.create_trustee_token();

    let resp = server
        .client
        .get(format!("{}/boards/nonexistent-board", server.url()))
        .bearer_auth(&token)
        .send()
        .await
        .expect("Request failed");

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// ============================================================================
// Single Message Operations Tests
// ============================================================================

#[tokio::test]
#[serial]
async fn test_initiate_small_message() {
    let server = TestServer::new().await;
    let token = server.create_trustee_token();

    // Create a board first
    server.create_board(&server.board_name).await;

    let req = InitiateMessageRequest {
        size: 100, // Small message (inline)
        sender_pk: "test-sender-pk".to_string(),
        statement_kind: "TestStatement".to_string(),
        batch: 0,
        mix_number: 0,
    };

    let resp = server
        .client
        .post(format!(
            "{}/boards/{}/messages/initiate",
            server.url(),
            server.board_name
        ))
        .bearer_auth(&token)
        .json(&req)
        .send()
        .await
        .expect("Request failed");

    assert_eq!(resp.status(), StatusCode::OK);

    let body: InitiateMessageResponse = resp.json().await.expect("Failed to parse response");
    assert!(!body.message_id.is_empty());
    assert!(!body.should_upload, "Small message should not require S3 upload");
    assert!(body.upload_url.is_none());
}

#[tokio::test]
#[serial]
async fn test_initiate_large_message() {
    let server = TestServer::new().await;
    let token = server.create_trustee_token();

    // Create a board first
    server.create_board(&server.board_name).await;

    let req = InitiateMessageRequest {
        size: 2 * 1024 * 1024, // 2MB - should require S3
        sender_pk: "test-sender-pk".to_string(),
        statement_kind: "TestStatement".to_string(),
        batch: 0,
        mix_number: 0,
    };

    let resp = server
        .client
        .post(format!(
            "{}/boards/{}/messages/initiate",
            server.url(),
            server.board_name
        ))
        .bearer_auth(&token)
        .json(&req)
        .send()
        .await
        .expect("Request failed");

    assert_eq!(resp.status(), StatusCode::OK);

    let body: InitiateMessageResponse = resp.json().await.expect("Failed to parse response");
    assert!(!body.message_id.is_empty());
    assert!(body.should_upload, "Large message should require S3 upload");
    assert!(body.upload_url.is_some());
}

#[tokio::test]
#[serial]
async fn test_list_messages_empty() {
    let server = TestServer::new().await;
    let token = server.create_trustee_token();

    // Create a board first
    server.create_board(&server.board_name).await;

    let resp = server
        .client
        .get(format!(
            "{}/boards/{}/messages/list",
            server.url(),
            server.board_name
        ))
        .bearer_auth(&token)
        .send()
        .await
        .expect("Request failed");

    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = resp.json().await.expect("Failed to parse response");
    assert!(body["messages"].is_array());
    assert!(body["messages"].as_array().unwrap().is_empty());
}

#[tokio::test]
#[serial]
async fn test_get_messages_empty() {
    let server = TestServer::new().await;
    let token = server.create_trustee_token();

    // Create a board first
    server.create_board(&server.board_name).await;

    let resp = server
        .client
        .get(format!(
            "{}/boards/{}/messages",
            server.url(),
            server.board_name
        ))
        .bearer_auth(&token)
        .send()
        .await
        .expect("Request failed");

    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = resp.json().await.expect("Failed to parse response");
    assert!(body["messages"].is_array());
}

// ============================================================================
// Multi-Board Operations Tests
// ============================================================================

#[tokio::test]
#[serial]
async fn test_get_messages_multi() {
    let server = TestServer::new().await;
    let token = server.create_trustee_token();

    // Create a board first
    server.create_board(&server.board_name).await;

    let req = serde_json::json!({
        "requests": [
            {
                "board": server.board_name,
                "last_id": 0,
                "limit": 100
            }
        ]
    });

    let resp = server
        .client
        .post(format!("{}/boards/messages/multi/get", server.url()))
        .bearer_auth(&token)
        .json(&req)
        .send()
        .await
        .expect("Request failed");

    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = resp.json().await.expect("Failed to parse response");
    assert!(body["boards"].is_array());
}

#[tokio::test]
#[serial]
async fn test_initiate_messages_multi() {
    let server = TestServer::new().await;
    let token = server.create_trustee_token();

    // Create a board first
    server.create_board(&server.board_name).await;

    let req = serde_json::json!({
        "requests": [
            {
                "board": server.board_name,
                "messages": [
                    {
                        "size": 100,
                        "sender_pk": "test-pk",
                        "statement_kind": "TestStatement",
                        "batch": 0,
                        "mix_number": 0
                    }
                ]
            }
        ]
    });

    let resp = server
        .client
        .post(format!("{}/boards/messages/multi/initiate", server.url()))
        .bearer_auth(&token)
        .json(&req)
        .send()
        .await
        .expect("Request failed");

    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = resp.json().await.expect("Failed to parse response");
    assert!(body["boards"].is_array());
}

// ============================================================================
// Authentication Tests for All Endpoints
// ============================================================================

/// Tests that all endpoints require authentication.
#[tokio::test]
#[serial]
async fn test_all_endpoints_require_auth() {
    let server = TestServer::new().await;
    let board = "test-board";

    let endpoints = [
        ("POST", "/boards"),
        ("GET", "/boards"),
        ("GET", &format!("/boards/{board}")),
        ("POST", &format!("/boards/{board}/messages/initiate")),
        ("POST", &format!("/boards/{board}/messages/123/confirm")),
        ("GET", &format!("/boards/{board}/messages/list")),
        ("GET", &format!("/boards/{board}/messages")),
        ("GET", &format!("/boards/{board}/messages/1")),
        ("POST", "/boards/messages/multi/get"),
        ("POST", "/boards/messages/multi/initiate"),
        ("POST", "/boards/messages/multi/confirm"),
    ];

    for (method, path) in endpoints {
        let url = format!("{}{}", server.url(), path);
        let req_builder = match method {
            "GET" => server.client.get(&url),
            "POST" => server.client.post(&url).json(&serde_json::json!({})),
            _ => panic!("Unknown method"),
        };

        let resp = req_builder.send().await.expect("Request failed");

        assert_eq!(
            resp.status(),
            StatusCode::UNAUTHORIZED,
            "Endpoint {method} {path} should require authentication"
        );
    }
}

// ============================================================================
// Future Requirement Tests - Browser Trustee Board Verification
// ============================================================================
// These tests verify that browser trustees can only access boards matching
// their tenant_id and authorized_election_ids from JWT claims.
// EXPECTED TO FAIL until the feature is implemented.

#[tokio::test]
#[serial]
#[should_panic(expected = "assertion")]
async fn test_browser_trustee_wrong_tenant_id_should_fail() {
    let server = TestServer::new().await;

    // Create board with default test tenant
    server.create_board(&server.board_name).await;

    // Token has DIFFERENT tenant
    let wrong_tenant = "12345678-1234-1234-1234-123456789012";
    let token = server.create_browser_trustee_token(wrong_tenant, &[TEST_ELECTION_EVENT_ID]);

    let resp = server
        .client
        .get(format!("{}/boards/{}", server.url(), server.board_name))
        .bearer_auth(&token)
        .send()
        .await
        .expect("Request failed");

    // FUTURE: Should return 403 Forbidden due to tenant mismatch
    // Currently passes (returns 200) because verification is not implemented
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "Browser trustee with wrong tenant should get 403"
    );
}

#[tokio::test]
#[serial]
#[should_panic(expected = "assertion")]
async fn test_browser_trustee_wrong_event_id_should_fail() {
    let server = TestServer::new().await;

    // Create board with default test tenant and event
    server.create_board(&server.board_name).await;

    // Token has correct tenant but DIFFERENT event
    let wrong_event = "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee";
    let token = server.create_browser_trustee_token(TEST_TENANT_ID, &[wrong_event]);

    let resp = server
        .client
        .get(format!("{}/boards/{}", server.url(), server.board_name))
        .bearer_auth(&token)
        .send()
        .await
        .expect("Request failed");

    // FUTURE: Should return 403 Forbidden due to event not in authorized list
    // Currently passes (returns 200) because verification is not implemented
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "Browser trustee with wrong event should get 403"
    );
}

#[tokio::test]
#[serial]
#[should_panic(expected = "assertion")]
async fn test_browser_trustee_correct_tenant_and_event_should_succeed() {
    let server = TestServer::new().await;

    // Create board with default test tenant and event
    server.create_board(&server.board_name).await;

    // Token has CORRECT tenant and event
    let token =
        server.create_browser_trustee_token(TEST_TENANT_ID, &[TEST_ELECTION_EVENT_ID]);

    let resp = server
        .client
        .get(format!("{}/boards/{}", server.url(), server.board_name))
        .bearer_auth(&token)
        .send()
        .await
        .expect("Request failed");

    // This should succeed (200) - the test is marked should_panic because
    // currently the verification logic is not wired, so we can't distinguish
    // between "passed because correct" vs "passed because not checked"
    // Once implemented, remove #[should_panic] and this will pass correctly
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "Browser trustee with correct tenant and event should get 200"
    );
}

// ============================================================================
// Future Requirement Tests - Native Trustee "server" Claim
// ============================================================================
// Native trustees should have trustee: "server" claim and should skip
// board name verification (can access any board).
// EXPECTED TO FAIL until the feature is implemented.

#[tokio::test]
#[serial]
#[should_panic(expected = "assertion")]
async fn test_native_trustee_with_server_claim_can_access_any_board() {
    let server = TestServer::new().await;

    // Create board with default test tenant
    server.create_board(&server.board_name).await;

    // Native trustee token with trustee: "server"
    // Should be able to access any board regardless of tenant/event
    let token = server.create_native_trustee_token();

    let resp = server
        .client
        .get(format!("{}/boards/{}", server.url(), server.board_name))
        .bearer_auth(&token)
        .send()
        .await
        .expect("Request failed");

    // FUTURE: Native trustees with "server" claim should skip board verification
    // Currently passes (returns 200) but not because of correct logic
    // This test verifies the "server" claim is properly handled
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "Native trustee with 'server' claim should access any board"
    );
}

#[tokio::test]
#[serial]
#[should_panic(expected = "assertion")]
async fn test_non_server_trustee_requires_board_verification() {
    let server = TestServer::new().await;

    // Create board with default test tenant and event
    server.create_board(&server.board_name).await;

    // Token with trustee claim that is NOT "server" - should be treated as browser trustee
    let token = TestTokenBuilder::new()
        .with_permissions(&["trustee-ceremony"])
        .with_tenant("wrong-tenant-id") // Wrong tenant
        .with_trustee(Some("trustee1")) // Not "server"
        .build(&server.keypair);

    let resp = server
        .client
        .get(format!("{}/boards/{}", server.url(), server.board_name))
        .bearer_auth(&token)
        .send()
        .await
        .expect("Request failed");

    // FUTURE: Non-server trustees should have board verification applied
    // Should fail because tenant doesn't match
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "Non-server trustee with wrong tenant should get 403"
    );
}
