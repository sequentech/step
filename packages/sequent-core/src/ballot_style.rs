// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::ballot::{
    self, AreaPresentation, CandidatePresentation, ContestPresentation,
    ElectionEventPresentation, ElectionPresentation, I18nContent,
    StringifiedPeriodDates,
};

use crate::serialization::deserialize_with_path::deserialize_value;
use crate::types::hasura::core as hasura_types;
use anyhow::{anyhow, Context, Result};
use std::collections::HashMap;
use std::env;

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

pub fn create_ballot_style(
    id: String,
    area: hasura_types::Area,                    // Area
    election_event: hasura_types::ElectionEvent, // Election Event
    election: hasura_types::Election,            // Election
    contests: Vec<hasura_types::Contest>,        // Contest
    candidates: Vec<hasura_types::Candidate>,    // Candidate
    election_dates: StringifiedPeriodDates,      // Election Dates
    public_key: Option<String>,                  // public key
) -> Result<ballot::BallotStyle> {
    let mut sorted_contests = contests
        .clone()
        .into_iter()
        .filter(|contest| contest.election_id == election.id)
        .collect::<Vec<hasura_types::Contest>>();
    sorted_contests.sort_by_key(|k| k.id.clone());
    let demo_public_key_env = env::var("DEMO_PUBLIC_KEY")
        .with_context(|| "DEMO_PUBLIC_KEY env var not found")?;
    let election_event_presentation: ElectionEventPresentation = election_event
        .presentation
        .clone()
        .map(|presentation| serde_json::from_value(presentation))
        .transpose()
        .map_err(|err| {
            anyhow!("Error parsing election Event presentation {:?}", err)
        })?
        .unwrap_or_default();

    let election_event_annotations: HashMap<String, String> = election_event
        .annotations
        .clone()
        .map(|annotations| serde_json::from_value(annotations))
        .transpose()
        .map_err(|err| {
            anyhow!("Error parsing election Event annotations {:?}", err)
        })?
        .unwrap_or_default();

    let election_presentation: ElectionPresentation = election
        .presentation
        .clone()
        .map(|presentation| serde_json::from_value(presentation))
        .transpose()
        .map_err(|err| {
            anyhow!("Error parsing election presentation {:?}", err)
        })?
        .unwrap_or_default();

    let election_annotations: HashMap<String, String> = election
        .annotations
        .clone()
        .map(|annotations| serde_json::from_value(annotations))
        .transpose()
        .map_err(|err| anyhow!("Error parsing election annotations {:?}", err))?
        .unwrap_or_default();

    let contests: Vec<ballot::Contest> = sorted_contests
        .into_iter()
        .map(|contest| {
            let election_candidates = candidates
                .clone()
                .into_iter()
                .filter(|c| c.contest_id == Some(contest.id.clone()))
                .collect::<Vec<hasura_types::Candidate>>();

            create_contest(contest, election_candidates)
        })
        .collect::<Result<Vec<ballot::Contest>>>()?;

    let area_presentation: AreaPresentation = match area.presentation {
        Some(presentation) => {
            deserialize_value(presentation).map_err(|err| {
                anyhow!("Error parsing area presentation: {}", err)
            })?
        }
        None => AreaPresentation::default(),
    };

    Ok(ballot::BallotStyle {
        id,
        tenant_id: election.tenant_id,
        election_event_id: election.election_event_id,
        election_id: election.id,
        num_allowed_revotes: election.num_allowed_revotes,
        description: election.description,
        public_key: Some(
            public_key
                .map(|key| ballot::PublicKeyConfig {
                    public_key: key,
                    is_demo: false,
                })
                .unwrap_or(ballot::PublicKeyConfig {
                    public_key: demo_public_key_env.to_string(),
                    is_demo: true,
                }),
        ),
        area_id: area.id,
        area_presentation: Some(area_presentation),
        contests,
        election_event_presentation: Some(election_event_presentation.clone()),
        election_presentation: Some(election_presentation),
        election_dates: Some(election_dates),
        election_event_annotations: Some(election_event_annotations),
        election_annotations: Some(election_annotations),
    })
}

fn create_contest(
    contest: hasura_types::Contest,
    candidates: Vec<hasura_types::Candidate>,
) -> Result<ballot::Contest> {
    let mut sorted_candidates = candidates.clone();
    sorted_candidates.sort_by_key(|k| k.id.clone());

    let contest_presentation = contest
        .presentation
        .clone()
        .map(|presentation_value| serde_json::from_value(presentation_value))
        .unwrap_or(Ok(ContestPresentation::new()))?;
    let name_i18n = parse_i18n_field(&contest_presentation.i18n, "name");
    let description_i18n =
        parse_i18n_field(&contest_presentation.i18n, "description");
    let alias_i18n = parse_i18n_field(&contest_presentation.i18n, "alias");

    let candidates: Vec<ballot::Candidate> = sorted_candidates
        .iter()
        .enumerate()
        .map(|(_i, candidate)| {
            let candidate_presentation = candidate
                .presentation
                .clone()
                .map(|presentation_value| {
                    serde_json::from_value(presentation_value)
                })
                .unwrap_or(Ok(CandidatePresentation::new()))?;

            let name_i18n =
                parse_i18n_field(&candidate_presentation.i18n, "name");
            let description_i18n =
                parse_i18n_field(&candidate_presentation.i18n, "description");
            let alias_i18n =
                parse_i18n_field(&candidate_presentation.i18n, "alias");

            Ok(ballot::Candidate {
                id: candidate.id.clone(),
                tenant_id: (candidate.tenant_id.clone()),
                election_event_id: (candidate.election_event_id.clone()),
                election_id: (contest.election_id.clone()),
                contest_id: (contest.id.clone()),
                name: candidate.name.clone(),
                name_i18n,
                description: candidate.description.clone(),
                description_i18n,
                alias: candidate.alias.clone(),
                alias_i18n: alias_i18n,
                candidate_type: candidate.r#type.clone(),
                presentation: Some(candidate_presentation),
                annotations: candidate
                    .annotations
                    .clone()
                    .map(|value| deserialize_value(value))
                    .transpose()?,
            })
        })
        .collect::<Result<Vec<ballot::Candidate>>>()?;

    Ok(ballot::Contest {
        id: contest.id.clone(),
        tenant_id: (contest.tenant_id),
        election_event_id: (contest.election_event_id),
        election_id: (contest.election_id.clone()),
        name: contest.name,
        name_i18n,
        description: contest.description,
        description_i18n,
        alias: contest.alias.clone(),
        alias_i18n,
        max_votes: (contest.max_votes.unwrap_or(0)),
        min_votes: (contest.min_votes.unwrap_or(0)),
        winning_candidates_num: contest.winning_candidates_num.unwrap_or(1),
        voting_type: contest.voting_type,
        counting_algorithm: contest.counting_algorithm,
        is_encrypted: (contest.is_encrypted.unwrap_or(false)),
        candidates,
        presentation: Some(contest_presentation),
        created_at: contest.created_at.map(|date| date.to_rfc3339()),
        annotations: contest
            .annotations
            .clone()
            .map(|value| deserialize_value(value))
            .transpose()?,
    })
}
