// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::ports::identity::IdentityAdmin;
use sequent_core::services::keycloak::KeycloakAdminClient;

/// The admin client with the process-wide cached admin token.
pub struct KeycloakIdentityAdmin;

#[rocket::async_trait]
impl IdentityAdmin for KeycloakIdentityAdmin {
    async fn client(&self) -> anyhow::Result<KeycloakAdminClient> {
        KeycloakAdminClient::new().await
    }
}
