// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::repositories::voters::EligibleUserRepository;
use crate::services::users::list_keycloak_enabled_users_by_area_id_and_authorized_elections;
use anyhow::Result;
use async_trait::async_trait;
use deadpool_postgres::Transaction;
use std::path::Path;

/// Keycloak-backed implementation of `EligibleUserRepository`.
///
/// The adapter exports enabled users for one area and election alias using the
/// caller-provided Keycloak transaction.
pub struct KeycloakEligibleUserRepository<'a> {
    transaction: &'a Transaction<'a>,
}

impl<'a> KeycloakEligibleUserRepository<'a> {
    /// Creates an eligible-user repository bound to the provided transaction.
    pub fn new(transaction: &'a Transaction<'a>) -> Self {
        Self { transaction }
    }
}

#[async_trait]
impl EligibleUserRepository for KeycloakEligibleUserRepository<'_> {
    async fn export_enabled_users(
        &self,
        realm: &str,
        area_id: &str,
        election_alias: &str,
        output_path: &Path,
        delegated_voting_enabled: bool,
    ) -> Result<()> {
        list_keycloak_enabled_users_by_area_id_and_authorized_elections(
            self.transaction,
            realm,
            area_id,
            election_alias,
            &output_path.to_path_buf(),
            delegated_voting_enabled,
        )
        .await
    }
}
