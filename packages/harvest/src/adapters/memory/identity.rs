// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::ports::identity::IdentityAdmin;
use anyhow::anyhow;
use keycloak::{KeycloakAdmin, KeycloakAdminToken};
use sequent_core::services::keycloak::{
    KeycloakAdminClient, ParsedRealmPasswordPolicy,
};
use serde_json::json;
use std::sync::Mutex;
use std::time::Duration;
use windmill::types::error::Result as WindmillResult;

/// A change to how the event's voters authenticate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AuthenticationUpdate {
    Enrollment(bool),
    Otp(String),
}

/// Keycloak for route tests: admin clients for a local stand-in at `url`
/// holding a synthetic token, and fixed answers from the helpers that open
/// clients of their own. Unset answers fail as when Keycloak refuses the
/// admin credentials.
#[derive(Default)]
pub struct LocalIdentityAdmin {
    pub url: Option<String>,
    pub password_policy: Option<ParsedRealmPasswordPolicy>,
    pub refuses_updates: bool,
    pub updates: Mutex<Vec<AuthenticationUpdate>>,
}

impl LocalIdentityAdmin {
    pub fn updates(&self) -> Vec<AuthenticationUpdate> {
        self.updates.lock().unwrap().clone()
    }

    fn update(&self, update: AuthenticationUpdate) -> WindmillResult<()> {
        if self.refuses_updates {
            return Err(refused().into());
        }
        self.updates.lock().unwrap().push(update);
        Ok(())
    }
}

fn refused() -> anyhow::Error {
    anyhow!("Keycloak refused the admin credentials")
}

#[rocket::async_trait]
impl IdentityAdmin for LocalIdentityAdmin {
    async fn client(&self) -> anyhow::Result<KeycloakAdminClient> {
        let url = self.url.as_deref().ok_or_else(refused)?;
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

    async fn realm_password_policy(
        &self,
        _tenant_id: &str,
        _election_event_id: &str,
    ) -> anyhow::Result<ParsedRealmPasswordPolicy> {
        self.password_policy.clone().ok_or_else(refused)
    }



    async fn update_voter_enrollment(
        &self,
        _tenant_id: Option<String>,
        _election_event_id: Option<String>,
        enable_enrollment: bool,
    ) -> WindmillResult<()> {
        self.update(AuthenticationUpdate::Enrollment(enable_enrollment))
    }

    async fn update_voter_otp(
        &self,
        _tenant_id: Option<String>,
        _election_event_id: Option<String>,
        new_otp_state: String,
    ) -> WindmillResult<()> {
        self.update(AuthenticationUpdate::Otp(new_otp_state))
    }


}
