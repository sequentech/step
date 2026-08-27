// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{Context, Result};
use graphql_client::GraphQLQuery;
use sequent_core::ballot::{VotingStatus, VotingStatusChannel};

use crate::hasura::HasuraClient;
use crate::types::hasura::*;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/update_event_voting_status.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct UpdateEventVotingStatus;

impl From<VotingStatus> for update_event_voting_status::VotingStatus {
    fn from(value: VotingStatus) -> Self {
        match value {
            VotingStatus::OPEN => Self::OPEN,
            VotingStatus::CLOSED => Self::CLOSED,
            VotingStatus::PAUSED => Self::PAUSED,
            VotingStatus::NOT_STARTED => Self::NOT_STARTED,
        }
    }
}

impl From<VotingStatusChannel> for update_event_voting_status::VotingStatusChannel {
    fn from(value: VotingStatusChannel) -> Self {
        match value {
            VotingStatusChannel::ONLINE => Self::ONLINE,
            VotingStatusChannel::KIOSK => Self::KIOSK,
            VotingStatusChannel::EARLY_VOTING => Self::EARLY_VOTING,
            VotingStatusChannel::TELEPHONE => Self::TELEPHONE,
        }
    }
}

/// Opens `ONLINE` voting — the only channel Phase 2's voter login (client
/// id `voting-portal`) is authorized for
/// (`packages/sequent-core/src/services/authorization.rs:108-113`).
pub async fn open_voting(client: &HasuraClient, election_event_id: &str) -> Result<()> {
    let variables = update_event_voting_status::Variables {
        election_event_id: election_event_id.to_string(),
        voting_status: VotingStatus::OPEN.into(),
        voting_channels: Some(vec![Some(VotingStatusChannel::ONLINE.into())]),
    };
    client
        .data_or_bail::<UpdateEventVotingStatus>(variables)
        .await
        .context("failed to open voting")?;
    Ok(())
}
