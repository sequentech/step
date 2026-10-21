// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::serialization::deserialize_with_path::deserialize_str;
use crate::services::connection;
use crate::services::connection::PRE_EXPIRATION_SECS;
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
use std::sync::RwLock;
use std::time::{Duration, Instant};
use tracing::{event, info, instrument, warn, Level};

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
            anyhow!(format!("Error serializing: {err:?}, Token: {token:?}"))
        })?;
        deserialize_str(&json).map_err(|err| {
            anyhow!(format!("Error deserializing: {err:?}, Token: {json:?}"))
        })
    }
}

impl TryFrom<PubKeycloakAdminToken> for KeycloakAdminToken {
    type Error = anyhow::Error;

    fn try_from(token: PubKeycloakAdminToken) -> Result<Self, Self::Error> {
        let json = serde_json::to_string(&token)
            .map_err(|err| anyhow!(format!("{err:?}, Token: {token:?}")))?;

        deserialize_str(&json).map_err(|err| {
            anyhow!(format!("Error deserializing: {err:?}, Token: {json:?}"))
        })
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

#[instrument(err)]
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

// Client Credentials OpenID Authentication flow.
// This enables servers to authenticate, without using a browser.
#[instrument(err)]
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

#[instrument(err)]
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
#[instrument(err)]
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

/// TokenResponse, timestamp before sending the request and url to avoid having
/// to retrieve it again from the ENV.
#[derive(Debug, Clone)]
struct TokenResponseAdminCli {
    token_resp: PubKeycloakAdminToken,
    timestamp: Instant,
    url: String,
}

/// Last access token can be reused if it´s not expired, this is to avoid
/// requesting a new token to Keycloak everytime.
type LastAdminCliToken = RwLock<Option<TokenResponseAdminCli>>;
static LAST_ADMIN_CLI_TOKEN: LastAdminCliToken = RwLock::new(None);

/// Reads the access token if it has been requested successfully before and
/// it is not expired.
#[instrument(skip_all)]
async fn read_access_token() -> Option<(PubKeycloakAdminToken, String)> {
    let token_resp_ext_opt = match LAST_ADMIN_CLI_TOKEN.read() {
        Ok(read) => read.clone(),
        Err(err) => {
            warn!("Error acquiring read lock {err:?}");
            return None;
        }
    };

    if let Some(data) = token_resp_ext_opt {
        let pre_expiration_time: i64 =
            data.token_resp.expires_in as i64 - PRE_EXPIRATION_SECS; // Renew the token 5 seconds before it expires
        if pre_expiration_time.is_positive()
            && data.timestamp.elapsed()
                < Duration::from_secs(pre_expiration_time as u64)
        {
            return Some((data.token_resp, data.url));
        }
    }
    return None;
}

/// Request a new access token and writes it to the cache
#[instrument(err, skip_all)]
async fn write_access_token(
    token_resp: PubKeycloakAdminToken,
    url: String,
    timestamp: Instant,
) -> Result<()> {
    let mut write = LAST_ADMIN_CLI_TOKEN
        .write()
        .map_err(|err| anyhow!("Error acquiring write lock: {err:?}"))?;

    *write = Some(TokenResponseAdminCli {
        token_resp,
        timestamp,
        url,
    });

    Ok(())
} // release the lock

impl KeycloakAdminClient {
    /// Tries to read the token from the cache, if expired requests it to
    /// Keycloak.
    #[instrument(err)]
    pub async fn new() -> Result<KeycloakAdminClient> {
        match read_access_token().await {
            Some((token_resp, url)) => {
                Self::new_with(token_resp.try_into()?, &url).await
            }
            None => {
                let login_config = get_keycloak_login_admin_config();
                let timestamp: Instant = Instant::now(); // Capture the stamp before sending the request
                let client = reqwest::Client::new();
                let admin_token = KeycloakAdminToken::acquire(
                    &login_config.url,
                    &login_config.client_id,
                    &login_config.client_secret,
                    &client,
                )
                .await
                .map_err(|err| {
                    anyhow!("KeycloakAdminToken::acquire error {err:?}")
                })?;
                info!("Successfully acquired credentials");
                let token_resp: PubKeycloakAdminToken =
                    admin_token.clone().try_into()?;
                write_access_token(
                    token_resp,
                    login_config.url.clone(),
                    timestamp,
                )
                .await
                .map_err(|err| {
                    anyhow!(
                        "KeycloakAdminClient: write_access_token error {err:?}"
                    )
                })?;
                let keycloak_admin =
                    KeycloakAdmin::new(&login_config.url, admin_token, client);
                Ok(KeycloakAdminClient {
                    client: keycloak_admin,
                })
            }
        }
    }

    /// Creates a KeycloakAdminClient via fresh token requesting to Keycloak
    #[instrument(err)]
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

    #[instrument(err, skip_all)]
    async fn new_with(
        admin_token: KeycloakAdminToken,
        url: &str,
    ) -> Result<KeycloakAdminClient> {
        let client = reqwest::Client::new();
        let client = KeycloakAdmin::new(url, admin_token, client);
        Ok(KeycloakAdminClient { client })
    }

    #[instrument(err)]
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
