// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Keycloak Resource Owner Password Credentials (`grant_type=password`)
//! login, shared by the admin login (tenant realm, confidential client) and
//! voter login (event realm, public `voting-portal` client, no
//! `client_secret`).

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub expires_in: Option<i64>,
    #[serde(default)]
    pub token_type: Option<String>,
}

#[derive(Debug)]
pub enum LoginError {
    Transport(reqwest::Error),
    /// Keycloak rejected the credentials themselves (`invalid_grant`).
    InvalidCredentials,
    /// Any other client-side rejection (bad client id, disabled user, ...).
    Rejected {
        status: u16,
        body: String,
    },
    /// Keycloak itself is unreachable or erroring.
    ServiceUnavailable {
        status: u16,
        body: String,
    },
}

impl std::fmt::Display for LoginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoginError::Transport(err) => write!(f, "transport error: {err}"),
            LoginError::InvalidCredentials => {
                write!(f, "invalid username or password")
            }
            LoginError::Rejected { status, body } => {
                write!(f, "login rejected (HTTP {status}): {body}")
            }
            LoginError::ServiceUnavailable { status, body } => {
                write!(f, "keycloak unavailable (HTTP {status}): {body}")
            }
        }
    }
}

impl std::error::Error for LoginError {}

/// `client_secret` is `None` for public clients (e.g. `voting-portal`) —
/// sending an empty string instead is worse than omitting the field, since
/// Keycloak's client-authenticator chain can read its mere presence as an
/// attempted (and then missing) confidential-client auth.
pub async fn login(
    http: &reqwest::Client,
    keycloak_url: &str,
    realm: &str,
    client_id: &str,
    client_secret: Option<&str>,
    username: &str,
    password: &str,
) -> Result<TokenResponse, LoginError> {
    let url = format!(
        "{}/realms/{}/protocol/openid-connect/token",
        keycloak_url.trim_end_matches('/'),
        realm
    );

    let mut form: Vec<(&str, &str)> = vec![
        ("grant_type", "password"),
        ("scope", "openid"),
        ("client_id", client_id),
        ("username", username),
        ("password", password),
    ];
    if let Some(secret) = client_secret {
        form.push(("client_secret", secret));
    }

    let response = http
        .post(&url)
        .form(&form)
        .send()
        .await
        .map_err(LoginError::Transport)?;

    let status = response.status();
    if status.is_success() {
        response
            .json::<TokenResponse>()
            .await
            .map_err(LoginError::Transport)
    } else {
        let body = response.text().await.unwrap_or_default();
        if status.is_client_error() {
            if body.contains("invalid_grant") {
                Err(LoginError::InvalidCredentials)
            } else {
                Err(LoginError::Rejected {
                    status: status.as_u16(),
                    body,
                })
            }
        } else {
            Err(LoginError::ServiceUnavailable {
                status: status.as_u16(),
                body,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_response_ignores_unknown_fields_and_defaults_missing_ones() {
        let parsed: TokenResponse = serde_json::from_str(
            r#"{"access_token":"abc","token_type":"bearer","not_modeled":true}"#,
        )
        .unwrap();
        assert_eq!(parsed.access_token, "abc");
        assert_eq!(parsed.token_type.as_deref(), Some("bearer"));
        assert!(parsed.refresh_token.is_none());
        assert!(parsed.expires_in.is_none());
    }
}
