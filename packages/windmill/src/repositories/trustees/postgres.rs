// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::trustee::get_trustees_by_name;
use crate::repositories::trustees::TrusteeRepository;
use anyhow::Result;
use async_trait::async_trait;
use deadpool_postgres::Transaction;
use sequent_core::types::hasura::core::Trustee;

/// Hasura-backed implementation of `TrusteeRepository`.
///
/// This adapter delegates trustee lookup to the existing Postgres query helper
/// while preserving the transaction supplied by the caller.
pub struct HasuraTrusteeRepository<'a> {
    transaction: &'a Transaction<'a>,
}

impl<'a> HasuraTrusteeRepository<'a> {
    /// Creates a trustee repository bound to the provided Hasura transaction.
    pub fn new(transaction: &'a Transaction<'a>) -> Self {
        Self { transaction }
    }
}

#[async_trait]
impl TrusteeRepository for HasuraTrusteeRepository<'_> {
    async fn get_trustees_by_name(
        &self,
        tenant_id: &str,
        trustee_names: &[String],
    ) -> Result<Vec<Trustee>> {
        get_trustees_by_name(self.transaction, tenant_id, &trustee_names.to_vec()).await
    }
}
