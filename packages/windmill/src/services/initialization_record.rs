// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! What a completed initialization report records (VOTE-LIFECYCLE design §9):
//! a dated row per country (area) of the Post it covered, with the report's
//! hash and document, and the Post's `initialization_report_generated` flag;
//! then, after the commit, one `ElectionInitialized` electoral log step per
//! row, staged in the signing log outbox.

use crate::adapters::tally_ceremony::PgTallyCreationReader;
use crate::domain::tally_ceremony::{EXECUTER_USERNAME_ANNOTATION, EXECUTER_USER_ID_ANNOTATION};
use crate::ports::tally_ceremony::TallyCreationReader;
use crate::postgres::election::set_election_initialization_report_generated;
use crate::postgres::election_initialization::{
    find_post_report, insert_election_initialization, list_election_initializations,
    list_post_ballot_style_areas, lock_post_for_initialization, lock_unstaged_initializations,
    mark_initialization_log_staged, ElectionInitialization, NewElectionInitialization,
};
use crate::postgres::signing::lock_signing_event;
use crate::postgres::tally_session::get_tally_session_by_id;
use crate::postgres::tally_session_execution::get_last_tally_session_execution;
use crate::services::initialization_scope::{post_area_ids, post_display_name};
use crate::services::signing::log::{stage_step, Actor, LogScope, LogStep, SystemOutcome};
use anyhow::{anyhow, Result};
use deadpool_postgres::{Client, Transaction};
use electoral_log::messages::newtypes::SigningStatementKind;
use sequent_core::types::hasura::core::TallySession;
use sequent_core::types::results::ResultDocuments;
use serde_json::{json, Value};
use std::collections::BTreeSet;
use tracing::{info, instrument, warn};
use uuid::Uuid;

/// The tally session annotation that lists the countries an area-filtered
/// initialization report covers. A session without it covers the Posts.
pub const INITIALIZATION_AREA_IDS_ANNOTATION: &str = "initialization_area_ids";

/// The countries an area-filtered initialization session covers, if it is one.
pub fn initialization_area_filter(tally_session: &TallySession) -> Option<Vec<String>> {
    tally_session
        .annotations
        .as_ref()
        .and_then(|annotations| annotations.get(INITIALIZATION_AREA_IDS_ANNOTATION))
        .and_then(|value| serde_json::from_value(value.clone()).ok())
}

/// What a completed initialization report records for one Post.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitializationPlan {
    /// One row per country the report covered; `[None]` for a Post without
    /// countries.
    pub areas: Vec<Option<String>>,
    /// Whether the Post is initialized afterwards. A report of the whole
    /// Post initializes it, as always; an area-filtered one once every
    /// country of the Post has been initialized.
    pub post_initialized: bool,
}

/// What to record for a Post with countries `post_areas` (some already
/// initialized) when a report covering `session_areas` completes.
pub fn initialization_plan(
    post_areas: &BTreeSet<String>,
    session_areas: &[String],
    area_filtered: bool,
    already_initialized: &BTreeSet<String>,
) -> InitializationPlan {
    if post_areas.is_empty() {
        return InitializationPlan {
            areas: vec![None],
            post_initialized: true,
        };
    }
    let covered: BTreeSet<&String> = post_areas
        .iter()
        .filter(|area| session_areas.contains(area))
        .collect();
    let post_initialized = !area_filtered
        || post_areas
            .iter()
            .all(|area| covered.contains(area) || already_initialized.contains(area));
    InitializationPlan {
        areas: covered.into_iter().cloned().map(Some).collect(),
        post_initialized,
    }
}

fn annotation(annotations: &Option<Value>, key: &str) -> Option<String> {
    annotations
        .as_ref()
        .and_then(|annotations| annotations.get(key))
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

/// The hash and document of the Post's report in the session's newest
/// results; `(None, None)` with a warning when there is none (the log
/// entry then says the hash is missing).
async fn report_of(
    hasura_transaction: &Transaction<'_>,
    tenant: Uuid,
    election: Uuid,
    results_event_id: Option<Uuid>,
) -> Result<(Option<String>, Option<Uuid>)> {
    let Some(results_event_id) = results_event_id else {
        warn!("Initialization of {election}: the tally has no results, so no report hash");
        return Ok((None, None));
    };
    let Some((annotations, documents)) =
        find_post_report(hasura_transaction, tenant, election, results_event_id).await?
    else {
        warn!("Initialization of {election}: results {results_event_id} have no report of it");
        return Ok((None, None));
    };
    let report_hash = annotation(&annotations, "results_hash");
    let document_id = documents
        .and_then(|documents| serde_json::from_value::<ResultDocuments>(documents).ok())
        .and_then(|documents| documents.pdf.or(documents.html))
        .and_then(|id| Uuid::parse_str(&id).ok());
    if report_hash.is_none() {
        warn!("Initialization of {election}: the report has no hash");
    }
    Ok((report_hash, document_id))
}

/// Stores the initialization of `election_id` by the completed
/// initialization report tally `tally_session_id`: one row per country (see
/// [`initialization_plan`]) and the Post's flag. Under a lock on the Post,
/// so reports of its countries completing at once see each other's rows. A
/// session already recorded for the Post (a re-run of its tally) records
/// nothing again. The electoral log entries are staged after the commit
/// ([`stage_initialization_log`]).
#[instrument(skip(hasura_transaction), err)]
pub async fn store_initialization(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    tally_session_id: &str,
    election_id: &str,
) -> Result<Vec<ElectionInitialization>> {
    let tenant = Uuid::parse_str(tenant_id)?;
    let event = Uuid::parse_str(election_event_id)?;
    let election = Uuid::parse_str(election_id)?;
    let session = Uuid::parse_str(tally_session_id)?;
    lock_post_for_initialization(hasura_transaction, tenant, event, election).await?;
    let rows = list_election_initializations(hasura_transaction, tenant, event).await?;
    let recorded: Vec<ElectionInitialization> = rows
        .iter()
        .filter(|row| row.election_id == election && row.tally_session_id == session)
        .cloned()
        .collect();
    if !recorded.is_empty() {
        info!("Initialization of {election_id} by {tally_session_id} already recorded");
        return Ok(recorded);
    }
    let already_initialized: BTreeSet<String> = rows
        .iter()
        .filter(|row| row.election_id == election)
        .filter_map(|row| row.area_id.map(|id| id.to_string()))
        .collect();

    let tally_session = get_tally_session_by_id(
        hasura_transaction,
        tenant_id,
        election_event_id,
        tally_session_id,
    )
    .await?;
    let results_event_id = get_last_tally_session_execution(
        hasura_transaction,
        tenant_id,
        election_event_id,
        tally_session_id,
    )
    .await?
    .and_then(|execution| execution.results_event_id)
    .map(|id| Uuid::parse_str(&id))
    .transpose()?;
    let snapshot = PgTallyCreationReader::new(hasura_transaction)
        .event_snapshot(tenant_id, election_event_id)
        .await?;
    let post = snapshot
        .elections
        .iter()
        .find(|post| post.id == election_id)
        .ok_or_else(|| anyhow!("Post {election_id} not found"))?;
    let styled = list_post_ballot_style_areas(hasura_transaction, tenant, event).await?;
    let post_areas = post_area_ids(
        &snapshot.areas,
        &snapshot.area_contests,
        &snapshot.contests,
        std::slice::from_ref(&post.id),
        &styled,
    )?;
    let plan = initialization_plan(
        &post_areas,
        &tally_session.area_ids.clone().unwrap_or_default(),
        initialization_area_filter(&tally_session).is_some(),
        &already_initialized,
    );
    let (report_hash, document_id) =
        report_of(hasura_transaction, tenant, election, results_event_id).await?;
    anyhow::ensure!(
        report_hash.is_some() && document_id.is_some(),
        "The initialization report is missing its hash or document; voting remains uninitialized"
    );
    let created_by = annotation(&tally_session.annotations, EXECUTER_USER_ID_ANNOTATION);
    let created_by_username = annotation(&tally_session.annotations, EXECUTER_USERNAME_ANNOTATION);
    let election_name = post_display_name(post, &snapshot.election_event.get_default_language());

    let mut stored = Vec::new();
    for area_id in &plan.areas {
        let area_name = area_id.as_ref().map(|area_id| {
            snapshot
                .areas
                .iter()
                .find(|area| area.id == *area_id)
                .and_then(|area| area.name.clone())
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| area_id.clone())
        });
        stored.push(
            insert_election_initialization(
                hasura_transaction,
                &NewElectionInitialization {
                    tenant_id: tenant,
                    election_event_id: event,
                    election_id: election,
                    area_id: area_id.as_deref().map(Uuid::parse_str).transpose()?,
                    tally_session_id: session,
                    results_event_id,
                    report_hash: report_hash.clone(),
                    document_id,
                    created_by: created_by.clone(),
                    created_by_username: created_by_username.clone(),
                    election_name: Some(election_name.clone()),
                    area_name,
                },
            )
            .await?,
        );
    }
    if plan.post_initialized {
        crate::postgres::trusted_write(hasura_transaction).await?;
        set_election_initialization_report_generated(
            hasura_transaction,
            tenant_id,
            election_event_id,
            election_id,
            &true,
        )
        .await?;
    }
    Ok(stored)
}

/// The electoral log step of a recorded initialization: one
/// `ElectionInitialized` USER entry (the person who generated the report)
/// and one SYSTEM entry, filed under the Post and the country. A missing
/// report hash makes the SYSTEM entry an error that says so.
pub fn initialization_log_step(row: &ElectionInitialization) -> LogStep {
    let post = row
        .election_name
        .clone()
        .unwrap_or_else(|| row.election_id.to_string());
    let place = match (&row.area_id, &row.area_name) {
        (Some(area_id), name) => format!(
            "Post {post}, country {}",
            name.clone().unwrap_or_else(|| area_id.to_string())
        ),
        (None, _) => format!("Post {post}"),
    };
    let initialized_at = row.created_at.to_rfc3339();
    let (description, system) = match &row.report_hash {
        Some(hash) => (
            format!("Initialized {place} at {initialized_at} with initialization report hash {hash}."),
            SystemOutcome::Info,
        ),
        None => (
            format!(
                "Initialized {place} at {initialized_at}; the initialization report hash is missing (the report's results have none)."
            ),
            SystemOutcome::Error,
        ),
    };
    LogStep {
        kind: SigningStatementKind::ElectionInitialized,
        user: Actor {
            user_id: row.created_by.clone().unwrap_or_default(),
            username: row.created_by_username.clone().unwrap_or_default(),
        },
        system,
        scope: LogScope {
            tenant_id: row.tenant_id,
            election_event_id: row.election_event_id,
            election_id: Some(row.election_id),
            area_id: row.area_id,
        },
        description,
        details: json!({
            "election_id": row.election_id,
            "election_name": row.election_name,
            "area_id": row.area_id,
            "area_name": row.area_name,
            "report_hash": row.report_hash,
            "document_id": row.document_id,
            "tally_session_id": row.tally_session_id,
            "results_event_id": row.results_event_id,
            "initialized_at": initialized_at,
            "initialization_id": row.id,
        }),
    }
}

/// Stages the electoral log entries of the event's recorded initializations
/// that have none yet, in a transaction of its own that takes the event's
/// signing lock first (the tally's transaction holds tally locks, so it
/// doesn't take it). Each row's step id is the row's id, so an entry is
/// staged once; rows left by a crash before this runs are staged by the
/// next call for the event. Returns how many it staged.
#[instrument(skip(hasura_client), err)]
pub async fn stage_initialization_log(
    hasura_client: &mut Client,
    tenant_id: Uuid,
    election_event_id: Uuid,
) -> Result<usize> {
    let transaction = hasura_client.transaction().await?;
    lock_signing_event(&transaction, tenant_id, election_event_id).await?;
    let rows = lock_unstaged_initializations(&transaction, tenant_id, election_event_id).await?;
    for row in &rows {
        stage_step(&transaction, &initialization_log_step(row), row.id).await?;
        mark_initialization_log_staged(&transaction, row.id).await?;
    }
    if let Some(row) = rows.first() {
        // This after-commit transaction holds no tally locks; signing always comes first.
        crate::services::scheduled_outcome::recompute_predictions(
            &transaction,
            &tenant_id.to_string(),
            &election_event_id.to_string(),
            &Actor {
                user_id: row
                    .created_by
                    .clone()
                    .unwrap_or_else(|| "initialization".into()),
                username: row
                    .created_by_username
                    .clone()
                    .unwrap_or_else(|| "initialization".into()),
            },
        )
        .await?;
    }
    transaction.commit().await?;
    Ok(rows.len())
}

#[cfg(test)]
#[path = "initialization_record_tests.rs"]
mod tests;
