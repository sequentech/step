// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::ballot::{
    self, CandidatePresentation, CandidatesOrder, ContestPresentation,
    ElectionEventPresentation, I18nContent,
};
use crate::types::hasura_types;
use anyhow::{anyhow, Result};
use serde_json::Value;
use std::str::FromStr;

pub const DEMO_PUBLIC_KEY: &str = "eh8l6lsmKSnzhMewrdLXEKGe9KVxxo//QsCT2wwAkBo";

pub fn create_ballot_style(
    id: String,
    area: hasura_types::Area,                    // Area
    election_event: hasura_types::ElectionEvent, // Election Event
    election: hasura_types::Election,            // Election
    contests: Vec<hasura_types::Contest>,        // Contest
    candidates: Vec<hasura_types::Candidate>,    // Candidate
) -> Result<ballot::BallotStyle> {
    let mut sorted_contests = contests.clone();
    sorted_contests.sort_by_key(|k| k.id.clone());

    let election_event_presentation: ElectionEventPresentation = election_event
        .presentation
        .clone()
        .map(|presentation| serde_json::from_value(presentation))
        .transpose()
        .map_err(|err| {
            anyhow!("Error parsing election Event presentation {:?}", err)
        })?
        .unwrap_or(Default::default());

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

    Ok(ballot::BallotStyle {
        id,
        tenant_id: election.tenant_id,
        election_event_id: election.election_event_id,
        election_id: election.id,
        num_allowed_revotes: election.num_allowed_revotes,
        description: election.description,
        public_key: Some(
            election_event
                .public_key
                .map(|key| ballot::PublicKeyConfig {
                    public_key: key,
                    is_demo: false,
                })
                .unwrap_or(ballot::PublicKeyConfig {
                    public_key: DEMO_PUBLIC_KEY.to_string(),
                    is_demo: true,
                }),
        ),
        area_id: area.id,
        contests: contests,
        election_event_presentation: Some(election_event_presentation.clone()),
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
    let name_i18n = contest_presentation
        .i18n
        .clone()
        .map(|i18n| i18n.get("name").map(|val| val.clone()))
        .flatten();
    let description_i18n = contest_presentation
        .i18n
        .clone()
        .map(|i18n| i18n.get("description").map(|val| val.clone()))
        .flatten();

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

            let name_i18n = candidate_presentation
                .i18n
                .clone()
                .map(|i18n| i18n.get("name").map(|val| val.clone()))
                .flatten();
            let description_i18n = candidate_presentation
                .i18n
                .clone()
                .map(|i18n| i18n.get("description").map(|val| val.clone()))
                .flatten();

            Ok(ballot::Candidate {
                id: candidate.id.clone(),
                tenant_id: candidate.tenant_id.clone(),
                election_event_id: candidate.election_event_id.clone(),
                election_id: contest.election_id.clone(),
                contest_id: contest.id.clone(),
                name: candidate.name.clone(),
                name_i18n,
                description: candidate.description.clone(),
                description_i18n,
                alias: None,
                alias_i18n: None,
                candidate_type: candidate.r#type.clone(),
                presentation: Some(candidate_presentation),
            })
        })
        .collect::<Result<Vec<ballot::Candidate>>>()?;

    Ok(ballot::Contest {
        id: contest.id.clone(),
        tenant_id: contest.tenant_id,
        election_event_id: contest.election_event_id,
        election_id: contest.election_id.clone(),
        name: contest.name,
        name_i18n,
        description: contest.description,
        description_i18n,
        alias: None,
        alias_i18n: None,
        max_votes: contest.max_votes.unwrap_or(0),
        min_votes: contest.min_votes.unwrap_or(0),
        winning_candidates_num: contest.winning_candidates_num.unwrap_or(1),
        voting_type: contest.voting_type,
        counting_algorithm: contest.counting_algorithm,
        is_encrypted: contest.is_encrypted.unwrap_or(false),
        candidates: candidates,
        presentation: Some(contest_presentation),
        created_at: contest.created_at.map(|date| date.to_rfc3339()),
    })
}
