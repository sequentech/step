// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! `monitoring_voter`: one row per voter and election they can vote in,
//! holding only what the monitoring sources count by, derived as the event's
//! settings say. The snapshot job refreshes it, so dashboards never scan the
//! voters' accounts themselves.
//!
//! The refresh has two tiers:
//! - a full pass over the event realm's voters, which reads their accounts
//!   (areas, attributes, credentials) from Keycloak. It runs when there is no
//!   projection yet, when the settings changed, and otherwise every
//!   `full_pass_every`. A voter whose facts did not change is not rewritten,
//!   and one who is gone is removed.
//! - on every run, the latest application of each voter and their first
//!   valid vote in each election, both read from the backend's own tables in
//!   one statement each.

use crate::postgres::monitoring_config::EventRef;
use anyhow::{Context, Result};
use chrono::{DateTime, Duration, NaiveDate, TimeZone, Utc};
use deadpool_postgres::Transaction;
use sequent_core::monitoring::config::Settings;
use sequent_core::monitoring::voter::{voter_facts, VoterFacts, VoterSources};
use sequent_core::services::keycloak::get_event_realm;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};
use tracing::{info, instrument};
use uuid::Uuid;

/// How many accounts one Keycloak read returns.
pub const VOTER_PAGE_SIZE: i64 = 2_000;

/// The Keycloak attribute that names a voter's area.
const AREA_ATTRIBUTE: &str = "area-id";

/// A voter's account, as the projection reads it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VoterAccount {
    /// Keycloak's user id, which `cast_vote.voter_id_string` and
    /// `applications.applicant_id` carry.
    pub voter_id: String,
    pub attributes: BTreeMap<String, Vec<String>>,
    /// When the account was first given a credential.
    pub credentials_at: Option<DateTime<Utc>>,
}

/// One page of the realm's voters (members of `voter_group`), after
/// `after` in id order; an empty page ends the pass.
#[instrument(err, skip(keycloak_transaction))]
pub async fn fetch_voter_accounts(
    keycloak_transaction: &Transaction<'_>,
    realm: &str,
    voter_group: &str,
    after: Option<&str>,
    limit: i64,
) -> Result<Vec<VoterAccount>> {
    let rows = keycloak_transaction
        .query(
            "SELECT u.id,
                    COALESCE(
                        (SELECT json_agg(json_build_array(ua.name, ua.value))
                         FROM user_attribute ua WHERE ua.user_id = u.id),
                        '[]'::json
                    ) AS attributes,
                    (SELECT min(c.created_date) FROM credential c WHERE c.user_id = u.id)
                        AS credentials_at
             FROM user_entity u
             JOIN realm r ON r.id = u.realm_id
             WHERE r.name = $1
               AND u.id > $2
               AND EXISTS (
                   SELECT 1 FROM user_group_membership m
                   JOIN keycloak_group g ON g.id = m.group_id AND g.realm_id = u.realm_id
                   WHERE m.user_id = u.id AND g.name = $3
               )
             ORDER BY u.id
             LIMIT $4",
            &[&realm, &after.unwrap_or(""), &voter_group, &limit],
        )
        .await
        .context("Failed to read the realm's voters")?;
    rows.iter()
        .map(|row| {
            let attributes: Value = row.get("attributes");
            let credentials_at: Option<i64> = row.get("credentials_at");
            Ok(VoterAccount {
                voter_id: row.get("id"),
                attributes: attribute_map(&attributes),
                credentials_at: credentials_at.and_then(DateTime::from_timestamp_millis),
            })
        })
        .collect()
}

fn attribute_map(pairs: &Value) -> BTreeMap<String, Vec<String>> {
    let mut attributes: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for pair in pairs.as_array().into_iter().flatten() {
        if let (Some(name), Some(value)) = (
            pair.get(0).and_then(Value::as_str),
            pair.get(1).and_then(Value::as_str),
        ) {
            attributes
                .entry(name.to_string())
                .or_default()
                .push(value.to_string());
        }
    }
    for values in attributes.values_mut() {
        values.sort();
    }
    attributes
}

/// The event's elections and areas, as the projection reads a voter's
/// place from them.
#[derive(Debug, Clone, Default)]
pub struct EventPlaces {
    /// Each election's string annotations.
    pub elections: HashMap<Uuid, BTreeMap<String, String>>,
    /// Each area's string annotations, and the elections it votes in.
    pub areas: HashMap<Uuid, (BTreeMap<String, String>, Vec<Uuid>)>,
}

fn string_annotations(annotations: Option<Value>) -> BTreeMap<String, String> {
    match annotations {
        Some(Value::Object(map)) => map
            .into_iter()
            .filter_map(|(key, value)| match value {
                Value::String(value) => Some((key, value)),
                _ => None,
            })
            .collect(),
        _ => BTreeMap::new(),
    }
}

#[instrument(err, skip(transaction))]
pub async fn load_event_places(
    transaction: &Transaction<'_>,
    event: EventRef,
) -> Result<EventPlaces> {
    let params: [&(dyn tokio_postgres::types::ToSql + Sync); 2] =
        [&event.tenant_id, &event.election_event_id];
    let elections = transaction
        .query(
            "SELECT id, annotations FROM sequent_backend.election
             WHERE tenant_id = $1 AND election_event_id = $2",
            &params,
        )
        .await
        .context("Failed to read the event's elections")?
        .iter()
        .map(|row| (row.get::<_, Uuid>(0), string_annotations(row.get(1))))
        .collect();
    let mut areas: HashMap<Uuid, (BTreeMap<String, String>, Vec<Uuid>)> = transaction
        .query(
            "SELECT id, annotations FROM sequent_backend.area
             WHERE tenant_id = $1 AND election_event_id = $2",
            &params,
        )
        .await
        .context("Failed to read the event's areas")?
        .iter()
        .map(|row| (row.get(0), (string_annotations(row.get(1)), Vec::new())))
        .collect();
    for row in transaction
        .query(
            "SELECT DISTINCT ac.area_id, c.election_id
             FROM sequent_backend.area_contest ac
             JOIN sequent_backend.contest c
               ON c.id = ac.contest_id AND c.tenant_id = ac.tenant_id
              AND c.election_event_id = ac.election_event_id
             WHERE ac.tenant_id = $1 AND ac.election_event_id = $2
               AND ac.area_id IS NOT NULL AND c.election_id IS NOT NULL
             ORDER BY ac.area_id, c.election_id",
            &params,
        )
        .await
        .context("Failed to read which elections each area votes in")?
    {
        if let Some((_, elections)) = areas.get_mut(&row.get::<_, Uuid>(0)) {
            elections.push(row.get(1));
        }
    }
    Ok(EventPlaces { elections, areas })
}

/// What the projection is refreshed with.
#[derive(Debug, Clone, Copy)]
pub struct ProjectionContext<'a> {
    pub event: EventRef,
    pub settings: &'a Settings,
    /// The revision of the settings document the rows are derived under.
    pub settings_revision: i32,
    pub places: &'a EventPlaces,
    /// The event's first day, as of which ages are counted.
    pub on: NaiveDate,
}

/// One row the projection writes.
#[derive(Debug, Clone, PartialEq)]
struct VoterRow {
    election_id: Uuid,
    voter_id: String,
    area_id: Uuid,
    facts: VoterFacts,
    credentials_at: Option<DateTime<Utc>>,
    hash: String,
}

fn voter_rows(context: &ProjectionContext<'_>, account: &VoterAccount) -> Vec<VoterRow> {
    let Some(area_id) = account
        .attributes
        .get(AREA_ATTRIBUTE)
        .and_then(|values| values.first())
        .and_then(|value| Uuid::parse_str(value.trim()).ok())
    else {
        return Vec::new();
    };
    let Some((area_annotations, elections)) = context.places.areas.get(&area_id) else {
        return Vec::new();
    };
    let none = BTreeMap::new();
    elections
        .iter()
        .map(|election_id| {
            let facts = voter_facts(
                context.settings,
                VoterSources {
                    attributes: &account.attributes,
                    election_annotations: context
                        .places
                        .elections
                        .get(election_id)
                        .unwrap_or(&none),
                    area_annotations,
                },
                context.on,
            );
            let hash = hex::encode(Sha256::digest(
                json!([
                    area_id,
                    facts.region,
                    facts.country,
                    facts.dims,
                    facts.pre_enrolled,
                    account.credentials_at.map(|at| at.timestamp_millis()),
                ])
                .to_string(),
            ));
            VoterRow {
                election_id: *election_id,
                voter_id: account.voter_id.clone(),
                area_id,
                facts,
                credentials_at: account.credentials_at,
                hash,
            }
        })
        .collect()
}

/// Starts a full pass: the pass then marks each voter it writes as seen.
async fn begin_pass(transaction: &Transaction<'_>) -> Result<()> {
    transaction
        .batch_execute(
            "CREATE TEMP TABLE IF NOT EXISTS monitoring_voter_seen (
                 election_id uuid NOT NULL,
                 voter_id text NOT NULL,
                 PRIMARY KEY (election_id, voter_id)
             ) ON COMMIT DROP;
             TRUNCATE monitoring_voter_seen;",
        )
        .await
        .context("Failed to start a pass over the voters")
}

/// Writes what one page of accounts amounts to: how many rows changed.
#[instrument(err, skip_all, fields(accounts = accounts.len()))]
pub async fn write_voter_page(
    transaction: &Transaction<'_>,
    context: &ProjectionContext<'_>,
    accounts: &[VoterAccount],
) -> Result<u64> {
    let rows: Vec<VoterRow> = accounts
        .iter()
        .flat_map(|account| voter_rows(context, account))
        .collect();
    if rows.is_empty() {
        return Ok(0);
    }
    let elections: Vec<Uuid> = rows.iter().map(|row| row.election_id).collect();
    let voters: Vec<&str> = rows.iter().map(|row| row.voter_id.as_str()).collect();
    let areas: Vec<Uuid> = rows.iter().map(|row| row.area_id).collect();
    let regions: Vec<Option<&str>> = rows.iter().map(|row| row.facts.region.as_deref()).collect();
    let countries: Vec<Option<&str>> = rows
        .iter()
        .map(|row| row.facts.country.as_deref())
        .collect();
    let dims: Vec<Value> = rows.iter().map(|row| json!(row.facts.dims)).collect();
    let pre_enrolled: Vec<bool> = rows.iter().map(|row| row.facts.pre_enrolled).collect();
    let credentials: Vec<Option<DateTime<Utc>>> =
        rows.iter().map(|row| row.credentials_at).collect();
    let hashes: Vec<&str> = rows.iter().map(|row| row.hash.as_str()).collect();
    transaction
        .execute(
            "INSERT INTO monitoring_voter_seen (election_id, voter_id)
             SELECT * FROM unnest($1::uuid[], $2::text[])
             ON CONFLICT DO NOTHING",
            &[&elections, &voters],
        )
        .await
        .context("Failed to mark the voters seen")?;
    transaction
        .execute(
            "INSERT INTO sequent_backend.monitoring_voter AS v
                 (tenant_id, election_event_id, election_id, voter_id, area_id, region, country,
                  dims, pre_enrolled_at, credentials_at, attributes_hash, settings_revision)
             SELECT $1, $2, r.election_id, r.voter_id, r.area_id, r.region, r.country, r.dims,
                    CASE WHEN r.pre_enrolled THEN now() END, r.credentials_at, r.hash, $3
             FROM unnest($4::uuid[], $5::text[], $6::uuid[], $7::text[], $8::text[],
                         $9::jsonb[], $10::bool[], $11::timestamptz[], $12::text[])
                 AS r(election_id, voter_id, area_id, region, country, dims, pre_enrolled,
                      credentials_at, hash)
             ORDER BY r.election_id, r.voter_id
             ON CONFLICT (tenant_id, election_event_id, election_id, voter_id) DO UPDATE SET
                 area_id = EXCLUDED.area_id,
                 region = EXCLUDED.region,
                 country = EXCLUDED.country,
                 dims = EXCLUDED.dims,
                 pre_enrolled_at = CASE
                     WHEN EXCLUDED.pre_enrolled_at IS NULL THEN NULL
                     ELSE COALESCE(v.pre_enrolled_at, EXCLUDED.pre_enrolled_at)
                 END,
                 credentials_at = EXCLUDED.credentials_at,
                 attributes_hash = EXCLUDED.attributes_hash,
                 settings_revision = EXCLUDED.settings_revision,
                 updated_at = now()
             WHERE v.attributes_hash IS DISTINCT FROM EXCLUDED.attributes_hash
                OR v.settings_revision IS DISTINCT FROM EXCLUDED.settings_revision",
            &[
                &context.event.tenant_id,
                &context.event.election_event_id,
                &context.settings_revision,
                &elections,
                &voters,
                &areas,
                &regions,
                &countries,
                &dims,
                &pre_enrolled,
                &credentials,
                &hashes,
            ],
        )
        .await
        .context("Failed to write the voters")
}

/// Ends a full pass: removes the rows of voters it did not see, who are
/// gone or no longer vote in the election. How many.
async fn end_pass(transaction: &Transaction<'_>, event: EventRef) -> Result<u64> {
    transaction
        .execute(
            "DELETE FROM sequent_backend.monitoring_voter v
             WHERE v.tenant_id = $1 AND v.election_event_id = $2
               AND NOT EXISTS (
                   SELECT 1 FROM monitoring_voter_seen s
                   WHERE s.election_id = v.election_id AND s.voter_id = v.voter_id
               )",
            &[&event.tenant_id, &event.election_event_id],
        )
        .await
        .context("Failed to remove the voters no longer there")
}

/// Brings each voter's enrollment and first vote up to date, from the
/// latest application and the valid votes: how many rows changed.
#[instrument(err, skip(transaction))]
pub async fn refresh_voter_activity(transaction: &Transaction<'_>, event: EventRef) -> Result<u64> {
    let params: [&(dyn tokio_postgres::types::ToSql + Sync); 2] =
        [&event.tenant_id, &event.election_event_id];
    let enrollment = transaction
        .execute(
            "WITH latest AS (
                 SELECT DISTINCT ON (applicant_id) applicant_id,
                        CASE WHEN status IN ('PENDING', 'ACCEPTED', 'REJECTED') THEN status END
                            AS status,
                        NULLIF(btrim(annotations->>'rejection_reason'), '') AS reason,
                        updated_at
                 FROM sequent_backend.applications
                 WHERE tenant_id = $1 AND election_event_id = $2
                 ORDER BY applicant_id, updated_at DESC, id DESC
             ),
             wanted AS (
                 SELECT v.election_id, v.voter_id, l.status,
                        CASE WHEN l.status = 'REJECTED' THEN l.reason END AS reason,
                        CASE WHEN l.status IN ('ACCEPTED', 'REJECTED') THEN l.updated_at END
                            AS decided_at
                 FROM sequent_backend.monitoring_voter v
                 LEFT JOIN latest l ON l.applicant_id = v.voter_id
                 WHERE v.tenant_id = $1 AND v.election_event_id = $2
             )
             UPDATE sequent_backend.monitoring_voter v SET
                 enrollment_state = w.status,
                 enrollment_reason = w.reason,
                 enrollment_decided_at = w.decided_at,
                 -- A pre-enrolled voter was so from the approval on, when
                 -- that is earlier than the pass that first saw it.
                 pre_enrolled_at = CASE
                     WHEN v.pre_enrolled_at IS NOT NULL AND w.status = 'ACCEPTED'
                     THEN LEAST(v.pre_enrolled_at, w.decided_at)
                     ELSE v.pre_enrolled_at
                 END,
                 updated_at = now()
             FROM wanted w
             WHERE v.tenant_id = $1 AND v.election_event_id = $2
               AND v.election_id = w.election_id AND v.voter_id = w.voter_id
               AND (v.enrollment_state, v.enrollment_reason, v.enrollment_decided_at)
                   IS DISTINCT FROM (w.status, w.reason, w.decided_at)",
            &params,
        )
        .await
        .context("Failed to bring the voters' enrollment up to date")?;
    let votes = transaction
        .execute(
            "WITH first_votes AS (
                 SELECT v.election_id, v.voter_id,
                        (SELECT min(cv.created_at) FROM sequent_backend.cast_vote cv
                         WHERE cv.tenant_id = $1 AND cv.election_event_id = $2
                           AND cv.election_id = v.election_id
                           AND cv.voter_id_string = v.voter_id
                           AND cv.status = 'valid') AS first_voted_at
                 FROM sequent_backend.monitoring_voter v
                 WHERE v.tenant_id = $1 AND v.election_event_id = $2
             )
             UPDATE sequent_backend.monitoring_voter v SET
                 first_voted_at = f.first_voted_at,
                 updated_at = now()
             FROM first_votes f
             WHERE v.tenant_id = $1 AND v.election_event_id = $2
               AND v.election_id = f.election_id AND v.voter_id = f.voter_id
               AND v.first_voted_at IS DISTINCT FROM f.first_voted_at",
            &params,
        )
        .await
        .context("Failed to bring the voters' first votes up to date")?;
    Ok(enrollment + votes)
}

/// What a refresh did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ProjectionRun {
    pub full_pass: bool,
    pub written: u64,
    pub removed: u64,
    pub activity: u64,
}

/// When the last full pass ran, and under which settings revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LastPass {
    pub at: DateTime<Utc>,
    pub settings_revision: i32,
}

/// Whether a full pass is due.
pub fn full_pass_due(
    last: Option<LastPass>,
    settings_revision: i32,
    now: DateTime<Utc>,
    every: Duration,
) -> bool {
    match last {
        None => true,
        Some(last) => last.settings_revision != settings_revision || now - last.at >= every,
    }
}

const PASS_AT: &str = "voters_full_pass_at";
const PASS_SETTINGS: &str = "voters_settings_revision";

async fn last_pass(transaction: &Transaction<'_>, event: EventRef) -> Result<Option<LastPass>> {
    let row = transaction
        .query_opt(
            "SELECT watermarks FROM sequent_backend.monitoring_snapshot_state
             WHERE tenant_id = $1 AND election_event_id = $2",
            &[&event.tenant_id, &event.election_event_id],
        )
        .await
        .context("Failed to read the snapshot state")?;
    let Some(watermarks) = row.map(|row| row.get::<_, Value>(0)) else {
        return Ok(None);
    };
    let at = watermarks
        .get(PASS_AT)
        .and_then(Value::as_str)
        .and_then(|at| DateTime::parse_from_rfc3339(at).ok())
        .map(|at| at.with_timezone(&Utc));
    let revision = watermarks
        .get(PASS_SETTINGS)
        .and_then(Value::as_i64)
        .and_then(|revision| i32::try_from(revision).ok());
    Ok(at.zip(revision).map(|(at, settings_revision)| LastPass {
        at,
        settings_revision,
    }))
}

async fn record_pass(transaction: &Transaction<'_>, event: EventRef, pass: LastPass) -> Result<()> {
    let marks = json!({ PASS_AT: pass.at.to_rfc3339(), PASS_SETTINGS: pass.settings_revision });
    transaction
        .execute(
            "INSERT INTO sequent_backend.monitoring_snapshot_state
                 (tenant_id, election_event_id, watermarks)
             VALUES ($1, $2, $3)
             ON CONFLICT (tenant_id, election_event_id) DO UPDATE SET
                 watermarks = sequent_backend.monitoring_snapshot_state.watermarks
                              || EXCLUDED.watermarks",
            &[&event.tenant_id, &event.election_event_id, &marks],
        )
        .await
        .context("Failed to record the pass over the voters")?;
    Ok(())
}

/// The event's first day in `settings`' time zone: when its first voting
/// period is scheduled to start, otherwise `now`.
#[instrument(err, skip(transaction, settings))]
pub async fn event_first_day(
    transaction: &Transaction<'_>,
    event: EventRef,
    settings: &Settings,
    now: DateTime<Utc>,
) -> Result<NaiveDate> {
    let starts: Vec<String> = transaction
        .query(
            "SELECT cron_config->>'scheduled_date' FROM sequent_backend.scheduled_event
             WHERE tenant_id = $1 AND election_event_id = $2
               AND event_processor = 'START_VOTING_PERIOD' AND archived_at IS NULL
               AND cron_config->>'scheduled_date' IS NOT NULL",
            &[&event.tenant_id, &event.election_event_id],
        )
        .await
        .context("Failed to read when the event starts")?
        .iter()
        .map(|row| row.get(0))
        .collect();
    let start = starts
        .iter()
        .filter_map(|start| DateTime::parse_from_rfc3339(start).ok())
        .map(|start| start.with_timezone(&Utc))
        .min()
        .unwrap_or(now);
    Ok(match settings.time_zone.parse::<chrono_tz::Tz>() {
        Ok(zone) => zone.from_utc_datetime(&start.naive_utc()).date_naive(),
        Err(_) => start.date_naive(),
    })
}

/// Brings the event's projection up to date: a full pass over the voters
/// when one is due, then their enrollment and first votes.
#[instrument(err, skip_all, fields(event = ?event))]
pub async fn refresh_voter_projection(
    transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    event: EventRef,
    settings: &Settings,
    settings_revision: i32,
    voter_group: &str,
    now: DateTime<Utc>,
    full_pass_every: Duration,
) -> Result<ProjectionRun> {
    let mut run = ProjectionRun::default();
    if full_pass_due(
        last_pass(transaction, event).await?,
        settings_revision,
        now,
        full_pass_every,
    ) {
        let places = load_event_places(transaction, event).await?;
        let context = ProjectionContext {
            event,
            settings,
            settings_revision,
            places: &places,
            on: event_first_day(transaction, event, settings, now).await?,
        };
        let realm = get_event_realm(
            &event.tenant_id.to_string(),
            &event.election_event_id.to_string(),
        );
        begin_pass(transaction).await?;
        let mut after: Option<String> = None;
        loop {
            let page = fetch_voter_accounts(
                keycloak_transaction,
                &realm,
                voter_group,
                after.as_deref(),
                VOTER_PAGE_SIZE,
            )
            .await?;
            let Some(last) = page.last() else {
                break;
            };
            after = Some(last.voter_id.clone());
            run.written += write_voter_page(transaction, &context, &page).await?;
        }
        run.removed = end_pass(transaction, event).await?;
        record_pass(
            transaction,
            event,
            LastPass {
                at: now,
                settings_revision,
            },
        )
        .await?;
        run.full_pass = true;
    }
    run.activity = refresh_voter_activity(transaction, event).await?;
    info!(?run, "Refreshed the monitoring voters");
    Ok(run)
}

/// A full pass over `accounts` as if Keycloak had returned them, for tests
/// and tools that already hold the accounts.
pub async fn project_accounts(
    transaction: &Transaction<'_>,
    context: &ProjectionContext<'_>,
    accounts: &[VoterAccount],
) -> Result<(u64, u64)> {
    begin_pass(transaction).await?;
    let written = write_voter_page(transaction, context, accounts).await?;
    let removed = end_pass(transaction, context.event).await?;
    Ok((written, removed))
}
