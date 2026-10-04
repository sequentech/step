// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Per-Post enrollment windows in the event realm (VOTE-LIFECYCLE §8).
//!
//! The event realm attribute [`ENROLLMENT_WINDOWS_ATTRIBUTE`] holds one
//! window per `embassy` user-profile option: the Post (election) that option
//! maps to, when its enrollment opens and closes, and the Post's zone. The
//! keycloak-extensions form action `enrollment-window-check` refuses a
//! registration for a Post outside its window, and `register.ftl` shows the
//! times. The scheduler (stream B) calls [`refresh`] after saving or running
//! an enrollment event; publishing calls it too, so a changed Post zone
//! reaches the enrollment pages with the publication.
//!
//! - Opening: the Post's own START_ENROLLMENT_PERIOD, else the event-wide one.
//! - Closing: the event-wide END_ENROLLMENT_PERIOD (it stays event-wide).
//! - Zones: the opening in the Post's effective zone, the common close in the
//!   event's primary zone (`sequent_core::time_zones`).
//! - An option that can't be mapped to exactly one Post is written as a
//!   problem, and the form action refuses it: nothing passes unchecked.

use crate::postgres::election::get_elections;
use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::scheduled_event::find_scheduled_event_by_election_event_id;
use crate::services::database::get_hasura_pool;
use crate::services::election_event_board::get_election_event_board;
use crate::services::electoral_log::ElectoralLog;
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, SecondsFormat, Utc};
use deadpool_postgres::Transaction;
use sequent_core::ballot::{ElectionEventPresentation, ElectionPresentation};
use sequent_core::serialization::deserialize_with_path::deserialize_value;
use sequent_core::services::keycloak::{
    get_event_realm, update_realm_attributes, KeycloakAdminClient,
};
use sequent_core::services::uuid_validation::parse_uuid_v4;
use sequent_core::time_zones::{effective_time_zone, primary_time_zone};
use sequent_core::types::keycloak::UserProfileAttribute;
use sequent_core::types::scheduled_event::{EventProcessors, ScheduledEvent};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use tracing::{error, info, instrument, warn};

/// The event realm attribute the form action and `register.ftl` read.
pub const ENROLLMENT_WINDOWS_ATTRIBUTE: &str = "enrollment_windows";

/// A non-object JSON value makes Keycloak refuse every Post, including a
/// missing selection. Blank values or objects with unknown keys do not.
const ENROLLMENT_SYNC_PAUSED: &str = "null";

/// The user-profile attribute whose option values key the windows.
pub const POST_ATTRIBUTE: &str = "embassy";

/// The electoral log event type of a refresh that found Post problems.
pub const PROBLEM_LOG_EVENT_TYPE: &str = "enrollment-windows-problem";

/// One Post's enrollment window. Instants are RFC 3339 UTC; `None` means the
/// schedule has no such row, so that side is not limited. The opening shows
/// in the Post's zone (`time_zone`); the common close shows in the event's
/// primary zone (`close_time_zone`), as on the voter screens.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct EnrollmentWindow {
    pub election_id: String,
    pub opens_at: Option<String>,
    pub closes_at: Option<String>,
    pub time_zone: String,
    pub close_time_zone: String,
}

/// Why an `embassy` option has no window although the event has an
/// enrollment schedule. The form action refuses these options
/// (`enrollment.postNotConfigured`): enrollment there can't be checked.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum PostProblemKind {
    /// The option matches areas of more than one election.
    AmbiguousPost,
    /// The option matches no area that votes in an election.
    NoPost,
}

/// What the attribute holds for one option.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(untagged)]
pub enum WindowEntry {
    Window(EnrollmentWindow),
    Problem { problem: PostProblemKind },
}

/// An option that couldn't be mapped to one Post.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct PostProblem {
    pub option: String,
    pub kind: PostProblemKind,
    /// The elections an ambiguous option matches.
    pub election_ids: Vec<String>,
}

/// What [`refresh`] wrote: how many windows, and the options it refuses
/// because they can't be mapped to one Post.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RefreshReport {
    pub windows: usize,
    pub problems: Vec<PostProblem>,
}

/// How an `embassy` option maps to the event's Posts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PostMatch {
    Election(String),
    Problem(PostProblem),
}

/// The attribute's entries and the options with a problem.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ComputedWindows {
    pub entries: BTreeMap<String, WindowEntry>,
    pub problems: Vec<PostProblem>,
}

/// An area of the event (a country under a Post) and the election it votes
/// in. `description` holds the Post name the `embassy` options come from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostArea {
    pub name: String,
    pub description: String,
    pub election_id: String,
}

/// An election and its presentation (for its zone).
#[derive(Debug, Clone)]
pub struct PostElection {
    pub id: String,
    pub presentation: Option<ElectionPresentation>,
}

/// Rewrites the event realm's `enrollment_windows` attribute from the
/// event's enrollment schedule. With no enrollment schedule, the attribute is
/// removed and only the realm's registration switch applies. Options that
/// can't be mapped to one Post are written as problems (refused), logged at
/// ERROR, posted to the electoral log and returned.
#[instrument(skip(hasura_transaction), err)]
pub async fn refresh(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<RefreshReport> {
    let election_event = get_election_event_by_id(hasura_transaction, tenant_id, election_event_id)
        .await
        .with_context(|| "Error obtaining the election event")?;
    let event_presentation: Option<ElectionEventPresentation> = election_event
        .presentation
        .clone()
        .map(deserialize_value)
        .transpose()
        .map_err(|err| anyhow!("Error parsing the event presentation: {err:?}"))?;

    let elections: Vec<PostElection> =
        get_elections(hasura_transaction, tenant_id, election_event_id)
            .await
            .with_context(|| "Error obtaining the elections")?
            .into_iter()
            .map(|election| PostElection {
                presentation: election.get_presentation(),
                id: election.id,
            })
            .collect();

    let scheduled_events =
        find_scheduled_event_by_election_event_id(hasura_transaction, tenant_id, election_event_id)
            .await
            .with_context(|| "Error obtaining the scheduled events")?;

    let areas = get_post_areas(hasura_transaction, tenant_id, election_event_id).await?;

    let realm = get_event_realm(tenant_id, election_event_id);
    let profile_attributes = KeycloakAdminClient::new()
        .await?
        .get_user_profile_attributes(&realm)
        .await
        .with_context(|| "Error reading the event realm's user profile")?;
    let options = post_options(&profile_attributes);

    let computed = compute_windows(
        event_presentation.as_ref(),
        &elections,
        &scheduled_events,
        &options,
        &areas,
    );
    let report = RefreshReport {
        windows: computed
            .entries
            .values()
            .filter(|entry| matches!(entry, WindowEntry::Window(_)))
            .count(),
        problems: computed.problems.clone(),
    };
    info!(
        "Writing {} enrollment windows and {} refused options to realm {realm}",
        report.windows,
        computed.entries.len() - report.windows
    );

    let value = if computed.entries.is_empty() {
        // An empty value removes the attribute.
        String::new()
    } else {
        serde_json::to_string(&computed.entries)?
    };
    update_realm_attributes(
        tenant_id,
        election_event_id,
        HashMap::from([(ENROLLMENT_WINDOWS_ATTRIBUTE.to_string(), value)]),
    )
    .await?;

    if !report.problems.is_empty() {
        let problems = serde_json::to_string(&report.problems)?;
        error!(
            "Event {election_event_id}: {} {POST_ATTRIBUTE} options can't be mapped to one Post, \
             so registrations for them are refused: {problems}",
            report.problems.len()
        );
        if let Err(err) = log_problems(
            hasura_transaction,
            tenant_id,
            election_event_id,
            election_event.bulletin_board_reference.clone(),
            problems,
        )
        .await
        {
            error!("Event {election_event_id}: the Post problems were not posted to the electoral log: {err:?}");
        }
    }
    Ok(report)
}

/// Pause registration immediately before committing enrollment schedule
/// changes, while the caller still holds the scheduling lock. Run validation,
/// schedule writes and domain/audit checks first, then invoke this and abort if
/// the realm cannot be paused. Uncommitted edits leave the old committed
/// schedule authoritative. Once paused, rollback or crash deliberately keeps
/// enrollment paused until synchronization is retried.
#[instrument(skip(hasura_transaction), err)]
pub async fn begin_synchronization(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<()> {
    crate::postgres::scheduled_event::lock_scheduling_event(
        hasura_transaction,
        tenant_id,
        election_event_id,
    )
    .await?;
    update_realm_attributes(
        tenant_id,
        election_event_id,
        HashMap::from([(
            ENROLLMENT_WINDOWS_ATTRIBUTE.to_string(),
            ENROLLMENT_SYNC_PAUSED.to_string(),
        )]),
    )
    .await
    .context("Could not pause enrollment; no enrollment schedule changes were saved")
}

/// Restore registration only from committed schedules, under the same lock
/// used by writers. Call after committing the schedule transaction. A failed
/// synchronization must be retried after its underlying problem is corrected;
/// never clear the pause with windows read from an uncommitted transaction.
#[instrument(err)]
pub async fn complete_synchronization(
    tenant_id: &str,
    election_event_id: &str,
) -> Result<RefreshReport> {
    let result: Result<RefreshReport> = async {
        let mut client = get_hasura_pool().await.get().await?;
        let transaction = client.transaction().await?;
        crate::postgres::scheduled_event::lock_scheduling_event(
            &transaction,
            tenant_id,
            election_event_id,
        )
        .await?;
        let report = refresh_in_savepoint(&transaction, tenant_id, election_event_id).await?;
        transaction.commit().await?;
        Ok(report)
    }
    .await;
    if result.is_err() {
        // A lost response may have applied the remote update. Reinstall the
        // denial marker; this operation only denies enrollment and never
        // installs possibly stale allowing windows.
        update_realm_attributes(
            tenant_id,
            election_event_id,
            HashMap::from([(
                ENROLLMENT_WINDOWS_ATTRIBUTE.to_string(),
                ENROLLMENT_SYNC_PAUSED.to_string(),
            )]),
        )
        .await
        .context("Schedule saved, but enrollment synchronization and its pause could not be confirmed; retry synchronization after correcting the failure")?;
    }
    result.context("Schedule saved but enrollment remains paused; retry after correcting the synchronization failure")
}

/// [`refresh`] inside a savepoint of the caller's transaction, so a failure
/// can't abort the rest of it (a publication, a scheduled event). The caller
/// logs the error.
pub async fn refresh_in_savepoint(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<RefreshReport> {
    hasura_transaction
        .batch_execute("SAVEPOINT enrollment_windows")
        .await
        .map_err(|err| anyhow!("SAVEPOINT: {err}"))?;
    let result = refresh(hasura_transaction, tenant_id, election_event_id).await;
    let end = if result.is_ok() {
        "RELEASE SAVEPOINT enrollment_windows"
    } else {
        "ROLLBACK TO SAVEPOINT enrollment_windows"
    };
    match hasura_transaction.batch_execute(end).await {
        Ok(()) => result,
        Err(err) => result.and(Err(anyhow!("{end}: {err}"))),
    }
}

async fn log_problems(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    bulletin_board_reference: Option<serde_json::Value>,
    problems: String,
) -> Result<()> {
    let board = get_election_event_board(bulletin_board_reference)
        .ok_or_else(|| anyhow!("The event has no bulletin board"))?;
    let electoral_log = ElectoralLog::new(
        hasura_transaction,
        tenant_id,
        Some(election_event_id),
        &board,
    )
    .await?;
    electoral_log
        .post_keycloak_event(
            election_event_id.to_string(),
            PROBLEM_LOG_EVENT_TYPE.to_string(),
            format!(
                "Registrations are refused for {POST_ATTRIBUTE} options that can't be mapped \
                 to one Post: {problems}"
            ),
            None,
            None,
        )
        .await
}

/// The attribute's entries keyed by `embassy` option value. Without any
/// enrollment schedule there is nothing to enforce and no entry is written;
/// the problems are still returned.
pub fn compute_windows(
    event: Option<&ElectionEventPresentation>,
    elections: &[PostElection],
    scheduled_events: &[ScheduledEvent],
    options: &[String],
    areas: &[PostArea],
) -> ComputedWindows {
    let by_election = election_windows(event, elections, scheduled_events);
    let matches = post_elections(options, areas);
    let problems: Vec<PostProblem> = matches
        .values()
        .filter_map(|post| match post {
            PostMatch::Problem(problem) => Some(problem.clone()),
            PostMatch::Election(_) => None,
        })
        .collect();
    if by_election.is_empty() {
        return ComputedWindows {
            entries: BTreeMap::new(),
            problems,
        };
    }
    let entries = matches
        .into_iter()
        .filter_map(|(option, post)| match post {
            // A Post without any enrollment row has no window to enforce.
            PostMatch::Election(election_id) => by_election
                .get(&election_id)
                .map(|window| (option, WindowEntry::Window(window.clone()))),
            PostMatch::Problem(problem) => Some((
                option,
                WindowEntry::Problem {
                    problem: problem.kind,
                },
            )),
        })
        .collect();
    ComputedWindows { entries, problems }
}

/// The window of every election that has an enrollment start or end.
pub fn election_windows(
    event: Option<&ElectionEventPresentation>,
    elections: &[PostElection],
    scheduled_events: &[ScheduledEvent],
) -> BTreeMap<String, EnrollmentWindow> {
    let event_start = latest(
        scheduled_events,
        EventProcessors::START_ENROLLMENT_PERIOD,
        None,
    );
    let event_end = latest(
        scheduled_events,
        EventProcessors::END_ENROLLMENT_PERIOD,
        None,
    );
    for row in scheduled_events.iter().filter(|row| {
        row.archived_at.is_none()
            && row.event_processor == Some(EventProcessors::END_ENROLLMENT_PERIOD)
            && payload_election_id(row).is_some()
    }) {
        warn!(
            "Scheduled event {} closes enrollment for one election, but the enrollment close is \
             event-wide: it doesn't change the enrollment windows",
            row.id
        );
    }

    elections
        .iter()
        .filter_map(|election| {
            let opens_at = latest(
                scheduled_events,
                EventProcessors::START_ENROLLMENT_PERIOD,
                Some(&election.id),
            )
            .or_else(|| event_start.clone());
            let closes_at = event_end.clone();
            if opens_at.is_none() && closes_at.is_none() {
                return None;
            }
            Some((
                election.id.clone(),
                EnrollmentWindow {
                    election_id: election.id.clone(),
                    opens_at,
                    closes_at,
                    time_zone: effective_time_zone(event, election.presentation.as_ref()),
                    close_time_zone: primary_time_zone(event),
                },
            ))
        })
        .collect()
}

/// Which election each `embassy` option belongs to. An area whose name or
/// description equals the option (ignoring case) decides; failing that, the
/// areas whose description contains it, the rule the enrollment application
/// uses to find the voter's area
/// (`postgres::application::get_permission_label_from_post`). Either way the
/// areas must vote in exactly one election, else the option is a problem.
pub fn post_elections(options: &[String], areas: &[PostArea]) -> BTreeMap<String, PostMatch> {
    options
        .iter()
        .map(|option| {
            let needle = option.trim().to_lowercase();
            let elections_of = |matches: &dyn Fn(&PostArea) -> bool| -> BTreeSet<String> {
                areas
                    .iter()
                    .filter(|area| matches(area))
                    .map(|area| area.election_id.clone())
                    .collect()
            };
            let mut elections = elections_of(&|area| {
                area.name.trim().to_lowercase() == needle
                    || area.description.trim().to_lowercase() == needle
            });
            if elections.is_empty() {
                elections = elections_of(&|area| area.description.to_lowercase().contains(&needle));
            }
            let post = match elections.len() {
                1 => PostMatch::Election(elections.into_iter().next().unwrap_or_default()),
                0 => PostMatch::Problem(PostProblem {
                    option: option.clone(),
                    kind: PostProblemKind::NoPost,
                    election_ids: vec![],
                }),
                _ => PostMatch::Problem(PostProblem {
                    option: option.clone(),
                    kind: PostProblemKind::AmbiguousPost,
                    election_ids: elections.into_iter().collect(),
                }),
            };
            (option.clone(), post)
        })
        .collect()
}

/// The `options` validation of the `embassy` user-profile attribute.
pub fn post_options(attributes: &[UserProfileAttribute]) -> Vec<String> {
    attributes
        .iter()
        .find(|attribute| attribute.name.as_deref() == Some(POST_ATTRIBUTE))
        .and_then(|attribute| attribute.validations.as_ref())
        .and_then(|validations| validations.get("options"))
        .and_then(|options| options.get("options"))
        .and_then(|options| options.as_array())
        .map(|options| {
            options
                .iter()
                .filter_map(|option| option.as_str())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// The instant (RFC 3339 UTC) of the newest active row of `processor` for
/// `election_id` (`None` = event-wide rows).
fn latest(
    scheduled_events: &[ScheduledEvent],
    processor: EventProcessors,
    election_id: Option<&str>,
) -> Option<String> {
    let mut rows: Vec<&ScheduledEvent> = scheduled_events
        .iter()
        .filter(|row| row.archived_at.is_none())
        .filter(|row| row.event_processor.as_ref() == Some(&processor))
        .filter(|row| payload_election_id(row).as_deref() == election_id)
        .collect();
    rows.sort_by_key(|row| row.created_at);
    if rows.len() > 1 {
        warn!(
            "{} {processor} rows for {election_id:?}: the newest one sets the enrollment window",
            rows.len()
        );
    }
    let row = rows.last()?;
    let scheduled_date = row
        .cron_config
        .as_ref()
        .and_then(|config| config.scheduled_date.as_deref())?;
    match DateTime::parse_from_rfc3339(scheduled_date) {
        Ok(instant) => Some(
            instant
                .with_timezone(&Utc)
                .to_rfc3339_opts(SecondsFormat::Secs, true),
        ),
        Err(err) => {
            warn!(
                "Scheduled event {} has no RFC 3339 instant ({scheduled_date:?}: {err}); \
                 it doesn't set an enrollment window",
                row.id
            );
            None
        }
    }
}

fn payload_election_id(row: &ScheduledEvent) -> Option<String> {
    row.event_payload
        .as_ref()
        .and_then(|payload| payload.get("election_id"))
        .and_then(|election_id| election_id.as_str())
        .map(str::to_string)
}

/// The event's areas that vote in an election, with their descriptions.
#[instrument(skip(hasura_transaction), err)]
async fn get_post_areas(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<Vec<PostArea>> {
    let statement = hasura_transaction
        .prepare(
            r#"
            SELECT DISTINCT a.name, a.description, con.election_id
            FROM sequent_backend.area a
                JOIN sequent_backend.area_contest ac
                    ON ac.area_id = a.id
                    AND ac.tenant_id = a.tenant_id
                    AND ac.election_event_id = a.election_event_id
                JOIN sequent_backend.contest con
                    ON con.id = ac.contest_id
                    AND con.tenant_id = a.tenant_id
                    AND con.election_event_id = a.election_event_id
            WHERE
                a.tenant_id = $1
                AND a.election_event_id = $2
                AND a.description IS NOT NULL
            "#,
        )
        .await
        .map_err(|err| anyhow!("Error preparing the post areas query: {err}"))?;
    let rows = hasura_transaction
        .query(
            &statement,
            &[
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error querying the post areas: {err}"))?;
    rows.into_iter()
        .map(|row| -> Result<PostArea> {
            Ok(PostArea {
                name: row
                    .try_get::<_, Option<String>>("name")?
                    .unwrap_or_default(),
                description: row.try_get("description")?,
                election_id: row.try_get::<_, uuid::Uuid>("election_id")?.to_string(),
            })
        })
        .collect()
}

#[cfg(test)]
#[path = "enrollment_windows_tests.rs"]
mod enrollment_windows_tests;
