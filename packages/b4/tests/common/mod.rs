// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Common test infrastructure for B4 integration tests.
//!
//! This module provides the TestServer which starts a real HTTP server
//! with PostgreSQL and LocalStack (S3) containers.

use axum::{
    routing::{get, post},
    Router,
};
use b4::{
    db::{self, PgConnectionParams},
    handlers,
    state::AppState,
};
use reqwest::{Client, StatusCode};
use sequent_core::services::{
    jwks::test_support::setup_test_jwks_cache,
    test_utils::{
        create_test_board_name, generate_test_keypair, TestKeyPair, TestTokenBuilder,
        TEST_ELECTION_EVENT_ID, TEST_SLUG, TEST_TENANT_ID,
    },
};
use std::net::SocketAddr;
use std::time::Duration;
use testcontainers::{runners::AsyncRunner, ContainerAsync, ImageExt};
use testcontainers_modules::{localstack::LocalStack, postgres::Postgres};
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};

/// Test server that runs the actual B4 HTTP server with real DB and S3.
pub struct TestServer {
    pub addr: SocketAddr,
    pub keypair: TestKeyPair,
    pub client: Client,
    pub board_name: String,
    _pg_container: ContainerAsync<Postgres>,
    _s3_container: ContainerAsync<LocalStack>,
}

impl TestServer {
    /// Creates a new test server with PostgreSQL and LocalStack containers.
    pub async fn new() -> Self {
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
        let pg_params = PgConnectionParams::new(
            &pg_host.to_string(),
            pg_port,
            "test",
            "test",
            "b4_test",
        );
        let db_pool = db::init_db_with_params(&pg_params)
            .await
            .expect("Failed to initialize database");

        // Initialize S3 client with LocalStack endpoint
        std::env::set_var("AWS_ENDPOINT_URL", &s3_endpoint);
        std::env::set_var("AWS_ACCESS_KEY_ID", "test");
        std::env::set_var("AWS_SECRET_ACCESS_KEY", "test");
        std::env::set_var("AWS_REGION", "us-east-1");

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

        // Create app state
        let state = AppState::new(db_pool, s3_client);

        // Build the actual B4 router (same as main.rs)
        let cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any);

        let app = Router::new()
            // Board management
            .route("/boards", post(handlers::create_board))
            .route("/boards", get(handlers::list_boards))
            .route("/boards/:board", get(handlers::get_board))
            // Message operations (board-specific)
            .route(
                "/boards/:board/messages/initiate",
                post(handlers::initiate_message),
            )
            .route(
                "/boards/:board/messages/:id/confirm",
                post(handlers::confirm_message),
            )
            .route("/boards/:board/messages/list", get(handlers::list_messages))
            .route("/boards/:board/messages", get(handlers::get_messages))
            .route("/boards/:board/messages/:id", get(handlers::get_message))
            // Multi-board operations (GET)
            .route(
                "/boards/messages/multi/get",
                post(handlers::get_messages_multi),
            )
            // Multi-board operations (PUT - S3 two-step flow)
            .route(
                "/boards/messages/multi/initiate",
                post(handlers::initiate_messages_multi),
            )
            .route(
                "/boards/messages/multi/confirm",
                post(handlers::confirm_messages_multi),
            )
            .layer(cors)
            .with_state(state);

        // Start HTTP server on random port
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("Failed to bind listener");
        let addr = listener.local_addr().expect("Failed to get local addr");

        // Spawn the server
        tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("Server error");
        });

        // Wait for server to be ready
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Create HTTP client
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        // Default test board name
        let board_name = create_test_board_name(TEST_TENANT_ID, TEST_ELECTION_EVENT_ID, TEST_SLUG);

        TestServer {
            addr,
            keypair,
            client,
            board_name,
            _pg_container: pg_container,
            _s3_container: s3_container,
        }
    }

    /// Returns the base URL for the test server.
    pub fn url(&self) -> String {
        format!("http://{}", self.addr)
    }

    /// Creates a valid token with the specified permissions.
    pub fn create_token(&self, permissions: &[&str]) -> String {
        TestTokenBuilder::new()
            .with_permissions(permissions)
            .build(&self.keypair)
    }

    /// Creates a valid trustee token.
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

    /// Creates a browser trustee token with specific tenant and authorized events.
    pub fn create_browser_trustee_token(
        &self,
        tenant_id: &str,
        authorized_event_ids: &[&str],
    ) -> String {
        TestTokenBuilder::new()
            .with_permissions(&["trustee-ceremony"])
            .with_tenant(tenant_id)
            .with_authorized_election_ids(authorized_event_ids)
            .with_trustee(Some("trustee1"))
            .build(&self.keypair)
    }

    /// Creates a native trustee token with the "server" claim.
    pub fn create_native_trustee_token(&self) -> String {
        TestTokenBuilder::new()
            .with_permissions(&["trustee-ceremony"])
            .with_trustee(Some("server"))
            .build(&self.keypair)
    }

    /// Creates a board and returns its name.
    pub async fn create_board(&self, name: &str) -> String {
        let token = self.create_trustee_token();
        let resp = self
            .client
            .post(format!("{}/boards", self.url()))
            .bearer_auth(&token)
            .json(&serde_json::json!({ "name": name }))
            .send()
            .await
            .expect("Failed to create board");

        assert_eq!(resp.status(), StatusCode::OK, "Failed to create board");
        name.to_string()
    }
}
