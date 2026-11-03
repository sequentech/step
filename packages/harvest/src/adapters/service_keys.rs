// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::ports::service_keys::ServiceKeySource;
use crate::services::service_auth::parse_key_set;
use anyhow::Context;
use jsonwebtoken::jwk::JwkSet;
use std::time::Duration;

/// A key set that doesn't arrive in this time is unavailable, and the
/// request is refused.
const FETCH_TIMEOUT: Duration = Duration::from_secs(5);

/// The realm keys Keycloak publishes at
/// `KEYCLOAK_URL/realms/{realm}/protocol/openid-connect/certs`.
pub struct KeycloakServiceKeys {
    http: reqwest::Client,
}

impl KeycloakServiceKeys {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(FETCH_TIMEOUT)
                .build()
                .unwrap_or_default(),
        }
    }
}

#[rocket::async_trait]
impl ServiceKeySource for KeycloakServiceKeys {
    async fn realm_keys(&self, realm: &str) -> anyhow::Result<JwkSet> {
        let base = std::env::var("KEYCLOAK_URL")
            .context("KEYCLOAK_URL must be set")?;
        let url = format!(
            "{}/realms/{realm}/protocol/openid-connect/certs",
            base.trim_end_matches('/')
        );
        let body: serde_json::Value = self
            .http
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(parse_key_set(&body))
    }
}
