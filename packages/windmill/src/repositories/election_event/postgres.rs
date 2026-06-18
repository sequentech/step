// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::repositories::election_event::ElectionEventRepository;
use crate::services::election::get_election_event_elections;
use anyhow::Result;
use async_trait::async_trait;
use deadpool_postgres::Transaction;
use std::collections::HashMap;

/// Hasura-backed implementation of `ElectionEventRepository`.
///
/// The adapter loads election aliases for a single event and reshapes the query
/// result into the map expected by the application layer.
pub struct HasuraElectionEventRepository<'a> {
    transaction: &'a Transaction<'a>,
}

impl<'a> HasuraElectionEventRepository<'a> {
    /// Creates an election-event repository bound to the provided transaction.
    pub fn new(transaction: &'a Transaction<'a>) -> Self {
        Self { transaction }
    }
}

#[async_trait]
impl ElectionEventRepository for HasuraElectionEventRepository<'_> {
    async fn get_election_aliases(
        &self,
        tenant_id: &str,
        election_event_id: &str,
    ) -> Result<HashMap<String, String>> {
        Ok(
            get_election_event_elections(self.transaction, tenant_id, election_event_id)
                .await?
                .into_iter()
                .filter_map(|election| election.external_id.map(|alias| (election.id, alias)))
                .collect(),
        )
    }
}
