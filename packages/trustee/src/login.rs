// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The trustee's login to the platform: Keycloak's password grant for the
//! trustee's user, as step-cli logs a trustee in, and the renewal of the token
//! it gets. The password, the client secret and the tokens are overwritten
//! with zero when they are dropped.

use std::time::{Duration, Instant};

use anyhow::{Context as _, Result};
use serde::Deserialize;
use tracing::{info, warn};
use zeroize::Zeroize;

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

impl Zeroize for Credentials {
    /// The secrets only: the password and the client secret.
    fn zeroize(&mut self) {
        self.password.zeroize();
        self.client_secret.zeroize();
    }
}

impl Drop for Credentials {
    fn drop(&mut self) {
        self.zeroize();
    }
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

impl Zeroize for Token {
    fn zeroize(&mut self) {
        self.access.zeroize();
        self.refresh.zeroize();
    }
}

impl Drop for Token {
    fn drop(&mut self) {
        self.zeroize();
    }
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
            // The token renewed or replaced is overwritten once this statement
            // ends.
            Some(Token {
                refresh: Some(ref refresh),
                ..
            }) => match self.renew(refresh).await {
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

#[cfg(test)]
mod tests {
    use super::*;

    // What `Drop` runs. The bytes are overwritten with zero before the length
    // is cleared, which is all safe code can observe.

    #[test]
    fn the_login_secrets_are_overwritten_when_they_are_dropped() {
        let mut credentials = Credentials {
            keycloak_url: "http://keycloak.invalid".into(),
            realm: "tenant-fixture".into(),
            client_id: "fixture-client".into(),
            client_secret: Some("fixture-secret".into()),
            username: "trustee1".into(),
            password: "fixture-password".into(),
        };
        credentials.zeroize();
        assert!(credentials.password.is_empty());
        assert_eq!(credentials.client_secret, None);
        assert_eq!(credentials.username, "trustee1");
    }

    #[test]
    fn a_token_is_overwritten_when_it_is_dropped() {
        let mut token = Token {
            access: "fixture-access-token".into(),
            refresh: Some("fixture-refresh-token".into()),
            renew_at: Instant::now(),
        };
        token.zeroize();
        assert!(token.access.is_empty());
        assert_eq!(token.refresh, None);
    }
}
