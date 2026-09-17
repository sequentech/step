use std::collections::HashMap;

// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
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
use tracing::{event, info, instrument, warn, Level};

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
    update_event_voting_status_impl(
        hasura_transaction,
        tenant_id,
        user_id,
        username,
        election_event_id,
        new_status,
        channels,
        false,
    )
    .await
}

#[instrument(err)]
pub async fn update_scheduled_event_voting_status(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    user_id: Option<&str>,
    username: Option<&str>,
    election_event_id: &str,
    new_status: &VotingStatus,
    channels: &Option<Vec<VotingStatusChannel>>,
) -> Result<ElectionEvent> {
    update_event_voting_status_impl(
        hasura_transaction,
        tenant_id,
        user_id,
        username,
        election_event_id,
        new_status,
        channels,
        true,
    )
    .await
}

#[instrument(err)]
async fn update_event_voting_status_impl(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    user_id: Option<&str>,
    username: Option<&str>,
    election_event_id: &str,
    new_status: &VotingStatus,
    channels: &Option<Vec<VotingStatusChannel>>,
    enabled_only: bool,
) -> Result<ElectionEvent> {
    let election_event = get_election_event_by_id(hasura_transaction, tenant_id, election_event_id)
        .await
        .with_context(|| "Error obtaining election event")?;

    let mut status =
        get_election_event_status(election_event.status.clone()).unwrap_or(Default::default());
    let elections = get_elections(hasura_transaction, tenant_id, election_event_id)
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

        if VotingStatusChannel::EARLY_VOTING
            .channel_from(&voting_channels)
            .unwrap_or(false)
        {
            event_channels.push(VotingStatusChannel::EARLY_VOTING)
        }

        if VotingStatusChannel::TELEPHONE
            .channel_from(&voting_channels)
            .unwrap_or(false)
        {
            event_channels.push(VotingStatusChannel::TELEPHONE)
        }

        event_channels
    } else {
        info!("Default voting channels");
        // Update all if none are configured
        vec![
            VotingStatusChannel::ONLINE,
            VotingStatusChannel::KIOSK,
            VotingStatusChannel::EARLY_VOTING,
            VotingStatusChannel::TELEPHONE,
        ]
    };

    if election_event.is_archived {
        info!("Election event is archived, skipping");
        return Ok(election_event);
    }

    let configured: HashMap<String, VotingChannels> = elections
        .iter()
        .filter(|_| enabled_only)
        .map(|election| {
            let channels = election
                .voting_channels
                .clone()
                .map(deserialize_value)
                .transpose()?
                .unwrap_or_default();
            Ok((election.id.clone(), channels))
        })
        .collect::<Result<_>>()?;

    for channel in channels {
        if enabled_only {
            let elections_ids = apply_scheduled_event_channel(
                &mut status,
                &mut elections_status,
                &configured,
                channel,
                new_status,
            )?;
            if elections_ids.is_empty() {
                info!("No election needs {channel:?} set to {new_status:?}, skipping");
                continue;
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
            continue;
        }

        let current_voting_status = status.status_by_channel(channel).clone();

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

        if channel == VotingStatusChannel::EARLY_VOTING
            && status.status_by_channel(VotingStatusChannel::ONLINE) != VotingStatus::NOT_STARTED
        {
            return Err(anyhow!(
                "It is not allowed to start EARLY_VOTING channel because ONLINE channel was already started in the past.",
            ));
        }

        status.close_early_voting_if_online_status_change(channel, new_status.clone());
        status.set_status_by_channel(channel, new_status.clone());

        let mut elections_ids: Vec<String> = Vec::new();
        if *new_status == VotingStatus::OPEN || *new_status == VotingStatus::CLOSED {
            for election in &elections {
                if let Some(status) = elections_status.get_mut(&election.id) {
                    status.close_early_voting_if_online_status_change(channel, new_status.clone());
                    status.set_status_by_channel(channel, new_status.clone());
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

    let current_voting_status = status.status_by_channel(channel).clone();

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

    if channel == VotingStatusChannel::EARLY_VOTING
        && status.status_by_channel(VotingStatusChannel::ONLINE) != VotingStatus::NOT_STARTED
    {
        return Err(anyhow!(
            "It is not allowed to start EARLY_VOTING channel because ONLINE channel was already started in the past.",
        ));
    }

    status.close_early_voting_if_online_status_change(channel, new_status.clone());
    status.set_status_by_channel(channel, new_status.clone());

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

/// Scheduled changes never reopen closed voting: a start opens channels that
/// never started or are paused, and an end closes open or paused channels.
pub fn scheduled_transition_applies(current: &VotingStatus, new_status: &VotingStatus) -> bool {
    match new_status {
        VotingStatus::OPEN => matches!(current, VotingStatus::NOT_STARTED | VotingStatus::PAUSED),
        VotingStatus::CLOSED => matches!(current, VotingStatus::OPEN | VotingStatus::PAUSED),
        _ => false,
    }
}

/// Applies a scheduled change to every election that enables the channel,
/// independently of the event-level status, and returns the elections that
/// changed.
fn apply_scheduled_event_channel(
    event_status: &mut ElectionEventStatus,
    elections_status: &mut HashMap<String, ElectionStatus>,
    configured: &HashMap<String, VotingChannels>,
    channel: VotingStatusChannel,
    new_status: &VotingStatus,
) -> Result<Vec<String>> {
    if channel == VotingStatusChannel::EARLY_VOTING
        && *new_status == VotingStatus::OPEN
        && event_status.status_by_channel(VotingStatusChannel::ONLINE) != VotingStatus::NOT_STARTED
    {
        return Err(anyhow!(
            "It is not allowed to start EARLY_VOTING channel because ONLINE channel was already started in the past.",
        ));
    }

    let mut changed = Vec::new();
    for (election_id, election_status) in elections_status.iter_mut() {
        let Some(election_channels) = configured.get(election_id) else {
            continue;
        };
        if channel == VotingStatusChannel::EARLY_VOTING
            && *new_status == VotingStatus::OPEN
            && election_status.status_by_channel(VotingStatusChannel::ONLINE)
                != VotingStatus::NOT_STARTED
        {
            warn!(
                "Election {election_id}: not starting EARLY_VOTING because ONLINE voting already started"
            );
            continue;
        }
        if apply_scheduled_channel(election_status, election_channels, channel, new_status) {
            changed.push(election_id.clone());
        }
    }
    changed.sort();

    if !changed.is_empty() && event_status.status_by_channel(channel) != *new_status {
        event_status.close_early_voting_if_online_status_change(channel, new_status.clone());
        event_status.set_status_by_channel(channel, new_status.clone());
    }
    Ok(changed)
}

fn apply_scheduled_channel(
    status: &mut ElectionStatus,
    configured: &VotingChannels,
    channel: VotingStatusChannel,
    new_status: &VotingStatus,
) -> bool {
    if channel.channel_from(configured) != Some(true)
        || !scheduled_transition_applies(&status.status_by_channel(channel), new_status)
    {
        return false;
    }
    status.close_early_voting_if_online_status_change(channel, new_status.clone());
    status.set_status_by_channel(channel, new_status.clone());
    true
}
#[cfg(test)]
mod scheduled_channel_tests {
    use super::*;
    #[test]
    fn start_and_end_only_change_enabled_election_channels() {
        for channel in [
            VotingStatusChannel::ONLINE,
            VotingStatusChannel::KIOSK,
            VotingStatusChannel::EARLY_VOTING,
            VotingStatusChannel::TELEPHONE,
        ] {
            for enabled in [None, Some(false), Some(true)] {
                let configured = VotingChannels {
                    online: enabled,
                    kiosk: enabled,
                    early_voting: enabled,
                    telephone: enabled,
                    paper: None,
                };
                for (old, next) in [
                    (VotingStatus::NOT_STARTED, VotingStatus::OPEN),
                    (VotingStatus::OPEN, VotingStatus::CLOSED),
                ] {
                    let mut status = ElectionStatus::default();
                    status.set_status_by_channel(channel, old);
                    let before = serde_json::to_value(&status).unwrap();
                    assert_eq!(
                        apply_scheduled_channel(&mut status, &configured, channel, &next),
                        enabled == Some(true)
                    );
                    if enabled == Some(true) {
                        assert_eq!(status.status_by_channel(channel), next);
                    } else {
                        assert_eq!(serde_json::to_value(status).unwrap(), before);
                    }
                }
            }
        }
    }
    #[test]
    fn end_skips_an_enabled_channel_that_never_started() {
        let mut status = ElectionStatus::default();
        assert!(!apply_scheduled_channel(
            &mut status,
            &VotingChannels::default(),
            VotingStatusChannel::ONLINE,
            &VotingStatus::CLOSED
        ));
        assert_eq!(status.voting_status, VotingStatus::NOT_STARTED);
    }

    fn all_enabled() -> VotingChannels {
        VotingChannels {
            online: Some(true),
            kiosk: Some(true),
            early_voting: Some(true),
            telephone: Some(true),
            paper: None,
        }
    }

    fn election(channel: VotingStatusChannel, current: VotingStatus) -> ElectionStatus {
        let mut status = ElectionStatus::default();
        status.set_status_by_channel(channel, current);
        status
    }

    #[test]
    fn scheduled_changes_only_open_unstarted_or_paused_and_only_close_open_or_paused() {
        use VotingStatus::*;
        for (current, next, applies) in [
            (NOT_STARTED, OPEN, true),
            (PAUSED, OPEN, true),
            (OPEN, OPEN, false),
            (CLOSED, OPEN, false),
            (OPEN, CLOSED, true),
            (PAUSED, CLOSED, true),
            (NOT_STARTED, CLOSED, false),
            (CLOSED, CLOSED, false),
        ] {
            assert_eq!(
                scheduled_transition_applies(&current, &next),
                applies,
                "{current:?} -> {next:?}"
            );
        }
    }

    #[test]
    fn event_wide_end_closes_elections_opened_at_election_level() {
        let mut event = ElectionEventStatus::default();
        let mut elections = HashMap::from([
            (
                "el1".to_string(),
                election(VotingStatusChannel::KIOSK, VotingStatus::OPEN),
            ),
            ("el2".to_string(), ElectionStatus::default()),
        ]);
        let configured = HashMap::from([
            ("el1".to_string(), all_enabled()),
            ("el2".to_string(), all_enabled()),
        ]);

        let changed = apply_scheduled_event_channel(
            &mut event,
            &mut elections,
            &configured,
            VotingStatusChannel::KIOSK,
            &VotingStatus::CLOSED,
        )
        .unwrap();

        assert_eq!(changed, vec!["el1".to_string()]);
        assert_eq!(elections["el1"].kiosk_voting_status, VotingStatus::CLOSED);
        assert_eq!(
            elections["el2"].kiosk_voting_status,
            VotingStatus::NOT_STARTED
        );
        assert_eq!(event.kiosk_voting_status, VotingStatus::CLOSED);
    }

    #[test]
    fn event_wide_start_never_reopens_a_closed_election() {
        let mut event = ElectionEventStatus::default();
        event.set_status_by_channel(VotingStatusChannel::ONLINE, VotingStatus::OPEN);
        let mut elections = HashMap::from([
            (
                "open".to_string(),
                election(VotingStatusChannel::ONLINE, VotingStatus::OPEN),
            ),
            (
                "closed".to_string(),
                election(VotingStatusChannel::ONLINE, VotingStatus::CLOSED),
            ),
            (
                "paused".to_string(),
                election(VotingStatusChannel::ONLINE, VotingStatus::PAUSED),
            ),
            ("new".to_string(), ElectionStatus::default()),
        ]);
        let configured = elections
            .keys()
            .map(|id| (id.clone(), all_enabled()))
            .collect::<HashMap<_, _>>();

        let mut changed = apply_scheduled_event_channel(
            &mut event,
            &mut elections,
            &configured,
            VotingStatusChannel::ONLINE,
            &VotingStatus::OPEN,
        )
        .unwrap();
        changed.sort();

        assert_eq!(changed, vec!["new".to_string(), "paused".to_string()]);
        assert_eq!(elections["closed"].voting_status, VotingStatus::CLOSED);
        assert_eq!(elections["open"].voting_status, VotingStatus::OPEN);
        assert_eq!(elections["paused"].voting_status, VotingStatus::OPEN);
        assert_eq!(elections["new"].voting_status, VotingStatus::OPEN);
    }

    #[test]
    fn event_wide_change_with_nothing_to_do_leaves_every_status_untouched() {
        let mut event = ElectionEventStatus::default();
        event.set_status_by_channel(VotingStatusChannel::ONLINE, VotingStatus::OPEN);
        let mut elections = HashMap::from([(
            "closed".to_string(),
            election(VotingStatusChannel::ONLINE, VotingStatus::CLOSED),
        )]);
        let configured = HashMap::from([("closed".to_string(), all_enabled())]);
        let before = serde_json::to_value(&event).unwrap();

        let changed = apply_scheduled_event_channel(
            &mut event,
            &mut elections,
            &configured,
            VotingStatusChannel::ONLINE,
            &VotingStatus::OPEN,
        )
        .unwrap();

        assert!(changed.is_empty());
        assert_eq!(serde_json::to_value(&event).unwrap(), before);
        assert_eq!(elections["closed"].voting_status, VotingStatus::CLOSED);
    }

    #[test]
    fn event_wide_early_voting_start_skips_elections_whose_online_voting_started() {
        let mut event = ElectionEventStatus::default();
        let mut elections = HashMap::from([
            (
                "online-open".to_string(),
                election(VotingStatusChannel::ONLINE, VotingStatus::OPEN),
            ),
            ("unstarted".to_string(), ElectionStatus::default()),
        ]);
        let configured = elections
            .keys()
            .map(|id| (id.clone(), all_enabled()))
            .collect::<HashMap<_, _>>();

        let changed = apply_scheduled_event_channel(
            &mut event,
            &mut elections,
            &configured,
            VotingStatusChannel::EARLY_VOTING,
            &VotingStatus::OPEN,
        )
        .unwrap();

        assert_eq!(changed, vec!["unstarted".to_string()]);
        assert_eq!(
            elections["online-open"].early_voting_status,
            VotingStatus::NOT_STARTED
        );
        assert_eq!(
            elections["unstarted"].early_voting_status,
            VotingStatus::OPEN
        );
    }
}
