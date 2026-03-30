// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

pub mod postgres;

use anyhow::Result;
use async_trait::async_trait;
use std::path::Path;

/// Repository contract for exporting cast ballots into an intermediate artifact.
///
/// Implementations are expected to be built with the outer Hasura transaction.
/// The transaction-bound constructor guarantees that orchestration controls
/// transaction propagation and that no hidden connection acquisition happens
/// during per-contest processing.
///
/// Use cases:
/// - Export the latest ballot per voter for a contest area and election.
/// - Produce a stable CSV artifact that can be consumed by a pure join stage.
///
/// Contract:
/// - Must write the export to the provided path.
/// - Must not create or commit transactions internally.
/// - The generated artifact must be consumable by the configured
///   `BallotProcessor` implementation.
#[async_trait]
pub trait BallotRepository: Send + Sync {
    /// Exports area ballots for one election into the provided output path.
    ///
    /// Implementations must write a file consumable by `BallotProcessor` and
    /// must not create or manage their own transactions.
    async fn export_area_ballots(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        area_id: &str,
        election_id: &str,
        output_path: &Path,
    ) -> Result<()>;
}
