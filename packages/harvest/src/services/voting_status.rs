// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{Context, Result};
use sequent_core::ballot::VotingStatus;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use windmill::services::election_event_board::get_election_event_board;
use windmill::services::election_event_status;
use windmill::services::electoral_log::*;

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateEventVotingStatusInput {
    pub election_event_id: String,
    pub voting_status: VotingStatus,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateEventVotingStatusOutput {
    pub election_event_id: String,
}

#[instrument(err)]
pub async fn update_event_status(
    input: UpdateEventVotingStatusInput,
    tenant_id: String,
) -> Result<UpdateEventVotingStatusOutput> {
    let election_event = election_event_status::update_event_voting_status(
        tenant_id.clone(),
        input.election_event_id.clone(),
        input.voting_status.clone(),
    )
    .await?;

    let board_name = get_election_event_board(
        election_event.bulletin_board_reference.clone(),
    )
    .with_context(|| "missing bulletin board")?;

    let electoral_log = ElectoralLog::new(board_name.as_str()).await?;

    match input.voting_status {
        VotingStatus::NOT_STARTED => {
            // Nothing to do?
        }
        VotingStatus::OPEN => {
            electoral_log
                .post_election_open(input.election_event_id.clone(), None)
                .await
                .with_context(|| "error posting to the electoral log")?;
        }
        VotingStatus::PAUSED => {
            electoral_log
                .post_election_pause(input.election_event_id.clone(), None)
                .await
                .with_context(|| "error posting to the electoral log")?;
        }
        VotingStatus::CLOSED => {
            electoral_log
                .post_election_close(input.election_event_id.clone(), None)
                .await
                .with_context(|| "error posting to the electoral log")?;
        }
    };

    Ok(UpdateEventVotingStatusOutput {
        election_event_id: input.election_event_id.clone(),
    })
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateElectionVotingStatusInput {
    pub election_event_id: String,
    pub election_id: String,
    pub voting_status: VotingStatus,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateElectionVotingStatusOutput {
    pub election_id: String,
}

#[instrument(err)]
pub async fn update_election_status(
    input: UpdateElectionVotingStatusInput,
    tenant_id: String,
) -> Result<UpdateElectionVotingStatusOutput> {
    election_event_status::update_election_voting_status_impl(
        tenant_id.clone(),
        input.election_event_id.clone(),
        input.election_id.clone(),
        input.voting_status.clone(),
    )
    .await?;

    Ok(UpdateElectionVotingStatusOutput {
        election_id: input.election_id.clone(),
    })
}
