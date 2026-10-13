// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The trustee's login to the platform: Keycloak's password grant for the
//! trustee's user, as step-cli logs a trustee in, and the renewal of the token
//! it gets.

use std::time::{Duration, Instant};

use anyhow::{Context as _, Result};
use serde::Deserialize;
use tracing::{info, warn};

use crate::platform::call;

/// How long before its expiry a token is renewed.
const RENEWAL_MARGIN: Duration = Duration::from_secs(30);

/// What the trustee logs in with. Read once from the environment and used
/// here only.
pub(crate) struct Credentials {
    pub(crate) keycloak_url: String,
    /// The tenant realm, `tenant-<tenant id>`.
    pub(crate) realm: String,
    pub(crate) client_id: String,
    /// Absent for a public client.
    pub(crate) client_secret: Option<String>,
    /// The trustee's name.
    pub(crate) username: String,
    pub(crate) password: String,
}

/// Access tokens of the trustee's user, renewed before they expire.
pub(crate) struct TokenSource {
    http: reqwest::Client,
    token_url: String,
    credentials: Credentials,
    token: Option<Token>,
}

struct Token {
    access: String,
    refresh: Option<String>,
    renew_at: Instant,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    /// Seconds.
    expires_in: u64,
    refresh_token: Option<String>,
}

impl TokenSource {
    pub(crate) fn new(
        http: reqwest::Client,
        credentials: Credentials,
    ) -> TokenSource {
        let token_url = format!(
            "{}/realms/{}/protocol/openid-connect/token",
            credentials.keycloak_url.trim_end_matches('/'),
            credentials.realm
        );
        TokenSource {
            http,
            token_url,
            credentials,
            token: None,
        }
    }

    /// A token that is not due for renewal: the last one, a renewal of it, or
    /// the token of a new login when there is nothing to renew or the renewal
    /// failed.
    pub(crate) async fn access_token(&mut self) -> Result<&str> {
        let token = match self.token.take() {
            Some(token) if Instant::now() < token.renew_at => token,
            Some(Token {
                refresh: Some(refresh),
                ..
            }) => match self.renew(&refresh).await {
                Ok(token) => token,
                Err(err) => {
                    warn!(
                        "renewing the token failed, logging in again: {err:#}"
                    );
                    self.log_in().await?
                }
            },
            Some(Token { refresh: None, .. }) | None => self.log_in().await?,
        };
        Ok(self.token.insert(token).access.as_str())
    }

    async fn log_in(&self) -> Result<Token> {
        let credentials = &self.credentials;
        let mut form = vec![
            ("grant_type", "password"),
            ("scope", "openid"),
            ("client_id", credentials.client_id.as_str()),
            ("username", credentials.username.as_str()),
            ("password", credentials.password.as_str()),
        ];
        if let Some(secret) = &credentials.client_secret {
            form.push(("client_secret", secret.as_str()));
        }
        let token = self.request(&form).await.with_context(|| {
            format!(
                "logging in to realm {} as {}",
                credentials.realm, credentials.username
            )
        })?;
        info!(
            realm = %credentials.realm,
            user = %credentials.username,
            "logged in to the platform"
        );
        Ok(token)
    }

    async fn renew(&self, refresh_token: &str) -> Result<Token> {
        let mut form = vec![
            ("grant_type", "refresh_token"),
            ("client_id", self.credentials.client_id.as_str()),
            ("refresh_token", refresh_token),
        ];
        if let Some(secret) = &self.credentials.client_secret {
            form.push(("client_secret", secret.as_str()));
        }
        self.request(&form).await.context("renewing the token")
    }

    async fn request(&self, form: &[(&str, &str)]) -> Result<Token> {
        let requested_at = Instant::now();
        let answer: TokenResponse =
            call(self.http.post(&self.token_url).form(form))
                .await
                .with_context(|| format!("POST {}", self.token_url))?;
        let lifetime = Duration::from_secs(answer.expires_in);
        Ok(Token {
            access: answer.access_token,
            refresh: answer.refresh_token,
            renew_at: requested_at + lifetime.saturating_sub(RENEWAL_MARGIN),
        })
    }
}
