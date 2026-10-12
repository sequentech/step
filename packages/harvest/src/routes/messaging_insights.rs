// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Messaging insights for the Admin Portal, as Hasura actions:
//!
//! - `messaging_event_stats`: the event tab's message counts per channel;
//! - `messaging_voter_counts`: the event's voters per channel, as last
//!   counted (`messaging_voter_count`), with when;
//! - `messaging_recount_voters`: asks windmill to recount them;
//! - `messaging_audience_preview`: Send Notification's "Each voter's
//!   channel" table;
//! - `messaging_monthly_volumes`: messages per month, channel, account,
//!   country and billing category, for invoice reconciliation;
//! - `find_applications_by_support_reference`: the Approvals queue's search
//!   by the reference a voter was given.
//!
//! Each route checks its permission and takes the tenant from the claims.
//! Voter counts are computed with the sender's own rule
//! (`windmill::services::messaging::insights`), by windmill's recount task;
//! only a send's explicit selection of voters is counted in the request,
//! within a time limit. No address leaves harvest.

use crate::routes::messaging::JsonRefusal;
use crate::services::authorization::authorize;
use crate::services::dependencies::HarvestServices;
use chrono::Utc;
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::Permissions;
use sequent_core::types::templates::AudienceSelection;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tracing::{error, info, instrument, warn};
use uuid::Uuid;
use windmill::postgres::messaging_insights::{
    find_applications_by_support_reference, monthly_volumes, recount_run,
    request_recount, VoterCountScope,
};
use windmill::services::messaging::config::get_event_messaging_config;
use windmill::services::messaging::insights::{
    audience_preview, cached_voters, messaging_event_stats as event_stats,
    month_range, parse_support_reference, sort_volumes, support_reference_text,
    Audience, VoterCountStatus, INVALID_MONTH, INVALID_SUPPORT_REFERENCE,
    RECOUNT_PENDING_FOR,
};
use windmill::tasks::recount_messaging_voters::recount_messaging_voters;

/// An `audience_selection` that isn't one of the send's audiences.
pub const UNKNOWN_AUDIENCE: &str = "Unknown audience";

type RouteError = (Status, String);

fn internal(error: anyhow::Error) -> RouteError {
    error!("messaging insights route failed: {error:#}");
    (
        Status::InternalServerError,
        crate::routes::messaging::INTERNAL_ERROR.to_string(),
    )
}

fn bad_request(reason: impl ToString) -> RouteError {
    (Status::BadRequest, reason.to_string())
}

fn parse_uuid(value: &str) -> Result<Uuid, RouteError> {
    Uuid::parse_str(value).map_err(|_| bad_request("invalid identifier"))
}

/// The caller's tenant, once they hold one of `permissions` in it.
fn authorize_tenant(
    claims: &JwtClaims,
    permissions: &[Permissions],
) -> Result<Uuid, RouteError> {
    let tenant = Some(claims.hasura_claims.tenant_id.clone());
    let mut refusal = None;
    for permission in permissions {
        match authorize(claims, true, tenant.clone(), vec![permission.clone()])
        {
            Ok(()) => return parse_uuid(&claims.hasura_claims.tenant_id),
            Err(error) => refusal = Some(error),
        }
    }
    Err(refusal.unwrap_or((Status::Unauthorized, "unauthorized".to_string())))
}

async fn hasura_client(
    services: &State<HarvestServices>,
) -> Result<DbClient, RouteError> {
    services
        .databases
        .hasura()
        .await
        .get()
        .await
        .map_err(|error| internal(anyhow::anyhow!("{error}")))
}

async fn keycloak_client(
    services: &State<HarvestServices>,
) -> Result<DbClient, RouteError> {
    services
        .databases
        .keycloak()
        .await
        .get()
        .await
        .map_err(|error| internal(anyhow::anyhow!("{error}")))
}

#[derive(Deserialize, Debug)]
pub struct EventStatsInput {
    election_event_id: String,
}

#[derive(Serialize, Debug, PartialEq, Eq)]
pub struct ChannelStatsOutput {
    /// `None`: messages of voters with no contact channel.
    channel: Option<String>,
    sent_last_24h: i64,
    queued: i64,
    accepted: i64,
    delivered: i64,
    failed: i64,
    unknown: i64,
    unresolved: i64,
}

#[derive(Serialize, Debug)]
pub struct EventStatsOutput {
    channels: Vec<ChannelStatsOutput>,
}

/// The event tab's message counts: per channel, messages with an attempt
/// in the last 24 hours, and messages by the state of their latest
/// attempt, with Unresolved apart from Unknown. Failed counts the messages
/// the Logs filter lists for that channel. Voters are counted apart
/// (`messaging_voter_counts`), so these never wait for them.
#[instrument(skip_all)]
#[post("/messaging/event-stats", format = "json", data = "<body>")]
pub async fn messaging_event_stats(
    claims: JwtClaims,
    body: Json<EventStatsInput>,
    services: &State<HarvestServices>,
) -> Result<Json<EventStatsOutput>, JsonRefusal> {
    let tenant_id =
        authorize_tenant(&claims, &[Permissions::MESSAGING_CONFIG_READ])?;
    let event_id = parse_uuid(&body.election_event_id)?;
    let mut hasura = hasura_client(services).await?;
    let hasura_tx =
        hasura.transaction().await.map_err(|e| internal(e.into()))?;
    let stats = event_stats(&hasura_tx, &tenant_id, &event_id, Utc::now())
        .await
        .map_err(internal)?;
    hasura_tx.commit().await.map_err(|e| internal(e.into()))?;
    Ok(Json(EventStatsOutput {
        channels: stats
            .into_iter()
            .map(|row| ChannelStatsOutput {
                channel: row.channel.map(|channel| channel.to_string()),
                sent_last_24h: row.sent_last_24h,
                queued: row.queued,
                accepted: row.accepted,
                delivered: row.delivered,
                failed: row.failed,
                unknown: row.unknown,
                unresolved: row.unresolved,
            })
            .collect(),
    }))
}

/// When the voter counts shown were counted, and whether a recount is on
/// its way or the last one didn't count.
#[derive(Serialize, Debug, PartialEq, Eq)]
pub struct VoterCountStatusOutput {
    /// RFC 3339; absent when never counted.
    computed_at: Option<String>,
    pending: bool,
    /// `COUNTED`, `TIMED_OUT` or `FAILED`.
    outcome: Option<String>,
}

impl From<VoterCountStatus> for VoterCountStatusOutput {
    fn from(status: VoterCountStatus) -> Self {
        VoterCountStatusOutput {
            computed_at: status.computed_at.map(|at| at.to_rfc3339()),
            pending: status.pending,
            outcome: status.outcome.map(|outcome| outcome.to_string()),
        }
    }
}

#[derive(Serialize, Debug, PartialEq, Eq)]
pub struct VoterChannelOutput {
    /// `None`: voters with no contact channel.
    channel: Option<String>,
    voters: i64,
    /// Of them, voters whose Post doesn't offer the channel for notices.
    not_offered: i64,
}

#[derive(Serialize, Debug)]
pub struct VoterCountsOutput {
    total: i64,
    channels: Vec<VoterChannelOutput>,
    status: VoterCountStatusOutput,
}

/// The event's voters per channel, as last counted, with when: the event
/// tab's Voters column. Never counts the realm here.
#[instrument(skip_all)]
#[post("/messaging/voter-counts", format = "json", data = "<body>")]
pub async fn messaging_voter_counts(
    claims: JwtClaims,
    body: Json<EventStatsInput>,
    services: &State<HarvestServices>,
) -> Result<Json<VoterCountsOutput>, JsonRefusal> {
    let tenant_id =
        authorize_tenant(&claims, &[Permissions::MESSAGING_CONFIG_READ])?;
    let event_id = parse_uuid(&body.election_event_id)?;
    let mut hasura = hasura_client(services).await?;
    let hasura_tx =
        hasura.transaction().await.map_err(|e| internal(e.into()))?;
    let (tally, status) = cached_voters(
        &hasura_tx,
        &tenant_id,
        &event_id,
        VoterCountScope::ALL,
        Utc::now(),
    )
    .await
    .map_err(internal)?;
    hasura_tx.commit().await.map_err(|e| internal(e.into()))?;
    let mut channels: Vec<VoterChannelOutput> = tally
        .channels
        .iter()
        .map(|(channel, counted)| VoterChannelOutput {
            channel: Some(channel.to_string()),
            voters: counted.voters,
            not_offered: counted.not_offered,
        })
        .collect();
    if tally.no_channel > 0 {
        channels.push(VoterChannelOutput {
            channel: None,
            voters: tally.no_channel,
            not_offered: 0,
        });
    }
    Ok(Json(VoterCountsOutput {
        total: tally.total,
        channels,
        status: status.into(),
    }))
}

/// Asks windmill to recount the event's voters, unless a recount is
/// already on its way. The request is recorded and the task sent in one
/// transaction: a task that can't be sent leaves nothing pending.
pub async fn ask_recount(
    services: &HarvestServices,
    tenant_id: &Uuid,
    election_event_id: &Uuid,
) -> anyhow::Result<bool> {
    let mut hasura = services
        .databases
        .hasura()
        .await
        .get()
        .await
        .map_err(|error| anyhow::anyhow!("{error}"))?;
    let tx = hasura.transaction().await?;
    let asked = request_recount(
        &tx,
        tenant_id,
        election_event_id,
        Utc::now(),
        RECOUNT_PENDING_FOR,
    )
    .await?;
    if asked {
        services
            .tasks
            .connect()
            .await
            .send_task(recount_messaging_voters::new(
                tenant_id.to_string(),
                election_event_id.to_string(),
            ))
            .await?;
    }
    tx.commit().await?;
    Ok(asked)
}

#[derive(Serialize, Debug)]
pub struct RecountOutput {
    /// Whether this request asked for the recount; `false`: one was
    /// already on its way.
    requested: bool,
    status: VoterCountStatusOutput,
}

/// The Recount button: asks windmill to recount the event's voters. Any
/// reader of the counts may ask; a recount on its way isn't asked twice.
#[instrument(skip_all)]
#[post("/messaging/recount-voters", format = "json", data = "<body>")]
pub async fn messaging_recount_voters(
    claims: JwtClaims,
    body: Json<EventStatsInput>,
    services: &State<HarvestServices>,
) -> Result<Json<RecountOutput>, JsonRefusal> {
    let tenant_id =
        authorize_tenant(&claims, &[Permissions::MESSAGING_CONFIG_READ])?;
    let event_id = parse_uuid(&body.election_event_id)?;
    let requested = ask_recount(services, &tenant_id, &event_id)
        .await
        .map_err(internal)?;
    let mut hasura = hasura_client(services).await?;
    let hasura_tx =
        hasura.transaction().await.map_err(|e| internal(e.into()))?;
    let run = recount_run(&hasura_tx, &tenant_id, &event_id)
        .await
        .map_err(internal)?;
    hasura_tx.commit().await.map_err(|e| internal(e.into()))?;
    info!(requested, "voter recount asked for");
    Ok(Json(RecountOutput {
        requested,
        status: VoterCountStatus::of(run.as_ref(), Utc::now()).into(),
    }))
}

/// Asks for a recount after the event's messaging configuration was saved
/// (its per-Post restrictions change who is offered what). A recount that
/// can't be asked for is logged; the save stands, and beat recounts later.
pub async fn recount_after_save(
    services: &HarvestServices,
    tenant_id: &Uuid,
    election_event_id: &Uuid,
) {
    if let Err(error) =
        ask_recount(services, tenant_id, election_event_id).await
    {
        warn!("asking for a voter recount after the save failed: {error:#}");
    }
}

/// A Keycloak transaction for counting a send's selected voters.
async fn read_only(
    client: &mut DbClient,
) -> Result<deadpool_postgres::Transaction<'_>, RouteError> {
    client
        .build_transaction()
        .read_only(true)
        .start()
        .await
        .map_err(|e| internal(e.into()))
}

#[derive(Deserialize, Debug)]
pub struct AudiencePreviewInput {
    election_event_id: String,
    /// As the send names it (`ALL_USERS`, `VOTED`, `NOT_VOTED`,
    /// `SELECTED`); all of the event's voters when absent.
    audience_selection: Option<String>,
    voter_ids: Option<Vec<String>>,
}

#[derive(Serialize, Debug, PartialEq, Eq)]
pub struct AudienceChannelOutput {
    /// `None`: voters with no contact channel, who get nothing.
    channel: Option<String>,
    voters: i64,
    /// Voters on the channel whose Post doesn't offer it for notices: their
    /// notice is refused and shows as failed, never sent elsewhere.
    not_offered: i64,
    /// `ACCOUNT`, `DEPLOYMENT` or `NONE`; absent when nothing is sent on
    /// the row (no contact channel, or not offered to any of its voters).
    sends_from: Option<String>,
    account_id: Option<String>,
    account_name: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct AudiencePreviewOutput {
    total: i64,
    /// Voters a notice would be handed to a sender for.
    will_send: i64,
    channels: Vec<AudienceChannelOutput>,
    status: VoterCountStatusOutput,
}

/// Send Notification's "Each voter's channel": per channel, how many of
/// the send's voters it reaches and the account it sends from, read the way
/// the send will read them. The event's audiences come from the last
/// count; a selection of voters is counted here, within a time limit.
#[instrument(skip_all)]
#[post("/messaging/audience-preview", format = "json", data = "<body>")]
pub async fn messaging_audience_preview(
    claims: JwtClaims,
    body: Json<AudiencePreviewInput>,
    services: &State<HarvestServices>,
) -> Result<Json<AudiencePreviewOutput>, JsonRefusal> {
    let tenant_id =
        authorize_tenant(&claims, &[Permissions::MESSAGING_CONFIG_READ])?;
    let input = body.into_inner();
    let event_id = parse_uuid(&input.election_event_id)?;
    let selection = input
        .audience_selection
        .as_deref()
        .map(|value| {
            AudienceSelection::from_str(value.trim())
                .map_err(|_| bad_request(UNKNOWN_AUDIENCE))
        })
        .transpose()?;
    let audience =
        Audience::of(selection, input.voter_ids).map_err(bad_request)?;
    let mut hasura = hasura_client(services).await?;
    let mut keycloak = keycloak_client(services).await?;
    let hasura_tx =
        hasura.transaction().await.map_err(|e| internal(e.into()))?;
    let keycloak_tx = read_only(&mut keycloak).await?;
    let config = get_event_messaging_config(&hasura_tx, &tenant_id, &event_id)
        .await
        .map_err(internal)?;
    let preview = audience_preview(
        &hasura_tx,
        &keycloak_tx,
        &tenant_id,
        &event_id,
        config.as_ref(),
        &audience,
        Utc::now(),
    )
    .await
    .map_err(internal)?;
    // Read only, and cancelled when counting the selection took too long.
    keycloak_tx
        .rollback()
        .await
        .map_err(|e| internal(e.into()))?;
    hasura_tx.commit().await.map_err(|e| internal(e.into()))?;
    info!(
        voters = preview.total,
        channels = preview.rows.len(),
        "audience preview"
    );
    Ok(Json(AudiencePreviewOutput {
        total: preview.total,
        will_send: preview.will_send,
        status: preview.status.into(),
        channels: preview
            .rows
            .into_iter()
            .map(|row| AudienceChannelOutput {
                channel: row.channel.map(|channel| channel.to_string()),
                voters: row.voters,
                not_offered: row.not_offered,
                sends_from: row.sends_from.map(|from| from.to_string()),
                account_id: row.account_id.map(|id| id.to_string()),
                account_name: row.account_name,
            })
            .collect(),
    }))
}

#[derive(Deserialize, Debug)]
pub struct MonthlyVolumesInput {
    /// `YYYY-MM`, in UTC.
    month: String,
    election_event_id: Option<String>,
}

#[derive(Serialize, Debug, PartialEq, Eq)]
pub struct MonthlyVolumeOutput {
    channel: String,
    account_id: Option<String>,
    account_name: Option<String>,
    destination_country: Option<String>,
    billing_category: Option<String>,
    billable: Option<bool>,
    messages: i64,
}

#[derive(Serialize, Debug)]
pub struct MonthlyVolumesOutput {
    month: String,
    rows: Vec<MonthlyVolumeOutput>,
}

/// The tenant's attempts handed to a provider in a month, by channel,
/// account, destination country, billing category and billable: each
/// attempt once, in the month it was handed over, as providers bill each
/// submission. No prices: providers' rate cards change and aren't Step's to
/// state.
#[instrument(skip_all)]
#[post("/messaging/monthly-volumes", format = "json", data = "<body>")]
pub async fn messaging_monthly_volumes(
    claims: JwtClaims,
    body: Json<MonthlyVolumesInput>,
    services: &State<HarvestServices>,
) -> Result<Json<MonthlyVolumesOutput>, JsonRefusal> {
    let tenant_id = authorize_tenant(
        &claims,
        &[
            Permissions::MESSAGING_ACCOUNT_READ,
            Permissions::MESSAGING_CONFIG_READ,
        ],
    )?;
    let input = body.into_inner();
    let (from, to) =
        month_range(&input.month).ok_or_else(|| bad_request(INVALID_MONTH))?;
    let event_id = input
        .election_event_id
        .as_deref()
        .filter(|id| !id.is_empty())
        .map(parse_uuid)
        .transpose()?;
    let mut hasura = hasura_client(services).await?;
    let hasura_tx =
        hasura.transaction().await.map_err(|e| internal(e.into()))?;
    let mut rows =
        monthly_volumes(&hasura_tx, &tenant_id, from, to, event_id.as_ref())
            .await
            .map_err(internal)?;
    hasura_tx.commit().await.map_err(|e| internal(e.into()))?;
    sort_volumes(&mut rows);
    Ok(Json(MonthlyVolumesOutput {
        month: input.month.trim().to_string(),
        rows: rows
            .into_iter()
            .map(|row| MonthlyVolumeOutput {
                channel: row.channel.to_string(),
                account_id: row.account_id.map(|id| id.to_string()),
                account_name: row.account_name,
                destination_country: row.destination_country,
                billing_category: row.billing_category,
                billable: row.billable,
                messages: row.messages,
            })
            .collect(),
    }))
}

#[derive(Deserialize, Debug)]
pub struct SupportReferenceInput {
    election_event_id: String,
    reference: String,
    /// The queue's label scope, when it shows one label's applications.
    permission_label: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct SupportReferenceOutput {
    /// The reference as read, such as `65SR-MSSD`.
    reference: String,
    application_ids: Vec<String>,
}

/// The applications of an election event whose support reference is the
/// one searched. The reference is derived from the id one way, so it is
/// compared in the database; nothing about the application is returned but
/// its id, which the queue then lists with the caller's own permissions.
#[instrument(skip_all)]
#[post("/applications/support-reference", format = "json", data = "<body>")]
pub async fn find_by_support_reference(
    claims: JwtClaims,
    body: Json<SupportReferenceInput>,
    services: &State<HarvestServices>,
) -> Result<Json<SupportReferenceOutput>, JsonRefusal> {
    let tenant_id =
        authorize_tenant(&claims, &[Permissions::APPLICATION_READ])?;
    let input = body.into_inner();
    let event_id = parse_uuid(&input.election_event_id)?;
    let bits = parse_support_reference(&input.reference)
        .ok_or_else(|| bad_request(INVALID_SUPPORT_REFERENCE))?;
    let mut hasura = hasura_client(services).await?;
    let hasura_tx =
        hasura.transaction().await.map_err(|e| internal(e.into()))?;
    let ids = find_applications_by_support_reference(
        &hasura_tx,
        &tenant_id,
        &event_id,
        bits,
        input.permission_label.as_deref().filter(|l| !l.is_empty()),
    )
    .await
    .map_err(internal)?;
    hasura_tx.commit().await.map_err(|e| internal(e.into()))?;
    info!(matches = ids.len(), "support reference search");
    Ok(Json(SupportReferenceOutput {
        reference: support_reference_text(bits),
        application_ids: ids.iter().map(Uuid::to_string).collect(),
    }))
}

#[cfg(test)]
#[path = "../../tests/support/messaging_insights_routes.rs"]
mod route_tests;
