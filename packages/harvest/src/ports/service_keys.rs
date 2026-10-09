// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use jsonwebtoken::jwk::JwkSet;

/// The keys a Keycloak realm signs its tokens with.
#[rocket::async_trait]
pub trait ServiceKeySource: Send + Sync {
    /// The realm's published key set. Keys this crate can't read are left
    /// out rather than failing the whole set.
    async fn realm_keys(&self, realm: &str) -> anyhow::Result<JwkSet>;
}
