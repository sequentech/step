// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::ballot::{
    self, AreaAnnotations, AreaPresentation, CandidatePresentation,
    ContestPresentation, ElectionEventPresentation, ElectionPresentation,
    I18nContent, StringifiedPeriodDates, TieBreakingPolicy,
    WeightedVotingPolicy,
};

use crate::serialization::deserialize_with_path::deserialize_value;
use crate::services::translations::{Alias, Name};
use crate::types::ceremonies::CountingAlgType;
use crate::types::hasura::core::{self as hasura_types};
use anyhow::{anyhow, Context, Result};
use std::collections::HashMap;
use std::env;
use std::str::FromStr;

/// Parse an i18n field.
#[must_use]
pub fn parse_i18n_field(
    i18n_opt: &Option<I18nContent<I18nContent<Option<String>>>>,
    field: &str,
) -> Option<I18nContent> {
    let Some(i18n) = i18n_opt else {
        return None;
    };
    let mut content = I18nContent::new();

    for (lang, details) in i18n {
        if let Some(field_value) = details.get(field) {
            content.insert(lang.clone(), field_value.clone());
        };
    }

    Some(content)
}

/// Create a ballot style from the provided parameters.
///
/// # Errors
/// Returns an error if the ballot style cannot be created due to missing or invalid data.
#[allow(clippy::too_many_arguments)]
pub fn create_ballot_style(
    id: String,
    area: &hasura_types::Area, // Area
    election_event: &hasura_types::ElectionEvent, // Election Event
    election: &hasura_types::Election, // Election
    contests: &[hasura_types::Contest], // Contest
    candidates: &[hasura_types::Candidate], // Candidate
    election_dates: StringifiedPeriodDates, // Election Dates
    public_key: Option<String>, // public key
) -> Result<ballot::BallotStyle> {
    let mut sorted_contests = contests
        .iter()
        .filter(|contest| contest.election_id == election.id)
        .cloned()
        .collect::<Vec<hasura_types::Contest>>();
    sorted_contests.sort_by_key(|k| k.id.clone());
    let demo_public_key_env = env::var("DEMO_PUBLIC_KEY")
        .with_context(|| "DEMO_PUBLIC_KEY env var not found")?;
    let election_event_presentation: ElectionEventPresentation = election_event
        .presentation
        .as_ref()
        .map(|v| deserialize_value(v.clone()))
        .transpose()
        .map_err(|err| {
            anyhow!("Error parsing election Event presentation {err:?}")
        })?
        .unwrap_or_default();

    let election_event_annotations: HashMap<String, String> = election_event
        .annotations
        .as_ref()
        .map(|v| deserialize_value(v.clone()))
        .transpose()
        .map_err(|err| {
            anyhow!("Error parsing election Event annotations {err:?}")
        })?
        .unwrap_or_default();

    let election_presentation: ElectionPresentation = election
        .presentation
        .as_ref()
        .map(|v| deserialize_value(v.clone()))
        .transpose()
        .map_err(|err| anyhow!("Error parsing election presentation {err:?}"))?
        .unwrap_or_default();

    let election_annotations: HashMap<String, String> = election
        .annotations
        .as_ref()
        .map(|v| deserialize_value(v.clone()))
        .transpose()
        .map_err(|err| anyhow!("Error parsing election annotations {err:?}"))?
        .unwrap_or_default();

    let default_language = election.get_default_language();

    let ballot_contests: Vec<ballot::Contest> = sorted_contests
        .into_iter()
        .map(|contest| {
            let election_candidates = candidates
                .iter()
                .filter(|c| c.contest_id == Some(contest.id.clone()))
                .cloned()
                .collect::<Vec<hasura_types::Candidate>>();

            create_contest(
                contest,
                election_candidates.as_slice(),
                default_language.clone(),
            )
        })
        .collect::<Result<Vec<ballot::Contest>>>()?;

    let area_annotations = area.read_annotations()?;
    let area_presentation: AreaPresentation = area
        .presentation
        .as_ref()
        .map(|presentation| {
            deserialize_value(presentation.clone()).map_err(|err| {
                anyhow!("Error parsing area presentation: {err}")
            })
        })
        .transpose()?
        .unwrap_or_default();

    Ok(ballot::BallotStyle {
        id,
        tenant_id: election.tenant_id.clone(),
        election_event_id: election.election_event_id.clone(),
        election_id: election.id.clone(),
        num_allowed_revotes: election.num_allowed_revotes,
        description: election.description.clone(),
        public_key: Some(
            public_key
                .map(|key| ballot::PublicKeyConfig {
                    public_key: key,
                    is_demo: false,
                })
                .map_or(
                    ballot::PublicKeyConfig {
                        public_key: demo_public_key_env,
                        is_demo: true,
                    },
                    |cfg| cfg,
                ),
        ),
        area_id: area.id.clone(),
        area_presentation: Some(area_presentation),
        contests: ballot_contests,
        election_event_presentation: Some(election_event_presentation.clone()),
        election_presentation: Some(election_presentation),
        election_dates: Some(election_dates),
        election_event_annotations: Some(election_event_annotations),
        election_annotations: Some(election_annotations),
        area_annotations,
    })
}

/// Create a contest from receiving data.
///
/// # Errors
/// Returns an error if deserialization or parsing fails.
#[allow(clippy::too_many_lines)]
fn create_contest(
    contest: hasura_types::Contest,
    candidates: &[hasura_types::Candidate],
    default_language: String,
) -> Result<ballot::Contest> {
    let mut sorted_candidates = candidates.to_owned();
    sorted_candidates.sort_by_key(|k| k.id.clone());

    let contest_presentation = contest
        .presentation
        .as_ref()
        .map(|v| deserialize_value(v.clone()))
        .map_or(Ok(ContestPresentation::new()), |r| r)?;
    let name_i18n = parse_i18n_field(&contest_presentation.i18n, "name");
    let description_i18n =
        parse_i18n_field(&contest_presentation.i18n, "description");
    let alias_i18n = parse_i18n_field(&contest_presentation.i18n, "alias");

    let ballot_candidates: Vec<ballot::Candidate> = sorted_candidates
        .iter()
        .map(|candidate| {
            let candidate_presentation = candidate
                .presentation
                .as_ref()
                .map(|value| deserialize_value(value.clone()))
                .map_or(Ok(CandidatePresentation::new()), |r| r)?;

            let cand_name_i18n =
                parse_i18n_field(&candidate_presentation.i18n, "name");
            let cand_description_i18n =
                parse_i18n_field(&candidate_presentation.i18n, "description");
            let cand_alias_i18n =
                parse_i18n_field(&candidate_presentation.i18n, "alias");

            let candidate_name = name_i18n
                .as_ref()
                .and_then(|i18n| i18n.get(&default_language))
                .and_then(|name| name.clone());

            let candidate_alias = alias_i18n
                .as_ref()
                .and_then(|i18n| i18n.get(&default_language))
                .and_then(|alias| alias.clone());

            Ok(ballot::Candidate {
                id: candidate.id.clone(),
                tenant_id: candidate.tenant_id.clone(),
                election_event_id: candidate.election_event_id.clone(),
                election_id: contest.election_id.clone(),
                contest_id: contest.id.clone(),
                name: candidate_name,
                name_i18n: cand_name_i18n,
                description: candidate.description.clone(),
                description_i18n: cand_description_i18n,
                alias: candidate_alias,
                alias_i18n: cand_alias_i18n,
                candidate_type: candidate.r#type.clone(),
                presentation: Some(candidate_presentation),
                annotations: candidate
                    .annotations
                    .as_ref()
                    .map(|value| deserialize_value(value.clone()))
                    .transpose()?,
            })
        })
        .collect::<Result<Vec<ballot::Candidate>>>()?;

    let counting_algorithm = CountingAlgType::from_str(
        &contest.counting_algorithm.clone().unwrap_or_default(),
    )
    .map_err(|err| {
        anyhow!(
            "Error parsing CountingAlgorithm from: {:?}. Error: {err:?}",
            contest.counting_algorithm
        )
    })?;

    let contest_name = name_i18n
        .as_ref()
        .and_then(|i18n| i18n.get(&default_language))
        .and_then(|name| name.clone());
    let contest_alias = alias_i18n
        .as_ref()
        .and_then(|i18n| i18n.get(&default_language))
        .and_then(|alias| alias.clone());

    // Extract tie_breaking_policy from tally_configuration JSON
    let tie_breaking_policy = contest
        .tally_configuration
        .as_ref()
        .and_then(|config| config.get("tie_breaking_policy"))
        .and_then(|policy| {
            serde_json::from_value::<TieBreakingPolicy>(policy.clone()).ok()
        });

    Ok(ballot::Contest {
        id: contest.id.clone(),
        tenant_id: contest.tenant_id,
        election_event_id: contest.election_event_id,
        election_id: contest.election_id.clone(),
        name: contest_name,
        name_i18n,
        description: contest.description,
        description_i18n,
        alias: contest_alias,
        alias_i18n,
        max_votes: contest.max_votes.unwrap_or(0),
        min_votes: contest.min_votes.unwrap_or(0),
        winning_candidates_num: contest.winning_candidates_num.unwrap_or(1),
        voting_type: contest.voting_type,
        counting_algorithm: Some(counting_algorithm),
        is_encrypted: contest.is_encrypted.unwrap_or(false),
        candidates: ballot_candidates,
        presentation: Some(contest_presentation),
        created_at: contest.created_at.map(|date| date.to_rfc3339()),
        annotations: contest
            .annotations
            .as_ref()
            .map(|value| deserialize_value(value.clone()))
            .transpose()?,
        tie_breaking_policy,
    })
}
