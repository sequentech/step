use std::collections::{BTreeMap, HashMap, HashSet};

// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::ballot_box_seal::list_for_elections;
use crate::postgres::election::{get_election_by_id, get_elections, update_election_voting_status};
use crate::postgres::election_event::{get_election_event_by_id, update_election_event_status};
use crate::postgres::trusted_write;
use anyhow::{Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::ballot::*;
use sequent_core::serialization::deserialize_with_path::deserialize_value;
use sequent_core::types::hasura::core::{ElectionEvent, VotingChannels};
use serde_json::value::Value;
use tracing::{event, info, instrument, warn, Level};

use super::ballot_box_seal::{self, CloseProvenance};
use super::initialization_scope::{
    initialization_refusal_for, initialization_refusals, name_list, post_display_name,
};
use super::voting_status::update_board_on_status_change;

/// A policy or state transition refusal safe for action clients to display.
/// Storage and external-service errors retain their ordinary internal error type.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct VotingTransitionError(String);

pub fn get_election_event_status(status_json_opt: Option<Value>) -> Option<ElectionEventStatus> {
    status_json_opt.and_then(|status_json| deserialize_value(status_json).ok())
}

pub fn get_election_status(status_json_opt: Option<Value>) -> Option<ElectionStatus> {
    status_json_opt.and_then(|status_json| deserialize_value(status_json).ok())
}

/// Who requested an event-wide voting status change. Scheduled changes only
/// touch channels enabled for each election and never reopen closed voting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VotingStatusUpdateSource {
    Manual,
    Scheduled,
}

/// A Post an event-wide change left as it was, and why: `reason` is the
/// refusal's code (e.g. `ballot-box-seal-policy`).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SkippedElection {
    pub election_id: String,
    pub election_name: String,
    pub reason: String,
}

/// A manual event-wide voting status change. Returns the event and the
/// Posts it left as they were (with Seal at close, an opening leaves closed
/// Posts closed).
#[instrument(err)]
pub async fn update_event_voting_status(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    user_id: Option<&str>,
    username: Option<&str>,
    election_event_id: &str,
    new_status: &VotingStatus,
    channels: &Option<Vec<VotingStatusChannel>>,
) -> Result<(ElectionEvent, Vec<SkippedElection>)> {
    let (election_event, skipped) = update_event_voting_status_impl(
        hasura_transaction,
        tenant_id,
        user_id,
        username,
        election_event_id,
        new_status,
        channels,
        VotingStatusUpdateSource::Manual,
        &HashSet::new(),
        None,
    )
    .await?;
    let skipped = skipped
        .into_iter()
        .map(|(election_id, (election_name, refusal))| SkippedElection {
            election_id,
            election_name,
            reason: refusal.code().to_string(),
        })
        .collect();
    Ok((election_event, skipped))
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
    // Posts with a scheduled row of their own for this change: that row
    // decides for them (VOTE-LIFECYCLE §5).
    own_rows: &HashSet<String>,
) -> Result<ElectionEvent> {
    Ok(update_event_voting_status_impl(
        hasura_transaction,
        tenant_id,
        user_id,
        username,
        election_event_id,
        new_status,
        channels,
        VotingStatusUpdateSource::Scheduled,
        own_rows,
        None,
    )
    .await?
    .0)
}

/// A scheduled event-wide change restricted to the Posts in `only` (all
/// when `None`). Returns the Posts it would have opened but that wait for
/// their initialization at the event's scope (VOTE-LIFECYCLE §9), with
/// their display names and why.
#[instrument(err)]
pub async fn update_scheduled_event_voting_status_for(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    new_status: &VotingStatus,
    channels: &Option<Vec<VotingStatusChannel>>,
    own_rows: &HashSet<String>,
    only: Option<&HashSet<String>>,
) -> Result<BTreeMap<String, (String, TransitionRefusal)>> {
    Ok(update_event_voting_status_impl(
        hasura_transaction,
        tenant_id,
        None,
        None,
        election_event_id,
        new_status,
        channels,
        VotingStatusUpdateSource::Scheduled,
        own_rows,
        only,
    )
    .await?
    .1)
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
    source: VotingStatusUpdateSource,
    // Posts with a scheduled row of their own for this change (B, §5).
    own_rows: &HashSet<String>,
    // Posts an event-wide opening still waits for (I, §9); all when `None`.
    only: Option<&HashSet<String>>,
) -> Result<(ElectionEvent, BTreeMap<String, (String, TransitionRefusal)>)> {
    // Read-modify-write of the event's and every Post's status: lock them
    // (the event row and the election rows) before reading.
    lock_status_rows(hasura_transaction, tenant_id, election_event_id, None).await?;
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
        return Ok((election_event, BTreeMap::new()));
    }

    // Posts that can't open yet with respect to their initialization at the
    // event's scope (design §9). A scheduled opening skips them; a manual
    // one is refused while one of them would open.
    let opening_refusals = if *new_status == VotingStatus::OPEN {
        initialization_refusals(hasura_transaction, &election_event, &elections).await?
    } else {
        Default::default()
    };

    // With Seal at close, an event-wide opening leaves closed Posts, and
    // Posts that have seals, closed (VOTE-FREEZE).
    let sealed_elections: HashSet<String> = if *new_status == VotingStatus::OPEN
        && ballot_box_seal::seal_policy(&election_event) == BallotBoxSealPolicy::SEAL_AT_CLOSE
    {
        let election_uuids = elections
            .iter()
            .map(|election| uuid::Uuid::parse_str(&election.id))
            .collect::<std::result::Result<Vec<_>, _>>()?;
        list_for_elections(
            hasura_transaction,
            &uuid::Uuid::parse_str(tenant_id)?,
            &uuid::Uuid::parse_str(election_event_id)?,
            &election_uuids,
        )
        .await?
        .into_iter()
        .map(|seal| seal.election_id.to_string())
        .collect()
    } else {
        HashSet::new()
    };
    let seal_policy_on =
        ballot_box_seal::seal_policy(&election_event) == BallotBoxSealPolicy::SEAL_AT_CLOSE;
    let configured: HashMap<String, VotingChannels> = elections
        .iter()
        .filter(|election| {
            source == VotingStatusUpdateSource::Scheduled && !own_rows.contains(&election.id)
        })
        .filter(|election| only.map_or(true, |only| only.contains(&election.id)))
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
    // A scheduled opening skips the Posts that would open but can't yet.
    let language = election_event.get_default_language();
    let mut skipped: BTreeMap<String, (String, TransitionRefusal)> = elections
        .iter()
        .filter_map(|election| {
            let refusal = opening_refusals.get(&election.id).cloned().or_else(|| {
                // A scheduled opening leaves Posts with seals closed too.
                (seal_policy_on && sealed_elections.contains(&election.id))
                    .then_some(TransitionRefusal::BallotBoxSealPolicy)
            })?;
            let election_channels = configured.get(&election.id)?;
            let status = elections_status.get(&election.id)?;
            channels
                .iter()
                .any(|channel| {
                    channel.channel_from(election_channels) == Some(true)
                        && scheduled_transition_applies(
                            &status.status_by_channel(*channel),
                            new_status,
                        )
                })
                .then(|| {
                    (
                        election.id.clone(),
                        (post_display_name(election, &language), refusal.clone()),
                    )
                })
        })
        .collect();
    let configured: HashMap<String, VotingChannels> = configured
        .into_iter()
        .filter(|(election_id, _)| {
            !opening_refusals.contains_key(election_id)
                && !(seal_policy_on && sealed_elections.contains(election_id))
        })
        .collect();
    for (election_id, (name, refusal)) in &skipped {
        info!(
            %election_id,
            refusal = refusal.code(),
            "Not opening Post {name:?} on schedule"
        );
    }

    // The board learns of the change once it is saved (and its seals made).
    let mut board_posts: Vec<(VotingStatusChannel, Vec<String>)> = vec![];

    for channel in channels {
        if source == VotingStatusUpdateSource::Scheduled {
            let elections_ids = apply_scheduled_event_channel(
                &mut status,
                &mut elections_status,
                &configured,
                channel,
                new_status,
                ballot_box_seal::seal_policy(&election_event),
            )?;
            if elections_ids.is_empty() {
                info!("No election needs {channel:?} set to {new_status:?}, skipping");
                continue;
            }
            board_posts.push((channel, elections_ids));
            continue;
        }

        let current_voting_status = status.status_by_channel(channel).clone();

        if current_voting_status == new_status.clone() {
            info!("Current voting status is the same as the new voting status, skipping");
            continue;
        }

        let expected_next_status = expected_next_statuses(
            &current_voting_status,
            ballot_box_seal::seal_policy(&election_event),
        );

        if !expected_next_status.contains(&new_status) {
            return Err(VotingTransitionError(format!(
            "Unexpected next status {new_status:?}, expected {expected_next_status:?}, current {current_voting_status:?}",
        )).into());
        }

        if let Some(refusal) = seal_policy_refusal(
            hasura_transaction,
            &election_event,
            None,
            &current_voting_status,
            new_status,
        )
        .await?
        {
            return Err(VotingTransitionError(refusal.message(
                election_event_id,
                new_status,
                &current_voting_status,
            ))
            .into());
        }

        if early_voting_locked(
            channel,
            status.status_by_channel(VotingStatusChannel::ONLINE),
            new_status,
            ballot_box_seal::seal_policy(&election_event),
        ) {
            return Err(VotingTransitionError(
                "It is not allowed to start EARLY_VOTING channel because ONLINE channel was already started in the past.".into(),
            ).into());
        }

        // Only the Posts this Start opens: the ones it leaves alone (not
        // enabled, or kept closed under Seal at close) don't refuse it (R11 S2).
        let mut blocked: Vec<String> = vec![];
        for election in &elections {
            let Some(refusal) = opening_refusals.get(&election.id) else {
                continue;
            };
            let Some(election_status) = elections_status.get(&election.id) else {
                continue;
            };
            if election_status.status_by_channel(channel) == VotingStatus::OPEN {
                continue;
            }
            let change = manual_post_change(
                ballot_box_seal::seal_policy(&election_event),
                &election.id,
                election.voting_channels.as_ref(),
                election_status,
                sealed_elections.contains(&election.id),
                channel,
                new_status,
            )?;
            if change == ManualPostChange::Apply {
                blocked.push(refusal.message(&election.id, new_status, &current_voting_status));
            }
        }
        if !blocked.is_empty() {
            return Err(VotingTransitionError(blocked.join("; ")).into());
        }

        status.close_early_voting_if_online_status_change(channel, new_status.clone());
        status.set_status_by_channel(channel, new_status.clone());

        let mut elections_ids: Vec<String> = Vec::new();
        if *new_status == VotingStatus::OPEN || *new_status == VotingStatus::CLOSED {
            for election in &elections {
                if let Some(status) = elections_status.get_mut(&election.id) {
                    match manual_post_change(
                        ballot_box_seal::seal_policy(&election_event),
                        &election.id,
                        election.voting_channels.as_ref(),
                        status,
                        sealed_elections.contains(&election.id),
                        channel,
                        new_status,
                    )? {
                        ManualPostChange::Apply => {}
                        ManualPostChange::NotEnabled => {
                            info!(
                                election_id = %election.id,
                                ?channel,
                                reason = NOT_ENABLED_REASON,
                                "Not opening this channel at this Post: the Post doesn't enable it, \
                                 and with the Ballot Box Seal Policy set to Seal at close an \
                                 event-wide Start opens a channel only where a Post enables it"
                            );
                            continue;
                        }
                        ManualPostChange::KeptClosed => {
                            info!(
                                election_id = %election.id,
                                ?channel,
                                "Not opening this Post: with the Ballot Box Seal Policy set to \
                                 Seal at close, a Post whose voting closed stays closed"
                            );
                            skipped.entry(election.id.clone()).or_insert_with(|| {
                                (
                                    post_display_name(election, &language),
                                    TransitionRefusal::BallotBoxSealPolicy,
                                )
                            });
                            continue;
                        }
                    }
                    status.close_early_voting_if_online_status_change(channel, new_status.clone());
                    status.set_status_by_channel(channel, new_status.clone());
                }
                elections_ids.push(election.id.clone());
            }
        }

        board_posts.push((channel, elections_ids));
    }

    save_event_voting_status(
        hasura_transaction,
        tenant_id,
        election_event_id,
        &elections,
        &elections_status,
        &status,
    )
    .await?;

    // The elections this left closed get their ballot boxes' seals (VOTE-FREEZE).
    if *new_status == VotingStatus::CLOSED {
        let provenance = match source {
            VotingStatusUpdateSource::Manual => CloseProvenance::User {
                username: username.map(str::to_owned),
            },
            VotingStatusUpdateSource::Scheduled => CloseProvenance::Scheduled,
        };
        let election_ids: Vec<String> = elections
            .iter()
            .map(|election| election.id.clone())
            .collect();
        ballot_box_seal::on_close(
            hasura_transaction,
            &election_event,
            &election_ids,
            provenance,
        )
        .await?;
    }

    for (channel, elections_ids) in board_posts {
        update_board_on_status_change(
            hasura_transaction,
            &tenant_id,
            user_id,
            username,
            election_event.id.to_string(),
            election_event.bulletin_board_reference.clone(),
            new_status.clone(),
            channel,
            None,
            Some(elections_ids),
        )
        .await
        .with_context(|| "Error updating electoral board on status change")?;
    }

    Ok((election_event, skipped))
}

/// Writes an event-wide voting status change, past its checks and logs:
/// every election's status and the event's, as the server's own write.
#[instrument(skip_all, err)]
pub async fn save_event_voting_status(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    elections: &[sequent_core::types::hasura::core::Election],
    elections_status: &HashMap<String, ElectionStatus>,
    status: &ElectionEventStatus,
) -> Result<()> {
    // Check the complete map before marking the transaction or writing any row.
    for election in elections {
        anyhow::ensure!(
            elections_status.contains_key(&election.id),
            "Missing voting status for election {}",
            election.id
        );
    }
    trusted_write(hasura_transaction).await?;
    for election in elections {
        let election_status = elections_status
            .get(&election.id)
            .with_context(|| format!("Missing voting status for election {}", election.id))?;

        update_election_voting_status(
            hasura_transaction,
            tenant_id,
            election_event_id,
            &election.id,
            serde_json::to_value(&election_status).with_context(|| "Error parsing status")?,
        )
        .await
        .with_context(|| "Error updating election voting status")?;
    }

    update_election_event_status(
        hasura_transaction,
        tenant_id,
        election_event_id,
        serde_json::to_value(status).with_context(|| "Error parsing status")?,
    )
    .await
    .with_context(|| "Error updating election event status")?;
    Ok(())
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
    if change_election_voting_status(
        &tenant_id,
        user_id,
        username,
        &election_event_id,
        &election_id,
        &new_status,
        channel,
        hasura_transaction,
    )
    .await?
    {
        post_election_status_change(
            hasura_transaction,
            &tenant_id,
            user_id,
            username,
            &election_event_id,
            &election_id,
            bulletin_board_reference,
            &new_status,
            channel,
        )
        .await?;
    }
    Ok(())
}

/// The board entry of a Post's channel change; post it once the change (and
/// any seals it made) is saved in the transaction.
#[allow(clippy::too_many_arguments)]
pub async fn post_election_status_change(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    user_id: Option<&str>,
    username: Option<&str>,
    election_event_id: &str,
    election_id: &str,
    bulletin_board_reference: Option<Value>,
    new_status: &VotingStatus,
    channel: VotingStatusChannel,
) -> Result<()> {
    update_board_on_status_change(
        hasura_transaction,
        tenant_id,
        user_id,
        username,
        election_event_id.to_string(),
        bulletin_board_reference,
        new_status.clone(),
        channel,
        Some(election_id.to_string()),
        None,
    )
    .await
    .with_context(|| "Error updating electoral board on status change")
}

/// Changes one channel of a Post, past its checks, and makes its seals when
/// that closes its voting, without the board entry (see
/// [`post_election_status_change`]). Returns whether it changed anything.
#[allow(clippy::too_many_arguments)]
#[instrument(err)]
pub async fn change_election_voting_status(
    tenant_id: &str,
    user_id: Option<&str>,
    username: Option<&str>,
    election_event_id: &str,
    election_id: &str,
    new_status: &VotingStatus,
    channel: VotingStatusChannel,
    hasura_transaction: &Transaction<'_>,
) -> Result<bool> {
    let tenant_id = tenant_id.to_string();
    let election_event_id = election_event_id.to_string();
    let election_id = election_id.to_string();
    let new_status = new_status.clone();
    let election_event =
        get_election_event_by_id(hasura_transaction, &tenant_id, &election_event_id)
            .await
            .with_context(|| "Error obtaining election event")?;

    if election_event.is_archived {
        info!("Election event is archived, skipping");
        return Ok(false);
    }

    // Read-modify-write of the Post's status: no concurrent change in
    // between.
    lock_status_rows(
        hasura_transaction,
        &tenant_id,
        &election_event_id,
        Some(&election_id),
    )
    .await?;
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
        return Ok(false);
    };

    let mut status = get_election_status(election.status.clone()).unwrap_or_default();

    let current_voting_status = status.status_by_channel(channel).clone();

    if new_status == current_voting_status {
        info!("New status is the same as the current voting status, skipping");
        return Ok(false);
    }

    let refusal = match voting_transition_refusal_with(
        &election,
        &status,
        channel,
        &new_status,
        ballot_box_seal::seal_policy(&election_event),
    ) {
        Some(refusal) => Some(refusal),
        None => {
            seal_policy_refusal(
                hasura_transaction,
                &election_event,
                Some(&election_id),
                &current_voting_status,
                &new_status,
            )
            .await?
        }
    };
    if let Some(refusal) = refusal {
        return Err(VotingTransitionError(refusal.message(
            &election_id,
            &new_status,
            &current_voting_status,
        ))
        .into());
    }

    if new_status == VotingStatus::OPEN {
        if let Some(refusal) =
            initialization_refusal_for(hasura_transaction, &election_event, &election).await?
        {
            return Err(VotingTransitionError(refusal.message(
                &election_id,
                &new_status,
                &current_voting_status,
            ))
            .into());
        }
    }

    status.close_early_voting_if_online_status_change(channel, new_status.clone());
    status.set_status_by_channel(channel, new_status.clone());

    let status_js = serde_json::to_value(&status).with_context(|| "Error parsing status")?;

    // The voting status changes here, past the checks above.
    trusted_write(hasura_transaction).await?;
    update_election_voting_status(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        &election_id,
        status_js,
    )
    .await
    .with_context(|| "Error updating election voting status")?;

    // Closing it may leave every enabled channel closed: its ballot boxes
    // then get their seals, in this transaction (VOTE-FREEZE).
    if new_status == VotingStatus::CLOSED {
        ballot_box_seal::on_close(
            hasura_transaction,
            &election_event,
            &[election_id.clone()],
            CloseProvenance::of_actor(user_id, username),
        )
        .await?;
    }

    Ok(true)
}

/// Why a channel of a Post can't change to a voting status now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransitionRefusal {
    /// The Post doesn't let voting end.
    VotingPeriodEndDisallowed,
    /// Opening needs the initialization report first.
    InitializationReportRequired,
    /// The status can't follow the current one.
    UnexpectedNextStatus(Vec<VotingStatus>),
    /// Early voting can't start once online voting has.
    EarlyVotingAfterOnline,
    /// The initialization scope is the event: other Posts that require
    /// their initialization report aren't initialized yet.
    EventNotInitialized {
        /// The Post that can't open.
        post: String,
        election_ids: Vec<String>,
        names: Vec<String>,
    },
    /// The initialization scope is the Post and its countries: some
    /// countries of the Post aren't initialized yet.
    CountriesNotInitialized {
        /// The Post that can't open.
        post: String,
        area_ids: Vec<String>,
        names: Vec<String>,
    },
    /// The event seals its ballot boxes at close: closed voting stays
    /// closed (VOTE-FREEZE).
    BallotBoxSealPolicy,
}

impl TransitionRefusal {
    /// A stable code for the refusal.
    pub fn code(&self) -> &'static str {
        match self {
            TransitionRefusal::VotingPeriodEndDisallowed => "voting-period-end-disallowed",
            TransitionRefusal::InitializationReportRequired => "initialization-report-required",
            TransitionRefusal::UnexpectedNextStatus(_) => "unexpected-next-status",
            TransitionRefusal::EarlyVotingAfterOnline => "early-voting-after-online",
            TransitionRefusal::EventNotInitialized { .. } => "event-not-initialized",
            TransitionRefusal::CountriesNotInitialized { .. } => "countries-not-initialized",
            TransitionRefusal::BallotBoxSealPolicy => "ballot-box-seal-policy",
        }
    }

    /// The message the status change has always failed with.
    pub fn message(
        &self,
        election_id: &str,
        new_status: &VotingStatus,
        current: &VotingStatus,
    ) -> String {
        match self {
            TransitionRefusal::VotingPeriodEndDisallowed => {
                format!("election {election_id:?} has the voting period end disallowed")
            }
            TransitionRefusal::InitializationReportRequired => format!(
                "election {election_id:?} initialization report must be generated before opening the election"
            ),
            TransitionRefusal::UnexpectedNextStatus(expected) => format!(
                "Unexpected next status {new_status:?}, expected {expected:?}, current {current:?}"
            ),
            TransitionRefusal::EarlyVotingAfterOnline => "It is not allowed to start EARLY_VOTING channel because ONLINE channel was already started in the past.".to_string(),
            TransitionRefusal::EventNotInitialized { post, names, .. } => format!(
                "Post {post:?} can't open yet: with the initialization scope \"event\", no Post opens until every Post whose initialization report is required is initialized. Not initialized yet: {}.",
                name_list(names)
            ),
            TransitionRefusal::CountriesNotInitialized { post, names, .. } => format!(
                "Post {post:?} can't open yet: with the initialization scope \"Post and country\", a Post whose initialization report is required opens once every country under it is initialized. Not initialized yet: {}.",
                name_list(names)
            ),
            TransitionRefusal::BallotBoxSealPolicy => "Voting can't start again: with the Ballot Box Seal Policy set to Seal at close, voting that has closed stays closed and its ballot boxes are sealed.".to_string(),
        }
    }
}

/// Whether the event's Ballot Box Seal Policy refuses a channel in status
/// `current` to change to `new_status`. With Seal at close, CLOSED never
/// goes back to OPEN, also before the seal; and once the election (when
/// given) has seals, none of its channels opens, not even one that never
/// started.
pub async fn seal_policy_refusal(
    hasura_transaction: &Transaction<'_>,
    election_event: &ElectionEvent,
    election_id: Option<&str>,
    current: &VotingStatus,
    new_status: &VotingStatus,
) -> Result<Option<TransitionRefusal>> {
    if *new_status != VotingStatus::OPEN
        || ballot_box_seal::seal_policy(election_event) != BallotBoxSealPolicy::SEAL_AT_CLOSE
    {
        return Ok(None);
    }
    if *current == VotingStatus::CLOSED {
        return Ok(Some(TransitionRefusal::BallotBoxSealPolicy));
    }
    if let Some(election_id) = election_id {
        if ballot_box_seal::election_has_seals(
            hasura_transaction,
            &election_event.tenant_id,
            &election_event.id,
            election_id,
        )
        .await?
        {
            return Ok(Some(TransitionRefusal::BallotBoxSealPolicy));
        }
    }
    Ok(None)
}

/// Whether `channel` of `election`, whose status is `status`, can't change to
/// `new_status`; `None` when it can. A change to the current status is no
/// change and is not refused.
pub fn voting_transition_refusal(
    election: &sequent_core::types::hasura::core::Election,
    status: &ElectionStatus,
    channel: VotingStatusChannel,
    new_status: &VotingStatus,
) -> Option<TransitionRefusal> {
    voting_transition_refusal_with(
        election,
        status,
        channel,
        new_status,
        BallotBoxSealPolicy::DO_NOT_SEAL,
    )
}

/// The statuses a channel in status `current` may move to. With the Ballot
/// Box Seal Policy set to Seal at close, a channel that never started may
/// also close, so it doesn't hold its election's seal back (VOTE-FREEZE).
pub fn expected_next_statuses(
    current: &VotingStatus,
    seal_policy: BallotBoxSealPolicy,
) -> Vec<VotingStatus> {
    match current {
        VotingStatus::NOT_STARTED if seal_policy == BallotBoxSealPolicy::SEAL_AT_CLOSE => {
            vec![VotingStatus::OPEN, VotingStatus::CLOSED]
        }
        VotingStatus::NOT_STARTED => vec![VotingStatus::OPEN],
        VotingStatus::OPEN => vec![VotingStatus::PAUSED, VotingStatus::CLOSED],
        VotingStatus::PAUSED => vec![VotingStatus::CLOSED, VotingStatus::OPEN],
        VotingStatus::CLOSED => vec![VotingStatus::OPEN],
    }
}

/// Whether early voting can't change because ONLINE voting has started:
/// it can't start then, and (as before) a policy-off event can't change it
/// at all; with Seal at close it may still close.
fn early_voting_locked(
    channel: VotingStatusChannel,
    online: VotingStatus,
    new_status: &VotingStatus,
    seal_policy: BallotBoxSealPolicy,
) -> bool {
    channel == VotingStatusChannel::EARLY_VOTING
        && online != VotingStatus::NOT_STARTED
        && (*new_status != VotingStatus::CLOSED
            || seal_policy != BallotBoxSealPolicy::SEAL_AT_CLOSE)
}

/// [`voting_transition_refusal`] under the event's Ballot Box Seal Policy.
pub fn voting_transition_refusal_with(
    election: &sequent_core::types::hasura::core::Election,
    status: &ElectionStatus,
    channel: VotingStatusChannel,
    new_status: &VotingStatus,
    seal_policy: BallotBoxSealPolicy,
) -> Option<TransitionRefusal> {
    let current_voting_status = status.status_by_channel(channel);
    if *new_status == current_voting_status {
        return None;
    }
    let election_presentation = election.get_presentation().unwrap_or_default();

    if VotingStatus::CLOSED == *new_status
        && VotingPeriodEnd::DISALLOWED
            == election_presentation
                .voting_period_end
                .clone()
                .unwrap_or_default()
    {
        return Some(TransitionRefusal::VotingPeriodEndDisallowed);
    }

    if *new_status == VotingStatus::OPEN
        && election_presentation
            .initialization_report_policy
            .unwrap_or(EInitializeReportPolicy::default())
            == EInitializeReportPolicy::REQUIRED
        && !election.initialization_report_generated.unwrap_or(false)
    {
        return Some(TransitionRefusal::InitializationReportRequired);
    }

    let expected_next_status = expected_next_statuses(&current_voting_status, seal_policy);

    if !expected_next_status.contains(new_status) {
        return Some(TransitionRefusal::UnexpectedNextStatus(
            expected_next_status,
        ));
    }

    if early_voting_locked(
        channel,
        status.status_by_channel(VotingStatusChannel::ONLINE),
        new_status,
        seal_policy,
    ) {
        return Some(TransitionRefusal::EarlyVotingAfterOnline);
    }
    None
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

/// Whether a scheduled change applies to `channel` of a Post in `status`
/// that enables `configured`: [`scheduled_transition_applies`], and, with
/// Seal at close, a scheduled close of a channel the schedule names that
/// the Post enables and never started, when another enabled channel of the
/// Post actually ran (so it doesn't hold the seal back). A Post that never
/// opened is left alone (VOTE-FREEZE, D1); see [`never_opened`].
pub fn scheduled_change_applies(
    status: &ElectionStatus,
    configured: &VotingChannels,
    channel: VotingStatusChannel,
    new_status: &VotingStatus,
    seal_policy: BallotBoxSealPolicy,
) -> bool {
    let current = status.status_by_channel(channel);
    if scheduled_transition_applies(&current, new_status) {
        return true;
    }
    *new_status == VotingStatus::CLOSED
        && seal_policy == BallotBoxSealPolicy::SEAL_AT_CLOSE
        && channel.channel_from(configured) == Some(true)
        && current == VotingStatus::NOT_STARTED
        && status.dates_by_channel(channel).first_started_at.is_none()
        && !never_opened(status, configured)
}

/// Whether no enabled channel of the Post ever started: with Seal at close
/// a scheduled close leaves such a Post alone ("never opened; nothing to
/// close").
pub fn never_opened(status: &ElectionStatus, configured: &VotingChannels) -> bool {
    ![
        VotingStatusChannel::ONLINE,
        VotingStatusChannel::KIOSK,
        VotingStatusChannel::EARLY_VOTING,
        VotingStatusChannel::TELEPHONE,
    ]
    .into_iter()
    .any(|channel| {
        channel.channel_from(configured) == Some(true)
            && status.dates_by_channel(channel).first_started_at.is_some()
    })
}

/// The reason a scheduled close records for a seal-at-close Post that never
/// opened: nothing to close, so it stays as it is.
pub const NEVER_OPENED_REASON: &str = "never-opened-kept-open";

/// The reason an event-wide manual Start logs for a channel it leaves alone
/// at a seal-at-close Post that doesn't enable it.
pub const NOT_ENABLED_REASON: &str = "channel-not-enabled-at-post";

/// What an event-wide manual status change does at one Post.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManualPostChange {
    /// The Post's channel is set.
    Apply,
    /// With Seal at close, a Start leaves a channel the Post doesn't enable
    /// as it is, as a scheduled Start does: it never runs there and never
    /// holds the Post's seal (VOTE-FREEZE, R10 B1).
    NotEnabled,
    /// With Seal at close, a Start leaves a Post whose channel is CLOSED, or
    /// that has seals, closed.
    KeptClosed,
}

/// What an event-wide manual change of `channel` to `new_status` does at the
/// Post `election_id`, with raw `voting_channels` (NULL enables ONLINE only,
/// `VotingChannels::default()`), `status` and whether it has seals. Policy
/// off, or not a Start: [`ManualPostChange::Apply`], without reading the
/// Post's channels, as before VOTE-FREEZE. The not-enabled check comes
/// before the closed one, so a Post is never reported as kept closed for a
/// channel it doesn't offer (R11 N3).
pub fn manual_post_change(
    seal_policy: BallotBoxSealPolicy,
    election_id: &str,
    voting_channels: Option<&Value>,
    status: &ElectionStatus,
    sealed: bool,
    channel: VotingStatusChannel,
    new_status: &VotingStatus,
) -> Result<ManualPostChange> {
    if seal_policy != BallotBoxSealPolicy::SEAL_AT_CLOSE || *new_status != VotingStatus::OPEN {
        return Ok(ManualPostChange::Apply);
    }
    let configured: VotingChannels = voting_channels
        .cloned()
        .map(deserialize_value)
        .transpose()
        .with_context(|| format!("Failed to deserialize the voting channels of {election_id}"))?
        .unwrap_or_default();
    if channel.channel_from(&configured) != Some(true) {
        return Ok(ManualPostChange::NotEnabled);
    }
    if sealed || status.status_by_channel(channel) == VotingStatus::CLOSED {
        return Ok(ManualPostChange::KeptClosed);
    }
    Ok(ManualPostChange::Apply)
}

/// Locks the event row and its election rows (all, or one) for a voting
/// status change, in a fixed order (event, then elections by id), with
/// `FOR NO KEY UPDATE`: status writers serialize, while the foreign-key
/// checks of casts (`FOR KEY SHARE`) don't wait. It takes no signing lock,
/// so the signed effect, which holds its request row, can't deadlock with a
/// claim that holds the signing lock and waits for that row.
pub async fn lock_status_rows(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: Option<&str>,
) -> Result<()> {
    let tenant = uuid::Uuid::parse_str(tenant_id)?;
    let event = uuid::Uuid::parse_str(election_event_id)?;
    let election = election_id.map(uuid::Uuid::parse_str).transpose()?;
    hasura_transaction
        .query(
            "SELECT id FROM sequent_backend.election_event
             WHERE tenant_id = $1 AND id = $2 FOR NO KEY UPDATE",
            &[&tenant, &event],
        )
        .await
        .context("Error locking the election event")?;
    hasura_transaction
        .query(
            "SELECT id FROM sequent_backend.election
             WHERE tenant_id = $1 AND election_event_id = $2 AND ($3::uuid IS NULL OR id = $3)
             ORDER BY id
             FOR NO KEY UPDATE",
            &[&tenant, &event, &election],
        )
        .await
        .context("Error locking the elections")?;
    Ok(())
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
    seal_policy: BallotBoxSealPolicy,
) -> Result<Vec<String>> {
    if channel == VotingStatusChannel::EARLY_VOTING
        && *new_status == VotingStatus::OPEN
        && event_status.status_by_channel(VotingStatusChannel::ONLINE) != VotingStatus::NOT_STARTED
    {
        return Err(VotingTransitionError(
            "It is not allowed to start EARLY_VOTING channel because ONLINE channel was already started in the past.".into(),
        ).into());
    }

    let mut changed = Vec::new();
    for (election_id, election_status) in elections_status.iter_mut() {
        let Some(election_channels) = configured.get(election_id) else {
            continue;
        };
        if *new_status == VotingStatus::CLOSED
            && seal_policy == BallotBoxSealPolicy::SEAL_AT_CLOSE
            && never_opened(election_status, election_channels)
        {
            info!(
                %election_id,
                reason = NEVER_OPENED_REASON,
                "Nothing to close on schedule: the Post never opened, so it stays as it is"
            );
            continue;
        }
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
        if apply_scheduled_channel_with(
            election_status,
            election_channels,
            channel,
            new_status,
            seal_policy,
        ) {
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

#[cfg(test)]
fn apply_scheduled_channel(
    status: &mut ElectionStatus,
    configured: &VotingChannels,
    channel: VotingStatusChannel,
    new_status: &VotingStatus,
) -> bool {
    apply_scheduled_channel_with(
        status,
        configured,
        channel,
        new_status,
        BallotBoxSealPolicy::DO_NOT_SEAL,
    )
}

fn apply_scheduled_channel_with(
    status: &mut ElectionStatus,
    configured: &VotingChannels,
    channel: VotingStatusChannel,
    new_status: &VotingStatus,
    seal_policy: BallotBoxSealPolicy,
) -> bool {
    if channel.channel_from(configured) != Some(true)
        || !scheduled_change_applies(status, configured, channel, new_status, seal_policy)
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

    fn ran(status: &mut ElectionStatus, channel: VotingStatusChannel, value: VotingStatus) {
        status.set_status_by_channel(channel, VotingStatus::OPEN);
        if value != VotingStatus::OPEN {
            status.set_status_by_channel(channel, value);
        }
    }

    fn enabled(online: bool, kiosk: bool, early: bool) -> VotingChannels {
        VotingChannels {
            online: Some(online),
            kiosk: Some(kiosk),
            telephone: None,
            paper: None,
            early_voting: Some(early),
        }
    }

    /// What an event-wide manual change does at a Post with `channels`
    /// (raw `voting_channels`) and `status`.
    fn post_change(
        seal_policy: BallotBoxSealPolicy,
        channels: Option<Value>,
        status: &ElectionStatus,
        sealed: bool,
        channel: VotingStatusChannel,
        new_status: VotingStatus,
    ) -> Result<ManualPostChange> {
        manual_post_change(
            seal_policy,
            "post",
            channels.as_ref(),
            status,
            sealed,
            channel,
            &new_status,
        )
    }

    #[test]
    fn with_seal_at_close_a_manual_start_opens_only_the_channels_a_post_enables() {
        let seal = BallotBoxSealPolicy::SEAL_AT_CLOSE;
        let online_only = Some(serde_json::json!({"online": true, "kiosk": false}));
        let fresh = ElectionStatus::default();
        let open = VotingStatus::OPEN;
        let change = |channels: Option<Value>, channel| {
            post_change(seal, channels, &fresh, false, channel, open.clone()).unwrap()
        };
        assert_eq!(
            change(online_only.clone(), VotingStatusChannel::ONLINE),
            ManualPostChange::Apply
        );
        assert_eq!(
            change(online_only.clone(), VotingStatusChannel::KIOSK),
            ManualPostChange::NotEnabled
        );
        // A NULL `voting_channels` enables ONLINE only, as the cast check
        // reads it (`VotingChannels::default()`).
        assert_eq!(
            change(None, VotingStatusChannel::ONLINE),
            ManualPostChange::Apply
        );
        assert_eq!(
            change(None, VotingStatusChannel::TELEPHONE),
            ManualPostChange::NotEnabled
        );
        // Closing and pausing still apply to every Post.
        for status in [VotingStatus::CLOSED, VotingStatus::PAUSED] {
            assert_eq!(
                post_change(
                    seal,
                    online_only.clone(),
                    &fresh,
                    false,
                    VotingStatusChannel::KIOSK,
                    status
                )
                .unwrap(),
                ManualPostChange::Apply
            );
        }
        // Policy off: as before VOTE-FREEZE.
        assert_eq!(
            post_change(
                BallotBoxSealPolicy::DO_NOT_SEAL,
                online_only,
                &fresh,
                false,
                VotingStatusChannel::KIOSK,
                open.clone()
            )
            .unwrap(),
            ManualPostChange::Apply
        );
    }

    /// R11 S3: the Post's channels are read only for a Start under Seal at
    /// close, so a value that doesn't parse changes nothing elsewhere.
    #[test]
    fn a_malformed_channel_value_only_matters_to_a_seal_at_close_start() {
        let malformed = Some(serde_json::json!({"online": "true"}));
        let fresh = ElectionStatus::default();
        for (policy, new_status) in [
            (BallotBoxSealPolicy::DO_NOT_SEAL, VotingStatus::OPEN),
            (BallotBoxSealPolicy::DO_NOT_SEAL, VotingStatus::CLOSED),
            (BallotBoxSealPolicy::SEAL_AT_CLOSE, VotingStatus::CLOSED),
            (BallotBoxSealPolicy::SEAL_AT_CLOSE, VotingStatus::PAUSED),
        ] {
            assert_eq!(
                post_change(
                    policy,
                    malformed.clone(),
                    &fresh,
                    false,
                    VotingStatusChannel::ONLINE,
                    new_status
                )
                .unwrap(),
                ManualPostChange::Apply
            );
        }
        let error = post_change(
            BallotBoxSealPolicy::SEAL_AT_CLOSE,
            malformed,
            &fresh,
            false,
            VotingStatusChannel::ONLINE,
            VotingStatus::OPEN,
        )
        .unwrap_err();
        assert!(
            error.to_string().contains("voting channels of post"),
            "{error}"
        );
    }

    /// R11 N3: a channel the Post doesn't enable is reported as such, also
    /// when it is closed or the Post has seals; an enabled one stays closed.
    #[test]
    fn not_enabled_comes_before_kept_closed() {
        let seal = BallotBoxSealPolicy::SEAL_AT_CLOSE;
        let online_only = Some(serde_json::json!({"online": true, "kiosk": false}));
        let mut closed = ElectionStatus::default();
        closed.set_status_by_channel(VotingStatusChannel::ONLINE, VotingStatus::OPEN);
        closed.set_status_by_channel(VotingStatusChannel::ONLINE, VotingStatus::CLOSED);
        closed.set_status_by_channel(VotingStatusChannel::KIOSK, VotingStatus::CLOSED);
        for (sealed, channel, expected) in [
            (
                false,
                VotingStatusChannel::KIOSK,
                ManualPostChange::NotEnabled,
            ),
            (
                true,
                VotingStatusChannel::KIOSK,
                ManualPostChange::NotEnabled,
            ),
            (
                false,
                VotingStatusChannel::ONLINE,
                ManualPostChange::KeptClosed,
            ),
        ] {
            assert_eq!(
                post_change(
                    seal,
                    online_only.clone(),
                    &closed,
                    sealed,
                    channel,
                    VotingStatus::OPEN
                )
                .unwrap(),
                expected
            );
        }
        // Sealed, the enabled channel stays closed even if it isn't CLOSED.
        assert_eq!(
            post_change(
                seal,
                online_only,
                &ElectionStatus::default(),
                true,
                VotingStatusChannel::ONLINE,
                VotingStatus::OPEN
            )
            .unwrap(),
            ManualPostChange::KeptClosed
        );
    }

    #[test]
    fn a_scheduled_close_closes_a_never_started_channel_only_when_the_post_ran() {
        let seal = BallotBoxSealPolicy::SEAL_AT_CLOSE;
        let close = VotingStatus::CLOSED;
        let online_kiosk = enabled(true, true, false);
        // Never opened: left alone, also with Seal at close.
        let never_opened = ElectionStatus::default();
        assert!(!scheduled_change_applies(
            &never_opened,
            &online_kiosk,
            VotingStatusChannel::ONLINE,
            &close,
            seal
        ));
        // ONLINE ran: a named never-started KIOSK closes with Seal at close.
        let mut online_ran = ElectionStatus::default();
        ran(
            &mut online_ran,
            VotingStatusChannel::ONLINE,
            VotingStatus::OPEN,
        );
        assert!(scheduled_change_applies(
            &online_ran,
            &online_kiosk,
            VotingStatusChannel::KIOSK,
            &close,
            seal
        ));
        // Not a KIOSK the Post doesn't enable (R8 B1).
        assert!(!scheduled_change_applies(
            &online_ran,
            &enabled(true, false, false),
            VotingStatusChannel::KIOSK,
            &close,
            seal
        ));
        // Not with the policy off, and never on an opening.
        assert!(!scheduled_change_applies(
            &online_ran,
            &online_kiosk,
            VotingStatusChannel::KIOSK,
            &close,
            BallotBoxSealPolicy::DO_NOT_SEAL
        ));
        let mut paused_kiosk = online_ran.clone();
        ran(
            &mut paused_kiosk,
            VotingStatusChannel::KIOSK,
            VotingStatus::PAUSED,
        );
        assert!(scheduled_change_applies(
            &paused_kiosk,
            &online_kiosk,
            VotingStatusChannel::KIOSK,
            &close,
            BallotBoxSealPolicy::DO_NOT_SEAL
        ));
        let mut closed = ElectionStatus::default();
        ran(
            &mut closed,
            VotingStatusChannel::ONLINE,
            VotingStatus::CLOSED,
        );
        assert!(!scheduled_change_applies(
            &closed,
            &online_kiosk,
            VotingStatusChannel::ONLINE,
            &VotingStatus::OPEN,
            seal
        ));
    }

    #[test]
    fn only_enabled_channels_count_as_ran() {
        let seal = BallotBoxSealPolicy::SEAL_AT_CLOSE;
        // A KIOSK-only Post whose ONLINE was set by an event-wide Start: it
        // never opened.
        let kiosk_only = enabled(false, true, false);
        let mut status = ElectionStatus::default();
        ran(&mut status, VotingStatusChannel::ONLINE, VotingStatus::OPEN);
        assert!(never_opened(&status, &kiosk_only));
        assert!(!scheduled_change_applies(
            &status,
            &kiosk_only,
            VotingStatusChannel::KIOSK,
            &VotingStatus::CLOSED,
            seal
        ));
        // An early-voting Post whose ONLINE isn't enabled: a never-started
        // ONLINE isn't closed (which would close the running early voting).
        let early_only = enabled(false, false, true);
        let mut status = ElectionStatus::default();
        ran(
            &mut status,
            VotingStatusChannel::EARLY_VOTING,
            VotingStatus::OPEN,
        );
        assert!(!scheduled_change_applies(
            &status,
            &early_only,
            VotingStatusChannel::ONLINE,
            &VotingStatus::CLOSED,
            seal
        ));
    }
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
            BallotBoxSealPolicy::DO_NOT_SEAL,
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
            BallotBoxSealPolicy::DO_NOT_SEAL,
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
            BallotBoxSealPolicy::DO_NOT_SEAL,
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
            BallotBoxSealPolicy::DO_NOT_SEAL,
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
