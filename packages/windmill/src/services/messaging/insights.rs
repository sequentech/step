// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! What the Admin Portal shows about messaging, computed the way sending
//! decides it:
//!
//! - the audience preview of Send Notification: per channel, the voters a
//!   notice would reach and the account it sends from;
//! - the event tab's counts: messages per state, from the ledger, each
//!   message once by its latest attempt; and, apart, voters per channel;
//! - monthly volumes for invoice reconciliation;
//! - the support reference a voter is given while their enrollment is
//!   reviewed (a port of Keycloak's `SupportReference`).
//!
//! A voter's channel is read with [`voter_channel`]'s rule, from the event
//! realm's users as the bulk send lists them. Counting a realm takes time
//! (about 20 s a million voters), so voters are counted by a recount task
//! into `messaging_voter_count` and read from there; only an explicit
//! selection of voters is counted in the request, with a time limit.

use crate::postgres::area::get_elections_by_area;
use crate::postgres::messaging::{count_event_messages, LedgerCount, MessagingAccount};
use crate::postgres::messaging_insights::{
    count_voter_channels, event_area_ids, finish_recount, is_statement_timeout, recount_run,
    replace_voter_counts, set_statement_timeout, start_recount, voter_counts, CachedVoterCount,
    MonthlyVolume, RecountOutcome, RecountRun, VoterChannelCount, VoterCountScope, Votes,
};
use crate::services::ballot_box::get_voters_who_participated;
use crate::services::external::utils::is_datafix_election_event_by_id;
use crate::services::messaging::config::get_event_messaging_config;
use crate::services::messaging::dispatch::{
    account_for, eligible_channels, ConfiguredAccountMissing,
};
use anyhow::{Context, Result};
use chrono::{DateTime, Datelike, Duration, NaiveDate, TimeZone, Utc};
use deadpool_postgres::Transaction;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::types::keycloak::User;
use sequent_core::types::messaging::{
    EventMessagingConfig, MessageAttemptState, MessageChannel, MessagePurpose, MessageReason,
    UNRESOLVED_AFTER_HOURS,
};
use sequent_core::types::templates::AudienceSelection;
use sequent_core::types::voter_channel::{voter_channel, CHANNEL_ORDER};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};
use std::time::Duration as StdDuration;
use strum_macros::{Display, EnumString};
use tracing::{info, instrument, warn};
use uuid::Uuid;

/// Who sends a channel's messages for an event, as sending picks it.
#[allow(non_camel_case_types)]
#[derive(Display, EnumString, Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Copy)]
pub enum SendsFrom {
    /// A messaging account: the event's choice, else the tenant's default.
    ACCOUNT,
    /// No account: email and SMS go out through the deployment's own sender.
    DEPLOYMENT,
    /// No account, and the channel has no deployment sender: nothing is sent
    /// ("No account for this channel").
    NONE,
    /// The account the event names was deleted: the send refuses its
    /// notices (`NO_ACCOUNT_FOR_CHANNEL`), never sending them through the
    /// deployment's own email or SMS sender.
    ACCOUNT_MISSING,
}

/// Who would send a channel's notices, as `dispatch::account_for` picks it.
#[derive(Debug, Clone)]
pub enum ChannelAccount {
    /// An account: the event's choice, else the tenant's default.
    Account(MessagingAccount),
    /// The account the event names no longer exists.
    Missing { account_id: String },
    /// Neither: email and SMS use the deployment's sender.
    Unset,
}

/// The account `channel` sends from, as the send picks it
/// (`dispatch::account_for`): the event's choice when the event configures
/// the channel, otherwise the tenant's default account of the channel.
pub async fn account_for_channel(
    tx: &Transaction<'_>,
    tenant_id: &Uuid,
    config: Option<&EventMessagingConfig>,
    channel: MessageChannel,
) -> Result<Option<MessagingAccount>> {
    account_for(tx, tenant_id, config, channel).await
}

/// The sender of each of `channels`, as the send picks it. A configured
/// account that was deleted is [`ChannelAccount::Missing`], as the send
/// refuses it.
pub async fn accounts_for_channels(
    tx: &Transaction<'_>,
    tenant_id: &Uuid,
    config: Option<&EventMessagingConfig>,
    channels: &[MessageChannel],
) -> Result<BTreeMap<MessageChannel, ChannelAccount>> {
    let mut accounts = BTreeMap::new();
    for channel in channels {
        let account = match account_for_channel(tx, tenant_id, config, *channel).await {
            Ok(Some(account)) => ChannelAccount::Account(account),
            Ok(None) => ChannelAccount::Unset,
            Err(error) => match error.downcast_ref::<ConfiguredAccountMissing>() {
                Some(missing) => ChannelAccount::Missing {
                    account_id: missing.account_id.clone(),
                },
                None => return Err(error),
            },
        };
        accounts.insert(*channel, account);
    }
    Ok(accounts)
}

/// What [`account_for_channel`]'s answer means for sending: email and SMS
/// without an account use the deployment's sender (`EnvironmentSender`).
pub fn sends_from(account: Option<&MessagingAccount>, channel: MessageChannel) -> SendsFrom {
    match account {
        Some(_) => SendsFrom::ACCOUNT,
        None if matches!(channel, MessageChannel::EMAIL | MessageChannel::SMS) => {
            SendsFrom::DEPLOYMENT
        }
        None => SendsFrom::NONE,
    }
}

/// Voters on one channel.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ChannelTally {
    pub voters: i64,
    /// Of them, voters whose Post (or the event) doesn't offer the channel
    /// for notices: their notice is refused with
    /// `MessageReason::channel_not_offered`, never sent elsewhere.
    pub not_offered: i64,
}

/// Voters by their one channel.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VoterTally {
    pub total: i64,
    pub channels: BTreeMap<MessageChannel, ChannelTally>,
    /// Voters with no channel on record ("No contact channel").
    pub no_channel: i64,
}

impl VoterTally {
    /// Counts one voter: `channel` is their one channel, `offered` whether
    /// a notice may be sent on it to them.
    pub fn add(&mut self, channel: Option<MessageChannel>, offered: bool) {
        self.add_many(channel, offered, 1);
    }

    /// Counts `voters` voters on `channel`, all offered it or all not.
    pub fn add_many(&mut self, channel: Option<MessageChannel>, offered: bool, voters: i64) {
        self.total += voters;
        match channel {
            Some(channel) => {
                let tally = self.channels.entry(channel).or_default();
                tally.voters += voters;
                if !offered {
                    tally.not_offered += voters;
                }
            }
            None => self.no_channel += voters,
        }
    }

    /// Whether `channel` may carry a notice to a voter of `area_id`: the
    /// event's per-Post restrictions on the elections of their area.
    fn offered(
        channel: Option<MessageChannel>,
        area_id: Option<&str>,
        config: Option<&EventMessagingConfig>,
        elections_by_area: &HashMap<String, Vec<String>>,
    ) -> bool {
        let election_ids = area_id
            .and_then(|area_id| elections_by_area.get(area_id))
            .cloned()
            .unwrap_or_default();
        channel.is_some_and(|channel| {
            eligible_channels(config, MessagePurpose::NOTICE, &election_ids).contains(&channel)
        })
    }

    pub fn voters(&self, channel: Option<MessageChannel>) -> i64 {
        match channel {
            Some(channel) => self.channels.get(&channel).map_or(0, |t| t.voters),
            None => self.no_channel,
        }
    }

    /// Counts a voter as the bulk send reads them: their one channel
    /// ([`voter_channel`]) and the elections of their area, which the
    /// event's per-Post channel restrictions apply to. The reference the
    /// SQL count ([`count_event_voters`]) is held equal to.
    pub fn add_user(
        &mut self,
        user: &User,
        config: Option<&EventMessagingConfig>,
        elections_by_area: &HashMap<String, Vec<String>>,
    ) {
        let channel = voter_channel(user.email.as_deref(), user.attributes.as_ref())
            .map(|found| found.channel);
        let area_id = user.area.as_ref().and_then(|area| area.id.as_deref());
        let offered = Self::offered(channel, area_id, config, elections_by_area);
        self.add(channel, offered);
    }
}

/// Which voters of the event a send reaches, as `SendTemplateBody` names
/// them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Audience {
    All,
    Voted,
    NotVoted,
    Selected(Vec<String>),
}

impl Audience {
    /// The audience a send would have. Selected voters must name at least
    /// one voter, as when the send is scheduled.
    pub fn of(
        selection: Option<AudienceSelection>,
        voter_ids: Option<Vec<String>>,
    ) -> std::result::Result<Self, MessageReason> {
        Ok(match selection.unwrap_or(AudienceSelection::ALL_USERS) {
            AudienceSelection::ALL_USERS => Audience::All,
            AudienceSelection::VOTED => Audience::Voted,
            AudienceSelection::NOT_VOTED => Audience::NotVoted,
            AudienceSelection::SELECTED => {
                let ids: Vec<String> = voter_ids
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|id| !id.trim().is_empty())
                    .collect();
                if ids.is_empty() {
                    return Err(MessageReason::SELECTED_WITHOUT_VOTERS);
                }
                Audience::Selected(ids)
            }
        })
    }
}

/// How long a recount may count an event's voters: a background task, so
/// generous. `MESSAGING_VOTER_RECOUNT_TIMEOUT_SECS` overrides it.
pub const RECOUNT_TIMEOUT: StdDuration = StdDuration::from_secs(600);
/// How long counting a send's selected voters may take in a request:
/// under Hasura's 30 s action timeout.
pub const SELECTED_COUNT_TIMEOUT: StdDuration = StdDuration::from_secs(20);
/// How long a requested recount counts as pending: a recount asked for
/// earlier that never finished (its task lost) may be asked for again.
pub const RECOUNT_PENDING_FOR: Duration = Duration::minutes(30);

/// The recount time limit this process uses.
pub fn recount_timeout() -> StdDuration {
    std::env::var("MESSAGING_VOTER_RECOUNT_TIMEOUT_SECS")
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|seconds| *seconds > 0)
        .map(StdDuration::from_secs)
        .unwrap_or(RECOUNT_TIMEOUT)
}

/// The cached rows of an aggregate: per scope, area and channel, the
/// voters and those whose Post doesn't offer their channel for notices
/// (the event's per-Post restrictions on the elections of their area).
pub fn voter_count_rows(
    counts: &[VoterChannelCount],
    config: Option<&EventMessagingConfig>,
    elections_by_area: &HashMap<String, Vec<String>>,
) -> Vec<CachedVoterCount> {
    let mut rows: BTreeMap<(VoterCountScope, Option<String>, Option<MessageChannel>), (i64, i64)> =
        BTreeMap::new();
    for count in counts {
        let offered = VoterTally::offered(
            count.channel,
            count.area_id.as_deref(),
            config,
            elections_by_area,
        );
        let not_offered = if offered || count.channel.is_none() {
            0
        } else {
            count.voters
        };
        let by_vote = if count.voted {
            VoterCountScope::VOTED
        } else {
            VoterCountScope::NOT_VOTED
        };
        for scope in [VoterCountScope::ALL, by_vote] {
            let row = rows
                .entry((scope, count.area_id.clone(), count.channel))
                .or_default();
            row.0 += count.voters;
            row.1 += not_offered;
        }
    }
    rows.into_iter()
        .map(
            |((scope, area_id, channel), (voters, not_offered))| CachedVoterCount {
                scope,
                area_id,
                channel,
                voters,
                not_offered,
            },
        )
        .collect()
}

/// A scope's counts as a tally.
pub fn tally_of(counts: &[CachedVoterCount], scope: VoterCountScope) -> VoterTally {
    let mut tally = VoterTally::default();
    for count in counts.iter().filter(|count| count.scope == scope) {
        tally.add_many(count.channel, true, count.voters - count.not_offered);
        tally.add_many(count.channel, false, count.not_offered);
    }
    tally
}

/// Counts the event's voters by channel for every scope: the voters the
/// bulk send reaches (`send_template`'s `list_users`, with `votes_info` for
/// the voted filters: a voter voted with a pending or valid ballot in the
/// event's ballot box), counted in one aggregate on the Keycloak database
/// ([`count_voter_channels`], which applies [`voter_channel`]'s rule), with
/// `timeout` on the statement. The recount task stores the result.
#[instrument(skip(hasura_tx, keycloak_tx, config), err)]
pub async fn count_event_voters(
    hasura_tx: &Transaction<'_>,
    keycloak_tx: &Transaction<'_>,
    tenant_id: &Uuid,
    election_event_id: &Uuid,
    config: Option<&EventMessagingConfig>,
    timeout: StdDuration,
) -> Result<Vec<CachedVoterCount>> {
    let tenant = tenant_id.to_string();
    let event = election_event_id.to_string();
    let realm = get_event_realm(&tenant, &event);
    let elections_by_area = get_elections_by_area(hasura_tx, &tenant, &event)
        .await
        .context("listing elections by area")?;
    let area_ids = event_area_ids(hasura_tx, tenant_id, election_event_id).await?;
    let votes = Votes {
        voter_ids: get_voters_who_participated(hasura_tx, &tenant, &event)
            .await
            .context("reading who voted from the event's private participation")?
            .into_iter()
            .collect(),
        datafix: is_datafix_election_event_by_id(hasura_tx, &tenant, &event).await?,
    };
    set_statement_timeout(keycloak_tx, timeout).await?;
    let counts = count_voter_channels(keycloak_tx, &realm, None, &area_ids, &votes).await?;
    let rows = voter_count_rows(&counts, config, &elections_by_area);
    info!(rows = rows.len(), "counted the event's voters by channel");
    Ok(rows)
}

/// Counts a send's selected voters by channel in the request, with
/// `timeout` on the statement. `Ok(None)`: it took longer.
#[instrument(skip(hasura_tx, keycloak_tx, config, voter_ids), fields(selected = voter_ids.len()), err)]
pub async fn count_selected_voters(
    hasura_tx: &Transaction<'_>,
    keycloak_tx: &Transaction<'_>,
    tenant_id: &Uuid,
    election_event_id: &Uuid,
    config: Option<&EventMessagingConfig>,
    voter_ids: &[String],
    timeout: StdDuration,
) -> Result<Option<VoterTally>> {
    let tenant = tenant_id.to_string();
    let event = election_event_id.to_string();
    let realm = get_event_realm(&tenant, &event);
    let elections_by_area = get_elections_by_area(hasura_tx, &tenant, &event)
        .await
        .context("listing elections by area")?;
    let area_ids = event_area_ids(hasura_tx, tenant_id, election_event_id).await?;
    set_statement_timeout(keycloak_tx, timeout).await?;
    let counts = match count_voter_channels(
        keycloak_tx,
        &realm,
        Some(voter_ids),
        &area_ids,
        &Votes::default(),
    )
    .await
    {
        Ok(counts) => counts,
        Err(error) if is_statement_timeout(&error) => return Ok(None),
        Err(error) => return Err(error),
    };
    let rows = voter_count_rows(&counts, config, &elections_by_area);
    Ok(Some(tally_of(&rows, VoterCountScope::ALL)))
}

/// Recounts the event's voters into `messaging_voter_count`: records the
/// start, counts within `timeout` on a read-only Keycloak transaction, and
/// stores the counts with the run's outcome. A count that takes longer or
/// fails keeps the previous counts and records why.
#[instrument(skip(hasura, keycloak), err)]
pub async fn recount_event_voters(
    hasura: &mut deadpool_postgres::Client,
    keycloak: &mut deadpool_postgres::Client,
    tenant_id: &Uuid,
    election_event_id: &Uuid,
    timeout: StdDuration,
) -> Result<RecountOutcome> {
    {
        let tx = hasura.transaction().await?;
        start_recount(&tx, tenant_id, election_event_id, Utc::now()).await?;
        tx.commit().await?;
    }
    let counted = async {
        let hasura_tx = hasura.build_transaction().read_only(true).start().await?;
        let keycloak_tx = keycloak.build_transaction().read_only(true).start().await?;
        let config = get_event_messaging_config(&hasura_tx, tenant_id, election_event_id).await?;
        let rows = count_event_voters(
            &hasura_tx,
            &keycloak_tx,
            tenant_id,
            election_event_id,
            config.as_ref(),
            timeout,
        )
        .await?;
        keycloak_tx.commit().await?;
        hasura_tx.commit().await?;
        anyhow::Ok(rows)
    }
    .await;
    let now = Utc::now();
    let tx = hasura.transaction().await?;
    let outcome = match counted {
        Ok(rows) => {
            replace_voter_counts(&tx, tenant_id, election_event_id, &rows, now).await?;
            finish_recount(
                &tx,
                tenant_id,
                election_event_id,
                now,
                RecountOutcome::COUNTED,
                None,
            )
            .await?;
            RecountOutcome::COUNTED
        }
        Err(error) if is_statement_timeout(&error) => {
            let reason = format!(
                "Counting the voters took longer than {} seconds",
                timeout.as_secs()
            );
            warn!(%reason, "voter recount timed out; the previous counts stay");
            finish_recount(
                &tx,
                tenant_id,
                election_event_id,
                now,
                RecountOutcome::TIMED_OUT,
                Some(&reason),
            )
            .await?;
            RecountOutcome::TIMED_OUT
        }
        Err(error) => {
            warn!("voter recount failed; the previous counts stay: {error:#}");
            finish_recount(
                &tx,
                tenant_id,
                election_event_id,
                now,
                RecountOutcome::FAILED,
                Some("The voters could not be counted"),
            )
            .await?;
            RecountOutcome::FAILED
        }
    };
    tx.commit().await?;
    Ok(outcome)
}

/// Where a count shown comes from, and how fresh it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoterCountStatus {
    /// When the counts shown were counted; `None`: never counted.
    pub computed_at: Option<DateTime<Utc>>,
    /// A recount was asked for and hasn't finished.
    pub pending: bool,
    /// How the last count ended; `TIMED_OUT` or `FAILED` keep the
    /// previous counts.
    pub outcome: Option<RecountOutcome>,
}

impl VoterCountStatus {
    pub fn of(run: Option<&RecountRun>, now: DateTime<Utc>) -> Self {
        VoterCountStatus {
            computed_at: run.and_then(|run| run.computed_at),
            pending: run.is_some_and(|run| run.pending(now, RECOUNT_PENDING_FOR)),
            outcome: run.and_then(|run| run.outcome),
        }
    }
}

/// The event's cached voter counts of `scope`, and their status.
#[instrument(skip(tx), err)]
pub async fn cached_voters(
    tx: &Transaction<'_>,
    tenant_id: &Uuid,
    election_event_id: &Uuid,
    scope: VoterCountScope,
    now: DateTime<Utc>,
) -> Result<(VoterTally, VoterCountStatus)> {
    let counts = voter_counts(tx, tenant_id, election_event_id, scope).await?;
    let run = recount_run(tx, tenant_id, election_event_id).await?;
    Ok((
        tally_of(&counts, scope),
        VoterCountStatus::of(run.as_ref(), now),
    ))
}

impl Audience {
    /// The cached scope of an audience; `None` for a selection, counted in
    /// the request.
    pub fn scope(&self) -> Option<VoterCountScope> {
        match self {
            Audience::All => Some(VoterCountScope::ALL),
            Audience::Voted => Some(VoterCountScope::VOTED),
            Audience::NotVoted => Some(VoterCountScope::NOT_VOTED),
            Audience::Selected(_) => None,
        }
    }

    /// The audience's kind, for logs: never the selected ids.
    pub fn kind(&self) -> &'static str {
        match self {
            Audience::All => "ALL_USERS",
            Audience::Voted => "VOTED",
            Audience::NotVoted => "NOT_VOTED",
            Audience::Selected(_) => "SELECTED",
        }
    }

    pub fn selected(&self) -> usize {
        match self {
            Audience::Selected(ids) => ids.len(),
            _ => 0,
        }
    }
}

/// One row of the audience preview.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudienceRow {
    /// `None`: voters with no contact channel. They get no notice.
    pub channel: Option<MessageChannel>,
    pub voters: i64,
    pub not_offered: i64,
    /// `None`: nothing is sent on the row: voters with no contact channel,
    /// or a channel no voter of the row may get (not offered to any).
    pub sends_from: Option<SendsFrom>,
    pub account_id: Option<Uuid>,
    pub account_name: Option<String>,
}

impl AudienceRow {
    /// Voters of the row a notice is handed to a sender for.
    pub fn will_send(&self) -> i64 {
        match self.sends_from {
            Some(SendsFrom::ACCOUNT) | Some(SendsFrom::DEPLOYMENT) => {
                self.voters - self.not_offered
            }
            _ => 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudiencePreview {
    pub total: i64,
    /// Voters a notice would be handed to a sender for: the total less
    /// those not offered their channel, with no channel, or on a channel
    /// without an account.
    pub will_send: i64,
    pub rows: Vec<AudienceRow>,
    pub status: VoterCountStatus,
}

/// The preview's channels, in channel order: every channel the event
/// offers for notices, and every channel some voter is on even if it isn't
/// offered (those voters' notices are refused, and the row says so).
pub fn preview_channels(
    tally: &VoterTally,
    config: Option<&EventMessagingConfig>,
) -> Vec<MessageChannel> {
    let offered = eligible_channels(config, MessagePurpose::NOTICE, &[]);
    CHANNEL_ORDER
        .into_iter()
        .filter(|channel| offered.contains(channel) || tally.channels.contains_key(channel))
        .collect()
}

/// The preview's rows: [`preview_channels`] with who sends each
/// (`accounts`, as [`accounts_for_channels`] reads them), then the voters
/// with no contact channel.
pub fn audience_rows(
    tally: &VoterTally,
    config: Option<&EventMessagingConfig>,
    accounts: &BTreeMap<MessageChannel, ChannelAccount>,
    status: VoterCountStatus,
) -> AudiencePreview {
    let mut rows: Vec<AudienceRow> = preview_channels(tally, config)
        .into_iter()
        .map(|channel| {
            let counted = tally.channels.get(&channel).copied().unwrap_or_default();
            // Shown only because voters are on it: none of them gets it.
            if counted.voters > 0 && counted.not_offered == counted.voters {
                return AudienceRow {
                    channel: Some(channel),
                    voters: counted.voters,
                    not_offered: counted.not_offered,
                    sends_from: None,
                    account_id: None,
                    account_name: None,
                };
            }
            let (sends, account_id, account_name) = match accounts.get(&channel) {
                Some(ChannelAccount::Account(account)) => (
                    SendsFrom::ACCOUNT,
                    Some(account.id),
                    Some(account.name.clone()),
                ),
                Some(ChannelAccount::Missing { account_id }) => (
                    SendsFrom::ACCOUNT_MISSING,
                    Uuid::parse_str(account_id).ok(),
                    None,
                ),
                Some(ChannelAccount::Unset) | None => (sends_from(None, channel), None, None),
            };
            AudienceRow {
                channel: Some(channel),
                voters: counted.voters,
                not_offered: counted.not_offered,
                sends_from: Some(sends),
                account_id,
                account_name,
            }
        })
        .collect();
    if tally.no_channel > 0 {
        rows.push(AudienceRow {
            channel: None,
            voters: tally.no_channel,
            not_offered: 0,
            sends_from: None,
            account_id: None,
            account_name: None,
        });
    }
    AudiencePreview {
        total: tally.total,
        will_send: rows.iter().map(AudienceRow::will_send).sum(),
        rows,
        status,
    }
}

/// The audience preview of a send: its voters by channel and who sends
/// each channel, as the send would pick them. The event's audiences are
/// read from the cached counts; a selection is counted here, within
/// [`SELECTED_COUNT_TIMEOUT`] (`outcome` `TIMED_OUT` and no rows when it
/// takes longer).
#[instrument(
    skip(hasura_tx, keycloak_tx, config, audience),
    fields(audience = audience.kind(), selected = audience.selected()),
    err
)]
pub async fn audience_preview(
    hasura_tx: &Transaction<'_>,
    keycloak_tx: &Transaction<'_>,
    tenant_id: &Uuid,
    election_event_id: &Uuid,
    config: Option<&EventMessagingConfig>,
    audience: &Audience,
    now: DateTime<Utc>,
) -> Result<AudiencePreview> {
    let (tally, status) = match (audience, audience.scope()) {
        (_, Some(scope)) => {
            cached_voters(hasura_tx, tenant_id, election_event_id, scope, now).await?
        }
        (Audience::Selected(ids), None) => {
            match count_selected_voters(
                hasura_tx,
                keycloak_tx,
                tenant_id,
                election_event_id,
                config,
                ids,
                SELECTED_COUNT_TIMEOUT,
            )
            .await?
            {
                Some(tally) => (
                    tally,
                    VoterCountStatus {
                        computed_at: Some(now),
                        pending: false,
                        outcome: Some(RecountOutcome::COUNTED),
                    },
                ),
                None => {
                    return Ok(AudiencePreview {
                        total: 0,
                        will_send: 0,
                        rows: vec![],
                        status: VoterCountStatus {
                            computed_at: None,
                            pending: false,
                            outcome: Some(RecountOutcome::TIMED_OUT),
                        },
                    })
                }
            }
        }
        (_, None) => unreachable!("only a selection has no cached scope"),
    };
    let channels = preview_channels(&tally, config);
    let accounts = accounts_for_channels(hasura_tx, tenant_id, config, &channels).await?;
    Ok(audience_rows(&tally, config, &accounts, status))
}

/// One channel's row of the event tab.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChannelStats {
    /// `None`: messages of voters with no contact channel.
    pub channel: Option<MessageChannel>,
    /// Messages with an attempt in the last 24 hours, in any state.
    pub sent_last_24h: i64,
    pub queued: i64,
    pub accepted: i64,
    pub delivered: i64,
    pub failed: i64,
    /// Unknown for less than [`UNRESOLVED_AFTER_HOURS`].
    pub unknown: i64,
    /// Unknown for longer: delivery couldn't be confirmed. Never failed.
    pub unresolved: i64,
}

/// The event tab's message rows from the ledger counts (each message once,
/// by its latest attempt: `count_event_messages`). The Failed count selects
/// the same messages as the Logs filter (`latest_attempt_ids` with outcome
/// FAILED). Rows in channel order, then the no-channel row; a channel with
/// no message has none. Voters are counted apart ([`cached_voters`]).
pub fn event_stats(all: &[LedgerCount], last_day: &[LedgerCount]) -> Vec<ChannelStats> {
    // Keyed by channel order; the no-channel row (`None`) is moved last.
    let mut rows: BTreeMap<Option<usize>, ChannelStats> = BTreeMap::new();
    fn row(
        rows: &mut BTreeMap<Option<usize>, ChannelStats>,
        channel: Option<MessageChannel>,
    ) -> &mut ChannelStats {
        let key = channel.map(|channel| {
            CHANNEL_ORDER
                .iter()
                .position(|c| *c == channel)
                .unwrap_or(CHANNEL_ORDER.len())
        });
        rows.entry(key).or_insert_with(|| ChannelStats {
            channel,
            ..Default::default()
        })
    }
    for count in all {
        let stats = row(&mut rows, count.channel);
        match count.outcome.as_str() {
            "UNRESOLVED" => stats.unresolved += count.messages,
            outcome => match outcome.parse::<MessageAttemptState>() {
                Ok(MessageAttemptState::QUEUED) => stats.queued += count.messages,
                Ok(MessageAttemptState::ACCEPTED) => stats.accepted += count.messages,
                Ok(MessageAttemptState::DELIVERED) => stats.delivered += count.messages,
                Ok(MessageAttemptState::FAILED) => stats.failed += count.messages,
                Ok(MessageAttemptState::UNKNOWN) => stats.unknown += count.messages,
                _ => {}
            },
        }
    }
    for count in last_day {
        row(&mut rows, count.channel).sent_last_24h += count.messages;
    }
    let mut ordered: Vec<ChannelStats> = rows.into_values().collect();
    if let Some(index) = ordered.iter().position(|stats| stats.channel.is_none()) {
        let no_channel = ordered.remove(index);
        ordered.push(no_channel);
    }
    ordered
}

/// The event tab's message counts at `now`, from the ledger.
#[instrument(skip(hasura_tx), err)]
pub async fn messaging_event_stats(
    hasura_tx: &Transaction<'_>,
    tenant_id: &Uuid,
    election_event_id: &Uuid,
    now: DateTime<Utc>,
) -> Result<Vec<ChannelStats>> {
    let unresolved_before = now - Duration::hours(UNRESOLVED_AFTER_HOURS);
    let all = count_event_messages(
        hasura_tx,
        tenant_id,
        election_event_id,
        None,
        unresolved_before,
    )
    .await?;
    let last_day = count_event_messages(
        hasura_tx,
        tenant_id,
        election_event_id,
        Some(now - Duration::hours(24)),
        unresolved_before,
    )
    .await?;
    Ok(event_stats(&all, &last_day))
}

/// A month as `YYYY-MM`, read as the UTC instants `[start, end)`.
pub fn month_range(month: &str) -> Option<(DateTime<Utc>, DateTime<Utc>)> {
    let (year, month) = month.trim().split_once('-')?;
    if year.len() != 4 || month.len() != 2 {
        return None;
    }
    let start = NaiveDate::from_ymd_opt(year.parse().ok()?, month.parse().ok()?, 1)?;
    let next = if start.month() == 12 {
        NaiveDate::from_ymd_opt(start.year() + 1, 1, 1)?
    } else {
        NaiveDate::from_ymd_opt(start.year(), start.month() + 1, 1)?
    };
    Some((
        Utc.from_utc_datetime(&start.and_hms_opt(0, 0, 0)?),
        Utc.from_utc_datetime(&next.and_hms_opt(0, 0, 0)?),
    ))
}

/// Volumes in a stable order: channel order, then account, country,
/// billing category and billable.
pub fn sort_volumes(volumes: &mut [MonthlyVolume]) {
    let position = |channel: MessageChannel| CHANNEL_ORDER.iter().position(|c| *c == channel);
    volumes.sort_by(|a, b| {
        (
            position(a.channel),
            &a.account_name,
            a.account_id,
            &a.destination_country,
            &a.billing_category,
            a.billable,
        )
            .cmp(&(
                position(b.channel),
                &b.account_name,
                b.account_id,
                &b.destination_country,
                &b.billing_category,
                b.billable,
            ))
    });
}

/// Crockford base32, as Keycloak's `SupportReference` writes it.
const SUPPORT_REFERENCE_ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// The first 40 bits of SHA-256 of `"application:" + id`, the id in lower
/// case: what the support reference encodes.
fn support_reference_bits(application_id: &Uuid) -> i64 {
    let digest = Sha256::digest(format!("application:{application_id}").as_bytes());
    digest[..5]
        .iter()
        .fold(0i64, |bits, byte| (bits << 8) | i64::from(*byte))
}

/// The support reference of an application, such as `65SR-MSSD`. One way:
/// staff find the application by it, nobody gets the id back from it.
pub fn support_reference(application_id: &Uuid) -> String {
    support_reference_text(support_reference_bits(application_id))
}

/// A reference's 40 bits as written: two groups of four characters.
pub fn support_reference_text(bits: i64) -> String {
    let mut reference = String::with_capacity(9);
    for i in (0..8).rev() {
        let index = ((bits >> (i * 5)) & 31) as usize;
        reference.push(char::from(SUPPORT_REFERENCE_ALPHABET[index]));
        if i == 4 {
            reference.push('-');
        }
    }
    reference
}

/// A text read as a support reference the way Crockford base32 reads it
/// (any case, with or without the dash or spaces, I and L as 1, O as 0),
/// as its 40 bits. `None` when the text isn't one.
pub fn parse_support_reference(text: &str) -> Option<i64> {
    let compact: Vec<u8> = text
        .trim()
        .to_uppercase()
        .bytes()
        .filter(|byte| !matches!(byte, b'-' | b' ' | b'\t'))
        .map(|byte| match byte {
            b'I' | b'L' => b'1',
            b'O' => b'0',
            other => other,
        })
        .collect();
    if compact.len() != 8 {
        return None;
    }
    compact.iter().try_fold(0i64, |bits, byte| {
        let value = SUPPORT_REFERENCE_ALPHABET.iter().position(|c| c == byte)?;
        Some((bits << 5) | value as i64)
    })
}

/// The reason given when a month isn't `YYYY-MM`.
pub const INVALID_MONTH: &str = "The month must be written as YYYY-MM";

/// The reason given when a search isn't a support reference.
pub const INVALID_SUPPORT_REFERENCE: &str = "Not a support reference";

#[cfg(test)]
#[path = "insights_tests.rs"]
mod insights_tests;
