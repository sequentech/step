// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::ports::identity::IdentityAdmin;
use anyhow::anyhow;
use keycloak::{KeycloakAdmin, KeycloakAdminToken};
use sequent_core::services::keycloak::KeycloakAdminClient;
use serde_json::json;
use std::time::Duration;

/// Admin clients for a local Keycloak stand-in at `url`, holding a synthetic
/// token. Without a URL, obtaining a client fails as when Keycloak refuses
/// the admin credentials.
pub struct LocalIdentityAdmin {
    pub url: Option<String>,
}

#[rocket::async_trait]
impl IdentityAdmin for LocalIdentityAdmin {
    async fn client(&self) -> anyhow::Result<KeycloakAdminClient> {
        let url = self
            .url
            .as_deref()
            .ok_or_else(|| anyhow!("Keycloak refused the admin credentials"))?;
        let token: KeycloakAdminToken = serde_json::from_value(json!({
            "access_token": "synthetic-access-token", "expires_in": 300,
            "scope": "openid", "token_type": "Bearer"
        }))?;
        let http = reqwest::Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(5))
            .build()?;
        Ok(KeycloakAdminClient {
            client: KeycloakAdmin::new(url, token, http),
        })
    }
}
