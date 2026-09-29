// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The snapshot job's passes, and how Harvest and the export read what they
//! counted.
//!
//! A pass of an event, under the event's lock:
//! 1. brings `monitoring_voter` up to date (its own transaction);
//! 2. records a RUNNING run (its own transaction, so a failure can be
//!    recorded against it), after marking FAILED any run a crash left
//!    RUNNING;
//! 3. in one REPEATABLE READ transaction, counts every source for every set
//!    of elections viewers asked for, writes only the scopes whose figures
//!    changed, records whether each source was counted, completes the run
//!    and shows it. When nothing changed it writes nothing: it deletes its
//!    run and marks the shown one checked instead, so viewers keep the
//!    revision they have;
//! 4. prunes runs past the export window, then the figures and payloads no
//!    kept run holds (its own transaction).
//!
//! The tables' triggers hold the protocol (see the migration); this module
//! follows it.

use super::config_store::get_live_config;
use super::producers::{
    poll_state, produce, zone_of, ElectionSet, Enrollment, EventFacts, LoginRow, Post,
    SourceFigures, SourceStatus, VoterRow,
};
use super::projection::{load_event_places, refresh_voter_projection};
use crate::postgres::monitoring_config::EventRef;
use crate::types::miru_plugin::{MiruServerDocumentStatus, MiruTallySessionData};
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Duration, Utc};
use deadpool_postgres::{Client, Transaction};
use sequent_core::monitoring::config::{ConfigKind, Settings};
use sequent_core::monitoring::payload::ScopePayload;
use sequent_core::monitoring::revision::DashboardMode;
use sequent_core::monitoring::scope::{election_set_key, ScopeKey};
use sequent_core::monitoring::sources::{DataSourceId, PendingProducer, PostState};
use sequent_core::monitoring::voter::dimension_value;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use tokio_postgres::error::SqlState;
use tokio_postgres::IsolationLevel;
use tracing::{info, instrument, warn};
use uuid::Uuid;

/// How long a run stays readable after a later one is shown: what an export
/// of a figure a viewer saw can still name.
pub const EXPORT_WINDOW: Duration = Duration::hours(2);

/// How long a set of elections nobody asks for again is still counted.
pub const SET_KEPT_FOR: Duration = Duration::hours(24);

/// How often a viewer asking for a set refreshes when it was last asked for.
pub const SET_REQUEST_EVERY: Duration = Duration::minutes(5);

/// A complete run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveSnapshot {
    pub revision: i64,
    pub as_of: DateTime<Utc>,
    pub checked_at: Option<DateTime<Utc>>,
    pub settings_revision: i32,
    pub config_generation: i64,
}

fn live_from(row: &tokio_postgres::Row) -> LiveSnapshot {
    LiveSnapshot {
        revision: row.get("revision"),
        as_of: row.get("as_of"),
        checked_at: row.get("checked_at"),
        settings_revision: row.get("settings_revision"),
        config_generation: row.get("config_generation"),
    }
}

/// The run viewers are shown; `None` before the first completes.
#[instrument(err, skip(transaction))]
pub async fn live_snapshot(
    transaction: &Transaction<'_>,
    event: EventRef,
) -> Result<Option<LiveSnapshot>> {
    let row = transaction
        .query_opt(
            "SELECT r.revision, r.as_of, r.checked_at, r.settings_revision, r.config_generation
             FROM sequent_backend.monitoring_snapshot_state s
             JOIN sequent_backend.monitoring_snapshot_run r
               ON r.tenant_id = s.tenant_id AND r.election_event_id = s.election_event_id
              AND r.revision = s.live_snapshot_revision
             WHERE s.tenant_id = $1 AND s.election_event_id = $2",
            &[&event.tenant_id, &event.election_event_id],
        )
        .await
        .context("Failed to read the shown snapshot")?;
    Ok(row.as_ref().map(live_from))
}

/// The run at `revision`, if it completed and is still kept.
#[instrument(err, skip(transaction))]
pub async fn complete_snapshot(
    transaction: &Transaction<'_>,
    event: EventRef,
    revision: i64,
) -> Result<Option<LiveSnapshot>> {
    let row = transaction
        .query_opt(
            "SELECT revision, as_of, checked_at, settings_revision, config_generation
             FROM sequent_backend.monitoring_snapshot_run
             WHERE tenant_id = $1 AND election_event_id = $2 AND revision = $3
               AND status = 'COMPLETE'",
            &[&event.tenant_id, &event.election_event_id, &revision],
        )
        .await
        .context("Failed to read a snapshot run")?;
    Ok(row.as_ref().map(live_from))
}

/// What a run holds for one source, set of elections and scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScopeRead {
    /// The set was not counted in the run: the next pass will.
    NotCounted,
    NotConnected {
        reason: PendingProducer,
    },
    /// Counted, with nothing in the scope: zeros, [`empty_payload`].
    Empty,
    /// The payload as stored: the exact JSON its hash is of.
    Payload {
        sha256_hex: String,
        text: String,
    },
}

fn pending_producer(reason: &str) -> Option<PendingProducer> {
    serde_json::from_value(Value::String(reason.to_string())).ok()
}

/// One statement, so a run pruned meanwhile is read whole or not at all.
#[instrument(err, skip(transaction))]
pub async fn read_scope(
    transaction: &Transaction<'_>,
    event: EventRef,
    revision: i64,
    source: DataSourceId,
    election_set_key: &str,
    scope_key: &str,
) -> Result<ScopeRead> {
    let row = transaction
        .query_opt(
            "SELECT s.producer_status, s.reason, encode(p.sha256, 'hex') AS sha256, p.payload
             FROM sequent_backend.monitoring_snapshot_source s
             JOIN sequent_backend.monitoring_snapshot_run r
               ON r.tenant_id = s.tenant_id AND r.election_event_id = s.election_event_id
              AND r.revision = s.revision AND r.status = 'COMPLETE'
             LEFT JOIN sequent_backend.monitoring_snapshot_figure f
               ON f.tenant_id = s.tenant_id AND f.election_event_id = s.election_event_id
              AND f.source = s.source AND f.election_set_key = s.election_set_key
              AND f.scope_key = $6 AND int8range(f.from_revision, f.to_revision) @> s.revision
             LEFT JOIN sequent_backend.monitoring_snapshot_payload p
               ON p.tenant_id = f.tenant_id AND p.election_event_id = f.election_event_id
              AND p.sha256 = f.payload_sha256
             WHERE s.tenant_id = $1 AND s.election_event_id = $2 AND s.revision = $3
               AND s.source = $4 AND s.election_set_key = $5",
            &[
                &event.tenant_id,
                &event.election_event_id,
                &revision,
                &source.to_string(),
                &election_set_key,
                &scope_key,
            ],
        )
        .await
        .context("Failed to read a snapshot figure")?;
    let Some(row) = row else {
        return Ok(ScopeRead::NotCounted);
    };
    let status: String = row.get("producer_status");
    if status == "NOT_CONNECTED" {
        let reason: Option<String> = row.get("reason");
        let reason = reason
            .as_deref()
            .and_then(pending_producer)
            .ok_or_else(|| anyhow!("A source is not connected for no known reason: {reason:?}"))?;
        return Ok(ScopeRead::NotConnected { reason });
    }
    let sha256: Option<String> = row.get("sha256");
    let text: Option<String> = row.get("payload");
    Ok(match (sha256, text) {
        (Some(sha256_hex), Some(text)) => ScopeRead::Payload { sha256_hex, text },
        _ => ScopeRead::Empty,
    })
}

/// A counted scope with nothing in it: every measure of the source at zero.
pub fn empty_payload(source: DataSourceId) -> ScopePayload {
    ScopePayload {
        totals: source
            .spec()
            .measures
            .iter()
            .map(|measure| (*measure, 0))
            .collect(),
        ..ScopePayload::default()
    }
}

/// The regions, Posts and countries a set of elections can be narrowed to.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScopeCatalogue {
    pub regions: Vec<String>,
    pub posts: Vec<Uuid>,
    pub countries: Vec<String>,
}

/// From the scopes counted for the set at `revision`.
#[instrument(err, skip(transaction))]
pub async fn scope_catalogue(
    transaction: &Transaction<'_>,
    event: EventRef,
    revision: i64,
    election_set_key: &str,
) -> Result<ScopeCatalogue> {
    let rows = transaction
        .query(
            "SELECT DISTINCT f.scope_key
             FROM sequent_backend.monitoring_snapshot_figure f
             WHERE f.tenant_id = $1 AND f.election_event_id = $2 AND f.election_set_key = $3
               AND int8range(f.from_revision, f.to_revision) @> $4::bigint
               AND f.scope_key NOT LIKE '%&%'",
            &[
                &event.tenant_id,
                &event.election_event_id,
                &election_set_key,
                &revision,
            ],
        )
        .await
        .context("Failed to read the scopes counted")?;
    let mut regions = BTreeSet::new();
    let mut posts = BTreeSet::new();
    let mut countries = BTreeSet::new();
    for row in rows {
        let Some(key) = ScopeKey::from_canonical(row.get(0)) else {
            continue;
        };
        if let Some(region) = key.region {
            regions.insert(region);
        }
        if let Some(post) = key.post.and_then(|post| post.parse::<Uuid>().ok()) {
            posts.insert(post);
        }
        if let Some(country) = key.country {
            countries.insert(country);
        }
    }
    Ok(ScopeCatalogue {
        regions: regions.into_iter().collect(),
        posts: posts.into_iter().collect(),
        countries: countries.into_iter().collect(),
    })
}

/// Records that a viewer asks for `election_ids`, so passes count the set;
/// `requested_at` is refreshed at most every [`SET_REQUEST_EVERY`]. The set's
/// key.
#[instrument(err, skip(transaction))]
pub async fn request_election_set(
    transaction: &Transaction<'_>,
    event: EventRef,
    election_ids: &[Uuid],
) -> Result<String> {
    let ids: Vec<Uuid> = election_ids
        .iter()
        .copied()
        .collect::<BTreeSet<Uuid>>()
        .into_iter()
        .collect();
    let key =
        election_set_key(ids.iter().map(Uuid::to_string)).map_err(|error| anyhow!("{error}"))?;
    let every = SET_REQUEST_EVERY.num_seconds() as f64;
    transaction
        .execute(
            "INSERT INTO sequent_backend.monitoring_election_set AS s
                 (tenant_id, election_event_id, election_set_key, election_ids)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (tenant_id, election_event_id, election_set_key) DO UPDATE
                 SET requested_at = now()
                 WHERE s.requested_at < now() - make_interval(secs => $5)",
            &[
                &event.tenant_id,
                &event.election_event_id,
                &key,
                &ids,
                &every,
            ],
        )
        .await
        .context("Failed to record the set of elections asked for")?;
    Ok(key)
}

/// What a pass of an event did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PassOutcome {
    /// The event is not on configured dashboards.
    NotConfigured,
    /// A new run is shown.
    Completed {
        revision: i64,
        figures_written: usize,
    },
    /// Nothing changed; the shown run was checked.
    Unchanged { shown: Option<i64> },
    /// A later run completed first.
    Superseded { revision: i64 },
    /// Another pass got ahead while counting; the next pass retries.
    Conflicted { revision: i64 },
}

/// How a pass runs.
#[derive(Debug, Clone, Copy)]
pub struct PassOptions<'a> {
    pub voter_group: &'a str,
    pub full_pass_every: Duration,
    pub now: DateTime<Utc>,
}

/// One pass of `event`. The caller holds the event's lock.
#[instrument(err, skip(hasura, keycloak, options), fields(event = ?event))]
pub async fn refresh_event_snapshot(
    hasura: &mut Client,
    keycloak: &mut Client,
    event: EventRef,
    options: PassOptions<'_>,
) -> Result<PassOutcome> {
    let configured = {
        let transaction = hasura.transaction().await?;
        let live = get_live_config(&transaction, event).await?;
        transaction.commit().await?;
        live
    };
    let Some(config) = configured.filter(|config| config.mode == DashboardMode::Configured) else {
        return Ok(PassOutcome::NotConfigured);
    };
    let Some(settings) = config.assembled.set.settings.clone() else {
        warn!("A configured event without settings is not counted");
        return Ok(PassOutcome::NotConfigured);
    };
    let settings_revision = config
        .documents
        .iter()
        .find(|document| document.kind == ConfigKind::Settings)
        .map(|document| document.revision)
        .ok_or_else(|| anyhow!("The settings document has no revision"))?;

    {
        let transaction = hasura.transaction().await?;
        let keycloak_transaction = keycloak.transaction().await?;
        refresh_voter_projection(
            &transaction,
            &keycloak_transaction,
            event,
            &settings,
            settings_revision,
            options.voter_group,
            options.now,
            options.full_pass_every,
        )
        .await?;
        keycloak_transaction.rollback().await?;
        transaction.commit().await?;
    }

    count_event(
        hasura,
        event,
        &settings,
        settings_revision,
        config.generation,
    )
    .await
}

/// Counts `event` from its projection as it stands, and shows the result:
/// steps 2 and 3 of a pass. The caller holds the event's lock.
#[instrument(err, skip(hasura, settings), fields(event = ?event))]
pub async fn count_event(
    hasura: &mut Client,
    event: EventRef,
    settings: &Settings,
    settings_revision: i32,
    config_generation: i64,
) -> Result<PassOutcome> {
    let revision = start_run(hasura, event).await?;
    let outcome = {
        let transaction = hasura
            .build_transaction()
            .isolation_level(IsolationLevel::RepeatableRead)
            .start()
            .await?;
        let counted = count(
            &transaction,
            event,
            revision,
            settings,
            settings_revision,
            config_generation,
        )
        .await;
        match counted {
            Ok(outcome @ PassOutcome::Completed { .. })
            | Ok(outcome @ PassOutcome::Superseded { .. }) => match transaction.commit().await {
                Ok(()) => Ok(outcome),
                Err(error) => Err(anyhow::Error::from(error)),
            },
            Ok(outcome) => {
                transaction.rollback().await?;
                Ok(outcome)
            }
            Err(error) => {
                transaction.rollback().await.ok();
                Err(error)
            }
        }
    };
    match outcome {
        Ok(PassOutcome::Unchanged { shown }) => {
            let transaction = hasura.transaction().await?;
            transaction
                .execute(
                    "DELETE FROM sequent_backend.monitoring_snapshot_run
                     WHERE tenant_id = $1 AND election_event_id = $2 AND revision = $3
                       AND status = 'RUNNING'",
                    &[&event.tenant_id, &event.election_event_id, &revision],
                )
                .await?;
            if let Some(shown) = shown {
                transaction
                    .execute(
                        "UPDATE sequent_backend.monitoring_snapshot_run SET checked_at = now()
                         WHERE tenant_id = $1 AND election_event_id = $2 AND revision = $3
                           AND (checked_at IS NULL OR checked_at < now())",
                        &[&event.tenant_id, &event.election_event_id, &shown],
                    )
                    .await?;
            }
            transaction.commit().await?;
            Ok(PassOutcome::Unchanged { shown })
        }
        Ok(outcome) => Ok(outcome),
        Err(error) => {
            let conflicted = error
                .chain()
                .filter_map(|cause| cause.downcast_ref::<tokio_postgres::Error>())
                .any(|cause| cause.code() == Some(&SqlState::T_R_SERIALIZATION_FAILURE));
            let message = if conflicted {
                "another pass got ahead".to_string()
            } else {
                format!("{error:#}")
            };
            fail_run(hasura, event, revision, &message).await?;
            if conflicted {
                Ok(PassOutcome::Conflicted { revision })
            } else {
                Err(error)
            }
        }
    }
}

async fn start_run(hasura: &mut Client, event: EventRef) -> Result<i64> {
    let transaction = hasura.transaction().await?;
    transaction
        .execute(
            "UPDATE sequent_backend.monitoring_snapshot_run
             SET status = 'FAILED', finished_at = now(), error = 'abandoned by its pass'
             WHERE tenant_id = $1 AND election_event_id = $2 AND status = 'RUNNING'",
            &[&event.tenant_id, &event.election_event_id],
        )
        .await
        .context("Failed to mark abandoned runs")?;
    let revision: i64 = transaction
        .query_one(
            "INSERT INTO sequent_backend.monitoring_snapshot_run
                 (tenant_id, election_event_id, status)
             VALUES ($1, $2, 'RUNNING') RETURNING revision",
            &[&event.tenant_id, &event.election_event_id],
        )
        .await
        .context("Failed to record the run")?
        .get(0);
    transaction.commit().await?;
    Ok(revision)
}

async fn fail_run(hasura: &mut Client, event: EventRef, revision: i64, error: &str) -> Result<()> {
    let transaction = hasura.transaction().await?;
    transaction
        .execute(
            "UPDATE sequent_backend.monitoring_snapshot_run
             SET status = 'FAILED', finished_at = now(), error = $4
             WHERE tenant_id = $1 AND election_event_id = $2 AND revision = $3
               AND status = 'RUNNING'",
            &[
                &event.tenant_id,
                &event.election_event_id,
                &revision,
                &error,
            ],
        )
        .await
        .context("Failed to record the failed run")?;
    transaction.commit().await?;
    Ok(())
}

/// A scope's figures as stored: the payload's exact text and its hash.
struct Stored {
    text: String,
    sha256: Vec<u8>,
}

fn stored(payload: &ScopePayload) -> Result<Stored> {
    let text = serde_json::to_string(payload)?.replace('\u{0}', "\u{FFFD}");
    let sha256 = Sha256::digest(text.as_bytes()).to_vec();
    Ok(Stored { text, sha256 })
}

type FigureKey = (String, String, String);

/// Counts, writes what changed and completes the run, in the pass's
/// transaction. `Unchanged` means the caller rolls back.
async fn count(
    transaction: &Transaction<'_>,
    event: EventRef,
    revision: i64,
    settings: &Settings,
    settings_revision: i32,
    config_generation: i64,
) -> Result<PassOutcome> {
    let params: [&(dyn tokio_postgres::types::ToSql + Sync); 2] =
        [&event.tenant_id, &event.election_event_id];
    let newer: bool = transaction
        .query_one(
            "SELECT EXISTS (SELECT 1 FROM sequent_backend.monitoring_snapshot_run
                            WHERE tenant_id = $1 AND election_event_id = $2
                              AND revision > $3 AND status = 'COMPLETE')",
            &[&event.tenant_id, &event.election_event_id, &revision],
        )
        .await?
        .get(0);
    if newer {
        transaction
            .execute(
                "UPDATE sequent_backend.monitoring_snapshot_run
                 SET status = 'FAILED', finished_at = now(), error = 'superseded by a later run'
                 WHERE tenant_id = $1 AND election_event_id = $2 AND revision = $3",
                &[&event.tenant_id, &event.election_event_id, &revision],
            )
            .await?;
        return Ok(PassOutcome::Superseded { revision });
    }

    let facts = load_facts(transaction, event, settings).await?;
    let all: Vec<Uuid> = facts.posts.iter().map(|post| post.id).collect();
    request_election_set(transaction, event, &all).await?;
    let sets = load_sets(transaction, event).await?;
    let figures = produce(&facts, &sets);

    let shown = live_snapshot(transaction, event).await?;
    let mut open: HashMap<FigureKey, Vec<u8>> = HashMap::new();
    for row in transaction
        .query(
            "SELECT source, election_set_key, scope_key, payload_sha256
             FROM sequent_backend.monitoring_snapshot_figure
             WHERE tenant_id = $1 AND election_event_id = $2 AND to_revision IS NULL",
            &params,
        )
        .await?
    {
        open.insert((row.get(0), row.get(1), row.get(2)), row.get(3));
    }

    let mut wanted: BTreeMap<FigureKey, Stored> = BTreeMap::new();
    for figure in &figures {
        for (scope, payload) in &figure.scopes {
            wanted.insert(
                (
                    figure.source.to_string(),
                    figure.election_set_key.clone(),
                    scope.clone(),
                ),
                stored(payload)?,
            );
        }
    }
    let to_close: Vec<&FigureKey> = open
        .iter()
        .filter(|(key, sha256)| wanted.get(*key).map(|stored| &stored.sha256) != Some(*sha256))
        .map(|(key, _)| key)
        .collect();
    let to_open: Vec<(&FigureKey, &Stored)> = wanted
        .iter()
        .filter(|(key, stored)| open.get(*key) != Some(&stored.sha256))
        .collect();

    let sources_unchanged = match &shown {
        Some(shown) => {
            shown.settings_revision == settings_revision
                && shown.config_generation == config_generation
                && source_rows(transaction, event, shown.revision).await?
                    == wanted_sources(&figures)
        }
        None => false,
    };
    if to_close.is_empty() && to_open.is_empty() && sources_unchanged {
        return Ok(PassOutcome::Unchanged {
            shown: shown.map(|shown| shown.revision),
        });
    }

    for (source, set, scope) in &to_close {
        transaction
            .execute(
                "UPDATE sequent_backend.monitoring_snapshot_figure SET to_revision = $6
                 WHERE tenant_id = $1 AND election_event_id = $2 AND source = $3
                   AND election_set_key = $4 AND scope_key = $5 AND to_revision IS NULL",
                &[
                    &event.tenant_id,
                    &event.election_event_id,
                    source,
                    set,
                    scope,
                    &revision,
                ],
            )
            .await
            .context("Failed to close a figure")?;
    }
    for ((source, set, scope), stored) in &to_open {
        transaction
            .execute(
                "INSERT INTO sequent_backend.monitoring_snapshot_payload
                     (tenant_id, election_event_id, sha256, payload)
                 VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING",
                &[
                    &event.tenant_id,
                    &event.election_event_id,
                    &stored.sha256,
                    &stored.text,
                ],
            )
            .await
            .context("Failed to store a payload")?;
        // A payload stored before is kept while this pass names it.
        transaction
            .execute(
                "SELECT 1 FROM sequent_backend.monitoring_snapshot_payload
                 WHERE tenant_id = $1 AND election_event_id = $2 AND sha256 = $3
                 FOR KEY SHARE",
                &[&event.tenant_id, &event.election_event_id, &stored.sha256],
            )
            .await?;
        transaction
            .execute(
                "INSERT INTO sequent_backend.monitoring_snapshot_figure
                     (tenant_id, election_event_id, source, election_set_key, scope_key,
                      from_revision, payload_sha256)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)",
                &[
                    &event.tenant_id,
                    &event.election_event_id,
                    source,
                    set,
                    scope,
                    &revision,
                    &stored.sha256,
                ],
            )
            .await
            .context("Failed to open a figure")?;
    }
    for (source, set, status, reason) in wanted_sources(&figures) {
        transaction
            .execute(
                "INSERT INTO sequent_backend.monitoring_snapshot_source
                     (tenant_id, election_event_id, revision, source, election_set_key,
                      producer_status, reason)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)",
                &[
                    &event.tenant_id,
                    &event.election_event_id,
                    &revision,
                    &source,
                    &set,
                    &status,
                    &reason,
                ],
            )
            .await
            .context("Failed to record a source")?;
    }
    transaction
        .execute(
            "UPDATE sequent_backend.monitoring_snapshot_run
             SET status = 'COMPLETE', finished_at = now(), as_of = now(),
                 settings_revision = $4, config_generation = $5
             WHERE tenant_id = $1 AND election_event_id = $2 AND revision = $3",
            &[
                &event.tenant_id,
                &event.election_event_id,
                &revision,
                &settings_revision,
                &config_generation,
            ],
        )
        .await
        .context("Failed to complete the run")?;
    transaction
        .execute(
            "UPDATE sequent_backend.monitoring_snapshot_state SET live_snapshot_revision = $3
             WHERE tenant_id = $1 AND election_event_id = $2",
            &[&event.tenant_id, &event.election_event_id, &revision],
        )
        .await
        .context("Failed to show the run")?;
    let figures_written = to_open.len();
    info!(
        revision,
        figures_written,
        closed = to_close.len(),
        "Completed a snapshot run"
    );
    Ok(PassOutcome::Completed {
        revision,
        figures_written,
    })
}

type SourceRow = (String, String, String, Option<String>);

fn wanted_sources(figures: &[SourceFigures]) -> BTreeSet<SourceRow> {
    figures
        .iter()
        .map(|figure| {
            let (status, reason) = match figure.status {
                SourceStatus::Connected => ("CONNECTED", None),
                SourceStatus::NotConnected(pending) => ("NOT_CONNECTED", Some(pending.to_string())),
            };
            (
                figure.source.to_string(),
                figure.election_set_key.clone(),
                status.to_string(),
                reason,
            )
        })
        .collect()
}

async fn source_rows(
    transaction: &Transaction<'_>,
    event: EventRef,
    revision: i64,
) -> Result<BTreeSet<SourceRow>> {
    Ok(transaction
        .query(
            "SELECT source, election_set_key, producer_status, reason
             FROM sequent_backend.monitoring_snapshot_source
             WHERE tenant_id = $1 AND election_event_id = $2 AND revision = $3",
            &[&event.tenant_id, &event.election_event_id, &revision],
        )
        .await?
        .iter()
        .map(|row| (row.get(0), row.get(1), row.get(2), row.get(3)))
        .collect())
}

async fn load_sets(transaction: &Transaction<'_>, event: EventRef) -> Result<Vec<ElectionSet>> {
    let kept = SET_KEPT_FOR.num_seconds() as f64;
    Ok(transaction
        .query(
            "SELECT election_set_key, election_ids FROM sequent_backend.monitoring_election_set
             WHERE tenant_id = $1 AND election_event_id = $2
               AND requested_at > now() - make_interval(secs => $3)
             ORDER BY election_set_key",
            &[&event.tenant_id, &event.election_event_id, &kept],
        )
        .await
        .context("Failed to read the sets of elections asked for")?
        .iter()
        .map(|row| ElectionSet {
            key: row.get(0),
            elections: row.get::<_, Vec<Uuid>>(1).into_iter().collect(),
        })
        .collect())
}

/// The name the portal shows for an election: its English alias or name,
/// else any language's, else its external id, else its id.
fn post_name(presentation: Option<Value>, external_id: Option<String>, id: Uuid) -> String {
    let i18n = presentation
        .as_ref()
        .and_then(|presentation| presentation.get("i18n"))
        .and_then(Value::as_object);
    let field = |lang: &str, field: &str| {
        i18n?
            .get(lang)?
            .get(field)?
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    };
    field("en", "alias")
        .or_else(|| field("en", "name"))
        .or_else(|| {
            i18n?
                .keys()
                .find_map(|lang| field(lang, "alias").or_else(|| field(lang, "name")))
        })
        .or(external_id.filter(|external_id| !external_id.trim().is_empty()))
        .unwrap_or_else(|| id.to_string())
}

/// Reads everything a pass counts from.
pub async fn load_facts<'s>(
    transaction: &Transaction<'_>,
    event: EventRef,
    settings: &'s Settings,
) -> Result<EventFacts<'s>> {
    let params: [&(dyn tokio_postgres::types::ToSql + Sync); 2] =
        [&event.tenant_id, &event.election_event_id];
    let counting = counting_states(transaction, event).await?;
    let posts = transaction
        .query(
            "SELECT id, presentation, external_id, annotations, status,
                    COALESCE(initialization_report_generated, false) AS initialized
             FROM sequent_backend.election
             WHERE tenant_id = $1 AND election_event_id = $2
             ORDER BY id",
            &params,
        )
        .await
        .context("Failed to read the event's elections")?
        .iter()
        .map(|row| {
            let id: Uuid = row.get("id");
            let annotations: Option<Value> = row.get("annotations");
            let status: Option<Value> = row.get("status");
            let voting_status = status
                .as_ref()
                .and_then(|status| status.get("voting_status"))
                .and_then(Value::as_str)
                .map(str::to_string);
            let region = settings
                .scope
                .region
                .election_annotation
                .as_deref()
                .and_then(|key| annotations.as_ref()?.get(key)?.as_str().map(str::to_string))
                .and_then(|raw| {
                    dimension_value(&settings.scope.region, Some(&raw), Utc::now().date_naive())
                });
            Post {
                id,
                name: post_name(row.get("presentation"), row.get("external_id"), id),
                region,
                poll: poll_state(voting_status.as_deref(), row.get("initialized")),
                counting: counting.get(&id).copied().unwrap_or(PostState::NotTallied),
            }
        })
        .collect();
    let voters = transaction
        .query(
            "SELECT voter_id, election_id, region, country, dims, pre_enrolled_at,
                    first_voted_at, enrollment_state, enrollment_reason, enrollment_decided_at
             FROM sequent_backend.monitoring_voter
             WHERE tenant_id = $1 AND election_event_id = $2
             ORDER BY voter_id, election_id",
            &params,
        )
        .await
        .context("Failed to read the monitoring voters")?
        .iter()
        .map(|row| {
            let dims: Value = row.get("dims");
            let state: Option<String> = row.get("enrollment_state");
            VoterRow {
                voter_id: row.get("voter_id"),
                election_id: row.get("election_id"),
                region: row.get("region"),
                country: row.get("country"),
                dims: serde_json::from_value(dims).unwrap_or_default(),
                pre_enrolled_at: row.get("pre_enrolled_at"),
                first_voted_at: row.get("first_voted_at"),
                enrollment: match state.as_deref() {
                    Some("PENDING") => Some(Enrollment::Pending),
                    Some("ACCEPTED") => Some(Enrollment::Accepted),
                    Some("REJECTED") => Some(Enrollment::Rejected),
                    _ => None,
                },
                enrollment_reason: row.get("enrollment_reason"),
                enrollment_decided_at: row.get("enrollment_decided_at"),
            }
        })
        .collect();
    let logins = transaction
        .query(
            "SELECT bucket_start, event_type, registration, area_id, attempts
             FROM sequent_backend.monitoring_login_counter
             WHERE tenant_id = $1 AND election_event_id = $2",
            &params,
        )
        .await
        .context("Failed to read the sign-in counters")?
        .iter()
        .map(|row| {
            let registration: String = row.get("registration");
            let attempts: i64 = row.get("attempts");
            LoginRow {
                bucket_start: row.get("bucket_start"),
                event_type: row.get("event_type"),
                registered: registration == "REGISTERED",
                area_id: row.get("area_id"),
                attempts: u64::try_from(attempts).unwrap_or(0),
            }
        })
        .collect();
    let area_elections = load_event_places(transaction, event)
        .await?
        .areas
        .into_iter()
        .map(|(area, (_, elections))| (area, elections))
        .collect();
    Ok(EventFacts {
        settings,
        zone: zone_of(settings),
        posts,
        voters,
        area_elections,
        logins,
    })
}

const TALLY_SESSION_DATA: &str = "miru:tally-session-data";

/// Where each election stands in counting and transmission: tallied once a
/// tally session of it succeeded; transmitted once every package of it was
/// received by as many servers as its threshold asks; failed while one
/// was refused and not yet received enough.
async fn counting_states(
    transaction: &Transaction<'_>,
    event: EventRef,
) -> Result<HashMap<Uuid, PostState>> {
    let rows = transaction
        .query(
            "SELECT election_ids, execution_status, is_execution_completed, annotations
             FROM sequent_backend.tally_session
             WHERE tenant_id = $1 AND election_event_id = $2",
            &[&event.tenant_id, &event.election_event_id],
        )
        .await
        .context("Failed to read the tally sessions")?;
    let mut tallied: BTreeSet<Uuid> = BTreeSet::new();
    let mut packages: HashMap<Uuid, Vec<(bool, bool)>> = HashMap::new();
    for row in rows {
        let elections: Option<Vec<Uuid>> = row.get("election_ids");
        let status: Option<String> = row.get("execution_status");
        let completed: Option<bool> = row.get("is_execution_completed");
        if status.as_deref() == Some("SUCCESS") || completed == Some(true) {
            tallied.extend(elections.unwrap_or_default());
        }
        let annotations: Option<Value> = row.get("annotations");
        let Some(data) = annotations
            .as_ref()
            .and_then(|annotations| annotations.get(TALLY_SESSION_DATA))
            .and_then(Value::as_str)
            .and_then(|text| serde_json::from_str::<MiruTallySessionData>(text).ok())
        else {
            continue;
        };
        for package in data {
            let Ok(election) = package.election_id.parse::<Uuid>() else {
                continue;
            };
            let latest = package
                .documents
                .iter()
                .max_by(|a, b| a.created_at.cmp(&b.created_at));
            let (received, refused) = latest
                .map(|document| {
                    let received: BTreeSet<&str> = document
                        .servers_sent_to
                        .iter()
                        .filter(|sent| sent.status == MiruServerDocumentStatus::SUCCESS)
                        .map(|sent| sent.name.as_str())
                        .collect();
                    let refused = document
                        .servers_sent_to
                        .iter()
                        .any(|sent| sent.status == MiruServerDocumentStatus::ERROR);
                    (received.len() as i64 >= package.threshold.max(1), refused)
                })
                .unwrap_or((false, false));
            packages
                .entry(election)
                .or_default()
                .push((received, refused));
        }
    }
    Ok(tallied
        .into_iter()
        .map(|election| {
            let sent = packages.get(&election);
            let state = match sent {
                Some(sent) if !sent.is_empty() && sent.iter().all(|(received, _)| *received) => {
                    PostState::Transmitted
                }
                Some(sent) if sent.iter().any(|(received, refused)| *refused && !received) => {
                    PostState::TransmissionFailed
                }
                _ => PostState::Tallied,
            };
            (election, state)
        })
        .collect())
}

/// What pruning removed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Pruned {
    pub runs: u64,
    pub figures: u64,
    pub payloads: u64,
}

/// Removes finished runs past `window` (never the shown one), then the
/// figures no remaining complete run holds, then the payloads no figure
/// names. The caller holds the event's lock.
#[instrument(err, skip(hasura))]
pub async fn prune_snapshots(
    hasura: &mut Client,
    event: EventRef,
    window: Duration,
) -> Result<Pruned> {
    let seconds = window.num_seconds() as f64;
    let transaction = hasura.transaction().await?;
    transaction
        .batch_execute("SET CONSTRAINTS ALL IMMEDIATE")
        .await?;
    let runs = transaction
        .execute(
            "DELETE FROM sequent_backend.monitoring_snapshot_run r
             WHERE r.tenant_id = $1 AND r.election_event_id = $2 AND r.status <> 'RUNNING'
               AND COALESCE(r.finished_at, r.started_at) < now() - make_interval(secs => $3)
               AND r.revision IS DISTINCT FROM (
                   SELECT live_snapshot_revision FROM sequent_backend.monitoring_snapshot_state s
                   WHERE s.tenant_id = r.tenant_id AND s.election_event_id = r.election_event_id
               )",
            &[&event.tenant_id, &event.election_event_id, &seconds],
        )
        .await
        .context("Failed to prune runs")?;
    let released: Vec<Vec<u8>> = transaction
        .query(
            "DELETE FROM sequent_backend.monitoring_snapshot_figure f
             WHERE f.tenant_id = $1 AND f.election_event_id = $2
               AND NOT EXISTS (
                   SELECT 1 FROM sequent_backend.monitoring_snapshot_run r
                   WHERE r.tenant_id = f.tenant_id AND r.election_event_id = f.election_event_id
                     AND r.status IN ('COMPLETE', 'RUNNING')
                     AND int8range(f.from_revision, f.to_revision) @> r.revision
               )
               AND f.to_revision IS NOT NULL
             RETURNING f.payload_sha256",
            &[&event.tenant_id, &event.election_event_id],
        )
        .await
        .context("Failed to prune figures")?
        .iter()
        .map(|row| row.get(0))
        .collect::<BTreeSet<Vec<u8>>>()
        .into_iter()
        .collect();
    let payloads = transaction
        .execute(
            "DELETE FROM sequent_backend.monitoring_snapshot_payload p
             WHERE p.tenant_id = $1 AND p.election_event_id = $2 AND p.sha256 = ANY($3)
               AND NOT EXISTS (
                   SELECT 1 FROM sequent_backend.monitoring_snapshot_figure f
                   WHERE f.tenant_id = p.tenant_id AND f.election_event_id = p.election_event_id
                     AND f.payload_sha256 = p.sha256
               )",
            &[&event.tenant_id, &event.election_event_id, &released],
        )
        .await
        .context("Failed to prune payloads")?;
    transaction.commit().await?;
    Ok(Pruned {
        runs,
        figures: released.len() as u64,
        payloads,
    })
}
