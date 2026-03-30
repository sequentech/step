// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

pub mod postgres;

use anyhow::Result;
use async_trait::async_trait;
use sequent_core::types::hasura::core::Trustee;

/// Repository contract for loading trustees involved in a tally ceremony.
///
/// Implementations are expected to be constructed with the caller-owned Hasura
/// transaction for the orchestration that is invoking the service. That
/// constructor requirement is deliberate: transaction propagation must be
/// controlled by the orchestration layer rather than hidden inside repository
/// methods.
///
/// Use cases:
/// - Resolve trustee names into persisted trustee records.
/// - Retrieve public keys needed to derive the selected trustee set for the
///   bulletin board.
///
/// Contract:
/// - Returns all trustees matching the provided names within the tenant scope.
/// - Must not open a new connection or transaction internally.
/// - Errors should be propagated when the underlying persistence layer cannot be
///   queried or when tenant scoping cannot be enforced.
#[async_trait]
pub trait TrusteeRepository: Send + Sync {
    /// Loads trustees by name within the given tenant scope.
    ///
    /// Implementations must use the transaction supplied at construction time
    /// and propagate persistence errors directly to the caller.
    async fn get_trustees_by_name(
        &self,
        tenant_id: &str,
        trustee_names: &[String],
    ) -> Result<Vec<Trustee>>;
}
