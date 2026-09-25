// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use sequent_core::services::keycloak::{
    KeycloakAdminClient, ParsedRealmPasswordPolicy,
};
use windmill::types::error::Result as WindmillResult;

/// Keycloak, through an admin client or the Sequent Core and Windmill
/// helpers that open their own.
#[rocket::async_trait]
pub trait IdentityAdmin: Send + Sync {
    async fn client(&self) -> anyhow::Result<KeycloakAdminClient>;
    /// The password policy of the election event's realm.
    async fn realm_password_policy(
        &self,
        tenant_id: &str,
        election_event_id: &str,
    ) -> anyhow::Result<ParsedRealmPasswordPolicy>;

    async fn update_voter_enrollment(
        &self,
        tenant_id: Option<String>,
        election_event_id: Option<String>,
        enable_enrollment: bool,
    ) -> WindmillResult<()>;
    async fn update_voter_otp(
        &self,
        tenant_id: Option<String>,
        election_event_id: Option<String>,
        new_otp_state: String,
    ) -> WindmillResult<()>;
}
