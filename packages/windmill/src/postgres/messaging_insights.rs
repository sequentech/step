// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Queries behind the messaging insights: monthly volumes from the message
//! ledger, finding an application by the support reference its voter was
//! given, counting an event realm's voters by their one channel in a single
//! aggregate on the Keycloak database, and the per-event cache of those
//! counts (`messaging_voter_count`) with its recount runs.

use crate::services::cast_votes::CastVoteStatus;
use anyhow::{anyhow, Result};
use chrono::{DateTime, Duration, Utc};
use deadpool_postgres::Transaction;
use sequent_core::types::keycloak::{
    AREA_ID_ATTR_NAME, ATTR_RESET_VALUE, MESSAGE_CHANNEL_ATTR_NAME, MESSAGE_NUMBER_ATTR_NAME,
    MESSENGER_ID_ATTR_NAME, MESSENGER_PAGE_ATTR_NAME, MOBILE_PHONE_ATTR_NAME,
    VERIFIED_CHANNELS_ATTR_NAME, VIBER_NUMBER_ATTR_NAME, VOTED_CHANNEL,
    VOTED_CHANNEL_INTERNET_VALUE, WHATSAPP_NUMBER_ATTR_NAME,
};
use sequent_core::types::messaging::{MessageAttemptState, MessageChannel};
use sequent_core::types::voter_channel::CHANNEL_ORDER;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use strum::IntoEnumIterator;
use strum_macros::{Display, EnumString};
use tokio_postgres::error::SqlState;
use tracing::instrument;
use uuid::Uuid;

/// Attempts of one month that share a channel, account, destination
/// country, billing category and billable flag. Each attempt handed to a
/// provider counts once, in the month it was handed over, as providers bill
/// each submission: a retry counts again, in its own month.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonthlyVolume {
    pub channel: MessageChannel,
    /// `None`: the deployment's own email or SMS sender, or an account
    /// deleted since (the ledger keeps no account once it's deleted).
    pub account_id: Option<Uuid>,
    /// The account's name now.
    pub account_name: Option<String>,
    /// ISO 3166-1 alpha-2 for phone numbers; `None` for email and Messenger.
    pub destination_country: Option<String>,
    /// The category expected when sending, or the one the provider reported.
    pub billing_category: Option<String>,
    /// What the provider reported; `None` when it reported nothing.
    pub billable: Option<bool>,
    /// Attempts handed to the provider.
    pub messages: i64,
}

/// The tenant's attempts handed to a provider in `[from, to)`, optionally
/// for one election event. Handed over: `dispatched_at` is set, or, for
/// rows from before `dispatched_at` existed, the provider gave an id or
/// accepted or delivered it. Refused attempts (channel not offered, no
/// account, capacity busy, a code expired before sending, a stale QUEUED
/// the sweep failed) never reached a provider and aren't counted.
#[instrument(skip(tx), err)]
pub async fn monthly_volumes(
    tx: &Transaction<'_>,
    tenant_id: &Uuid,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
    election_event_id: Option<&Uuid>,
) -> Result<Vec<MonthlyVolume>> {
    let accepted = MessageAttemptState::ACCEPTED.to_string();
    let delivered = MessageAttemptState::DELIVERED.to_string();
    tx.query(
        r#"
        SELECT m.channel, m.account_id, account.name AS account_name,
               m.destination_country, m.billing_category, m.billable,
               count(*) AS messages
        FROM sequent_backend.message AS m
        LEFT JOIN sequent_backend.messaging_account AS account
            ON account.id = m.account_id AND account.tenant_id = $1
        WHERE m.tenant_id = $1 AND m.direction = 'OUTBOUND' AND m.channel IS NOT NULL
          AND (m.dispatched_at IS NOT NULL OR m.provider_message_id IS NOT NULL
               OR m.state IN ($5, $6))
          AND COALESCE(m.dispatched_at, m.created_at) >= $2
          AND COALESCE(m.dispatched_at, m.created_at) < $3
          AND ($4::uuid IS NULL OR m.election_event_id = $4)
        GROUP BY m.channel, m.account_id, account.name, m.destination_country,
                 m.billing_category, m.billable
        "#,
        &[
            tenant_id,
            &from,
            &to,
            &election_event_id,
            &accepted,
            &delivered,
        ],
    )
    .await?
    .into_iter()
    .map(|row| {
        let channel: String = row.try_get("channel")?;
        Ok(MonthlyVolume {
            channel: MessageChannel::from_str(&channel)
                .map_err(|_| anyhow!("unexpected channel {channel}"))?,
            account_id: row.try_get("account_id")?,
            account_name: row.try_get("account_name")?,
            destination_country: row.try_get("destination_country")?,
            billing_category: row.try_get("billing_category")?,
            billable: row.try_get("billable")?,
            messages: row.try_get("messages")?,
        })
    })
    .collect()
}

/// The applications of an election event whose support reference is
/// `reference` (its 40 bits, [`crate::services::messaging::insights::parse_support_reference`]).
/// The reference is computed in the database the same way Keycloak's
/// `SupportReference` derives it from the id, so no reference is stored.
/// `permission_label`: only applications with that label, as the queue
/// shows them.
#[instrument(skip(tx), err)]
pub async fn find_applications_by_support_reference(
    tx: &Transaction<'_>,
    tenant_id: &Uuid,
    election_event_id: &Uuid,
    reference: i64,
    permission_label: Option<&str>,
) -> Result<Vec<Uuid>> {
    tx.query(
        r#"
        SELECT id FROM sequent_backend.applications
        WHERE tenant_id = $1 AND election_event_id = $2
          AND ($4::text IS NULL OR permission_label = $4)
          AND ('x' || encode(substring(
                  sha256(convert_to('application:' || lower(id::text), 'UTF8')) FROM 1 FOR 5
              ), 'hex'))::bit(40)::bigint = $3
        ORDER BY id
        "#,
        &[tenant_id, election_event_id, &reference, &permission_label],
    )
    .await?
    .into_iter()
    .map(|row| Ok(row.try_get("id")?))
    .collect()
}

/// The ids of the event's areas: the voters `list_users` lists for the event
/// are those of these areas, and those with no area.
#[instrument(skip(tx), err)]
pub async fn event_area_ids(
    tx: &Transaction<'_>,
    tenant_id: &Uuid,
    election_event_id: &Uuid,
) -> Result<Vec<String>> {
    tx.query(
        "SELECT id::text AS id FROM sequent_backend.area
         WHERE tenant_id = $1 AND election_event_id = $2",
        &[tenant_id, election_event_id],
    )
    .await?
    .into_iter()
    .map(|row| Ok(row.try_get("id")?))
    .collect()
}

/// The voters of the event with a ballot that wasn't discarded, as
/// `get_users_with_vote_info` decides "has voted" (any election of the
/// event).
#[instrument(skip(tx), err)]
pub async fn event_voter_ids_who_voted(
    tx: &Transaction<'_>,
    tenant_id: &Uuid,
    election_event_id: &Uuid,
) -> Result<Vec<String>> {
    tx.query(
        "SELECT DISTINCT voter_id_string FROM sequent_backend.cast_vote
         WHERE tenant_id = $1 AND election_event_id = $2
           AND voter_id_string IS NOT NULL AND status <> $3",
        &[
            tenant_id,
            election_event_id,
            &CastVoteStatus::Discarded.to_string(),
        ],
    )
    .await?
    .into_iter()
    .map(|row| Ok(row.try_get("voter_id_string")?))
    .collect()
}

/// What the count reads of votes: the ids of voters with a ballot, and
/// whether the event is a Datafix event (whose `voted-channel` records
/// votes outside the internet channel). Empty: every voter counts as not
/// voted, for counts that don't split by vote.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Votes {
    pub voter_ids: Vec<String>,
    pub datafix: bool,
}

/// Voters of one area on one channel, who voted or didn't.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoterChannelCount {
    /// The voter's `area-id`; `None` for voters with no area.
    pub area_id: Option<String>,
    /// The voter's one channel; `None`: no contact channel.
    pub channel: Option<MessageChannel>,
    pub voted: bool,
    pub voters: i64,
}

fn sql_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

/// When a first-release record reaches `channel`: `voter_channel`'s
/// `verified_destination`.
fn verified_destination_sql(channel: MessageChannel) -> &'static str {
    match channel {
        MessageChannel::WHATSAPP => "whatsapp_number IS NOT NULL",
        MessageChannel::VIBER => "viber_number IS NOT NULL",
        MessageChannel::SMS => "mobile_number IS NOT NULL",
        MessageChannel::MESSENGER => "messenger_id IS NOT NULL AND messenger_page IS NOT NULL",
        MessageChannel::EMAIL => "email IS NOT NULL",
    }
}

/// When a recorded record reaches `channel`: `voter_channel`'s
/// `recorded_destination`.
fn recorded_destination_sql(channel: MessageChannel) -> &'static str {
    match channel {
        MessageChannel::WHATSAPP | MessageChannel::VIBER => "message_number IS NOT NULL",
        MessageChannel::SMS => "(message_number IS NOT NULL OR mobile_number IS NOT NULL)",
        MessageChannel::MESSENGER => "messenger_id IS NOT NULL AND messenger_page IS NOT NULL",
        MessageChannel::EMAIL => "email IS NOT NULL",
    }
}

/// `sequent_core::types::voter_channel::voter_channel` as a SQL expression
/// over the `records` columns. Built from the same per-channel rules, in
/// the same order; `voter_channel` stays the rule's source of truth and the
/// database test holds the two equal on every case of its tests.
fn voter_channel_sql() -> String {
    let lit = |channel: MessageChannel| sql_literal(&channel.to_string());
    let preferred = CHANNEL_ORDER
        .into_iter()
        .map(|channel| {
            format!(
                "(recorded = {} AND {})",
                lit(channel),
                verified_destination_sql(channel)
            )
        })
        .collect::<Vec<_>>()
        .join(" OR ");
    let in_order = CHANNEL_ORDER
        .into_iter()
        .map(|channel| {
            format!(
                "WHEN {c} = ANY(verified) AND {} THEN {c}",
                verified_destination_sql(channel),
                c = lit(channel)
            )
        })
        .collect::<Vec<_>>()
        .join("\n                ");
    let recorded = CHANNEL_ORDER
        .into_iter()
        .map(|channel| {
            format!(
                "WHEN recorded = {} AND {} THEN recorded",
                lit(channel),
                recorded_destination_sql(channel)
            )
        })
        .collect::<Vec<_>>()
        .join("\n                ");
    format!(
        r#"CASE
            WHEN has_verified THEN CASE
                WHEN recorded = ANY(verified) AND ({preferred}) THEN recorded
                {in_order}
            END
            WHEN recorded IS NOT NULL THEN CASE
                {recorded}
            END
            WHEN email IS NOT NULL THEN {email}
            WHEN mobile_number IS NOT NULL THEN {sms}
        END"#,
        email = lit(MessageChannel::EMAIL),
        sms = lit(MessageChannel::SMS),
    )
}

/// A channel value as `voter_channel::parse_channel_value` reads it:
/// trimmed and in any case.
fn channel_value_sql(value: &str) -> String {
    format!("upper(btrim({value}))")
}

/// The statement behind [`count_voter_channels`]. Its parameters: `$1`
/// realm, `$2` the selected voter ids or `NULL`, `$3` the event's area ids,
/// `$4` the ids of voters who voted, `$5` whether the event is a Datafix
/// event.
///
/// The attributes are read in one grouped pass over `user_attribute` (a
/// hash join on the scoped users), not one lookup per voter. An
/// attribute's "first value" is its first row by `user_attribute.id`;
/// Keycloak writes these attributes single-valued, and `list_users`'
/// `json_agg` reads the same value then.
fn count_voter_channels_sql() -> String {
    let first = |name: &str| {
        format!(
            "NULLIF((array_agg(ua.value ORDER BY ua.id) FILTER (WHERE ua.name = {}))[1], '')",
            sql_literal(name)
        )
    };
    let channels = MessageChannel::iter()
        .map(|channel| sql_literal(&channel.to_string()))
        .collect::<Vec<_>>()
        .join(", ");
    let names = [
        VERIFIED_CHANNELS_ATTR_NAME,
        MESSAGE_CHANNEL_ATTR_NAME,
        MESSAGE_NUMBER_ATTR_NAME,
        MOBILE_PHONE_ATTR_NAME,
        WHATSAPP_NUMBER_ATTR_NAME,
        VIBER_NUMBER_ATTR_NAME,
        MESSENGER_ID_ATTR_NAME,
        MESSENGER_PAGE_ATTR_NAME,
        VOTED_CHANNEL,
    ]
    .map(sql_literal)
    .join(", ");
    format!(
        r#"
        WITH scoped AS MATERIALIZED (
            -- The voters `list_users` lists for the event (same realm, no
            -- service accounts, the event's areas or none, the selection),
            -- each once: a voter with several `area-id` values counts in
            -- the first.
            SELECT DISTINCT ON (u.id)
                u.id, NULLIF(u.email, '') AS email, area_attr.value AS area_id
            FROM user_entity AS u
            INNER JOIN realm AS ra ON ra.id = u.realm_id
            LEFT JOIN user_attribute AS area_attr
                ON area_attr.user_id = u.id AND area_attr.name = {area}
            WHERE ra.name = $1
              AND u.service_account_client_link IS NULL
              AND (u.id = ANY($2::text[]) OR $2::text[] IS NULL)
              AND (area_attr.value = ANY($3::text[]) OR area_attr.user_id IS NULL)
            ORDER BY u.id, area_attr.id
        ),
        attributes AS (
            SELECT ua.user_id,
                   bool_or(ua.name = {verified_name}) AS has_verified,
                   array_agg({verified_value} ORDER BY ua.id)
                       FILTER (WHERE ua.name = {verified_name}) AS verified,
                   {channel_value} AS message_channel,
                   {message_number} AS message_number,
                   {mobile_number} AS mobile_number,
                   {whatsapp_number} AS whatsapp_number,
                   {viber_number} AS viber_number,
                   {messenger_id} AS messenger_id,
                   {messenger_page} AS messenger_page,
                   array_agg(ua.value ORDER BY ua.id)
                       FILTER (WHERE ua.name = {voted_name}) AS voted_channel
            FROM user_attribute AS ua
            WHERE ua.user_id IN (SELECT id FROM scoped) AND ua.name IN ({names})
            GROUP BY ua.user_id
        ),
        records AS (
            SELECT s.id, s.email, s.area_id,
                   COALESCE(a.has_verified, false) AS has_verified,
                   COALESCE(a.verified, '{{}}'::text[]) AS verified,
                   CASE WHEN a.message_channel IN ({channels}) THEN a.message_channel END
                       AS recorded,
                   a.message_number, a.mobile_number, a.whatsapp_number, a.viber_number,
                   a.messenger_id, a.messenger_page, a.voted_channel
            FROM scoped AS s
            LEFT JOIN attributes AS a ON a.user_id = s.id
        ),
        voted AS (
            SELECT DISTINCT voter_id FROM unnest($4::text[]) AS voter_id
        ),
        classified AS (
            SELECT r.area_id, {voter_channel} AS channel,
                   (v.voter_id IS NOT NULL OR (
                       $5::bool
                       AND r.voted_channel[cardinality(r.voted_channel)] <> ''
                       AND lower(r.voted_channel[cardinality(r.voted_channel)])
                           NOT IN (lower({reset}), lower({internet}))
                   )) IS TRUE AS voted
            FROM records AS r
            LEFT JOIN voted AS v ON v.voter_id = r.id
        )
        SELECT area_id, channel, voted, count(*) AS voters
        FROM classified
        GROUP BY area_id, channel, voted
        "#,
        area = sql_literal(AREA_ID_ATTR_NAME),
        verified_name = sql_literal(VERIFIED_CHANNELS_ATTR_NAME),
        verified_value = channel_value_sql("ua.value"),
        voted_name = sql_literal(VOTED_CHANNEL),
        channel_value = channel_value_sql(&first(MESSAGE_CHANNEL_ATTR_NAME)),
        message_number = first(MESSAGE_NUMBER_ATTR_NAME),
        mobile_number = first(MOBILE_PHONE_ATTR_NAME),
        whatsapp_number = first(WHATSAPP_NUMBER_ATTR_NAME),
        viber_number = first(VIBER_NUMBER_ATTR_NAME),
        messenger_id = first(MESSENGER_ID_ATTR_NAME),
        messenger_page = first(MESSENGER_PAGE_ATTR_NAME),
        voter_channel = voter_channel_sql(),
        reset = sql_literal(ATTR_RESET_VALUE),
        internet = sql_literal(VOTED_CHANNEL_INTERNET_VALUE),
    )
}

/// Bounds every statement that follows in `tx` (`SET LOCAL`, so it ends
/// with the transaction). A statement that runs longer is cancelled; see
/// [`is_statement_timeout`].
pub async fn set_statement_timeout(
    tx: &Transaction<'_>,
    timeout: std::time::Duration,
) -> Result<()> {
    tx.batch_execute(&format!(
        "SET LOCAL statement_timeout = {}",
        timeout.as_millis().max(1)
    ))
    .await?;
    Ok(())
}

/// Whether `error` is a statement cancelled by its `statement_timeout`.
pub fn is_statement_timeout(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| {
        cause
            .downcast_ref::<tokio_postgres::Error>()
            .and_then(tokio_postgres::Error::code)
            == Some(&SqlState::QUERY_CANCELED)
    })
}

/// The event realm's voters by area, by their one channel and by whether
/// they voted, in one aggregate on the Keycloak database: no voter is read
/// into memory. The voters are those the bulk send lists (`list_users` with
/// the event, each once); the channel is `voter_channel`'s; "voted" is
/// `get_users_with_vote_info`'s.
#[instrument(skip(keycloak_tx, user_ids, area_ids, votes), fields(selected = user_ids.map(<[String]>::len), voted = votes.voter_ids.len()), err)]
pub async fn count_voter_channels(
    keycloak_tx: &Transaction<'_>,
    realm: &str,
    user_ids: Option<&[String]>,
    area_ids: &[String],
    votes: &Votes,
) -> Result<Vec<VoterChannelCount>> {
    let user_ids: Option<Vec<String>> = user_ids.map(<[String]>::to_vec);
    let area_ids = area_ids.to_vec();
    keycloak_tx
        .query(
            &count_voter_channels_sql(),
            &[
                &realm,
                &user_ids,
                &area_ids,
                &votes.voter_ids,
                &votes.datafix,
            ],
        )
        .await?
        .into_iter()
        .map(|row| {
            let channel: Option<String> = row.try_get("channel")?;
            Ok(VoterChannelCount {
                area_id: row.try_get("area_id")?,
                channel: channel
                    .map(|channel| {
                        MessageChannel::from_str(&channel)
                            .map_err(|_| anyhow!("unexpected channel {channel}"))
                    })
                    .transpose()?,
                voted: row.try_get("voted")?,
                voters: row.try_get("voters")?,
            })
        })
        .collect()
}

/// Which voters a cached count is of: as a send's audience names them.
#[allow(non_camel_case_types)]
#[derive(
    Display,
    EnumString,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Eq,
    Clone,
    Copy,
    Hash,
    PartialOrd,
    Ord,
)]
pub enum VoterCountScope {
    ALL,
    VOTED,
    NOT_VOTED,
}

/// One row of `messaging_voter_count`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedVoterCount {
    pub scope: VoterCountScope,
    pub area_id: Option<String>,
    /// `None`: no contact channel.
    pub channel: Option<MessageChannel>,
    pub voters: i64,
    /// Of them, voters whose Post doesn't offer the channel for notices.
    pub not_offered: i64,
}

/// Replaces the event's cached counts with `counts`, counted at
/// `computed_at`.
#[instrument(skip(tx, counts), fields(rows = counts.len()), err)]
pub async fn replace_voter_counts(
    tx: &Transaction<'_>,
    tenant_id: &Uuid,
    election_event_id: &Uuid,
    counts: &[CachedVoterCount],
    computed_at: DateTime<Utc>,
) -> Result<()> {
    tx.execute(
        "DELETE FROM sequent_backend.messaging_voter_count
         WHERE tenant_id = $1 AND election_event_id = $2",
        &[tenant_id, election_event_id],
    )
    .await?;
    let scopes: Vec<String> = counts.iter().map(|c| c.scope.to_string()).collect();
    let areas: Vec<Option<String>> = counts.iter().map(|c| c.area_id.clone()).collect();
    let channels: Vec<Option<String>> = counts
        .iter()
        .map(|c| c.channel.map(|channel| channel.to_string()))
        .collect();
    let voters: Vec<i64> = counts.iter().map(|c| c.voters).collect();
    let not_offered: Vec<i64> = counts.iter().map(|c| c.not_offered).collect();
    tx.execute(
        "INSERT INTO sequent_backend.messaging_voter_count
             (tenant_id, election_event_id, scope, area_id, channel, voters, not_offered,
              computed_at)
         SELECT $1, $2, scope, area_id, channel, voters, not_offered, $8
         FROM unnest($3::text[], $4::text[], $5::text[], $6::bigint[], $7::bigint[])
             AS row(scope, area_id, channel, voters, not_offered)",
        &[
            tenant_id,
            election_event_id,
            &scopes,
            &areas,
            &channels,
            &voters,
            &not_offered,
            &computed_at,
        ],
    )
    .await?;
    Ok(())
}

/// The event's cached counts of `scope`.
#[instrument(skip(tx), err)]
pub async fn voter_counts(
    tx: &Transaction<'_>,
    tenant_id: &Uuid,
    election_event_id: &Uuid,
    scope: VoterCountScope,
) -> Result<Vec<CachedVoterCount>> {
    tx.query(
        "SELECT area_id, channel, voters, not_offered
         FROM sequent_backend.messaging_voter_count
         WHERE tenant_id = $1 AND election_event_id = $2 AND scope = $3",
        &[tenant_id, election_event_id, &scope.to_string()],
    )
    .await?
    .into_iter()
    .map(|row| {
        let channel: Option<String> = row.try_get("channel")?;
        Ok(CachedVoterCount {
            scope,
            area_id: row.try_get("area_id")?,
            channel: channel
                .map(|channel| {
                    MessageChannel::from_str(&channel)
                        .map_err(|_| anyhow!("unexpected channel {channel}"))
                })
                .transpose()?,
            voters: row.try_get("voters")?,
            not_offered: row.try_get("not_offered")?,
        })
    })
    .collect()
}

/// How an event's last recount ended.
#[allow(non_camel_case_types)]
#[derive(Display, EnumString, Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Copy)]
pub enum RecountOutcome {
    /// The counts were stored.
    COUNTED,
    /// The count took longer than its time limit; the previous counts stay.
    TIMED_OUT,
    /// The count failed; the previous counts stay.
    FAILED,
}

/// An event's recounts (`messaging_voter_count_run`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecountRun {
    /// When a recount was last asked for.
    pub requested_at: Option<DateTime<Utc>>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    /// When the cached counts were counted.
    pub computed_at: Option<DateTime<Utc>>,
    pub outcome: Option<RecountOutcome>,
    /// Why the last recount didn't count, for the administrator.
    pub reason: Option<String>,
}

impl RecountRun {
    /// Whether a recount was asked for, within `pending_for`, and hasn't
    /// finished since.
    pub fn pending(&self, now: DateTime<Utc>, pending_for: Duration) -> bool {
        match self.requested_at {
            Some(requested) => {
                requested > now - pending_for
                    && self
                        .finished_at
                        .map_or(true, |finished| finished < requested)
            }
            None => false,
        }
    }
}

/// The event's recount run, if it was ever counted or asked for.
#[instrument(skip(tx), err)]
pub async fn recount_run(
    tx: &Transaction<'_>,
    tenant_id: &Uuid,
    election_event_id: &Uuid,
) -> Result<Option<RecountRun>> {
    tx.query_opt(
        "SELECT requested_at, started_at, finished_at, computed_at, outcome, reason
         FROM sequent_backend.messaging_voter_count_run
         WHERE tenant_id = $1 AND election_event_id = $2",
        &[tenant_id, election_event_id],
    )
    .await?
    .map(|row| {
        let outcome: Option<String> = row.try_get("outcome")?;
        Ok(RecountRun {
            requested_at: row.try_get("requested_at")?,
            started_at: row.try_get("started_at")?,
            finished_at: row.try_get("finished_at")?,
            computed_at: row.try_get("computed_at")?,
            outcome: outcome
                .map(|outcome| {
                    RecountOutcome::from_str(&outcome)
                        .map_err(|_| anyhow!("unexpected recount outcome {outcome}"))
                })
                .transpose()?,
            reason: row.try_get("reason")?,
        })
    })
    .transpose()
}

/// Records that a recount of the event is asked for at `now`, unless one is
/// already pending (asked for within `pending_for` and not finished). The
/// row is locked while deciding, so two requests at once ask once. Returns
/// whether this request is the one to send.
#[instrument(skip(tx), err)]
pub async fn request_recount(
    tx: &Transaction<'_>,
    tenant_id: &Uuid,
    election_event_id: &Uuid,
    now: DateTime<Utc>,
    pending_for: Duration,
) -> Result<bool> {
    tx.execute(
        "INSERT INTO sequent_backend.messaging_voter_count_run (tenant_id, election_event_id)
         VALUES ($1, $2) ON CONFLICT (election_event_id) DO NOTHING",
        &[tenant_id, election_event_id],
    )
    .await?;
    let row = tx
        .query_one(
            "SELECT requested_at, finished_at FROM sequent_backend.messaging_voter_count_run
             WHERE tenant_id = $1 AND election_event_id = $2 FOR UPDATE",
            &[tenant_id, election_event_id],
        )
        .await?;
    let run = RecountRun {
        requested_at: row.try_get("requested_at")?,
        finished_at: row.try_get("finished_at")?,
        started_at: None,
        computed_at: None,
        outcome: None,
        reason: None,
    };
    if run.pending(now, pending_for) {
        return Ok(false);
    }
    tx.execute(
        "UPDATE sequent_backend.messaging_voter_count_run SET requested_at = $3
         WHERE tenant_id = $1 AND election_event_id = $2",
        &[tenant_id, election_event_id, &now],
    )
    .await?;
    Ok(true)
}

/// Records that a recount started at `now`.
#[instrument(skip(tx), err)]
pub async fn start_recount(
    tx: &Transaction<'_>,
    tenant_id: &Uuid,
    election_event_id: &Uuid,
    now: DateTime<Utc>,
) -> Result<()> {
    tx.execute(
        "INSERT INTO sequent_backend.messaging_voter_count_run
             (tenant_id, election_event_id, started_at)
         VALUES ($1, $2, $3)
         ON CONFLICT (election_event_id) DO UPDATE SET started_at = $3",
        &[tenant_id, election_event_id, &now],
    )
    .await?;
    Ok(())
}

/// Records how a recount ended at `now`. Only a counted one moves
/// `computed_at`; a timed-out or failed one keeps the counts shown and
/// says why.
#[instrument(skip(tx), err)]
pub async fn finish_recount(
    tx: &Transaction<'_>,
    tenant_id: &Uuid,
    election_event_id: &Uuid,
    now: DateTime<Utc>,
    outcome: RecountOutcome,
    reason: Option<&str>,
) -> Result<()> {
    tx.execute(
        "INSERT INTO sequent_backend.messaging_voter_count_run
             (tenant_id, election_event_id, finished_at, computed_at, outcome, reason)
         VALUES ($1, $2, $3::timestamptz,
                 CASE WHEN $4::text = 'COUNTED' THEN $3::timestamptz END, $4::text, $5::text)
         ON CONFLICT (election_event_id) DO UPDATE SET
             finished_at = $3::timestamptz,
             computed_at = CASE WHEN $4::text = 'COUNTED' THEN $3::timestamptz
                                ELSE messaging_voter_count_run.computed_at END,
             outcome = $4::text,
             reason = $5::text",
        &[
            tenant_id,
            election_event_id,
            &now,
            &outcome.to_string(),
            &reason,
        ],
    )
    .await?;
    Ok(())
}

/// Events with a messaging configuration whose voters weren't counted
/// since `counted_before`, never-counted first, then the oldest, at most
/// `limit`. Events with a recount pending are left to it.
#[instrument(skip(tx), err)]
pub async fn events_due_for_recount(
    tx: &Transaction<'_>,
    counted_before: DateTime<Utc>,
    pending_after: DateTime<Utc>,
    limit: i64,
) -> Result<Vec<(Uuid, Uuid)>> {
    tx.query(
        "SELECT config.tenant_id, config.election_event_id
         FROM sequent_backend.messaging_event_config AS config
         LEFT JOIN sequent_backend.messaging_voter_count_run AS run
             ON run.election_event_id = config.election_event_id
         WHERE (run.finished_at IS NULL OR run.finished_at < $1)
           AND NOT (run.requested_at IS NOT NULL AND run.requested_at > $2
                    AND (run.finished_at IS NULL OR run.finished_at < run.requested_at))
         ORDER BY run.finished_at NULLS FIRST, config.election_event_id
         LIMIT $3",
        &[&counted_before, &pending_after, &limit],
    )
    .await?
    .into_iter()
    .map(|row| Ok((row.try_get(0)?, row.try_get(1)?)))
    .collect()
}
