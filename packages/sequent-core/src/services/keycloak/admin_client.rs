// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::connection;
use crate::services::connection::PRE_EXPIRATION_SECS;
use crate::services::keycloak::realm::get_tenant_realm;
use anyhow::{anyhow, Result};
use keycloak::{
    KeycloakAdmin, KeycloakAdminToken, KeycloakError, KeycloakTokenSupplier,
};
use reqwest;
use reqwest_middleware::ClientBuilder;
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json;
use std::convert::TryFrom;
use std::env;
use std::fmt;
use std::sync::RwLock;
use std::time::{Duration, Instant};
use tracing::{event, info, instrument, warn, Level};

#[derive(Clone, PartialEq, Eq, Deserialize, Serialize)]
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

// This type is included in cached-token Debug output. Its serialized form still
// carries credentials; diagnostic output must not.
impl fmt::Debug for PubKeycloakAdminToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PubKeycloakAdminToken")
            .field("expires_in", &self.expires_in)
            .field("has_refresh_token", &self.refresh_token.is_some())
            .finish_non_exhaustive()
    }
}

fn decode_token_response<T: DeserializeOwned>(text: &str) -> Result<T> {
    // A serde error can quote the offending value. Keep the useful position,
    // without attaching the token response or its raw deserialization error.
    serde_json::from_str(text).map_err(|err| {
        anyhow!(
            "Invalid Keycloak token response at line {}, column {}",
            err.line(),
            err.column()
        )
    })
}

fn credential_request_error(error: KeycloakError) -> anyhow::Error {
    // The dependency's HttpFailure carries the complete server response body.
    match error {
        KeycloakError::HttpFailure { status, .. } => {
            anyhow!("Keycloak credential request failed with HTTP {status}")
        }
        KeycloakError::ReqwestFailure(error) => {
            let kind = if error.is_timeout() {
                "timeout"
            } else if error.is_connect() {
                "connection"
            } else if error.is_decode() {
                "invalid response"
            } else {
                "transport"
            };
            anyhow!("Keycloak credential request failed: {kind}")
        }
    }
}

impl TryFrom<KeycloakAdminToken> for PubKeycloakAdminToken {
    type Error = anyhow::Error;

    fn try_from(token: KeycloakAdminToken) -> Result<Self, Self::Error> {
        let json = serde_json::to_string(&token)
            .map_err(|_| anyhow!("Unable to serialize Keycloak token"))?;
        decode_token_response(&json)
    }
}

impl TryFrom<PubKeycloakAdminToken> for KeycloakAdminToken {
    type Error = anyhow::Error;

    fn try_from(token: PubKeycloakAdminToken) -> Result<Self, Self::Error> {
        let json = serde_json::to_string(&token)
            .map_err(|_| anyhow!("Unable to serialize Keycloak token"))?;
        decode_token_response(&json)
    }
}

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
        let url = env::var("KEYCLOAK_URL").expect("KEYCLOAK_URL must be set");
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
    let client_id =
        env::var("KEYCLOAK_CLIENT_ID").expect("KEYCLOAK_CLIENT_ID must be set");
    let client_secret = env::var("KEYCLOAK_CLIENT_SECRET")
        .expect("KEYCLOAK_CLIENT_SECRET must be set");
    let tenant_id = env::var("SUPER_ADMIN_TENANT_ID")
        .expect("SUPER_ADMIN_TENANT_ID must be set");
    KeycloakLoginConfig::new(client_id, client_secret, tenant_id)
}

fn get_keycloak_login_admin_config() -> KeycloakLoginConfig {
    let client_id = env::var("KEYCLOAK_ADMIN_CLIENT_ID")
        .expect("KEYCLOAK_ADMIN_CLIENT_ID must be set");
    let client_secret = env::var("KEYCLOAK_ADMIN_CLIENT_SECRET")
        .expect("KEYCLOAK_ADMIN_CLIENT_SECRET must be set");
    let tenant_id = env::var("SUPER_ADMIN_TENANT_ID")
        .expect("SUPER_ADMIN_TENANT_ID must be set");
    KeycloakLoginConfig::new(client_id, client_secret, tenant_id)
}

#[instrument(skip_all, err)]
async fn get_credentials_inner(
    login_config: KeycloakLoginConfig,
) -> Result<String> {
    let body_string = serde_urlencoded::to_string::<[(String, String); 4]>([
        ("client_id".into(), login_config.client_id),
        ("scope".into(), "openid".into()),
        ("client_secret".into(), login_config.client_secret),
        ("grant_type".into(), "client_credentials".into()),
    ])
    .map_err(|_| anyhow!("Unable to encode Keycloak credential request"))?;

    let keycloak_endpoint = format!(
        "{}/realms/{}/protocol/openid-connect/token",
        login_config.url, login_config.realm
    );

    // Retry up to 3 times with increasing intervals between attempts.
    let retry_policy = ExponentialBackoff::builder().build_with_max_retries(3);
    let client = ClientBuilder::new(reqwest::Client::new())
        .with(RetryTransientMiddleware::new_with_policy(retry_policy))
        .build();
    info!("Acquiring Keycloak client credentials");
    let response = client
        .post(keycloak_endpoint)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body_string)
        .send()
        .await?
        .error_for_status()?;
    info!("Keycloak credential request completed");
    response.text().await.map_err(Into::into)
}

// Client Credentials OpenID Authentication flow.
// This enables servers to authenticate, without using a browser.
#[instrument(err)]
pub async fn get_client_credentials() -> Result<connection::AuthHeaders> {
    let login_config = get_keycloak_login_config();
    let text = get_credentials_inner(login_config).await?;
    let credentials: KeycloakAdminToken = decode_token_response(&text)?;

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
    let credentials: KeycloakAdminToken = decode_token_response(&text)?;
    event!(Level::INFO, "Successfully acquired credentials");
    Ok(credentials)
}

/// Authenticate a party client in keycloak with specific client credentials and
/// tenant_id
#[instrument(skip_all, err)]
pub async fn get_third_party_client_access_token(
    client_id: String,
    client_secret: String,
    tenant_id: String,
) -> Result<KeycloakAdminToken> {
    let login_config =
        KeycloakLoginConfig::new(client_id, client_secret, tenant_id);

    let text = get_credentials_inner(login_config).await?;
    let keycloak_adm_tkn: KeycloakAdminToken = decode_token_response(&text)?;

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
                .map_err(credential_request_error)?;
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
        .map_err(credential_request_error)?;
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
        .map_err(credential_request_error)?;
        event!(Level::INFO, "Successfully acquired credentials");
        Ok(PubKeycloakAdmin {
            url: login_config.url,
            client,
            token_supplier: admin_token,
        })
    }
}

#[cfg(all(test, feature = "log"))]
mod credential_contract_tests {
    use super::*;
    use crate::services::keycloak::test_support::*;
    use std::collections::HashMap;

    fn login_config(url: String) -> KeycloakLoginConfig {
        KeycloakLoginConfig {
            url,
            client_id: "synthetic-client".into(),
            client_secret: SECRET.into(),
            realm: "synthetic-realm".into(),
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn credentials_remain_on_wire_and_out_of_trace() {
        let log = LogCapture::default();
        let _guard = tracing::subscriber::set_default(log.subscriber());
        let body = serde_json::to_string(&token()).unwrap();
        let service = server("200 OK", &body).await;
        let result = get_credentials_inner(login_config(service.url))
            .await
            .unwrap();
        assert_eq!(result, body);
        let observed = service.request.await.unwrap();
        assert_eq!(observed.request_line, "POST /realms/synthetic-realm/protocol/openid-connect/token HTTP/1.1\r\n");
        let fields: HashMap<String, String> =
            serde_urlencoded::from_bytes(&observed.body).unwrap();
        assert_eq!(
            fields.get("client_secret").map(String::as_str),
            Some(SECRET)
        );
        assert_eq!(
            fields.get("client_id").map(String::as_str),
            Some("synthetic-client")
        );
        assert_eq!(
            fields.get("grant_type").map(String::as_str),
            Some("client_credentials")
        );
        let output = log.output();
        assert!(!output.is_empty(), "the trace capture must be active");
        for secret in [SECRET, ACCESS, REFRESH] {
            assert!(
                !output.contains(secret),
                "a synthetic credential reached trace output"
            );
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn rejected_credentials_are_an_error_without_response_body() {
        let service = server("401 Unauthorized", SECRET).await;
        let result = get_credentials_inner(login_config(service.url)).await;
        service.request.await.unwrap();
        assert!(
            result.is_err(),
            "a rejected credential request must not report success"
        );
        let diagnostic = format!("{:?}", result.unwrap_err());
        assert!(diagnostic.contains("401"));
        assert!(!diagnostic.contains(SECRET));
    }

    #[test]
    fn token_debug_omits_access_and_refresh_credentials() {
        let diagnostic = format!("{:?}", token());
        assert!(!diagnostic.contains(ACCESS));
        assert!(!diagnostic.contains(REFRESH));
        let private: KeycloakAdminToken = token().try_into().unwrap();
        let restored: PubKeycloakAdminToken = private.try_into().unwrap();
        assert_eq!(restored, token());
    }

    #[test]
    fn invalid_token_fields_and_http_errors_do_not_echo_credentials() {
        let mut invalid = serde_json::to_value(token()).unwrap();
        invalid["expires_in"] = serde_json::json!(SECRET);
        let error =
            decode_token_response::<KeycloakAdminToken>(&invalid.to_string())
                .unwrap_err();
        let diagnostic = format!("{error:?}");
        assert!(diagnostic.contains("Invalid Keycloak token response"));
        for secret in [SECRET, ACCESS, REFRESH] {
            assert!(!diagnostic.contains(secret));
        }
        let error = credential_request_error(KeycloakError::HttpFailure {
            status: 401,
            body: None,
            text: format!("{SECRET} {ACCESS} {REFRESH}"),
        });
        let diagnostic = format!("{error:?}");
        assert!(diagnostic.contains("401"));
        for secret in [SECRET, ACCESS, REFRESH] {
            assert!(!diagnostic.contains(secret));
        }
    }
}
