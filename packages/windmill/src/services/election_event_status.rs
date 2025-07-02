use std::collections::HashMap;

// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::election::{get_election_by_id, get_elections, update_election_voting_status};
use crate::postgres::election_event::{get_election_event_by_id, update_election_event_status};
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::ballot::*;
use sequent_core::serialization::deserialize_with_path::deserialize_value;
use sequent_core::types::hasura::core::{ElectionEvent, VotingChannels};
use serde_json::value::Value;
use tracing::{event, info, instrument, Level};

use super::voting_status::update_board_on_status_change;

pub fn get_election_event_status(status_json_opt: Option<Value>) -> Option<ElectionEventStatus> {
    status_json_opt.and_then(|status_json| deserialize_value(status_json).ok())
}

pub fn get_election_status(status_json_opt: Option<Value>) -> Option<ElectionStatus> {
    status_json_opt.and_then(|status_json| deserialize_value(status_json).ok())
}

#[instrument(err)]
pub async fn update_event_voting_status(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    user_id: Option<&str>,
    username: Option<&str>,
    election_event_id: &str,
    new_status: &VotingStatus,
    channels: &Option<Vec<VotingStatusChannel>>,
) -> Result<ElectionEvent> {
    let election_event = get_election_event_by_id(hasura_transaction, tenant_id, election_event_id)
        .await
        .with_context(|| "Error obtaining election event")?;

    let mut status =
        get_election_event_status(election_event.status.clone()).unwrap_or(Default::default());
    let elections = get_elections(hasura_transaction, tenant_id, election_event_id, None)
        .await
        .with_context(|| "Error obtaining elections")?;

    let mut elections_status = HashMap::new();

    for election in &elections {
        let election_status =
            get_election_status(election.status.clone()).unwrap_or(Default::default());

        elections_status.insert(election.id.clone(), election_status);
    }

    let channels: Vec<VotingStatusChannel> = if let Some(channel) = channels {
        info!("Reading input voting channels {channel:?}");
        channel.clone()
    } else if let Some(channels) = election_event.voting_channels.clone() {
        info!("Reading Event voting channels {channels:?}");
        let voting_channels: VotingChannels =
            deserialize_value(channels).context("Failed to deserialize event voting_channels")?;

        let mut event_channels = vec![];

        if VotingStatusChannel::ONLINE
            .channel_from(&voting_channels)
            .unwrap_or(false)
        {
            event_channels.push(VotingStatusChannel::ONLINE)
        }

        if VotingStatusChannel::KIOSK
            .channel_from(&voting_channels)
            .unwrap_or(false)
        {
            event_channels.push(VotingStatusChannel::KIOSK)
        }

        event_channels
    } else {
        info!("Default voting channels");
        // Update all if none are configured
        vec![VotingStatusChannel::ONLINE, VotingStatusChannel::KIOSK]
    };

    if election_event.is_archived {
        info!("Election event is archived, skipping");
        return Ok(election_event);
    }

    for channel in channels {
        let current_voting_status = status.status_by_channel(&channel).clone();

        if current_voting_status == new_status.clone() {
            info!("Current voting status is the same as the new voting status, skipping");
            continue;
        }

        let expected_next_status = match current_voting_status {
            VotingStatus::NOT_STARTED => {
                vec![VotingStatus::OPEN]
            }
            VotingStatus::OPEN => {
                vec![VotingStatus::PAUSED, VotingStatus::CLOSED]
            }
            VotingStatus::PAUSED => {
                vec![VotingStatus::CLOSED, VotingStatus::OPEN]
            }
            VotingStatus::CLOSED => {
                vec![VotingStatus::OPEN]
            }
        };

        if !expected_next_status.contains(&new_status) {
            return Err(anyhow!(
            "Unexpected next status {new_status:?}, expected {expected_next_status:?}, current {current_voting_status:?}",
        ));
        }

        status.set_status_by_channel(&channel, new_status.clone());

        let mut elections_ids: Vec<String> = Vec::new();
        if *new_status == VotingStatus::OPEN || *new_status == VotingStatus::CLOSED {
            for election in &elections {
                if let Some(status) = elections_status.get_mut(&election.id) {
                    status.set_status_by_channel(&channel, new_status.clone());
                }

                elections_ids.push(election.id.clone());
            }
        }

        update_board_on_status_change(
            hasura_transaction,
            &tenant_id,
            user_id,
            username,
            election_event.id.to_string(),
            election_event.bulletin_board_reference.clone(),
            new_status.clone(),
            channel.clone(),
            None,
            Some(elections_ids),
        )
        .await
        .with_context(|| "Error updating electoral board on status change")?;
    }

    for election in &elections {
        let election_status = elections_status.get(&election.id);

        update_election_voting_status(
            &hasura_transaction,
            &tenant_id,
            &election_event_id,
            &election.id,
            serde_json::to_value(&election_status).with_context(|| "Error parsing status")?,
        )
        .await
        .with_context(|| "Error updating election voting status")?;
    }

    update_election_event_status(
        &hasura_transaction,
        &&tenant_id,
        election_event_id,
        serde_json::to_value(&status).with_context(|| "Error parsing status")?,
    )
    .await
    .with_context(|| "Error updating election event status")?;

    Ok(election_event)
}

#[instrument(err)]
pub async fn update_election_voting_status_impl(
    tenant_id: String,
    user_id: Option<&str>,
    username: Option<&str>,
    election_event_id: String,
    election_id: String,
    new_status: VotingStatus,
    channel: VotingStatusChannel,
    bulletin_board_reference: Option<Value>,
    hasura_transaction: &Transaction<'_>,
) -> Result<()> {
    let election_event =
        get_election_event_by_id(hasura_transaction, &tenant_id, &election_event_id)
            .await
            .with_context(|| "Error obtaining election event")?;

    if election_event.is_archived {
        info!("Election event is archived, skipping");
        return Ok(());
    }

    let Some(election) = get_election_by_id(
        hasura_transaction,
        &tenant_id,
        &election_event_id,
        &election_id,
    )
    .await
    .with_context(|| "Error getting election by id")?
    else {
        event!(Level::WARN, "Election not found");
        return Ok(());
    };

    let mut status = get_election_status(election.status.clone()).unwrap_or_default();

    let current_voting_status = status.status_by_channel(&channel).clone();

    if new_status == current_voting_status {
        info!("New status is the same as the current voting status, skipping");
        return Ok(());
    }

    let election_presentation = election.get_presentation().unwrap_or_default();

    if VotingStatus::CLOSED == new_status
        && VotingPeriodEnd::DISALLOWED
            == election_presentation
                .voting_period_end
                .clone()
                .unwrap_or_default()
    {
        return Err(anyhow!(
            "election {:?} has the voting period end disallowed",
            election_id,
        ));
    }

    if new_status == VotingStatus::OPEN
        && election_presentation
            .initialization_report_policy
            .unwrap_or(EInitializeReportPolicy::default())
            == EInitializeReportPolicy::REQUIRED
        && !election.initialization_report_generated.unwrap_or(false)
    {
        return Err(anyhow!(
            "election {:?} initialization report must be generated before opening the election",
            election_id,
        ));
    }

    let expected_next_status = match current_voting_status {
        VotingStatus::NOT_STARTED => {
            vec![VotingStatus::OPEN]
        }
        VotingStatus::OPEN => {
            vec![VotingStatus::PAUSED, VotingStatus::CLOSED]
        }
        VotingStatus::PAUSED => {
            vec![VotingStatus::CLOSED, VotingStatus::OPEN]
        }
        VotingStatus::CLOSED => {
            vec![VotingStatus::OPEN]
        }
    };

    if !expected_next_status.contains(&new_status) {
        return Err(anyhow!(
            "Unexpected next status {new_status:?}, expected {expected_next_status:?}, current {current_voting_status:?}",
        ));
    }

    status.set_status_by_channel(&channel, new_status.clone());

    let status_js = serde_json::to_value(&status).with_context(|| "Error parsing status")?;

    update_election_voting_status(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        &election_id,
        status_js,
    )
    .await
    .with_context(|| "Error updating election voting status")?;

    update_board_on_status_change(
        &hasura_transaction,
        &tenant_id,
        user_id,
        username,
        election_event_id.to_string(),
        bulletin_board_reference.clone(),
        new_status.clone(),
        channel.clone(),
        Some(election_id.to_string()),
        None,
    )
    .await
    .with_context(|| "Error updating electoral board on status change")?;

    Ok(())
}
