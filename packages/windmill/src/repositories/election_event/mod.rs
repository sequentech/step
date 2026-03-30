// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

pub mod postgres;

use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;

/// Repository contract for election metadata needed by ballot insertion.
///
/// Implementations are expected to be created from the caller-owned Hasura
/// transaction so the tally orchestration decides the transaction scope. The
/// returned mapping is used to translate internal election identifiers into the
/// external aliases that the Keycloak export query understands.
///
/// Use cases:
/// - Resolve election aliases before exporting eligible users.
/// - Keep Keycloak lookups detached from Hasura-specific query details.
///
/// Contract:
/// - Returns a map from election id to external alias.
/// - Missing aliases are allowed and should simply be omitted from the map.
/// - Must not acquire its own transaction.
#[async_trait]
pub trait ElectionEventRepository: Send + Sync {
    /// Returns the mapping from election id to external alias for one event.
    ///
    /// Missing aliases may be omitted from the map, but persistence failures
    /// must be propagated to the caller.
    async fn get_election_aliases(
        &self,
        tenant_id: &str,
        election_event_id: &str,
    ) -> Result<HashMap<String, String>>;
}
