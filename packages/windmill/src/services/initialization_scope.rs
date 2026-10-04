// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! When a Post may open with respect to its initialization (VOTE-LIFECYCLE
//! design §9), at the event's `lifecycle_policies.initialization_scope`:
//!
//! - `POST`: the Post is initialized (its initialization report exists);
//! - `EVENT`: besides, every Post of the event that requires its report is;
//! - `POST_AND_COUNTRY`: besides, every country (area) under the Post is.
//!
//! Only Posts whose `initialization_report_policy` is `REQUIRED` wait for
//! initialization; the others neither wait nor are waited for. The scope is
//! configuration in both the published and the current copy (design §5b):
//! a Post opens only when the requirement of every copy holds.

use crate::adapters::tally_ceremony::PgTallyCreationReader;
use crate::ports::tally_ceremony::TallyCreationReader;
use crate::postgres::election_initialization::{
    list_election_initializations, list_post_ballot_style_areas,
};
use crate::services::ceremonies::tally_validation::TallyValidationError;
use crate::services::election_event_status::TransitionRefusal;
use crate::services::scheduled_outcome::EventState;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::ballot::{EInitializeReportPolicy, InitializationScope};
use sequent_core::services::area_tree::TreeNode;
use sequent_core::services::translations::{Alias, Name};
use sequent_core::time_zones::lifecycle_policies;
use sequent_core::types::ceremonies::TallyType;
use sequent_core::types::hasura::core::{Area, AreaContest, Contest, Election, ElectionEvent};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use tracing::instrument;
use uuid::Uuid;

/// The initialization scope in each copy of the configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeCopies {
    /// The event's presentation as it is now.
    pub current: InitializationScope,
    /// The newest publication's snapshot; `None` while nothing is published
    /// (the default, `POST`, applies, which adds nothing to today's gate).
    pub published: Option<InitializationScope>,
}

impl ScopeCopies {
    /// Every scope a Post's opening must satisfy.
    pub fn scopes(&self) -> Vec<InitializationScope> {
        let mut scopes = vec![self.current.clone()];
        if let Some(published) = &self.published {
            if *published != self.current {
                scopes.push(published.clone());
            }
        }
        scopes
    }

    /// Whether any copy asks for more than the Post itself.
    pub fn beyond_post(&self) -> bool {
        self.scopes()
            .iter()
            .any(|scope| *scope != InitializationScope::POST)
    }

    /// Whether any copy initializes country by country.
    pub fn per_country(&self) -> bool {
        self.scopes()
            .contains(&InitializationScope::POST_AND_COUNTRY)
    }
}

/// The scope copies of an event, from its current presentation.
///
/// The published copy is the newest publication's lifecycle snapshot
/// (stream B2, `LifecycleSnapshot.policies.initialization_scope`); until it
/// is recorded there is none.
pub fn scope_copies_of(election_event: &ElectionEvent) -> Result<ScopeCopies> {
    let presentation = election_event
        .get_presentation()
        .map_err(|error| anyhow!("Error reading the event presentation: {error:?}"))?;
    Ok(ScopeCopies {
        current: lifecycle_policies(presentation.as_ref()).initialization_scope,
        published: None,
    })
}

/// The scope copies of an event; see [`scope_copies_of`].
#[instrument(skip(hasura_transaction), err)]
pub async fn initialization_scope_for(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<ScopeCopies> {
    initialization_scope_for_target(hasura_transaction, tenant_id, election_event_id, None).await
}

/// Resolve each Post's published scope independently from trusted snapshots.
pub async fn initialization_scope_for_target(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    post: Option<Uuid>,
) -> Result<ScopeCopies> {
    Ok(EventState::read(
        hasura_transaction,
        Uuid::parse_str(tenant_id)?,
        Uuid::parse_str(election_event_id)?,
    )
    .await?
    .initialization_scopes(post))
}

/// Per-Post publication takes precedence only when it is the newest applicable snapshot.
pub async fn initialization_scope_for_post(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    post: Uuid,
) -> Result<ScopeCopies> {
    initialization_scope_for_target(hasura_transaction, tenant_id, election_event_id, Some(post))
        .await
}

/// The countries (areas) of the Posts: the areas where their voters vote,
/// which are those with a ballot style of one of the Posts (`styled`:
/// (Post, area) pairs) among the areas holding one of their contests,
/// directly or through a parent area. Grouping areas without a ballot style
/// aren't countries.
pub fn post_area_ids(
    areas: &[Area],
    area_contests: &[AreaContest],
    contests: &[Contest],
    election_ids: &[String],
    styled: &[(String, String)],
) -> Result<BTreeSet<String>> {
    let contest_ids: HashSet<String> = contests
        .iter()
        .filter(|contest| election_ids.contains(&contest.election_id))
        .map(|contest| contest.id.clone())
        .collect();
    let area_contests: Vec<AreaContest> = area_contests
        .iter()
        .filter(|area_contest| contest_ids.contains(&area_contest.contest_id))
        .cloned()
        .collect();
    let voted_in: HashSet<&String> = styled
        .iter()
        .filter(|(election_id, _)| election_ids.contains(election_id))
        .map(|(_, area_id)| area_id)
        .collect();
    let tree = TreeNode::<()>::from_areas(areas.iter().map(Into::into).collect())?;
    Ok(tree
        .get_contests_data_tree(&area_contests)
        .get_contest_matches(&contest_ids)
        .into_iter()
        .map(|area_contest| area_contest.area_id)
        .filter(|area_id| voted_in.contains(area_id))
        .collect())
}

/// At most [`NAMES_SHOWN`] names, then how many more.
pub const NAMES_SHOWN: usize = 5;

/// Names for a message: the first [`NAMES_SHOWN`], then "and N more".
pub fn name_list(names: &[String]) -> String {
    if names.len() <= NAMES_SHOWN {
        return names.join(", ");
    }
    format!(
        "{} and {} more",
        names[..NAMES_SHOWN].join(", "),
        names.len() - NAMES_SHOWN
    )
}

/// A Post's display name: its name in `language`, else its alias, else its id.
pub fn post_display_name(election: &Election, language: &str) -> String {
    Some(election.get_name(language))
        .filter(|name| !name.is_empty() && name != "-")
        .or_else(|| {
            election
                .get_alias(language)
                .filter(|alias| !alias.is_empty() && alias != "-")
        })
        .unwrap_or_else(|| election.id.clone())
}

/// Refuses an area filter on a tally unless it initializes one Post
/// country by country: an initialization report, of one Post, while a copy
/// of the initialization scope is POST_AND_COUNTRY, for countries of the Post.
pub fn check_initialization_area_filter(
    tally_type: TallyType,
    election_ids: &[String],
    scopes: &ScopeCopies,
    post_areas: &BTreeSet<String>,
    filter: &[String],
) -> std::result::Result<(), TallyValidationError> {
    if tally_type != TallyType::INITIALIZATION_REPORT {
        return Err(TallyValidationError::new(
            "Only an initialization report can be generated per country",
        ));
    }
    if election_ids.len() != 1 {
        return Err(TallyValidationError::new(
            "Initialize one Post at a time when initializing per country",
        ));
    }
    if !scopes.per_country() {
        return Err(TallyValidationError::new(
            "Countries are initialized one by one only with the initialization scope Post and country",
        ));
    }
    if filter.is_empty() {
        return Err(TallyValidationError::new(
            "Choose at least one country to initialize",
        ));
    }
    if let Some(area_id) = filter.iter().find(|area_id| !post_areas.contains(*area_id)) {
        return Err(TallyValidationError::new(format!(
            "Area {area_id} is not a country of the Post"
        )));
    }
    Ok(())
}

/// Checks an area filter for an initialization of `election_id` before
/// anyone signs it: the same rules as the tally creation applies, against
/// the Post's countries now.
#[instrument(skip(hasura_transaction), err)]
pub async fn check_area_filter_for(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
    filter: &[String],
) -> Result<std::result::Result<(), TallyValidationError>> {
    let snapshot = PgTallyCreationReader::new(hasura_transaction)
        .event_snapshot(tenant_id, election_event_id)
        .await?;
    let scopes = initialization_scope_for_target(
        hasura_transaction,
        tenant_id,
        election_event_id,
        Some(Uuid::parse_str(election_id)?),
    )
    .await?;
    let styled = list_post_ballot_style_areas(
        hasura_transaction,
        Uuid::parse_str(tenant_id)?,
        Uuid::parse_str(election_event_id)?,
    )
    .await?;
    let election_ids = vec![election_id.to_string()];
    let countries = post_area_ids(
        &snapshot.areas,
        &snapshot.area_contests,
        &snapshot.contests,
        &election_ids,
        &styled,
    )?;
    Ok(check_initialization_area_filter(
        TallyType::INITIALIZATION_REPORT,
        &election_ids,
        &scopes,
        &countries,
        filter,
    ))
}

/// A Post's initialization state.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PostInitialization {
    pub name: String,
    /// Its `initialization_report_policy` is `REQUIRED`.
    pub requires_report: bool,
    /// Its initialization report exists (`initialization_report_generated`).
    pub initialized: bool,
    /// Its countries (area ids).
    pub areas: BTreeSet<String>,
    /// The countries an initialization report covered.
    pub initialized_areas: BTreeSet<String>,
}

impl PostInitialization {
    /// Its countries no initialization report covered yet.
    pub fn pending_areas(&self) -> Vec<String> {
        self.areas
            .difference(&self.initialized_areas)
            .cloned()
            .collect()
    }
}

/// The initialization state of an event's Posts, by election id.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EventInitialization {
    pub posts: BTreeMap<String, PostInitialization>,
    /// Area names by id, for the refusals' messages.
    pub area_names: BTreeMap<String, String>,
}

impl EventInitialization {
    /// The state of the given Posts from the event's rows.
    pub fn build(
        elections: &[Election],
        areas: &[Area],
        area_contests: &[AreaContest],
        contests: &[Contest],
        styled: &[(String, String)],
        initialized: &[(String, Option<String>)],
        default_language: &str,
    ) -> Result<Self> {
        let mut posts = BTreeMap::new();
        for election in elections {
            let initialized_areas = initialized
                .iter()
                .filter(|(election_id, _)| *election_id == election.id)
                .filter_map(|(_, area_id)| area_id.clone())
                .collect();
            posts.insert(
                election.id.clone(),
                PostInitialization {
                    name: post_display_name(election, default_language),
                    requires_report: requires_report(election),
                    initialized: election.initialization_report_generated.unwrap_or(false),
                    areas: post_area_ids(
                        areas,
                        area_contests,
                        contests,
                        std::slice::from_ref(&election.id),
                        styled,
                    )?,
                    initialized_areas,
                },
            );
        }
        let area_names = areas
            .iter()
            .map(|area| {
                (
                    area.id.clone(),
                    area.name.clone().unwrap_or_else(|| area.id.clone()),
                )
            })
            .collect();
        Ok(EventInitialization { posts, area_names })
    }

    fn post_names(&self, ids: &[String]) -> Vec<String> {
        ids.iter()
            .map(|id| {
                self.posts
                    .get(id)
                    .map(|post| post.name.clone())
                    .unwrap_or_else(|| id.clone())
            })
            .collect()
    }

    fn area_names(&self, ids: &[String]) -> Vec<String> {
        ids.iter()
            .map(|id| {
                self.area_names
                    .get(id)
                    .cloned()
                    .unwrap_or_else(|| id.clone())
            })
            .collect()
    }
}

/// Why `election_id` can't open under `scope`, if it can't.
fn refusal_under(
    scope: &InitializationScope,
    event: &EventInitialization,
    election_id: &str,
    post: &PostInitialization,
) -> Option<TransitionRefusal> {
    match scope {
        InitializationScope::POST => None,
        InitializationScope::EVENT => {
            let pending: Vec<String> = event
                .posts
                .iter()
                .filter(|(id, other)| {
                    id.as_str() != election_id && other.requires_report && !other.initialized
                })
                .map(|(id, _)| id.clone())
                .collect();
            (!pending.is_empty()).then(|| TransitionRefusal::EventNotInitialized {
                post: post.name.clone(),
                names: event.post_names(&pending),
                election_ids: pending,
            })
        }
        InitializationScope::POST_AND_COUNTRY => {
            let pending = post.pending_areas();
            (!pending.is_empty()).then(|| TransitionRefusal::CountriesNotInitialized {
                post: post.name.clone(),
                names: event.area_names(&pending),
                area_ids: pending,
            })
        }
    }
}

/// Why `election_id` can't open now with respect to its initialization, if
/// it can't: the Post's own report first (every scope needs it), then each
/// copy's scope, the current copy first.
pub fn initialization_refusal(
    scopes: &ScopeCopies,
    event: &EventInitialization,
    election_id: &str,
) -> Option<TransitionRefusal> {
    let post = event.posts.get(election_id)?;
    if !post.requires_report {
        return None;
    }
    if !post.initialized {
        return Some(TransitionRefusal::InitializationReportRequired);
    }
    scopes
        .scopes()
        .iter()
        .find_map(|scope| refusal_under(scope, event, election_id, post))
}

/// The initialization state of every Post of the event.
#[instrument(skip(hasura_transaction), err)]
pub async fn load_event_initialization(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<EventInitialization> {
    let snapshot = PgTallyCreationReader::new(hasura_transaction)
        .event_snapshot(tenant_id, election_event_id)
        .await
        .context("Error reading the event's Posts and countries")?;
    let rows = list_election_initializations(
        hasura_transaction,
        Uuid::parse_str(tenant_id)?,
        Uuid::parse_str(election_event_id)?,
    )
    .await?;
    let initialized: Vec<(String, Option<String>)> = rows
        .iter()
        .map(|row| {
            (
                row.election_id.to_string(),
                row.area_id.map(|id| id.to_string()),
            )
        })
        .collect();
    let styled = list_post_ballot_style_areas(
        hasura_transaction,
        Uuid::parse_str(tenant_id)?,
        Uuid::parse_str(election_event_id)?,
    )
    .await?;
    EventInitialization::build(
        &snapshot.elections,
        &snapshot.areas,
        &snapshot.area_contests,
        &snapshot.contests,
        &styled,
        &initialized,
        &snapshot.election_event.get_default_language(),
    )
}

fn requires_report(election: &Election) -> bool {
    election
        .get_presentation()
        .and_then(|presentation| presentation.initialization_report_policy)
        .unwrap_or_default()
        == EInitializeReportPolicy::REQUIRED
}

/// The Posts of `elections` that can't open now with respect to their
/// initialization at the event's scope, including retained published REQUIRED
/// switches. Loads trusted scope and raw initialization once, then uses the same
/// pure check as scheduled predictions.
#[instrument(skip(hasura_transaction, election_event, elections), err)]
pub async fn initialization_refusals(
    hasura_transaction: &Transaction<'_>,
    election_event: &ElectionEvent,
    elections: &[Election],
) -> Result<BTreeMap<String, TransitionRefusal>> {
    let state = EventState::read(
        hasura_transaction,
        Uuid::parse_str(&election_event.tenant_id)?,
        Uuid::parse_str(&election_event.id)?,
    )
    .await?;
    let mut refusals = BTreeMap::new();
    for election in elections {
        if let Some(refusal) = state.initialization_refusal_at(Uuid::parse_str(&election.id)?) {
            refusals.insert(election.id.clone(), refusal);
        }
    }
    Ok(refusals)
}

/// Why `election` can't open now with respect to its initialization at the
/// event's scope, if it can't; see [`initialization_refusals`].
pub async fn initialization_refusal_for(
    hasura_transaction: &Transaction<'_>,
    election_event: &ElectionEvent,
    election: &Election,
) -> Result<Option<TransitionRefusal>> {
    Ok(initialization_refusals(
        hasura_transaction,
        election_event,
        std::slice::from_ref(election),
    )
    .await?
    .remove(&election.id))
}

#[cfg(test)]
#[path = "initialization_scope_tests.rs"]
mod tests;
