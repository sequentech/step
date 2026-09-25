// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use sequent_core::services::keycloak::KeycloakAdminClient;

/// Where route handlers get their Keycloak admin client.
#[rocket::async_trait]
pub trait IdentityAdmin: Send + Sync {
    async fn client(&self) -> anyhow::Result<KeycloakAdminClient>;
}
