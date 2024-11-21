// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::connection;
use crate::services::keycloak::realm::get_tenant_realm;
use anyhow::{anyhow, Result};
use keycloak::{KeycloakAdmin, KeycloakAdminToken};
use reqwest;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};
use serde::{Deserialize, Serialize};
use std::env;
use tracing::{event, instrument, Level};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct TokenResponse {
    pub access_token: String,
    pub expires_in: i64,
    pub refresh_expires_in: i64,
    pub token_type: String,
    pub id_token: String,
    #[serde(rename = "not-before-policy")]
    pub not_before_policy: i64,
    pub scope: String,
}

struct KeycloakLoginConfig {
    url: String,
    client_id: String,
    client_secret: String,
    realm: String,
}

fn get_keycloak_login_config() -> KeycloakLoginConfig {
    let url =
        env::var("KEYCLOAK_URL").expect(&format!("KEYCLOAK_URL must be set"));
    let client_id = env::var("KEYCLOAK_CLIENT_ID")
        .expect(&format!("KEYCLOAK_CLIENT_ID must be set"));
    let client_secret = env::var("KEYCLOAK_CLIENT_SECRET")
        .expect(&format!("KEYCLOAK_CLIENT_SECRET must be set"));
    let tenant_id = env::var("SUPER_ADMIN_TENANT_ID")
        .expect(&format!("SUPER_ADMIN_TENANT_ID must be set"));
    let realm = get_tenant_realm(&tenant_id);
    KeycloakLoginConfig {
        url,
        client_id,
        client_secret,
        realm,
    }
}

fn get_keycloak_login_admin_config() -> KeycloakLoginConfig {
    let url =
        env::var("KEYCLOAK_URL").expect(&format!("KEYCLOAK_URL must be set"));
    let client_id = env::var("KEYCLOAK_ADMIN_CLIENT_ID")
        .expect(&format!("KEYCLOAK_ADMIN_CLIENT_ID must be set"));
    let client_secret = env::var("KEYCLOAK_ADMIN_CLIENT_SECRET")
        .expect(&format!("KEYCLOAK_ADMIN_CLIENT_SECRET must be set"));
    let tenant_id = env::var("SUPER_ADMIN_TENANT_ID")
        .expect(&format!("SUPER_ADMIN_TENANT_ID must be set"));
    let realm = get_tenant_realm(&tenant_id);
    KeycloakLoginConfig {
        url,
        client_id,
        client_secret,
        realm,
    }
}

// Client Credentials OpenID Authentication flow.
// This enables servers to authenticate, without using a browser.
#[instrument(err)]
pub async fn get_client_credentials() -> Result<connection::AuthHeaders> {
    let login_config = get_keycloak_login_config();
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

    let text = res.text().await?;

    let credentials: TokenResponse = serde_json::from_str(&text)
        .map_err(|err| anyhow!(format!("{:?}, Response: {}", err, text)))?;
    event!(Level::INFO, "Successfully acquired credentials");
    Ok(connection::AuthHeaders {
        key: "authorization".into(),
        value: format!("Bearer {}", credentials.access_token),
    })
}

#[instrument(err)]
pub async fn get_auth_credentials() -> Result<TokenResponse> {
    let login_config = get_keycloak_login_config();
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

    let text = res.text().await?;

    let credentials: TokenResponse = serde_json::from_str(&text)
        .map_err(|err| anyhow!(format!("{:?}, Response: {}", err, text)))?;
    event!(Level::INFO, "Successfully acquired credentials");
    Ok(credentials)
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
