// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::ports::identity::IdentityAdmin;
use sequent_core::services::keycloak::{
    get_realm_password_policy, KeycloakAdminClient, ParsedRealmPasswordPolicy,
};
use windmill::tasks::manage_election_event_enrollment::{
    update_keycloak_enrollment, update_keycloak_otp,
};
use windmill::types::error::Result as WindmillResult;

/// The Keycloak the environment configures, with the process-wide cached
/// admin token.
pub struct KeycloakIdentityAdmin;

#[rocket::async_trait]
impl IdentityAdmin for KeycloakIdentityAdmin {
    async fn client(&self) -> anyhow::Result<KeycloakAdminClient> {
        KeycloakAdminClient::new().await
    }

    async fn realm_password_policy(
        &self,
        tenant_id: &str,
        election_event_id: &str,
    ) -> anyhow::Result<ParsedRealmPasswordPolicy> {
        get_realm_password_policy(tenant_id, election_event_id).await
    }



    async fn update_voter_enrollment(
        &self,
        tenant_id: Option<String>,
        election_event_id: Option<String>,
        enable_enrollment: bool,
    ) -> WindmillResult<()> {
        update_keycloak_enrollment(
            tenant_id,
            election_event_id,
            enable_enrollment,
        )
        .await
    }

    async fn update_voter_otp(
        &self,
        tenant_id: Option<String>,
        election_event_id: Option<String>,
        new_otp_state: String,
    ) -> WindmillResult<()> {
        update_keycloak_otp(tenant_id, election_event_id, new_otp_state).await
    }


}
