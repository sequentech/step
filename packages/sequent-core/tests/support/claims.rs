// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Synthetic JWT claims. [`Claims::bearer`] signs them with the public test key
//! of [`signing`], so the request guards accept them only after verifying the
//! signature, expiry and issuer, as they do for real tokens.
//!
//! Claims are built as JSON and parsed through the public claims schema, so a
//! renamed or newly required claim breaks every fixture that relies on it.

use sequent_core::services::jwt::JwtClaims;
use serde_json::{json, Value};

#[path = "signing.rs"]
pub mod signing;

const HASURA_CLAIMS: &str = "https://hasura.io/jwt/claims";

pub struct Claims(Value);

impl Claims {
    /// An admin portal identity in `tenant_id` that holds no roles.
    pub fn new(tenant_id: &str, user_id: &str) -> Self {
        Self(json!({
            "exp": signing::EXPIRY, "iat": 1_900_000_000,
            "jti": "synthetic", "iss": signing::issuer_of(tenant_id), "sub": user_id,
            "typ": "Bearer", "azp": "admin-portal", "acr": "1", "allowed-origins": [],
            "scope": "openid", "email_verified": false,
            HASURA_CLAIMS: {
                "x-hasura-default-role": "user", "x-hasura-tenant-id": tenant_id,
                "x-hasura-user-id": user_id, "x-hasura-allowed-roles": []
            }
        }))
    }

    pub fn roles<R: ToString>(
        mut self,
        roles: impl IntoIterator<Item = R>,
    ) -> Self {
        let roles: Vec<String> =
            roles.into_iter().map(|role| role.to_string()).collect();
        self.0[HASURA_CLAIMS]["x-hasura-allowed-roles"] = json!(roles);
        self
    }

    pub fn username(mut self, username: &str) -> Self {
        self.0["preferred_username"] = json!(username);
        self
    }

    /// The Keycloak client the token was issued to.
    pub fn azp(mut self, client_id: &str) -> Self {
        self.0["azp"] = json!(client_id);
        self
    }

    /// The authentication context class, such as the gold level.
    pub fn acr(mut self, acr: &str) -> Self {
        self.0["acr"] = json!(acr);
        self
    }

    pub fn auth_time(mut self, seconds: i64) -> Self {
        self.0["auth_time"] = json!(seconds);
        self
    }

    pub fn area(mut self, area_id: &str) -> Self {
        self.0[HASURA_CLAIMS]["x-hasura-area-id"] = json!(area_id);
        self
    }

    pub fn election_event(mut self, election_event_id: &str) -> Self {
        self.0[HASURA_CLAIMS]["x-hasura-election-event-id"] =
            json!(election_event_id);
        self
    }

    pub fn authorized_elections(mut self, election_ids: &[&str]) -> Self {
        self.0[HASURA_CLAIMS]["authorized-election-ids"] = json!(election_ids);
        self
    }

    /// The permission labels, in the Postgres array text Keycloak sends.
    pub fn permission_labels(mut self, labels: &[&str]) -> Self {
        self.0[HASURA_CLAIMS]["x-hasura-permission-labels"] =
            json!(format!("{{{}}}", labels.join(",")));
        self
    }

    pub fn build(&self) -> JwtClaims {
        serde_json::from_value(self.0.clone())
            .expect("synthetic claims should match the public claims schema")
    }

    /// The Authorization header value carrying these claims.
    pub fn bearer(&self) -> String {
        format!("Bearer {}", signing::token(&self.0))
    }
}
