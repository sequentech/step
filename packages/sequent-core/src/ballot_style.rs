// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::ballot;
use crate::hasura_types;

pub const DEMO_PUBLIC_KEY: &str = "/jXUkdSIgz8mXLZ4BIDPQzDx7ZFFIG3MWuacDLyhyhoCAAAAGORKDU/t+8fKNkZMFfXl1IMM+/0VmINTZCcbalZ/NSUi5SbzUTlyzh25lMuVALwvC/lk3j6SHn6BotYphk0QMA";

pub fn create_ballot_style(
    id: String,
    area: hasura_types::Area,                    // Area
    election_event: hasura_types::ElectionEvent, // Election Event
    election: hasura_types::Election,            // Election
    contests: Vec<hasura_types::Contest>,        // Contest
    candidates: Vec<hasura_types::Candidate>,    // Answer
) -> ballot::BallotStyle {
    ballot::BallotStyle {
        id: id,
        tenant_id: election.tenant_id,
        election_event_id: election.election_event_id,
        election_id: election.id,
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
        status: election
            .status
            .map(|status_js| serde_json::from_value(status_js).ok())
            .flatten(),
        contests: contests
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
    }
}

fn create_contest(
    contest: hasura_types::Contest,
    candidates: Vec<hasura_types::Candidate>,
) -> ballot::Contest {
    ballot::Contest {
        id: contest.id.clone(),
        tenant_id: contest.tenant_id,
        election_event_id: contest.election_event_id,
        election_id: contest.election_id.clone(),
        name: contest.name,
        description: contest.description,
        max_votes: contest.max_votes.unwrap_or(0),
        min_votes: contest.min_votes.unwrap_or(0),
        voting_type: contest.voting_type,
        counting_algorithm: contest.counting_algorithm,
        is_encrypted: contest.is_encrypted.unwrap_or(false),
        candidates: candidates
            .iter()
            .enumerate()
            .map(|(i, candidate)| ballot::Candidate {
                id: candidate.id.clone(),
                tenant_id: candidate.tenant_id.clone(),
                election_event_id: candidate.election_event_id.clone(),
                election_id: contest.election_id.clone(),
                contest_id: contest.id.clone(),
                name: candidate.name.clone(),
                description: candidate.description.clone(),
                candidate_type: candidate.r#type.clone(),
                presentation: candidate
                    .presentation
                    .clone()
                    .map(|presentation_js| {
                        serde_json::from_value(presentation_js).ok()
                    })
                    .flatten(),
            })
            .collect(),
        presentation: None,
    }
}
