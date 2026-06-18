// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

pub mod postgres;

use anyhow::Result;
use async_trait::async_trait;
use std::path::Path;

/// Repository contract for exporting the eligible user set used during ballot
/// reconciliation.
///
/// Implementations are expected to be created from the caller-owned Keycloak
/// transaction. The constructor requirement keeps transaction propagation at the
/// orchestration boundary and makes the service straightforward to test with fake
/// repositories.
///
/// Use cases:
/// - Export enabled users for one area and election alias.
/// - Include delegated-voting counts when that policy is enabled.
///
/// Contract:
/// - Must write the export to the provided path.
/// - Must not open its own transaction.
/// - The output must be ordered and shaped so the configured `BallotProcessor`
///   can join it with the ballot export.
#[async_trait]
pub trait EligibleUserRepository: Send + Sync {
    /// Exports enabled users for one area and election alias.
    ///
    /// The generated artifact must be compatible with the ballot export so the
    /// configured `BallotProcessor` can derive annotations from both files.
    async fn export_enabled_users(
        &self,
        realm: &str,
        area_id: &str,
        election_alias: &str,
        output_path: &Path,
        delegated_voting_enabled: bool,
    ) -> Result<()>;
}
