// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::ballot_publication::{
    get_ballot_publication_by_id, get_previous_publication, get_previous_publication_election,
    insert_ballot_publication, lock_publication_event, soft_delete_other_ballot_publications,
    update_ballot_publication,
};
use crate::postgres::ballot_style::get_publication_ballot_styles;
use crate::postgres::election::{get_election_by_id, get_elections_ids, update_election_status};
use crate::postgres::election_event::{get_election_event_by_id, update_election_event_status};
use crate::services::celery_app::get_celery_app;
use crate::services::election_event_board::get_election_event_board;
use crate::services::election_event_status::{get_election_event_status, get_election_status};
use crate::services::electoral_log::*;
use crate::services::tasks_execution::{
    post as post_task_execution, update_fail as update_task_execution_fail,
};
use crate::tasks::update_election_event_ballot_styles::update_election_event_ballot_styles;
use crate::types::tasks::ETasksExecution;
use anyhow::{anyhow, Context, Result};
use chrono::{Local, Utc};
use deadpool_postgres::Transaction;
use sequent_core::ballot::{
    BallotStyle, Contest, ContestEncryptionPolicy, EBlankVotePolicy, EDuplicatedRankPolicy,
    EOverVotePolicy, EPreferenceGapsPolicy, EUnderVotePolicy, ElectionEventStatus, ElectionStatus,
    InvalidVotePolicy, MultiContestEncodingMode,
};
use sequent_core::serialization::deserialize_with_path::*;
use sequent_core::services::connection;
use sequent_core::services::date::ISO8601;
use sequent_core::types::ceremonies::CountingAlgType;
use sequent_core::types::hasura::core::{BallotPublication, TasksExecution};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use thiserror::Error;
use tracing::{event, instrument, Level};

use super::ballot_style;

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
    fn new(reasons: Vec<String>) -> Self {
        let message = format!(
            "Ballot publication validation failed:\n- {}",
            reasons.join("\n- ")
        );
        Self { reasons, message }
    }
}

fn collect_acclamation_states(
    publication: &Value,
) -> Result<BTreeMap<String, ContestAcclamationState>> {
    let ballot_styles = publication
        .as_array()
        .context("Ballot publication must be an array of ballot styles")?;
    let mut states: BTreeMap<String, ContestAcclamationState> = BTreeMap::new();

    for ballot_style in ballot_styles {
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
    }

    Ok(states)
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

fn parse_ballot_styles(publication: &Value) -> Result<Vec<BallotStyle>> {
    publication
        .as_array()
        .context("Ballot publication must be an array of ballot styles")?
        .iter()
        .map(|style| {
            deserialize_value(style.clone())
                .map_err(|error| anyhow!("Could not read ballot style: {error:?}"))
        })
        .collect()
}

/// Rejects a publication that would change how ballots already cast with
/// `previous` decode: an area left without a ballot style, or any change to
/// a [`BallotStyleDecoding`] or [`ContestDecoding`] field.
fn validate_ballot_style_decoding(previous: &[BallotStyle], current: &[BallotStyle]) -> Result<()> {
    let current_by_area: HashMap<(&str, &str), &BallotStyle> = current
        .iter()
        .map(|style| ((style.election_id.as_str(), style.area_id.as_str()), style))
        .collect();
    let mut reasons = Vec::new();
    let mut changed_contests: BTreeMap<&str, (&Contest, BTreeSet<String>)> = BTreeMap::new();

    for previous_style in previous {
        let Some(current_style) = current_by_area.get(&(
            previous_style.election_id.as_str(),
            previous_style.area_id.as_str(),
        )) else {
            reasons.push(format!(
                "Area {} has no ballot style for election {} after voting started for that election. Restore its contests before publishing again.",
                previous_style.area_id, previous_style.election_id
            ));
            continue;
        };

        let style_changes = changed_fields(
            &BallotStyleDecoding::from(previous_style),
            &BallotStyleDecoding::from(*current_style),
        )?;
        if !style_changes.is_empty() {
            reasons.push(format!(
                "The ballot style of area {} in election {} changed {} after voting started for that election. Restore the published values before publishing again.",
                previous_style.area_id,
                previous_style.election_id,
                style_changes.into_iter().collect::<Vec<_>>().join(", ")
            ));
        }

        for previous_contest in &previous_style.contests {
            let Some(current_contest) = current_style
                .contests
                .iter()
                .find(|contest| contest.id == previous_contest.id)
            else {
                continue;
            };
            let contest_changes = changed_fields(
                &ContestDecoding::from(previous_contest),
                &ContestDecoding::from(current_contest),
            )?;
            if !contest_changes.is_empty() {
                changed_contests
                    .entry(previous_contest.id.as_str())
                    .or_insert_with(|| (current_contest, BTreeSet::new()))
                    .1
                    .extend(contest_changes);
            }
        }
    }

    reasons.extend(
        changed_contests
            .into_iter()
            .map(|(contest_id, (contest, fields))| {
                let contest_name = contest.name.as_deref().unwrap_or(contest_id);
                format!(
                    "Contest \"{contest_name}\" ({contest_id}) changed {} after voting started for its election. Restore the published values before publishing again.",
                    fields.into_iter().collect::<Vec<_>>().join(", ")
                )
            }),
    );

    if reasons.is_empty() {
        Ok(())
    } else {
        Err(BallotPublicationValidationError::new(reasons).into())
    }
}

/// Checks a publication against the latest published ballot styles of each
/// of its elections where voting has started. `previous_publications` holds
/// those published ballot styles, one publication per started election.
fn validate_started_elections_publication(
    previous_publications: &[Value],
    current_publication: &Value,
) -> Result<()> {
    let current = collect_acclamation_states(current_publication)?;
    let mut previous = BTreeMap::new();
    for publication in previous_publications {
        merge_acclamation_states(&mut previous, collect_acclamation_states(publication)?)?;
    }

    validate_acclamation_states(&previous, &current)?;

    if previous_publications.is_empty() {
        return Ok(());
    }
    let mut previous_styles = Vec::new();
    for publication in previous_publications {
        previous_styles.extend(parse_ballot_styles(publication)?);
    }
    validate_ballot_style_decoding(&previous_styles, &parse_ballot_styles(current_publication)?)
}

async fn validate_published_ballot_styles(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    ballot_publication: &BallotPublication,
) -> Result<()> {
    let current = get_publication_json(
        hasura_transaction,
        tenant_id.to_owned(),
        election_event_id.to_owned(),
        ballot_publication.id.clone(),
        None,
        None,
    )
    .await?;
    let mut previous_publications = Vec::new();
    let election_ids = ballot_publication
        .election_ids
        .as_ref()
        .context("Ballot publication is missing its election ids")?;
    if election_ids.is_empty() {
        return Err(anyhow!("Ballot publication has no election ids"));
    }

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

        let publication = get_publication_json(
            hasura_transaction,
            tenant_id.to_owned(),
            election_event_id.to_owned(),
            previous_publication.id,
            Some(election_id.clone()),
            None,
        )
        .await?;
        previous_publications.push(publication);
    }

    validate_started_elections_publication(&previous_publications, &current)
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
pub async fn add_ballot_publication(
    hasura_transaction: &Transaction<'_>,
    tenant_id: String,
    election_event_id: String,
    election_id: Option<String>,
    user_id: String,
    executer_name: &str,
) -> Result<(String, TasksExecution)> {
    let celery_app = get_celery_app().await;

    let election_ids = get_election_ids_for_publication(
        hasura_transaction,
        tenant_id.clone(),
        election_event_id.clone(),
        election_id.clone(),
    )
    .await?;

    let ballot_publication = insert_ballot_publication(
        hasura_transaction,
        &tenant_id.clone(),
        &election_event_id.clone(),
        election_ids.clone(),
        user_id.clone(),
        election_id.clone(),
    )
    .await?
    .with_context(|| "can't find inserted ballot publication")?;

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

    validate_published_ballot_styles(
        hasura_transaction,
        &tenant_id,
        &election_event_id,
        &ballot_publication,
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
            Some(user_id),
            Some(username),
        )
        .await
        .map_err(|e| anyhow!("error posting to the electoral log: {e}"))?;
    Ok(())
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

    fn publication(contests: Value) -> Value {
        json!([{ "contests": contests }])
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
