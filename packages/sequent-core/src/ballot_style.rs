use std::str::FromStr;

use anyhow::{anyhow, Result};
use serde_json::Value;

// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::ballot::{
    self, CandidatePresentation, CandidatesOrder, ContestPresentation,
    ElectionEventPresentation, I18nContent,
};
use crate::types::hasura_types;

pub const DEMO_PUBLIC_KEY: &str = "eh8l6lsmKSnzhMewrdLXEKGe9KVxxo//QsCT2wwAkBo";

fn parse_i18n_field(i18n_value: &Value, field: &str) -> Option<I18nContent> {
    let i18n = i18n_value.as_object()?;
    let mut content = I18nContent::new();

    for (lang, details) in i18n {
        if let Some(field_value) = details.get(field)?.as_str() {
            content.insert(lang.to_string(), field_value.to_string());
        } else {
            return None;
        }
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
) -> ballot::BallotStyle {
    let mut sorted_contests = contests.clone();
    sorted_contests.sort_by_key(|k| k.id.clone());

    let mut election_event_css = None;
    let mut election_event_logo_url = None;
    if let Some(election_event_presentation) = election_event.presentation {
        if let Some(val) = election_event_presentation
            .get("css")
            .and_then(Value::as_str)
        {
            election_event_css = Some(val.to_string())
        }

        if let Some(val) = election_event_presentation
            .get("logo_url")
            .and_then(Value::as_str)
        {
            election_event_logo_url = Some(val.to_string())
        }
    }

    ballot::BallotStyle {
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
        contests: sorted_contests
            .into_iter()
            .map(|contest| {
                let election_candidates = candidates
                    .clone()
                    .into_iter()
                    .filter(|c| c.contest_id == Some(contest.id.clone()))
                    .collect::<Vec<hasura_types::Candidate>>();

                create_contest(contest, election_candidates)
            })
            .collect(),
        election_event_presentation: Some(ElectionEventPresentation {
            css: election_event_css,
            logo_url: election_event_logo_url,
            ..Default::default()
        }),
    }
}

fn create_contest(
    contest: hasura_types::Contest,
    candidates: Vec<hasura_types::Candidate>,
) -> ballot::Contest {
    let mut sorted_candidates = candidates.clone();
    sorted_candidates.sort_by_key(|k| k.id.clone());

    let mut cp = ContestPresentation::new();
    let mut name_i18n = None;
    let mut description_i18n = None;

    if let Some(incoming_cp) = contest.presentation {
        if let Some(val) =
            incoming_cp.get("candidates_order").and_then(Value::as_str)
        {
            if let Ok(val) = CandidatesOrder::from_str(val) {
                cp.candidates_order = Some(val)
            }
        }

        if let Some(val) = incoming_cp.get("i18n") {
            name_i18n = parse_i18n_field(val, "name");
            description_i18n = parse_i18n_field(val, "description");
        }
    }

    ballot::Contest {
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
        candidates: sorted_candidates
            .iter()
            .enumerate()
            .map(|(_i, candidate)| {
                let mut cp = CandidatePresentation::new();
                let mut name_i18n = None;
                let mut description_i18n = None;

                if let Some(incoming_cp) = candidate.presentation.clone() {
                    if let Some(val) =
                        incoming_cp.get("sort_order").and_then(Value::as_i64)
                    {
                        cp.sort_order = Some(val);
                    }

                    if let Some(val) = incoming_cp.get("i18n") {
                        name_i18n = parse_i18n_field(val, "name");
                        description_i18n = parse_i18n_field(val, "description");
                    }
                }

                ballot::Candidate {
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
                    presentation: Some(cp),
                }
            })
            .collect(),
        presentation: Some(cp),
        created_at: contest.created_at.map(|date| date.to_rfc3339()),
    }
}
