use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::election_event::update_election_event_status;
// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::election_event_board::get_election_event_board;
use crate::services::election_event_status;
use crate::services::electoral_log::*;
use anyhow::{Context, Result};
use deadpool_postgres::Transaction;
use electoral_log::messages::newtypes::VotingChannelString;
use sequent_core::ballot::ElectionEventStatus;
use sequent_core::ballot::ElectionStatus;
use sequent_core::ballot::VotingStatus;
use sequent_core::ballot::VotingStatusChannel;
use sequent_core::serialization::deserialize_with_path::deserialize_value;
use sequent_core::types::hasura::core::Election;
use sequent_core::types::hasura::core::VotingChannels;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::info;
use tracing::instrument;

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateElectionVotingStatusInput {
    pub election_event_id: String,
    pub election_id: String,
    pub voting_status: VotingStatus,
    pub voting_channels: Option<Vec<VotingStatusChannel>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateElectionVotingStatusOutput {
    pub election_id: String,
}

#[instrument(err)]
pub async fn update_election_status(
    tenant_id: String,
    user_id: Option<&str>,
    username: Option<&str>,
    hasura_transaction: &Transaction<'_>,
    election_event_id: &str,
    election_id: &str,
    voting_status: &VotingStatus,
    voting_channels: &Option<Vec<VotingStatusChannel>>,
) -> Result<()> {
    let election_event =
        get_election_event_by_id(hasura_transaction, &tenant_id, election_event_id)
            .await
            .with_context(|| "error getting election event")?;

    let mut status = election_event.status.clone().unwrap_or_default();

    let voting_channels: Vec<VotingStatusChannel> = if let Some(channel) = voting_channels {
        info!("Reading input voting channels {channel:?}");
        channel.clone()
    } else if let Some(channels) = election_event.voting_channels.clone() {
        info!("Election voting channels {channels:?}");
        let voting_channels: VotingChannels =
            deserialize_value(channels).context("Failed to deserialize event voting_channels")?;

        let mut election_channels = vec![];

        if VotingStatusChannel::ONLINE
            .channel_from(&voting_channels)
            .unwrap_or(false)
        {
            election_channels.push(VotingStatusChannel::ONLINE)
        }

        if VotingStatusChannel::KIOSK
            .channel_from(&voting_channels)
            .unwrap_or(false)
        {
            election_channels.push(VotingStatusChannel::KIOSK)
        }

        election_channels
    } else {
        info!("Default voting channels");
        // Update all if none are configured
        vec![VotingStatusChannel::ONLINE, VotingStatusChannel::KIOSK]
    };

    for voting_channel in &voting_channels {
        election_event_status::update_election_voting_status_impl(
            tenant_id.clone(),
            user_id,
            username,
            election_event_id.to_string(),
            election_id.to_string(),
            voting_status.clone(),
            voting_channel.clone(),
            election_event.bulletin_board_reference.clone(),
            &hasura_transaction,
        )
        .await?;
        let current_event_status = status.status_by_channel(voting_channel);

        info!("current_voting_status={current_event_status:?} next_voting_status={voting_status:?}, voting_channel={voting_channel:?}");

        if voting_status.clone() == VotingStatus::OPEN
            && current_event_status == VotingStatus::NOT_STARTED
        {
            info!("Updating election event status to OPEN");
            status.set_status_by_channel(voting_channel, VotingStatus::OPEN);

            update_board_on_status_change(
                &hasura_transaction,
                &tenant_id,
                user_id,
                username,
                election_event.id.to_string(),
                election_event.bulletin_board_reference.clone(),
                voting_status.clone(),
                voting_channel.clone(),
                None,
                Some(vec![election_id.to_string()]),
            )
            .await?;
        }
    }
    update_election_event_status(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        serde_json::to_value(&status).with_context(|| "Error parsing status")?,
    )
    .await
    .with_context(|| "Error updating election event status")?;

    Ok(())
}

#[instrument(err)]
pub async fn update_board_on_status_change(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    user_id: Option<&str>,
    username: Option<&str>,
    election_event_id: String,
    board_reference: Option<Value>,
    voting_status: VotingStatus,
    voting_channel: VotingStatusChannel,
    election_id: Option<String>,
    elections_ids: Option<Vec<String>>,
) -> Result<()> {
    let board_name =
        get_election_event_board(board_reference).with_context(|| "missing bulletin board")?;
    let elections_ids_str = match election_id.clone() {
        Some(election_id) => Some(election_id),
        None => match elections_ids.clone() {
            Some(elections_ids) => Some(elections_ids.join(",")),
            None => None,
        },
    };

    let electoral_log = if let Some(user_id) = user_id {
        ElectoralLog::for_admin_user(
            hasura_transaction,
            &board_name,
            tenant_id,
            &election_event_id,
            user_id,
            username.map(|val| val.to_string()),
            elections_ids_str,
            None,
        )
        .await?
    } else {
        ElectoralLog::new(
            hasura_transaction,
            tenant_id,
            Some(&election_event_id),
            board_name.as_str(),
        )
        .await?
    };

    let maybe_election_id = match election_id {
        Some(election_id) => Some(election_id),
        None => None,
    };
    match voting_status {
        VotingStatus::NOT_STARTED => {
            // Nothing to do?
        }
        VotingStatus::OPEN => {
            electoral_log
                .post_election_open(
                    election_event_id,
                    maybe_election_id,
                    elections_ids,
                    VotingChannelString(voting_channel.to_string()),
                    user_id.map(|id| id.to_string()),
                    username.map(|username| username.to_string()),
                )
                .await
                .with_context(|| "error posting to the electoral log")?;
        }
        VotingStatus::PAUSED => {
            electoral_log
                .post_election_pause(
                    election_event_id,
                    maybe_election_id,
                    VotingChannelString(voting_channel.to_string()),
                    user_id.map(|id| id.to_string()),
                    username.map(|username| username.to_string()),
                )
                .await
                .with_context(|| "error posting to the electoral log")?;
        }
        VotingStatus::CLOSED => {
            electoral_log
                .post_election_close(
                    election_event_id,
                    maybe_election_id,
                    elections_ids,
                    VotingChannelString(voting_channel.to_string()),
                    user_id.map(|id| id.to_string()),
                    username.map(|username| username.to_string()),
                )
                .await
                .with_context(|| "error posting to the electoral log")?;
        }
    };
    Ok(())
}

#[derive(Debug)]
pub struct ElectionStatusInfo {
    pub total_not_started_votes: i64,
    pub total_open_votes: i64,
    pub total_closed_votes: i64,
    pub total_started_votes: i64,
}

#[instrument(skip(election), ret)]
pub fn get_election_status_info(election: &Election) -> ElectionStatusInfo {
    let mut total_not_started_votes: i64 = 0;
    let mut total_open_votes: i64 = 0;
    let mut total_closed_votes: i64 = 0;
    let mut total_started_votes: i64 = 0;

    let status: Option<ElectionStatus> = election.status.clone();
    let election_voting_channels = election.voting_channels.clone();
    let voting_channels: Option<VotingChannels> = election_voting_channels
        .and_then(|voting_channels_json| deserialize_value(voting_channels_json).ok());

    match status.clone() {
        Some(status) => {
            match status.voting_status {
                // If voting hasn't started yet, increment the count for not
                // opened votes
                VotingStatus::NOT_STARTED => total_not_started_votes += 1,
                // If voting is open, increment the count for
                //open votes and started votes
                VotingStatus::OPEN => {
                    total_open_votes += 1;
                    total_started_votes += 1;
                }
                // If voting is paused, increment the count for started votes
                // Consider the vote as having been open before paused
                VotingStatus::PAUSED => total_started_votes += 1,
                // If voting is closed:
                VotingStatus::CLOSED => {
                    // Consider the vote as having been open before closing
                    total_started_votes += 1;
                    // Check the voting channels to determine if any additional
                    // conditions apply
                    match voting_channels {
                        // If there is a kiosk channel active then check if the
                        // kiosk-specific voting status is closed.
                        Some(VotingChannels {
                            kiosk: Some(true), ..
                        }) => {
                            if status.kiosk_voting_status == VotingStatus::CLOSED {
                                total_closed_votes += 1;
                            }
                        }
                        // For all other cases, if online voting channel is
                        // closed, then the election is considered closed
                        _ => {
                            total_closed_votes += 1;
                        }
                    }
                }
            }
        }
        // If deserialization of the election status failed, do nothing.
        None => {}
    };

    ElectionStatusInfo {
        total_not_started_votes,
        total_open_votes,
        total_closed_votes,
        total_started_votes,
    }
}
