// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Keycloak user client for Resource Owner Password Credentials flow.
//!
//! This module provides authentication for users (e.g., trustees) using
//! the OAuth 2.0 password grant type instead of client credentials.

use crate::serialization::deserialize_with_path::deserialize_str;
use crate::services::keycloak::cache::{get_user_token_cache, TokenResponse};
use crate::services::keycloak::realm::get_tenant_realm;
use anyhow::{anyhow, Result};
use reqwest_middleware::ClientBuilder;
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};
use std::env;
use std::time::Instant;
use tracing::{event, info, instrument, Level};

/// Configuration for Resource Owner Password Credentials flow.
#[derive(Debug)]
pub struct KeycloakUserLoginConfig {
    pub url: String,
    pub client_id: String,
    pub client_secret: String,
    pub username: String,
    pub password: String,
    pub realm: String,
}

impl KeycloakUserLoginConfig {
    /// Creates a new user login configuration.
    ///
    /// # Arguments
    /// * `username` - The Keycloak username (e.g., "trustee1")
    /// * `password` - The user's password
    /// * `client_id` - OAuth client ID (must have directAccessGrantsEnabled)
    /// * `client_secret` - OAuth client secret
    /// * `tenant_id` - The tenant ID for realm construction
    pub fn new(
        username: String,
        password: String,
        client_id: String,
        client_secret: String,
        tenant_id: String,
    ) -> Self {
        let url = env::var("KEYCLOAK_URL").expect("KEYCLOAK_URL must be set");
        let realm = get_tenant_realm(&tenant_id);
        Self {
            url,
            client_id,
            client_secret,
            username,
            password,
            realm,
        }
    }
}

/// Fetches a token using the Resource Owner Password Credentials flow.
#[instrument(level = "trace", err, skip(login_config))]
async fn get_user_credentials_inner(
    login_config: &KeycloakUserLoginConfig,
) -> Result<String> {
    let body_string = serde_urlencoded::to_string::<[(String, String); 6]>([
        ("grant_type".into(), "password".into()),
        ("scope".into(), "openid".into()),
        ("client_id".into(), login_config.client_id.clone()),
        ("client_secret".into(), login_config.client_secret.clone()),
        ("username".into(), login_config.username.clone()),
        ("password".into(), login_config.password.clone()),
    ])
    .unwrap();

    let keycloak_endpoint = format!(
        "{}/realms/{}/protocol/openid-connect/token",
        login_config.url, login_config.realm
    );

    // Retry up to 3 times with increasing intervals between attempts.
    let retry_policy = ExponentialBackoff::builder().build_with_max_retries(3);
    let client = ClientBuilder::new(reqwest::Client::new())
        .with(RetryTransientMiddleware::new_with_policy(retry_policy))
        .build();

    event!(
        Level::INFO,
        "Acquiring user credentials to {keycloak_endpoint} for user {}",
        login_config.username
    );

    let res = client
        .post(&keycloak_endpoint)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body_string)
        .send()
        .await?;

    if !res.status().is_success() {
        let status = res.status();
        let error_text = res.text().await.unwrap_or_default();
        return Err(anyhow!(
            "Failed to get user token: HTTP {status}, {error_text}"
        ));
    }

    res.text().await.map_err(|e| anyhow!(e))
}

/// Client for user-based Keycloak authentication using password grant.
///
/// This client is used for authenticating as a specific user (e.g., a trustee)
/// rather than as a service account. It supports token caching to avoid
/// repeated authentication requests.
pub struct KeycloakUserClient;

impl KeycloakUserClient {
    /// Gets a cached access token for the specified user, fetching a new one if
    /// needed.
    ///
    /// This method uses a global cache with thundering herd prevention.
    /// The cache is simple (single token) since each trustee runs in a separate
    /// container.
    ///
    /// # Arguments
    /// * `login_config` - The Keycloak user login configuration
    #[instrument(level = "trace", err, skip(login_config))]
    pub async fn get_cached_token(
        login_config: &KeycloakUserLoginConfig,
    ) -> Result<String> {
        let cache = get_user_token_cache();

        // Fast path: check cache without fetch lock
        if let Some((token_resp, _url)) = cache.read_token() {
            return Ok(token_resp.access_token);
        }

        // Acquire fetch lock to prevent thundering herd
        let _fetch_guard = cache.fetch_lock.lock().await;

        // Double-check: someone else may have fetched while we waited
        if let Some((token_resp, _url)) = cache.read_token() {
            return Ok(token_resp.access_token);
        }

        // Still a cache miss, fetch from Keycloak
        let timestamp = Instant::now();
        let text = get_user_credentials_inner(login_config).await?;

        let token_resp: TokenResponse = deserialize_str(&text).map_err(|err| {
            anyhow!("Error deserializing user token: {err:?}, response: {text:?}")
        })?;

        info!(
            "Successfully acquired user credentials for {}",
            login_config.username
        );

        cache
            .write_token(
                token_resp.clone(),
                login_config.url.clone(),
                timestamp,
            )
            .map_err(|err| anyhow!("Failed to write token to cache: {err}"))?;

        Ok(token_resp.access_token)
    }

    /// Gets a fresh access token without using the cache.
    ///
    /// # Arguments
    /// * `login_config` - The Keycloak user login configuration
    #[instrument(level = "trace", err, skip(login_config))]
    pub async fn get_token_uncached(
        login_config: &KeycloakUserLoginConfig,
    ) -> Result<String> {
        let text = get_user_credentials_inner(login_config).await?;

        let token_resp: TokenResponse = deserialize_str(&text).map_err(|err| {
            anyhow!("Error deserializing user token: {err:?}, response: {text:?}")
        })?;

        info!(
            "Successfully acquired user credentials for {}",
            login_config.username
        );

        Ok(token_resp.access_token)
    }
}

/// Generates a Keycloak token using Resource Owner Password Credentials flow.
///
/// This is a blocking version for use in synchronous contexts (e.g., step-cli).
///
/// # Arguments
/// * `keycloak_url` - The Keycloak server URL
/// * `username` - The username to authenticate as
/// * `password` - The user's password
/// * `client_id` - OAuth client ID
/// * `client_secret` - OAuth client secret
/// * `tenant_id` - Tenant ID for realm construction
#[instrument(level = "trace", err, skip(password, client_secret))]
pub fn generate_keycloak_token(
    keycloak_url: &str,
    username: &str,
    password: &str,
    client_id: &str,
    client_secret: &str,
    tenant_id: &str,
) -> Result<TokenResponse> {
    let params = [
        ("grant_type", "password"),
        ("scope", "openid"),
        ("client_id", client_id),
        ("client_secret", client_secret),
        ("username", username),
        ("password", password),
    ];

    let realm = format!("tenant-{tenant_id}");
    let url =
        format!("{keycloak_url}/realms/{realm}/protocol/openid-connect/token");

    let client = reqwest::blocking::Client::new();
    let response = client.post(&url).form(&params).send()?;

    if response.status().is_success() {
        let token_response: TokenResponse = response.json()?;
        Ok(token_response)
    } else {
        let status = response.status();
        let error_message = response.text()?;
        Err(anyhow!(
            "HTTP Status: {status}\nError Message: {error_message}"
        ))
    }
}

/// Refreshes a Keycloak token using a refresh token.
///
/// This is a blocking version for use in synchronous contexts (e.g., step-cli).
#[instrument(level = "trace", err, skip(refresh_token, client_secret))]
pub fn refresh_keycloak_token(
    keycloak_url: &str,
    refresh_token: &str,
    client_id: &str,
    client_secret: &str,
    tenant_id: &str,
) -> Result<TokenResponse> {
    let params = [
        ("grant_type", "refresh_token"),
        ("client_id", client_id),
        ("client_secret", client_secret),
        ("refresh_token", refresh_token),
    ];

    let realm = format!("tenant-{tenant_id}");
    let url =
        format!("{keycloak_url}/realms/{realm}/protocol/openid-connect/token");

    let client = reqwest::blocking::Client::new();
    let response = client.post(&url).form(&params).send()?;

    if response.status().is_success() {
        let token_response: TokenResponse = response.json()?;
        Ok(token_response)
    } else {
        let status = response.status();
        let error_message = response.text()?;
        Err(anyhow!(
            "HTTP Status: {status}\nError Message: {error_message}"
        ))
    }
}
