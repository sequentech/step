// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Common test infrastructure for B4 integration tests.
//!
//! This module provides the TestServer which wraps the Axum router
//! with PostgreSQL and LocalStack (S3) containers using axum-test.
//!
//! The server is lazily initialized once and shared across all tests in the
//! same test binary, significantly reducing test execution time.

use axum_test::TestServer as AxumTestServer;
use b4::{
    auth::SERVER_DEFAULT_ROLE,
    db::{self, DbPool, PgConnectionParams},
    state::AppState,
};
use sequent_core::services::{
    jwks::test_support::setup_test_jwks_cache,
    test_utils::{
        create_test_board_name, generate_test_keypair, TestKeyPair, TestTokenBuilder,
        TEST_ELECTION_EVENT_ID, TEST_SLUG, TEST_TENANT_ID,
    },
};
use sequent_core::types::permissions::Permissions;
use std::sync::Arc;
use testcontainers::{runners::AsyncRunner, ContainerAsync, ImageExt};
use testcontainers_modules::{localstack::LocalStack, postgres::Postgres};
use tokio::sync::OnceCell;
use tower_http::cors::{Any, CorsLayer};

/// Global shared test server instance.
static SHARED_SERVER: OnceCell<Arc<TestServer>> = OnceCell::const_new();

/// Returns a shared TestServer instance, initializing it on first call.
///
/// This significantly improves test performance by reusing containers across tests.
pub async fn get_shared_server() -> Arc<TestServer> {
    SHARED_SERVER
        .get_or_init(|| async { Arc::new(TestServer::init().await) })
        .await
        .clone()
}

/// Test server that wraps the B4 router with axum-test.
pub struct TestServer {
    pub server: AxumTestServer,
    pub keypair: TestKeyPair,
    pub board_name: String,
    db_pool: DbPool,
    _pg_container: ContainerAsync<Postgres>,
    _s3_container: ContainerAsync<LocalStack>,
}

impl TestServer {
    /// Returns a shared test server instance (preferred for most tests).
    ///
    /// Uses a lazily-initialized shared server to avoid container startup overhead.
    pub async fn new() -> Arc<TestServer> {
        get_shared_server().await
    }

    /// Creates a fresh test server with new PostgreSQL and LocalStack containers.
    ///
    /// Use this only when you need isolated state (e.g., tests that modify DB
    /// in conflicting ways). Most tests should use `new()` instead.
    pub async fn new_isolated() -> Self {
        Self::init().await
    }

    /// Internal initialization - creates actual containers and server.
    async fn init() -> Self {
        // Start PostgreSQL container
        let pg_container = Postgres::default()
            .with_env_var("POSTGRES_DB", "b4_test")
            .with_env_var("POSTGRES_USER", "test")
            .with_env_var("POSTGRES_PASSWORD", "test")
            .start()
            .await
            .expect("Failed to start PostgreSQL container");

        let pg_host = pg_container.get_host().await.unwrap();
        let pg_port = pg_container.get_host_port_ipv4(5432).await.unwrap();

        // Start LocalStack container for S3
        let s3_container = LocalStack::default()
            .with_env_var("SERVICES", "s3")
            .start()
            .await
            .expect("Failed to start LocalStack container");

        let s3_host = s3_container.get_host().await.unwrap();
        let s3_port = s3_container.get_host_port_ipv4(4566).await.unwrap();
        let s3_endpoint = format!("http://{s3_host}:{s3_port}");

        // Initialize database
        let pg_params =
            PgConnectionParams::new(&pg_host.to_string(), pg_port, "test", "test", "b4_test");
        let db_pool = db::init_db_with_params(&pg_params)
            .await
            .expect("Failed to initialize database");

        // Initialize S3 client with LocalStack endpoint
        std::env::set_var("AWS_ENDPOINT_URL", &s3_endpoint);
        std::env::set_var("AWS_ACCESS_KEY_ID", "test");
        std::env::set_var("AWS_SECRET_ACCESS_KEY", "test");
        std::env::set_var("AWS_REGION", "us-east-1");
        std::env::set_var("AWS_S3_UPLOAD_EXPIRATION_SECS", "120");
        std::env::set_var("AWS_S3_FETCH_EXPIRATION_SECS", "3600");

        let s3_config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
        let s3_client = aws_sdk_s3::Client::from_conf(
            aws_sdk_s3::config::Builder::from(&s3_config)
                .endpoint_url(&s3_endpoint)
                .force_path_style(true)
                .build(),
        );

        // Create test bucket
        let bucket_name = "b4-test-bucket";
        std::env::set_var("S3_BUCKET_NAME", bucket_name);
        s3_client
            .create_bucket()
            .bucket(bucket_name)
            .send()
            .await
            .expect("Failed to create S3 bucket");

        // Set up JWKS cache with test keypair
        let keypair = generate_test_keypair("b4-http-test-key");
        setup_test_jwks_cache(vec![keypair.jwk.clone()]);

        // Create app state (clone pool so we can keep a reference for cleanup)
        let state = AppState::new(db_pool.clone(), s3_client);

        // Build the B4 router using the shared route table from the b4 library.
        let cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any);

        let app = b4::router::build_router().layer(cors).with_state(state);

        // Create axum-test server
        let server = AxumTestServer::new(app).expect("Failed to create test server");

        // Default test board name
        let board_name = create_test_board_name(TEST_TENANT_ID, TEST_ELECTION_EVENT_ID, TEST_SLUG);

        TestServer {
            server,
            keypair,
            board_name,
            db_pool,
            _pg_container: pg_container,
            _s3_container: s3_container,
        }
    }

    /// Creates a valid token with the specified permissions.
    ///
    /// Note: This creates a token without the "server" default_role, so it will
    /// require proper tenant/event claims for board access validation.
    pub fn create_token(&self, permissions: &[&str]) -> String {
        TestTokenBuilder::new()
            .with_permissions(permissions)
            .build(&self.keypair)
    }

    /// Creates a valid trustee token with server role (bypasses board validation).
    ///
    /// Uses `Permissions::TRUSTEE_CEREMONY` and `SERVER_DEFAULT_ROLE` constants.
    /// This is the default for most tests that just need basic auth.
    pub fn create_trustee_token(&self) -> String {
        let permission = Permissions::TRUSTEE_CEREMONY.to_string();
        TestTokenBuilder::new()
            .with_permissions(&[&permission])
            .with_default_role(SERVER_DEFAULT_ROLE)
            .with_trustee(Some(SERVER_DEFAULT_ROLE))
            .build(&self.keypair)
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

    /// Creates a browser trustee token with the given authorized board names.
    ///
    /// Browser trustees do NOT have the server role, so they are subject to
    /// board access validation based on authorized_boards in JWT claims.
    pub fn create_browser_trustee_token(
        &self,
        tenant_id: &str,
        authorized_boards: &[&str],
    ) -> String {
        let permission = Permissions::TRUSTEE_CEREMONY.to_string();
        TestTokenBuilder::new()
            .with_permissions(&[&permission])
            .with_tenant(tenant_id)
            .with_authorized_boards(authorized_boards)
            .with_trustee(Some("trustee1"))
            .build(&self.keypair)
    }

    /// Creates a native trustee token with the "server" default role.
    ///
    /// Native server trustees bypass board access validation and can access any board.
    /// Uses `SERVER_DEFAULT_ROLE` constant.
    pub fn create_native_trustee_token(&self) -> String {
        let permission = Permissions::TRUSTEE_CEREMONY.to_string();
        TestTokenBuilder::new()
            .with_permissions(&[&permission])
            .with_default_role(SERVER_DEFAULT_ROLE)
            .with_trustee(Some(SERVER_DEFAULT_ROLE))
            .build(&self.keypair)
    }

    /// Creates a board and returns its name.
    pub async fn create_board(&self, name: &str) -> String {
        let token = self.create_trustee_token();
        let resp = self
            .server
            .post("/boards")
            .add_header(axum::http::header::AUTHORIZATION, format!("Bearer {token}"))
            .json(&serde_json::json!({ "name": name }))
            .await;

        resp.assert_status_ok();
        name.to_string()
    }

    /// Cleans up all test data from the database.
    ///
    /// Call this at the start of tests that need a clean slate.
    pub async fn cleanup(&self) {
        let conn = self
            .db_pool
            .get()
            .await
            .expect("Failed to get DB connection");

        // Delete all boards (this will cascade to messages due to partitioning)
        conn.execute("DELETE FROM boards", &[])
            .await
            .expect("Failed to cleanup boards");
    }
}
