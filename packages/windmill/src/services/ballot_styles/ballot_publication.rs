// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::ballot_publication::{
    get_ballot_publication_by_id, get_previous_publication, get_previous_publication_election,
    insert_ballot_publication, lock_publication_event, soft_delete_other_ballot_publications,
    update_ballot_publication,
};
use crate::postgres::ballot_style::{
    get_publication_ballot_eml_page, get_publication_ballot_styles,
};
use crate::postgres::contest::export_contests;
use crate::postgres::election::{get_election_by_id, get_elections_ids, update_election_status};
use crate::postgres::election_event::{get_election_event_by_id, update_election_event_status};
use crate::services::celery_app::get_celery_app;
use crate::services::election_event_board::get_election_event_board;
use crate::services::election_event_status::{get_election_event_status, get_election_status};
use crate::services::electoral_log::*;
use crate::services::tasks_execution::{
    post as post_task_execution, update_fail as update_task_execution_fail,
};
use crate::services::{enrollment_windows, realm_localization};
use crate::tasks::update_election_event_ballot_styles::update_election_event_ballot_styles;
use crate::types::tasks::ETasksExecution;
use anyhow::{anyhow, Context, Result};
use chrono::{Local, Utc};
use deadpool_postgres::Transaction;
use sequent_core::ballot::PublicKeyConfig;
use sequent_core::ballot::{
    BallotStyle, Contest, ContestEncryptionPolicy, EBlankVotePolicy, EDuplicatedRankPolicy,
    EOverVotePolicy, EPreferenceGapsPolicy, EUnderVotePolicy, ElectionEventPresentation,
    ElectionEventStatus, ElectionStatus, InvalidVotePolicy, MultiContestEncodingMode,
};
use sequent_core::serialization::deserialize_with_path::*;
use sequent_core::services::connection;
use sequent_core::services::date::ISO8601;
use sequent_core::types::ceremonies::CountingAlgType;
use sequent_core::types::ceremonies::KeysCeremonyExecutionStatus;
use sequent_core::types::hasura::core::{BallotPublication, KeysCeremony, TasksExecution};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use thiserror::Error;
use tracing::{error, event, instrument, Level};

use super::ballot_style;
use super::slates;

const VALIDATION_PAGE_SIZE: i64 = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
struct ContestAcclamationState {
    name: Option<String>,
    is_acclaimed: bool,
}

#[derive(Debug, Error)]
#[error("{message}")]
pub struct BallotPublicationValidationError {
    pub reasons: Vec<String>,
    message: String,
}

impl BallotPublicationValidationError {
    pub(super) fn new(reasons: Vec<String>) -> Self {
        let message = format!(
            "Ballot publication validation failed:\n- {}",
            reasons.join("\n- ")
        );
        Self { reasons, message }
    }
}

fn add_acclamation_states(
    states: &mut BTreeMap<String, ContestAcclamationState>,
    ballot_style: &Value,
) -> Result<()> {
    let contests = ballot_style
        .get("contests")
        .and_then(Value::as_array)
        .context("Ballot style must contain a contests array")?;

    for contest in contests {
        let contest_id = contest
            .get("id")
            .and_then(Value::as_str)
            .context("Ballot contest must contain a string id")?;
        let is_acclaimed = match contest.get("is_acclaimed") {
            None | Some(Value::Null) => false,
            Some(Value::Bool(value)) => *value,
            Some(_) => {
                return Err(anyhow!(
                    "Ballot contest {contest_id} has a non-boolean is_acclaimed value"
                ))
            }
        };
        let state = ContestAcclamationState {
            name: contest
                .get("name")
                .and_then(Value::as_str)
                .map(str::to_owned),
            is_acclaimed,
        };

        if let Some(existing) = states.get(contest_id) {
            if existing.is_acclaimed != state.is_acclaimed {
                return Err(anyhow!(
                    "Contest {contest_id} has inconsistent is_acclaimed values in one ballot publication"
                ));
            }
        } else {
            states.insert(contest_id.to_owned(), state);
        }
    }

    Ok(())
}

fn merge_acclamation_states(
    target: &mut BTreeMap<String, ContestAcclamationState>,
    source: BTreeMap<String, ContestAcclamationState>,
) -> Result<()> {
    for (contest_id, state) in source {
        if let Some(existing) = target.get(&contest_id) {
            if existing.is_acclaimed != state.is_acclaimed {
                return Err(anyhow!(
                    "Contest {contest_id} has inconsistent is_acclaimed values across published ballot styles"
                ));
            }
        } else {
            target.insert(contest_id, state);
        }
    }
    Ok(())
}

fn validate_acclamation_states(
    previous: &BTreeMap<String, ContestAcclamationState>,
    current: &BTreeMap<String, ContestAcclamationState>,
) -> Result<()> {
    let reasons = current
        .iter()
        .filter_map(|(contest_id, current_state)| {
            let previous_state = previous.get(contest_id)?;
            (previous_state.is_acclaimed != current_state.is_acclaimed).then(|| {
                let contest_name = current_state.name.as_deref().unwrap_or(contest_id);
                format!(
                    "Contest \"{contest_name}\" ({contest_id}) changed is_acclaimed from {} to {} after voting started for its election. Restore the published value before publishing again.",
                    previous_state.is_acclaimed, current_state.is_acclaimed
                )
            })
        })
        .collect::<Vec<_>>();

    if reasons.is_empty() {
        Ok(())
    } else {
        Err(BallotPublicationValidationError::new(reasons).into())
    }
}

fn election_has_started(status: &ElectionStatus) -> bool {
    [
        status.voting_period_dates.first_started_at.as_ref(),
        status.kiosk_voting_period_dates.first_started_at.as_ref(),
        status.early_voting_period_dates.first_started_at.as_ref(),
        status
            .telephone_voting_period_dates
            .first_started_at
            .as_ref(),
    ]
    .into_iter()
    .any(|started_at| started_at.is_some())
}

/// The latest publication of each election in `ballot_publication` whose
/// voting has started, as (election id, publication id).
async fn started_elections_publications(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    ballot_publication: &BallotPublication,
) -> Result<Vec<(String, String)>> {
    let election_ids = ballot_publication
        .election_ids
        .as_ref()
        .context("Ballot publication is missing its election ids")?;
    if election_ids.is_empty() {
        return Err(anyhow!("Ballot publication has no election ids"));
    }

    let mut publications = Vec::new();
    for election_id in election_ids {
        let election = get_election_by_id(
            hasura_transaction,
            tenant_id,
            election_event_id,
            election_id,
        )
        .await?
        .with_context(|| format!("Can't find election {election_id}"))?;
        let election_status = get_election_status(election.status).unwrap_or_default();
        if !election_has_started(&election_status) {
            continue;
        }

        // Use the latest publication at final-publish time. A draft may have
        // been generated before another draft was published.
        let Some(previous_publication) = get_previous_publication_election(
            hasura_transaction,
            tenant_id,
            election_event_id,
            Some(Utc::now().with_timezone(&Local)),
            election_id,
        )
        .await?
        else {
            continue;
        };

        publications.push((election_id.clone(), previous_publication.id));
    }
    Ok(publications)
}

/// The parts of a ballot style that decide how its cast ballots decode.
#[derive(Serialize)]
struct BallotStyleDecoding {
    contest_ids: BTreeSet<String>,
    contest_encryption_policy: ContestEncryptionPolicy,
    multi_contest_encoding_mode: MultiContestEncodingMode,
    decline_to_vote_enabled: bool,
    blank_ballots_enabled: bool,
}

impl From<&BallotStyle> for BallotStyleDecoding {
    fn from(style: &BallotStyle) -> Self {
        Self {
            contest_ids: style
                .contests
                .iter()
                .map(|contest| contest.id.clone())
                .collect(),
            contest_encryption_policy: style
                .election_event_presentation
                .as_ref()
                .and_then(|presentation| presentation.contest_encryption_policy.clone())
                .unwrap_or_default(),
            multi_contest_encoding_mode: style.multi_contest_encoding_mode.unwrap_or_default(),
            decline_to_vote_enabled: style.decline_to_vote_enabled(),
            blank_ballots_enabled: style.blank_ballots_enabled(),
        }
    }
}

/// The parts of a contest that decide how its cast choices decode: the
/// codec's bases and candidate positions, and the policies the decoder
/// applies to mark a choice invalid.
#[derive(Serialize)]
struct ContestDecoding {
    candidates: BTreeMap<String, CandidateDecoding>,
    counting_algorithm: CountingAlgType,
    min_votes: i64,
    max_votes: i64,
    allow_writeins: bool,
    base32_writeins: bool,
    cumulative_number_of_checkboxes: u64,
    invalid_vote_policy: InvalidVotePolicy,
    under_vote_policy: EUnderVotePolicy,
    blank_vote_policy: EBlankVotePolicy,
    over_vote_policy: EOverVotePolicy,
    duplicated_rank_policy: EDuplicatedRankPolicy,
    preference_gaps_policy: EPreferenceGapsPolicy,
}

/// The candidate flags the codec reads to place and classify a choice.
#[derive(Serialize)]
struct CandidateDecoding {
    is_explicit_invalid: bool,
    is_explicit_blank: bool,
    is_write_in: bool,
}

impl From<&Contest> for ContestDecoding {
    fn from(contest: &Contest) -> Self {
        let presentation = contest.presentation.clone().unwrap_or_default();
        Self {
            candidates: contest
                .candidates
                .iter()
                .map(|candidate| {
                    (
                        candidate.id.clone(),
                        CandidateDecoding {
                            is_explicit_invalid: candidate.is_explicit_invalid(),
                            is_explicit_blank: candidate.is_explicit_blank(),
                            is_write_in: candidate.is_write_in(),
                        },
                    )
                })
                .collect(),
            counting_algorithm: contest.get_counting_algorithm(),
            min_votes: contest.min_votes,
            max_votes: contest.max_votes,
            allow_writeins: contest.allow_writeins(),
            base32_writeins: contest.base32_writeins(),
            cumulative_number_of_checkboxes: contest.cumulative_number_of_checkboxes(),
            invalid_vote_policy: presentation.invalid_vote_policy.unwrap_or_default(),
            under_vote_policy: presentation.under_vote_policy.unwrap_or_default(),
            blank_vote_policy: presentation.blank_vote_policy.unwrap_or_default(),
            over_vote_policy: presentation.over_vote_policy.unwrap_or_default(),
            duplicated_rank_policy: presentation.duplicated_rank_policy.unwrap_or_default(),
            preference_gaps_policy: presentation.preference_gaps_policy.unwrap_or_default(),
        }
    }
}

/// Names the top-level fields whose values differ between two values of
/// the same type.
fn changed_fields<T: Serialize>(previous: &T, current: &T) -> Result<BTreeSet<String>> {
    let previous = serde_json::to_value(previous)?;
    let current = serde_json::to_value(current)?;
    let previous = previous
        .as_object()
        .context("Expected an object when comparing ballot styles")?;
    let current = current
        .as_object()
        .context("Expected an object when comparing ballot styles")?;

    Ok(previous
        .iter()
        .filter(|(field, value)| current.get(*field) != Some(*value))
        .map(|(field, _)| field.clone())
        .collect())
}

/// How ballots cast with one area's published ballot style decode.
struct PublishedDecoding {
    style: BallotStyleDecoding,
    contests: Vec<(String, ContestDecoding)>,
}

/// Rejects a publication that would change how ballots already cast in the
/// started elections decode: an area left without a ballot style, or any
/// change to a [`BallotStyleDecoding`] or [`ContestDecoding`] field. It is
/// given the published ballot styles first, then the new publication's, one
/// style at a time.
#[derive(Default)]
struct DecodingCheck {
    published: BTreeMap<(String, String), PublishedDecoding>,
    seen: HashSet<(String, String)>,
    style_reasons: BTreeMap<(String, String), String>,
    changed_contests: BTreeMap<String, (String, BTreeSet<String>)>,
}

impl DecodingCheck {
    fn add_published(&mut self, style: &BallotStyle) {
        self.published
            .entry((style.election_id.clone(), style.area_id.clone()))
            .or_insert_with(|| PublishedDecoding {
                style: style.into(),
                contests: style
                    .contests
                    .iter()
                    .map(|contest| (contest.id.clone(), contest.into()))
                    .collect(),
            });
    }

    fn add_current(&mut self, style: &BallotStyle) -> Result<()> {
        let key = (style.election_id.clone(), style.area_id.clone());
        let Some(published) = self.published.get(&key) else {
            return Ok(());
        };
        if !self.seen.insert(key.clone()) {
            return Ok(());
        }

        let style_changes = changed_fields(&published.style, &BallotStyleDecoding::from(style))?;
        if !style_changes.is_empty() {
            self.style_reasons.insert(
                key,
                format!(
                    "The ballot style of area {} in election {} changed {} after voting started for that election. Restore the published values before publishing again.",
                    style.area_id,
                    style.election_id,
                    style_changes.into_iter().collect::<Vec<_>>().join(", ")
                ),
            );
        }

        for (contest_id, published_contest) in &published.contests {
            let Some(current_contest) = style
                .contests
                .iter()
                .find(|contest| &contest.id == contest_id)
            else {
                continue;
            };
            let contest_changes =
                changed_fields(published_contest, &ContestDecoding::from(current_contest))?;
            if !contest_changes.is_empty() {
                self.changed_contests
                    .entry(contest_id.clone())
                    .or_insert_with(|| {
                        (
                            current_contest
                                .name
                                .clone()
                                .unwrap_or_else(|| contest_id.clone()),
                            BTreeSet::new(),
                        )
                    })
                    .1
                    .extend(contest_changes);
            }
        }
        Ok(())
    }

    fn validate(self) -> Result<()> {
        let mut reasons = Vec::new();
        for key in self.published.keys() {
            if !self.seen.contains(key) {
                let (election_id, area_id) = key;
                reasons.push(format!(
                    "Area {area_id} has no ballot style for election {election_id} after voting started for that election. Restore its contests before publishing again."
                ));
            } else if let Some(reason) = self.style_reasons.get(key) {
                reasons.push(reason.clone());
            }
        }
        reasons.extend(self.changed_contests.into_iter().map(
            |(contest_id, (contest_name, fields))| {
                format!(
                    "Contest \"{contest_name}\" ({contest_id}) changed {} after voting started for its election. Restore the published values before publishing again.",
                    fields.into_iter().collect::<Vec<_>>().join(", ")
                )
            },
        ));

        if reasons.is_empty() {
            Ok(())
        } else {
            Err(BallotPublicationValidationError::new(reasons).into())
        }
    }
}

/// Publishing a native key requires completed custody, even through the API.
fn validate_publication_ceremonies(
    keys: &[PublicKeyConfig],
    ceremonies: &[KeysCeremony],
) -> Result<()> {
    for key in keys.iter().filter(|key| !key.is_demo) {
        let ceremony = key
            .keys_ceremony_id
            .as_deref()
            .and_then(|id| ceremonies.iter().find(|ceremony| ceremony.id == id));
        let complete = ceremony
            .map(|ceremony| -> Result<bool> {
                let status = ceremony.status()?;
                Ok(
                    ceremony.execution_status()? == KeysCeremonyExecutionStatus::SUCCESS
                        && status.public_key.as_deref() == Some(key.public_key.as_str())
                        && status
                            .public_key_hash
                            .as_deref()
                            .is_some_and(|hash| !hash.is_empty()),
                )
            })
            .transpose()?
            .unwrap_or(false);
        if !complete {
            return Err(BallotPublicationValidationError::new(vec![format!(
                "Keys ceremony {} must complete before ballot publication with its verified joint public key.",
                key.keys_ceremony_id.as_deref().unwrap_or("(missing)")
            )]).into());
        }
    }
    Ok(())
}

fn validate_election_ceremony_binding(
    key: Option<&PublicKeyConfig>,
    ceremony_id: Option<&str>,
) -> Result<()> {
    if let Some(ceremony_id) = ceremony_id {
        if !key
            .is_some_and(|key| !key.is_demo && key.keys_ceremony_id.as_deref() == Some(ceremony_id))
        {
            return Err(BallotPublicationValidationError::new(vec![format!(
                "Keys ceremony {ceremony_id} must complete before ballot publication; regenerate ballots with its verified joint public key."
            )]).into());
        }
    }
    Ok(())
}

/// Calls `visit` with each readable stored ballot style of a publication,
/// only those of `election_id` when given, reading them a page at a time.
async fn for_each_publication_style(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    ballot_publication_id: &str,
    election_id: Option<&str>,
    mut visit: impl FnMut(&Value) -> Result<()>,
) -> Result<()> {
    let mut after_id = None;
    loop {
        let page = get_publication_ballot_eml_page(
            hasura_transaction,
            tenant_id,
            election_event_id,
            ballot_publication_id,
            election_id,
            after_id,
            VALIDATION_PAGE_SIZE,
        )
        .await?;
        let Some(last_id) = page.last().map(|(id, _)| *id) else {
            return Ok(());
        };
        after_id = Some(last_id);
        for ballot_eml in page.into_iter().filter_map(|(_, ballot_eml)| ballot_eml) {
            if let Ok(ballot_style) = deserialize_str::<Value>(&ballot_eml) {
                visit(&ballot_style)?;
            }
        }
    }
}

/// Checks a generated publication before it goes live: its keys ceremonies,
/// its slates, and, for each election where voting has started, that it keeps
/// the published acclamation values, slates and ballot decoding. Ballot styles
/// are read a page at a time, so a publication with many areas is never held
/// in memory at once.
async fn validate_publication_styles(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    ballot_publication: &BallotPublication,
) -> Result<()> {
    let contests = export_contests(hasura_transaction, tenant_id, election_event_id).await?;
    let ceremonies = crate::postgres::keys_ceremony::get_keys_ceremonies(
        hasura_transaction,
        tenant_id,
        election_event_id,
    )
    .await?;
    // Generated styles can predate ceremony creation. Check the current
    // election binding before trusting a cached demo key or old ceremony key.
    let elections =
        crate::postgres::election::get_elections(hasura_transaction, tenant_id, election_event_id)
            .await?;

    let mut published_acclamation = BTreeMap::new();
    let mut published_slates = BTreeMap::new();
    let mut decoding = DecodingCheck::default();
    for (election_id, publication_id) in started_elections_publications(
        hasura_transaction,
        tenant_id,
        election_event_id,
        ballot_publication,
    )
    .await?
    {
        let mut acclamation = BTreeMap::new();
        for_each_publication_style(
            hasura_transaction,
            tenant_id,
            election_event_id,
            &publication_id,
            Some(&election_id),
            |stored| {
                add_acclamation_states(&mut acclamation, stored)?;
                let style = slates::read_ballot_style(stored)?;
                slates::record_published_slates(&mut published_slates, &style);
                decoding.add_published(&style);
                Ok(())
            },
        )
        .await?;
        merge_acclamation_states(&mut published_acclamation, acclamation)?;
    }

    let mut current_acclamation = BTreeMap::new();
    let mut current_slates = slates::StoredSlatesCheck::new(&contests);
    let mut keys = Vec::new();
    for_each_publication_style(
        hasura_transaction,
        tenant_id,
        election_event_id,
        &ballot_publication.id,
        None,
        |stored| {
            let style = slates::read_ballot_style(stored)?;
            let election = elections
                .iter()
                .find(|election| election.id == style.election_id)
                .context("Published ballot election no longer exists")?;
            validate_election_ceremony_binding(
                style.public_key.as_ref(),
                election.keys_ceremony_id.as_deref(),
            )?;
            add_acclamation_states(&mut current_acclamation, stored)?;
            current_slates.add(&style);
            decoding.add_current(&style)?;
            keys.extend(style.public_key);
            Ok(())
        },
    )
    .await?;

    validate_publication_ceremonies(&keys, &ceremonies)?;
    validate_acclamation_states(&published_acclamation, &current_acclamation)?;
    decoding.validate()?;
    current_slates.validate()?;
    slates::validate_slates_unchanged(&published_slates, current_slates.published())
}

#[instrument(skip(hasura_transaction), err)]
async fn get_election_ids_for_publication(
    hasura_transaction: &Transaction<'_>,
    tenant_id: String,
    election_event_id: String,
    election_id_opt: Option<String>,
) -> Result<Vec<String>> {
    if let Some(election_id) = election_id_opt {
        return Ok(vec![election_id]);
    }
    let elections_ids =
        get_elections_ids(hasura_transaction, &tenant_id, &election_event_id).await?;

    Ok(elections_ids)
}

#[instrument(err)]
pub async fn prepare_ballot_publication(
    hasura_transaction: &Transaction<'_>,
    tenant_id: String,
    election_event_id: String,
    election_id: Option<String>,
    user_id: String,
) -> Result<BallotPublication> {
    // A queued worker takes this same lock before reading the publication.
    // Hold it until commit while allowing the task ledger's foreign-key insert.
    lock_publication_event(hasura_transaction, &tenant_id, &election_event_id).await?;

    let election_ids = get_election_ids_for_publication(
        hasura_transaction,
        tenant_id.clone(),
        election_event_id.clone(),
        election_id.clone(),
    )
    .await?;

    insert_ballot_publication(
        hasura_transaction,
        &tenant_id.clone(),
        &election_event_id.clone(),
        election_ids.clone(),
        user_id.clone(),
        election_id.clone(),
    )
    .await?
    .with_context(|| "can't find inserted ballot publication")
}

#[instrument(err)]
pub async fn add_ballot_publication(
    hasura_transaction: &Transaction<'_>,
    tenant_id: String,
    election_event_id: String,
    election_id: Option<String>,
    user_id: String,
    executer_name: &str,
) -> Result<(String, TasksExecution)> {
    let celery_app = get_celery_app().await;
    let ballot_publication = prepare_ballot_publication(
        hasura_transaction,
        tenant_id.clone(),
        election_event_id.clone(),
        election_id,
        user_id,
    )
    .await?;

    let task_execution = post_task_execution(
        &tenant_id,
        Some(&election_event_id),
        ETasksExecution::GENERATE_BALLOT_PUBLICATION,
        executer_name,
    )
    .await
    .context("Failed to insert task execution record")?;

    let task = match celery_app
        .send_task(update_election_event_ballot_styles::new(
            tenant_id.clone(),
            election_event_id.clone(),
            ballot_publication.id.clone(),
            task_execution.clone(),
        ))
        .await
    {
        Ok(task) => task,
        Err(err) => {
            let message = format!("Failed to enqueue ballot style generation: {err}");
            update_task_execution_fail(&task_execution, &message)
                .await
                .ok();
            return Err(anyhow!(message));
        }
    };
    event!(
        Level::INFO,
        "Sent CREATE_ELECTION_EVENT_BALLOT_STYLES task {}",
        task.task_id
    );

    Ok((ballot_publication.id.clone(), task_execution))
}

#[instrument(err)]
pub async fn update_publish_ballot(
    hasura_transaction: &Transaction<'_>,
    user_id: String,
    username: String,
    tenant_id: String,
    election_event_id: String,
    ballot_publication_id: String,
    lifecycle: &crate::services::scheduled_outcome::PublicationLifecycle,
) -> Result<()> {
    lock_publication_event(hasura_transaction, &tenant_id, &election_event_id).await?;

    let ballot_publication = get_ballot_publication_by_id(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        &ballot_publication_id,
    )
    .await?
    .with_context(|| "Can't find ballot publication")?;

    if ballot_publication.deleted_at.is_some() {
        return Err(anyhow!("Cannot publish a deleted ballot publication"));
    }

    if ballot_publication.is_generated.unwrap_or(false) == false {
        return Err(anyhow!(
            "Ballot publication not generated yet, can't publish."
        ));
    }

    super::publication_files::require_publication_files(&ballot_publication)?;

    if ballot_publication.published_at.is_some() {
        return Ok(());
    }

    validate_publication_styles(
        hasura_transaction,
        &tenant_id,
        &election_event_id,
        &ballot_publication,
    )
    .await?;

    let design_check = super::design_check::check_publication_designs(
        hasura_transaction,
        &tenant_id,
        &election_event_id,
        &ballot_publication_id,
    )
    .await?;
    if let Some(check) = &design_check {
        if !check.mismatches.is_empty() {
            return Err(BallotPublicationValidationError::new(
                check.mismatches.iter().map(ToString::to_string).collect(),
            )
            .into());
        }
    }

    crate::services::vote_privacy::allocate_registry(
        hasura_transaction,
        &tenant_id,
        &election_event_id,
    )
    .await?;

    let _result = soft_delete_other_ballot_publications(
        &hasura_transaction,
        &ballot_publication_id,
        &election_event_id,
        &tenant_id,
        ballot_publication.election_id.clone(),
    )
    .await?;

    update_ballot_publication(
        hasura_transaction,
        &tenant_id,
        &election_event_id,
        &ballot_publication_id,
        true,
        Some(ISO8601::now()),
    )
    .await?;

    // What the publication says scheduled openings and closings may do
    // (VOTE-LIFECYCLE §5b): the current values, or what an approval signed.
    crate::services::scheduled_outcome::keep_publication_lifecycle(
        hasura_transaction,
        &tenant_id,
        &election_event_id,
        &ballot_publication_id,
        ballot_publication.election_id.as_deref(),
        lifecycle,
    )
    .await?;

    let election_event = get_election_event_by_id(
        hasura_transaction,
        &tenant_id.clone(),
        &election_event_id.clone(),
    )
    .await?;

    let mut new_status: ElectionEventStatus =
        get_election_event_status(election_event.status.clone()).unwrap_or(Default::default());
    new_status.is_published = Some(true);
    let new_status_js = serde_json::to_value(new_status)?;

    update_election_event_status(
        hasura_transaction,
        &tenant_id,
        &election_event_id,
        new_status_js,
    )
    .await?;

    // Update elections status
    let election_ids = ballot_publication.election_ids.clone().unwrap_or(vec![]);
    for election_id in election_ids.clone() {
        update_election_status(
            &hasura_transaction,
            &election_id,
            &tenant_id.clone(),
            &election_event_id.clone(),
            true,
        )
        .await
        .with_context(|| "error updating election status")?;
    }

    let board_name = get_election_event_board(election_event.bulletin_board_reference.clone())
        .with_context(|| "missing bulletin board")?;

    // let electoral_log = ElectoralLog::new(board_name.as_str()).await?;
    let electoral_log = ElectoralLog::for_admin_user(
        hasura_transaction,
        &board_name,
        &tenant_id,
        &election_event.id,
        &user_id,
        Some(username.clone()),
        Some(election_ids.clone()),
        None,
    )
    .await?;
    electoral_log
        .post_election_published(
            election_event_id.clone(),
            Some(election_ids.clone()),
            ballot_publication_id.clone(),
            design_check.map(super::design_check::DesignCheck::published),
            Some(user_id),
            Some(username),
        )
        .await
        .map_err(|e| anyhow!("error posting to the electoral log: {e}"))?;

    sync_enrollment_pages(
        hasura_transaction,
        &tenant_id,
        &election_event_id,
        election_event.presentation.clone(),
    )
    .await;
    Ok(())
}

/// The enrollment pages (event realm) follow the published configuration:
/// the event's timezone and enrollment texts, and each Post's enrollment
/// window in its zone (VOTE-LIFECYCLE §4, §8). A Keycloak failure doesn't
/// undo the publication: it is logged, and the next publication or
/// enrollment schedule change writes both again.
async fn sync_enrollment_pages(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    presentation: Option<Value>,
) {
    match presentation
        .map(deserialize_value::<ElectionEventPresentation>)
        .transpose()
    {
        Ok(presentation) => {
            if let Err(err) = realm_localization::sync_event_realm_localization(
                tenant_id,
                election_event_id,
                presentation.as_ref(),
            )
            .await
            {
                error!(
                    "Publication of event {election_event_id}: the event realm's timezone and \
                     enrollment texts were not updated: {err:?}"
                );
            }
        }
        Err(err) => error!(
            "Publication of event {election_event_id}: unreadable event presentation, the event \
             realm's timezone and enrollment texts were not updated: {err:?}"
        ),
    }
    // In a savepoint, so a failed read can't abort the publication's transaction.
    let refreshed =
        enrollment_windows::refresh_in_savepoint(hasura_transaction, tenant_id, election_event_id)
            .await;
    if let Err(err) = refreshed {
        error!(
            "Publication of event {election_event_id}: the enrollment windows were not \
             updated: {err:?}"
        );
    }
}

#[instrument(skip(hasura_transaction), err)]
pub async fn get_publication_json(
    hasura_transaction: &Transaction<'_>,
    tenant_id: String,
    election_event_id: String,
    ballot_publication_id: String,
    election_id: Option<String>,
    limit: Option<usize>,
) -> Result<Value> {
    let ballot_style = get_publication_ballot_styles(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        &ballot_publication_id,
        limit,
    )
    .await?;

    let ballot_style_strings: Vec<Option<String>> = ballot_style
        .into_iter()
        .filter(|ballot_style| {
            election_id
                .clone()
                .map(|id| ballot_style.election_id == id)
                .unwrap_or(true)
        })
        .map(|style| style.ballot_eml.clone())
        .collect();

    let val_arr: Vec<Value> = ballot_style_strings
        .iter()
        .map(|el| el.clone().map(|val| deserialize_str(&val).ok()).flatten())
        .filter(|el| el.is_some())
        .map(|el| el.ok_or(anyhow!("Empty ballot style!")))
        .collect::<Result<Vec<_>>>()?;

    Ok(serde_json::Value::Array(val_arr))
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PublicationStyles {
    ballot_publication_id: String,
    ballot_styles: Value,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PublicationDiff {
    current: PublicationStyles,
    previous: Option<PublicationStyles>,
}

#[instrument(err)]
pub async fn get_ballot_publication_diff(
    hasura_transaction: &Transaction<'_>,
    tenant_id: String,
    election_event_id: String,
    ballot_publication_id: String,
    limit: Option<usize>,
) -> Result<PublicationDiff> {
    let ballot_publication = get_ballot_publication_by_id(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        &ballot_publication_id,
    )
    .await?
    .with_context(|| "Can't find ballot publication")?;

    let previous_publication_id = if let Some(election_id) = ballot_publication.election_id.clone()
    {
        get_previous_publication_election(
            &hasura_transaction,
            &tenant_id,
            &election_event_id,
            ballot_publication.created_at.clone(),
            &election_id,
        )
        .await?
        .map(|pub_data| pub_data.id)
        .ok_or_else(|| {
            anyhow!(
                "Can't find ballot publication for election id {}",
                election_id
            )
        })
        .with_context(|| "Error retrieving previous ballot publication for election")
        .ok()
    } else {
        get_previous_publication(
            &hasura_transaction,
            &tenant_id,
            &election_event_id,
            ballot_publication.created_at.clone(),
        )
        .await?
        .map(|pub_data| pub_data.id)
        .ok_or_else(|| anyhow!("Can't find ballot publication"))
        .with_context(|| "Error retrieving previous ballot publication")
        .ok()
    };

    let current_json = get_publication_json(
        &hasura_transaction,
        tenant_id.clone(),
        election_event_id.clone(),
        ballot_publication.id.clone(),
        ballot_publication.election_id.clone(),
        limit,
    )
    .await?;

    let current = PublicationStyles {
        ballot_publication_id: ballot_publication_id.clone(),
        ballot_styles: current_json,
    };

    let previous = if let Some(previous_publication_id) = previous_publication_id {
        let previous_json = get_publication_json(
            &hasura_transaction,
            tenant_id.clone(),
            election_event_id.clone(),
            previous_publication_id.clone(),
            ballot_publication.election_id.clone(),
            limit,
        )
        .await?;

        Some(PublicationStyles {
            ballot_publication_id: previous_publication_id.clone(),
            ballot_styles: previous_json,
        })
    } else {
        None
    };

    Ok(PublicationDiff { current, previous })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn collect_acclamation_states(
        publication: &Value,
    ) -> Result<BTreeMap<String, ContestAcclamationState>> {
        let mut states = BTreeMap::new();
        for ballot_style in publication.as_array().unwrap() {
            add_acclamation_states(&mut states, ballot_style)?;
        }
        Ok(states)
    }

    fn validate_started_elections_publication(
        previous_publications: &[Value],
        current_publication: &Value,
    ) -> Result<()> {
        let mut previous = BTreeMap::new();
        for publication in previous_publications {
            merge_acclamation_states(&mut previous, collect_acclamation_states(publication)?)?;
        }
        validate_acclamation_states(&previous, &collect_acclamation_states(current_publication)?)?;
        if previous_publications.is_empty() {
            return Ok(());
        }

        let mut decoding = DecodingCheck::default();
        for publication in previous_publications {
            for ballot_style in publication.as_array().unwrap() {
                decoding.add_published(&slates::read_ballot_style(ballot_style)?);
            }
        }
        for ballot_style in current_publication.as_array().unwrap() {
            decoding.add_current(&slates::read_ballot_style(ballot_style)?)?;
        }
        decoding.validate()
    }

    fn publication(contests: Value) -> Value {
        json!([{ "contests": contests }])
    }

    #[test]
    fn native_publication_waits_for_completed_custody_and_a_matching_joint_key() {
        let key = PublicKeyConfig {
            public_key: "joint-key".into(),
            is_demo: false,
            keys_ceremony_id: Some("ceremony".into()),
            configuration_id: Some("123".into()),
        };
        let ceremony = |execution_status: KeysCeremonyExecutionStatus| -> KeysCeremony {
            serde_json::from_value(serde_json::json!({
                "id": "ceremony", "tenant_id": "tenant", "election_event_id": "event",
                "trustee_ids": [], "threshold": 2, "execution_status": execution_status.to_string(),
                "status": {"public_key": "joint-key", "public_key_hash": "verified-hash", "logs": [], "trustees": []}
            })).unwrap()
        };
        assert!(validate_publication_ceremonies(
            &[key.clone()],
            &[ceremony(KeysCeremonyExecutionStatus::SUCCESS)]
        )
        .is_ok());
        for status in [
            KeysCeremonyExecutionStatus::STARTED,
            KeysCeremonyExecutionStatus::IN_PROGRESS,
            KeysCeremonyExecutionStatus::FAILED,
            KeysCeremonyExecutionStatus::CANCELLED,
        ] {
            let error =
                validate_publication_ceremonies(&[key.clone()], &[ceremony(status)]).unwrap_err();
            assert!(error.is::<BallotPublicationValidationError>());
            assert!(error
                .to_string()
                .contains("must complete before ballot publication"));
        }
        for field in ["public_key", "public_key_hash"] {
            let mut incomplete = ceremony(KeysCeremonyExecutionStatus::SUCCESS);
            incomplete.status.as_mut().unwrap()[field] = Value::Null;
            assert!(
                validate_publication_ceremonies(&[key.clone()], &[incomplete]).is_err(),
                "{field}"
            );
        }
        let mut changed = ceremony(KeysCeremonyExecutionStatus::SUCCESS);
        changed.status.as_mut().unwrap()["public_key"] = Value::String("other-key".into());
        assert!(validate_publication_ceremonies(&[key.clone()], &[changed]).is_err());
        assert!(validate_publication_ceremonies(&[key.clone()], &[]).is_err());
        let mut unbound = key.clone();
        unbound.keys_ceremony_id = None;
        assert!(validate_publication_ceremonies(
            &[unbound],
            &[ceremony(KeysCeremonyExecutionStatus::SUCCESS)]
        )
        .is_err());
        let demo = PublicKeyConfig {
            is_demo: true,
            ..key
        };
        assert!(validate_publication_ceremonies(&[demo], &[]).is_ok());
        assert!(validate_publication_ceremonies(&[], &[]).is_ok());
    }

    #[test]
    fn stale_demo_publication_cannot_bypass_an_assigned_keys_ceremony() {
        let demo = PublicKeyConfig {
            public_key: "demo".into(),
            is_demo: true,
            keys_ceremony_id: None,
            configuration_id: None,
        };
        assert!(validate_election_ceremony_binding(Some(&demo), Some("ceremony")).is_err());
        assert!(validate_election_ceremony_binding(None, Some("ceremony")).is_err());
        assert!(validate_election_ceremony_binding(Some(&demo), None).is_ok());
        let native = PublicKeyConfig {
            is_demo: false,
            keys_ceremony_id: Some("ceremony".into()),
            ..demo
        };
        assert!(validate_election_ceremony_binding(Some(&native), Some("ceremony")).is_ok());
        assert!(validate_election_ceremony_binding(Some(&native), Some("other")).is_err());
    }

    #[test]
    fn absent_and_false_acclamation_values_are_equivalent() {
        let previous = collect_acclamation_states(&publication(json!([
            {"id": "contest-1", "name": "Mayor"}
        ])))
        .unwrap();
        let current = collect_acclamation_states(&publication(json!([
            {"id": "contest-1", "name": "Mayor", "is_acclaimed": false}
        ])))
        .unwrap();

        assert!(validate_acclamation_states(&previous, &current).is_ok());
    }

    #[test]
    fn first_publication_can_set_acclamation_values() {
        let current = collect_acclamation_states(&publication(json!([
            {"id": "contest-1", "name": "Mayor", "is_acclaimed": true}
        ])))
        .unwrap();

        assert!(validate_acclamation_states(&BTreeMap::new(), &current).is_ok());
    }

    #[test]
    fn detects_when_any_voting_channel_has_started() {
        let mut status = ElectionStatus::default();
        assert!(!election_has_started(&status));

        status.telephone_voting_period_dates.first_started_at =
            Some("2026-08-28T11:00:00Z".parse().unwrap());

        assert!(election_has_started(&status));
    }

    #[test]
    fn reports_every_changed_acclamation_value_in_contest_id_order() {
        let previous = collect_acclamation_states(&publication(json!([
            {"id": "contest-b", "name": "Council", "is_acclaimed": true},
            {"id": "contest-a", "name": "Mayor", "is_acclaimed": false}
        ])))
        .unwrap();
        let current = collect_acclamation_states(&publication(json!([
            {"id": "contest-b", "name": "Council", "is_acclaimed": false},
            {"id": "contest-a", "name": "Mayor", "is_acclaimed": true}
        ])))
        .unwrap();

        let error = validate_acclamation_states(&previous, &current).unwrap_err();
        let validation_error = error
            .downcast_ref::<BallotPublicationValidationError>()
            .unwrap();

        assert_eq!(validation_error.reasons.len(), 2);
        assert!(validation_error.reasons[0].contains("Mayor"));
        assert!(validation_error.reasons[0].contains("false to true"));
        assert!(validation_error.reasons[1].contains("Council"));
        assert!(validation_error.reasons[1].contains("true to false"));
    }

    fn candidate(contest_id: &str, candidate_id: &str) -> Value {
        json!({
            "id": candidate_id,
            "tenant_id": "tenant-1",
            "election_event_id": "event-1",
            "election_id": "election-1",
            "contest_id": contest_id,
            "name": candidate_id,
        })
    }

    fn contest(contest_id: &str, max_votes: i64, candidate_ids: &[&str]) -> Value {
        json!({
            "id": contest_id,
            "tenant_id": "tenant-1",
            "election_event_id": "event-1",
            "election_id": "election-1",
            "name": "Mayor",
            "description": "Elect the mayor",
            "max_votes": max_votes,
            "min_votes": 0,
            "winning_candidates_num": 1,
            "counting_algorithm": "plurality-at-large",
            "is_encrypted": true,
            "candidates": candidate_ids
                .iter()
                .map(|candidate_id| candidate(contest_id, candidate_id))
                .collect::<Vec<_>>(),
        })
    }

    fn area_style(area_id: &str, contests: Vec<Value>) -> Value {
        json!({
            "id": format!("style-{area_id}"),
            "tenant_id": "tenant-1",
            "election_event_id": "event-1",
            "election_id": "election-1",
            "area_id": area_id,
            "contests": contests,
        })
    }

    fn published_mayor_contest() -> Value {
        contest("contest-1", 1, &["candidate-b", "candidate-c"])
    }

    fn validation_reasons(previous: Value, current: Value) -> Vec<String> {
        let error = validate_started_elections_publication(&[previous], &current)
            .expect_err("the publication must be rejected");
        error
            .downcast_ref::<BallotPublicationValidationError>()
            .expect("a ballot publication validation error")
            .reasons
            .clone()
    }

    #[test]
    fn rejects_added_candidate_after_voting_started() {
        let previous = json!([area_style("area-1", vec![published_mayor_contest()])]);
        let current = json!([area_style(
            "area-1",
            vec![contest(
                "contest-1",
                1,
                &["candidate-a", "candidate-b", "candidate-c"]
            )]
        )]);

        let reasons = validation_reasons(previous, current);

        assert_eq!(reasons.len(), 1);
        assert!(reasons[0].contains("Mayor"));
        assert!(reasons[0].contains("candidates"));
    }

    #[test]
    fn rejects_replaced_candidate_id_after_voting_started() {
        let previous = json!([area_style("area-1", vec![published_mayor_contest()])]);
        let current = json!([area_style(
            "area-1",
            vec![contest("contest-1", 1, &["candidate-a", "candidate-c"])]
        )]);

        let reasons = validation_reasons(previous, current);

        assert_eq!(reasons.len(), 1);
        assert!(reasons[0].contains("candidates"));
    }

    #[test]
    fn rejects_changed_vote_limits_after_voting_started() {
        let previous = json!([area_style("area-1", vec![published_mayor_contest()])]);
        let mut changed = published_mayor_contest();
        changed["max_votes"] = json!(2);
        changed["min_votes"] = json!(1);
        let current = json!([area_style("area-1", vec![changed])]);

        let reasons = validation_reasons(previous, current);

        assert_eq!(reasons.len(), 1);
        assert!(reasons[0].contains("max_votes"));
        assert!(reasons[0].contains("min_votes"));
    }

    #[test]
    fn rejects_changed_counting_algorithm_after_voting_started() {
        let previous = json!([area_style("area-1", vec![published_mayor_contest()])]);
        let mut changed = published_mayor_contest();
        changed["counting_algorithm"] = json!("borda");
        let current = json!([area_style("area-1", vec![changed])]);

        let reasons = validation_reasons(previous, current);

        assert_eq!(reasons.len(), 1);
        assert!(reasons[0].contains("counting_algorithm"));
    }

    #[test]
    fn rejects_changed_vote_policy_after_voting_started() {
        let mut published = published_mayor_contest();
        published["presentation"] = json!({"blank_vote_policy": "allowed"});
        let previous = json!([area_style("area-1", vec![published])]);
        let mut changed = published_mayor_contest();
        changed["presentation"] = json!({"blank_vote_policy": "not-allowed"});
        let current = json!([area_style("area-1", vec![changed])]);

        let reasons = validation_reasons(previous, current);

        assert_eq!(reasons.len(), 1);
        assert!(reasons[0].contains("blank_vote_policy"));
    }

    #[test]
    fn rejects_changed_encoding_mode_after_voting_started() {
        let previous = json!([area_style("area-1", vec![published_mayor_contest()])]);
        let mut changed = area_style("area-1", vec![published_mayor_contest()]);
        changed["multi_contest_encoding_mode"] = json!("expanded-capacity");
        let current = json!([changed]);

        let reasons = validation_reasons(previous, current);

        assert_eq!(reasons.len(), 1);
        assert!(reasons[0].contains("area-1"));
        assert!(reasons[0].contains("multi_contest_encoding_mode"));
    }

    #[test]
    fn rejects_contest_added_to_area_after_voting_started() {
        let previous = json!([area_style("area-1", vec![published_mayor_contest()])]);
        let current = json!([area_style(
            "area-1",
            vec![
                published_mayor_contest(),
                contest("contest-2", 1, &["candidate-d"])
            ]
        )]);

        let reasons = validation_reasons(previous, current);

        assert_eq!(reasons.len(), 1);
        assert!(reasons[0].contains("area-1"));
        assert!(reasons[0].contains("contest_ids"));
    }

    #[test]
    fn rejects_missing_area_after_voting_started() {
        let previous = json!([
            area_style("area-1", vec![published_mayor_contest()]),
            area_style("area-2", vec![published_mayor_contest()])
        ]);
        let current = json!([area_style("area-1", vec![published_mayor_contest()])]);

        let reasons = validation_reasons(previous, current);

        assert_eq!(reasons.len(), 1);
        assert!(reasons[0].contains("area-2"));
    }

    #[test]
    fn reports_a_contest_changed_in_several_areas_once() {
        let previous = json!([
            area_style("area-1", vec![published_mayor_contest()]),
            area_style("area-2", vec![published_mayor_contest()])
        ]);
        let mut changed = published_mayor_contest();
        changed["max_votes"] = json!(2);
        let current = json!([
            area_style("area-1", vec![changed.clone()]),
            area_style("area-2", vec![changed])
        ]);

        let reasons = validation_reasons(previous, current);

        assert_eq!(reasons.len(), 1);
    }

    #[test]
    fn allows_text_changes_after_voting_started() {
        let previous = json!([area_style("area-1", vec![published_mayor_contest()])]);
        let mut changed = published_mayor_contest();
        changed["description"] = json!("Elect the mayor for four years");
        changed["candidates"][0]["description"] = json!("Former councillor");
        let current = json!([area_style("area-1", vec![changed])]);

        assert!(validate_started_elections_publication(&[previous], &current).is_ok());
    }

    #[test]
    fn allows_unset_values_that_decode_like_their_defaults() {
        let previous = json!([area_style("area-1", vec![published_mayor_contest()])]);
        let mut explicit = area_style("area-1", vec![published_mayor_contest()]);
        explicit["multi_contest_encoding_mode"] = json!("legacy");
        let current = json!([explicit]);

        assert!(validate_started_elections_publication(&[previous], &current).is_ok());
    }

    #[test]
    fn allows_new_area_after_voting_started() {
        let previous = json!([area_style("area-1", vec![published_mayor_contest()])]);
        let current = json!([
            area_style("area-1", vec![published_mayor_contest()]),
            area_style("area-2", vec![published_mayor_contest()])
        ]);

        assert!(validate_started_elections_publication(&[previous], &current).is_ok());
    }

    #[test]
    fn allows_any_change_when_no_election_has_started() {
        let current = json!([area_style(
            "area-1",
            vec![contest("contest-1", 3, &["candidate-a"])]
        )]);

        assert!(validate_started_elections_publication(&[], &current).is_ok());
    }

    #[test]
    fn rejects_inconsistent_values_within_one_publication() {
        let result = collect_acclamation_states(&json!([
            {"contests": [{"id": "contest-1", "is_acclaimed": false}]},
            {"contests": [{"id": "contest-1", "is_acclaimed": true}]}
        ]));

        assert!(result.is_err());
    }
}
