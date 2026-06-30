// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::serialization::deserialize_with_path::deserialize_str;
use crate::services::connection;
use crate::services::keycloak::cache::{get_admin_token_cache, TokenResponse};
use crate::services::keycloak::realm::get_tenant_realm;
use anyhow::{anyhow, Result};
use keycloak::{KeycloakAdmin, KeycloakAdminToken, KeycloakTokenSupplier};
use reqwest;
use reqwest_middleware::ClientBuilder;
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};
use serde::{Deserialize, Serialize};
use serde_json;
use std::convert::TryFrom;
use std::env;
use tracing::{event, info, instrument, warn, Level};

/// Public Keycloak admin token with all fields exposed.
///
/// This is kept for backward compatibility with code that uses the keycloak
/// crate's KeycloakAdminToken type.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct PubKeycloakAdminToken {
    pub access_token: String,
    pub expires_in: usize,
    #[serde(rename = "not-before-policy")]
    pub not_before_policy: Option<usize>,
    pub refresh_expires_in: Option<usize>,
    pub refresh_token: Option<String>,
    pub scope: String,
    pub session_state: Option<String>,
    pub token_type: String,
}

impl TryFrom<KeycloakAdminToken> for PubKeycloakAdminToken {
    type Error = anyhow::Error;

    fn try_from(token: KeycloakAdminToken) -> Result<Self, Self::Error> {
        let json = serde_json::to_string(&token).map_err(|err| {
            anyhow!("Error serializing: {err:?}, Token: {token:?}")
        })?;
        deserialize_str(&json).map_err(|err| {
            anyhow!("Error deserializing: {err:?}, Token: {json:?}")
        })
    }
}

impl TryFrom<PubKeycloakAdminToken> for KeycloakAdminToken {
    type Error = anyhow::Error;

    fn try_from(token: PubKeycloakAdminToken) -> Result<Self, Self::Error> {
        let json = serde_json::to_string(&token)
            .map_err(|err| anyhow!("{err:?}, Token: {token:?}"))?;

        deserialize_str(&json).map_err(|err| {
            anyhow!("Error deserializing: {err:?}, Token: {json:?}")
        })
    }
}

impl From<TokenResponse> for PubKeycloakAdminToken {
    fn from(token: TokenResponse) -> Self {
        PubKeycloakAdminToken {
            access_token: token.access_token,
            expires_in: token.expires_in,
            not_before_policy: token.not_before_policy,
            refresh_expires_in: token.refresh_expires_in,
            refresh_token: token.refresh_token,
            scope: token.scope.unwrap_or_default(),
            session_state: token.session_state,
            token_type: token
                .token_type
                .unwrap_or_else(|| "Bearer".to_string()),
        }
    }
}

impl From<PubKeycloakAdminToken> for TokenResponse {
    fn from(token: PubKeycloakAdminToken) -> Self {
        TokenResponse {
            access_token: token.access_token,
            expires_in: token.expires_in,
            not_before_policy: token.not_before_policy,
            refresh_expires_in: token.refresh_expires_in,
            refresh_token: token.refresh_token,
            scope: Some(token.scope),
            session_state: token.session_state,
            token_type: Some(token.token_type),
        }
    }
}

#[derive(Debug)]
struct KeycloakLoginConfig {
    url: String,
    client_id: String,
    client_secret: String,
    realm: String,
}

impl KeycloakLoginConfig {
    pub fn new(
        client_id: String,
        client_secret: String,
        tenant_id: String,
    ) -> KeycloakLoginConfig {
        let url = env::var("KEYCLOAK_URL")
            .expect(&format!("KEYCLOAK_URL must be set"));
        let realm = get_tenant_realm(&tenant_id);
        Self {
            url,
            client_id,
            client_secret,
            realm,
        }
    }
}

fn get_keycloak_login_config() -> KeycloakLoginConfig {
    let client_id = env::var("KEYCLOAK_CLIENT_ID")
        .expect(&format!("KEYCLOAK_CLIENT_ID must be set"));
    let client_secret = env::var("KEYCLOAK_CLIENT_SECRET")
        .expect(&format!("KEYCLOAK_CLIENT_SECRET must be set"));
    let tenant_id = env::var("SUPER_ADMIN_TENANT_ID")
        .expect(&format!("SUPER_ADMIN_TENANT_ID must be set"));
    KeycloakLoginConfig::new(client_id, client_secret, tenant_id)
}

fn get_keycloak_login_admin_config() -> KeycloakLoginConfig {
    let client_id = env::var("KEYCLOAK_ADMIN_CLIENT_ID")
        .expect(&format!("KEYCLOAK_ADMIN_CLIENT_ID must be set"));
    let client_secret = env::var("KEYCLOAK_ADMIN_CLIENT_SECRET")
        .expect(&format!("KEYCLOAK_ADMIN_CLIENT_SECRET must be set"));
    let tenant_id = env::var("SUPER_ADMIN_TENANT_ID")
        .expect(&format!("SUPER_ADMIN_TENANT_ID must be set"));
    KeycloakLoginConfig::new(client_id, client_secret, tenant_id)
}

#[instrument(level = "trace", err)]
pub async fn get_credentials_inner(
    login_config: KeycloakLoginConfig,
) -> Result<String> {
    let body_string = serde_urlencoded::to_string::<[(String, String); 4]>([
        ("client_id".into(), login_config.client_id.clone()),
        ("scope".into(), "openid".into()),
        ("client_secret".into(), login_config.client_secret.clone()),
        ("grant_type".into(), "client_credentials".into()),
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
        "Acquiring credentials to {} with {:?}",
        keycloak_endpoint,
        body_string
    );

    let res = async {
        let res_future = client
            .post(keycloak_endpoint)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body_string)
            .send();
        event!(Level::INFO, "Awaiting future from endpoint");
        let res = res_future.await;
        event!(Level::INFO, "Result from endpoint: {:?}", res);
        res
    }
    .await?;

    res.text().await.map_err(|e| anyhow!(e))
}

/// Refreshes an admin token using the refresh_token grant type.
///
/// This forces Keycloak to issue a brand new token pair, unlike the
/// client_credentials grant which may return the same near-expired token
/// from an active session.
#[instrument(level = "trace", err, skip(refresh_token))]
async fn refresh_admin_token_inner(
    login_config: &KeycloakLoginConfig,
    refresh_token: &str,
) -> Result<String> {
    let body_string = serde_urlencoded::to_string::<[(String, String); 4]>([
        ("grant_type".into(), "refresh_token".into()),
        ("client_id".into(), login_config.client_id.clone()),
        ("client_secret".into(), login_config.client_secret.clone()),
        ("refresh_token".into(), refresh_token.to_string()),
    ])
    .unwrap();

    let keycloak_endpoint = format!(
        "{}/realms/{}/protocol/openid-connect/token",
        login_config.url, login_config.realm
    );

    let retry_policy = ExponentialBackoff::builder().build_with_max_retries(3);
    let client = ClientBuilder::new(reqwest::Client::new())
        .with(RetryTransientMiddleware::new_with_policy(retry_policy))
        .build();

    event!(Level::INFO, "Refreshing admin token at {keycloak_endpoint}");

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
            "Failed to refresh admin token: HTTP {status}, {error_text}"
        ));
    }

    res.text().await.map_err(|e| anyhow!(e))
}

// Client Credentials OpenID Authentication flow.
// This enables servers to authenticate, without using a browser.
#[instrument(level = "trace", err)]
pub async fn get_client_credentials() -> Result<connection::AuthHeaders> {
    let login_config = get_keycloak_login_config();
    let text = get_credentials_inner(login_config).await?;
    let credentials: KeycloakAdminToken =
        deserialize_str(&text).map_err(|err| {
            anyhow!(format!(
                "Error deserializing: {err:?}, Inner credentials: {text:?}"
            ))
        })?;

    event!(Level::INFO, "Successfully acquired credentials");
    Ok(connection::AuthHeaders {
        key: "authorization".into(),
        value: format!(
            "Bearer {}",
            credentials.get("").await.unwrap_or_default()
        ),
    })
}

#[instrument(level = "trace", err)]
pub async fn get_auth_credentials() -> Result<KeycloakAdminToken> {
    let login_config = get_keycloak_login_config();
    let text = get_credentials_inner(login_config).await?;
    let credentials: KeycloakAdminToken =
        deserialize_str(&text).map_err(|err| {
            anyhow!(format!(
                "Error deserializing: {err:?}, Inner credentials: {text:?}"
            ))
        })?;
    event!(Level::INFO, "Successfully acquired credentials");
    Ok(credentials)
}

/// Authenticate a party client in keycloak with specific client credentials and
/// tenant_id
#[instrument(level = "trace", err)]
pub async fn get_third_party_client_access_token(
    client_id: String,
    client_secret: String,
    tenant_id: String,
) -> Result<KeycloakAdminToken> {
    let login_config =
        KeycloakLoginConfig::new(client_id, client_secret, tenant_id);

    let text = get_credentials_inner(login_config).await?;
    let keycloak_adm_tkn: KeycloakAdminToken =
        deserialize_str(&text).map_err(|err| {
            anyhow!(format!(
                "Error deserializing: {err:?}, Inner credentials: {text:?}"
            ))
        })?;

    event!(Level::INFO, "Successfully acquired credentials");
    Ok(keycloak_adm_tkn)
}

pub struct KeycloakAdminClient {
    pub client: KeycloakAdmin,
}

pub struct PubKeycloakAdmin {
    pub url: String,
    pub client: reqwest::Client,
    pub token_supplier: KeycloakAdminToken,
}

impl KeycloakAdminClient {
    /// Tries to read the token from the cache, if expired requests it to
    /// Keycloak.
    #[instrument(level = "trace", err)]
    pub async fn new() -> Result<KeycloakAdminClient> {
        let cache = get_admin_token_cache();

        // Fast path: check cache without fetch lock
        if let Some((token_resp, url)) = cache.read_token() {
            let pub_token: PubKeycloakAdminToken = token_resp.into();
            return Self::new_with(pub_token.try_into()?, &url).await;
        }

        // Acquire fetch lock to prevent thundering herd
        let _fetch_guard = cache.fetch_lock.lock().await;

        // Double-check: someone else may have fetched while we waited
        if let Some((token_resp, url)) = cache.read_token() {
            let pub_token: PubKeycloakAdminToken = token_resp.into();
            return Self::new_with(pub_token.try_into()?, &url).await;
        }

        let login_config = get_keycloak_login_admin_config();

        // Try to refresh using the cached refresh token first, since
        // client_credentials grant may return the same near-expired token
        // from Keycloak's active session.
        if let Some(cached) = cache.read_token_for_refresh() {
            if let Some(ref refresh_token) = cached.refresh_token {
                match refresh_admin_token_inner(&login_config, refresh_token)
                    .await
                {
                    Ok(text) => {
                        let token_resp: TokenResponse =
                            deserialize_str(&text).map_err(|err| {
                                anyhow!("Error deserializing refreshed admin token: {err:?}, response: {text:?}")
                            })?;
                        info!("Successfully refreshed admin credentials");
                        let pub_token: PubKeycloakAdminToken =
                            token_resp.clone().into();
                        cache
                            .write_token(
                                token_resp,
                                login_config.url.clone(),
                            )
                            .map_err(|err| {
                                anyhow!("KeycloakAdminClient: write_token error {err:?}")
                            })?;
                        let admin_token: KeycloakAdminToken =
                            pub_token.try_into()?;
                        let client = reqwest::Client::new();
                        let keycloak_admin = KeycloakAdmin::new(
                            &login_config.url,
                            admin_token,
                            client,
                        );
                        return Ok(KeycloakAdminClient {
                            client: keycloak_admin,
                        });
                    }
                    Err(err) => {
                        warn!(
                            "Admin refresh token failed, falling back to acquire: {err}"
                        );
                    }
                }
            }
        }

        // Fall back to full client_credentials authentication
        let client = reqwest::Client::new();
        let admin_token = KeycloakAdminToken::acquire(
            &login_config.url,
            &login_config.client_id,
            &login_config.client_secret,
            &client,
        )
        .await
        .map_err(|err| anyhow!("KeycloakAdminToken::acquire error {err:?}"))?;
        info!("Successfully acquired credentials");
        let pub_token: PubKeycloakAdminToken =
            admin_token.clone().try_into()?;
        let token_resp: TokenResponse = pub_token.into();
        cache
            .write_token(token_resp, login_config.url.clone())
            .map_err(|err| {
                anyhow!("KeycloakAdminClient: write_token error {err:?}")
            })?;
        let keycloak_admin =
            KeycloakAdmin::new(&login_config.url, admin_token, client);
        Ok(KeycloakAdminClient {
            client: keycloak_admin,
        })
    }

    /// Creates a KeycloakAdminClient via fresh token requesting to Keycloak
    #[instrument(level = "trace", err)]
    pub async fn new_requested() -> Result<KeycloakAdminClient> {
        let login_config = get_keycloak_login_admin_config();
        let client = reqwest::Client::new();
        let admin_token = KeycloakAdminToken::acquire(
            &login_config.url,
            &login_config.client_id,
            &login_config.client_secret,
            &client,
        )
        .await
        .map_err(|err| anyhow!("KeycloakAdminToken::acquire error {err:?}"))?;
        info!("Successfully acquired credentials");
        let client = KeycloakAdmin::new(&login_config.url, admin_token, client);
        Ok(KeycloakAdminClient { client })
    }

    #[instrument(level = "trace", err, skip_all)]
    async fn new_with(
        admin_token: KeycloakAdminToken,
        url: &str,
    ) -> Result<KeycloakAdminClient> {
        let client = reqwest::Client::new();
        let client = KeycloakAdmin::new(url, admin_token, client);
        Ok(KeycloakAdminClient { client })
    }

    /// Returns the cached access token string, fetching from Keycloak if needed.
    ///
    /// This method reads the token from the global cache. If the cache is empty
    /// or expired, it triggers a fetch via `KeycloakAdminClient::new()` to
    /// populate the cache, then reads again.
    #[instrument(level = "trace", err)]
    pub async fn get_cached_token() -> Result<String> {
        let cache = get_admin_token_cache();

        // Try reading from cache first
        if let Some((token_resp, _url)) = cache.read_token() {
            return Ok(token_resp.access_token);
        }

        // Cache miss - populate cache by calling new()
        let _ = KeycloakAdminClient::new().await?;

        // Read again after populating
        if let Some((token_resp, _url)) = cache.read_token() {
            return Ok(token_resp.access_token);
        }

        Err(anyhow!("Failed to get cached token after fetch"))
    }

    /// Not using the cache, creates a public KeycloakAdmin client requesting
    /// a new token from Keycloak.
    /// TODO: Consider removing PubKeycloakAdmin entirely and using only KeycloakAdminClient::new()
    #[instrument(level = "trace", err)]
    pub async fn pub_new() -> Result<PubKeycloakAdmin> {
        let login_config = get_keycloak_login_admin_config();
        let client = reqwest::Client::new();
        let admin_token = KeycloakAdminToken::acquire(
            &login_config.url,
            &login_config.client_id,
            &login_config.client_secret,
            &client,
        )
        .await
        .map_err(|err| anyhow!("KeycloakAdminToken::acquire error {err:?}"))?;
        event!(Level::INFO, "Successfully acquired credentials");
        Ok(PubKeycloakAdmin {
            url: login_config.url,
            client: client,
            token_supplier: admin_token,
        })
    }
}
