// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::connection;
use crate::services::keycloak::realm::get_tenant_realm;
use anyhow::{anyhow, Result};
use keycloak::{
    KeycloakAdmin, KeycloakAdminToken, KeycloakError, KeycloakTokenSupplier,
};
use reqwest;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};
use rocket::async_trait;
use serde::{Deserialize, Serialize};
use serde_json;
use std::convert::TryFrom;
use std::env;
use tracing::{event, info, instrument, Level};

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
    type Error = serde_json::Error;

    fn try_from(token: KeycloakAdminToken) -> Result<Self, Self::Error> {
        let json = serde_json::to_string(&token)?;
        serde_json::from_str(&json)
    }
}

impl TryFrom<PubKeycloakAdminToken> for KeycloakAdminToken {
    type Error = serde_json::Error;

    fn try_from(token: PubKeycloakAdminToken) -> Result<Self, Self::Error> {
        let json = serde_json::to_string(&token)?;
        serde_json::from_str(&json)
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
    let credentials: KeycloakAdminToken = serde_json::from_str(&text)
        .map_err(|err| anyhow!(format!("{:?}, Response: {}", err, text)))?;
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
    let credentials: KeycloakAdminToken = serde_json::from_str(&text)
        .map_err(|err| anyhow!(format!("{:?}, Response: {}", err, text)))?;
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

    let (text) = get_credentials_inner(login_config).await?;
    let keycloak_adm_tkn: KeycloakAdminToken = serde_json::from_str(&text)
        .map_err(|err| anyhow!(format!("{:?}, Response: {}", err, text)))?;

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
    #[instrument(err)]
    pub async fn new() -> Result<KeycloakAdminClient> {
        let login_config = get_keycloak_login_admin_config();
        let client = reqwest::Client::new();
        let admin_token = KeycloakAdminToken::acquire(
            &login_config.url,
            &login_config.client_id,
            &login_config.client_secret,
            &client,
        )
        .await?;
        event!(Level::INFO, "Successfully acquired credentials");
        let client = KeycloakAdmin::new(&login_config.url, admin_token, client);
        Ok(KeycloakAdminClient { client })
    }

    #[instrument(err)]
    pub async fn new_return_token(
    ) -> Result<(KeycloakAdminClient, PubKeycloakAdminToken)> {
        let login_config = get_keycloak_login_admin_config();
        let client = reqwest::Client::new();
        let admin_token = KeycloakAdminToken::acquire(
            &login_config.url,
            &login_config.client_id,
            &login_config.client_secret,
            &client,
        )
        .await?;
        event!(Level::INFO, "Successfully acquired credentials");
        let client =
            KeycloakAdmin::new(&login_config.url, admin_token.clone(), client);
        Ok((KeycloakAdminClient { client }, admin_token.try_into()?))
    }

    #[instrument(err)]
    pub async fn new_with_token(
        admin_token: KeycloakAdminToken,
    ) -> Result<KeycloakAdminClient> {
        let client = reqwest::Client::new();
        let login_config = get_keycloak_login_admin_config();
        let client = KeycloakAdmin::new(&login_config.url, admin_token, client);
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
        .await?;
        event!(Level::INFO, "Successfully acquired credentials");
        Ok(PubKeycloakAdmin {
            url: login_config.url,
            client: client,
            token_supplier: admin_token,
        })
    }
}
