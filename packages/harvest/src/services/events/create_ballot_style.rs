// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{Context, Result};
use immu_board::BoardClient;
use rocket::serde::{Deserialize, Serialize};
use sequent_core;
use std::collections::HashMap;
use std::convert::From;
use std::env;
use tracing::instrument;

use crate::connection;
use crate::hasura;
use crate::hasura::ballot_style::get_ballot_style_area;
use crate::routes::scheduled_event::ScheduledEvent;

impl From<&get_ballot_style_area::GetBallotStyleAreaSequentBackendElectionEvent>
    for sequent_core::hasura_types::ElectionEvent
{
    fn from(
        election_event: &get_ballot_style_area::GetBallotStyleAreaSequentBackendElectionEvent,
    ) -> Self {
        sequent_core::hasura_types::ElectionEvent {
            id: election_event.id.clone(),
            created_at: None, //election_event.created_at,
            updated_at: None, //election_event.updated_at,
            labels: election_event.labels.clone(),
            annotations: election_event.annotations.clone(),
            tenant_id: election_event.tenant_id.clone(),
            name: election_event.name.clone(),
            description: election_event.description.clone(),
            presentation: election_event.presentation.clone(),
            bulletin_board_reference: election_event
                .bulletin_board_reference
                .clone(),
            is_archived: election_event.is_archived.clone(),
            voting_channels: election_event.voting_channels.clone(),
            dates: election_event.dates.clone(),
            status: election_event.status.clone(),
            user_boards: election_event.user_boards.clone(),
            encryption_protocol: election_event.encryption_protocol.clone(),
            is_audit: election_event.is_audit.clone(),
            audit_election_event_id: election_event
                .audit_election_event_id
                .clone(),
        }
    }
}

impl From<&get_ballot_style_area::GetBallotStyleAreaSequentBackendElection>
    for sequent_core::hasura_types::Election
{
    fn from(
        election: &get_ballot_style_area::GetBallotStyleAreaSequentBackendElection,
    ) -> Self {
        sequent_core::hasura_types::Election {
            id: election.id.clone(),
            tenant_id: election.tenant_id.clone(),
            election_event_id: election.election_event_id.clone(),
            created_at: None,      //election.created_at,
            last_updated_at: None, //election.last_updated_at,
            labels: election.labels.clone(),
            annotations: election.annotations.clone(),
            name: election.name.clone(),
            description: election.description.clone(),
            presentation: election.presentation.clone(),
            dates: election.dates.clone(),
            status: election.status.clone(),
            eml: election.eml.clone(),
            num_allowed_revotes: election.num_allowed_revotes.clone(),
            is_consolidated_ballot_encoding: election
                .is_consolidated_ballot_encoding
                .clone(),
            spoil_ballot_option: election.spoil_ballot_option.clone(),
        }
    }
}

impl From<get_ballot_style_area::GetBallotStyleAreaSequentBackendAreaContestContest>
    for sequent_core::hasura_types::Contest
{
    fn from(
        contest: get_ballot_style_area::GetBallotStyleAreaSequentBackendAreaContestContest,
    ) -> Self {
        sequent_core::hasura_types::Contest {
            id: contest.id.clone(),
            tenant_id: contest.tenant_id.clone(),
            election_event_id: contest.election_event_id.clone(),
            election_id: contest.election_id.clone(),
            created_at: None, // contest.created_at.clone(),
            last_updated_at: None, // contest.last_updated_at.clone(),
            labels: contest.labels.clone(),
            annotations: contest.annotations.clone(),
            is_acclaimed: contest.is_acclaimed.clone(),
            is_active: contest.is_active.clone(),
            name: contest.name.clone(),
            description: contest.description.clone(),
            presentation: contest.presentation.clone(),
            min_votes: contest.min_votes.clone(),
            max_votes: contest.max_votes.clone(),
            voting_type: contest.voting_type.clone(),
            counting_algorithm: contest.counting_algorithm.clone(),
            is_encrypted: contest.is_encrypted.clone(),
            tally_configuration: contest.tally_configuration.clone(),
            conditions: contest.conditions.clone(),
        }
    }
}

impl From<get_ballot_style_area::GetBallotStyleAreaSequentBackendAreaContestContestCandidates>
    for sequent_core::hasura_types::Candidate
{
    fn from(
        candidate: get_ballot_style_area::GetBallotStyleAreaSequentBackendAreaContestContestCandidates,
    ) -> Self {
        sequent_core::hasura_types::Candidate {
            id: candidate.id.clone(),
            tenant_id: candidate.tenant_id.clone(),
            election_event_id: candidate.election_event_id.clone(),
            contest_id: candidate.contest_id.clone(),
            created_at: None, //candidate.created_at.clone(),
            last_updated_at: None, //candidate.last_updated_at.clone(),
            labels: candidate.labels.clone(),
            annotations: candidate.annotations.clone(),
            name: candidate.name.clone(),
            description: candidate.description.clone(),
            r#type: candidate.type_.clone(),
            presentation: candidate.presentation.clone(),
            is_public: candidate.is_public.clone(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
pub struct CreateBallotStylePayload {
    pub area_id: String,
}

#[instrument(skip(auth_headers))]
pub async fn create_ballot_style(
    auth_headers: connection::AuthHeaders,
    body: CreateBallotStylePayload,
    event: ScheduledEvent,
) -> Result<()> {
    // read tenant_id and election_event_id
    let tenant_id = event
        .tenant_id
        .clone()
        .with_context(|| "scheduled event is missing tenant_id")?;
    let election_event_id = event
        .election_event_id
        .clone()
        .with_context(|| "scheduled event is missing election_event_id")?;
    let hasura_response = hasura::ballot_style::get_ballot_style_area(
        auth_headers,
        tenant_id,
        election_event_id,
        body.area_id,
    )
    .await?
    .data
    .expect("expected data".into());
    let area = &hasura_response.sequent_backend_area[0];
    let election_event: &get_ballot_style_area::GetBallotStyleAreaSequentBackendElectionEvent = &hasura_response.sequent_backend_election_event[0];
    let elections = &hasura_response.sequent_backend_election;
    let area_contests = &hasura_response.sequent_backend_area_contest;

    // election_id, vec<contest_ids>
    let mut election_contest_map: HashMap<String, Vec<String>> = HashMap::new();

    for area_contest in area_contests.iter() {
        if area_contest.contest.is_none() {
            continue;
        }
        let contest = area_contest.contest.clone().unwrap();
        let election_id = contest.election_id.clone();
        election_contest_map
            .entry(contest.election_id.clone())
            .and_modify(|contest_ids| contest_ids.push(contest.id.clone()))
            .or_insert(vec![contest.id.clone()]);
    }

    for (election_id, contest_ids) in election_contest_map.into_iter() {
        let election = elections
            .iter()
            .find(|election| election.id == election_id)
            .unwrap();
        let contests: Vec<sequent_core::hasura_types::Contest> = contest_ids
            .clone()
            .into_iter()
            .map(|contest_id| -> sequent_core::hasura_types::Contest {
                let area_contest = area_contests
                    .iter()
                    .find(|area_contest| {
                        area_contest.contest_id == Some(contest_id.clone())
                    })
                    .unwrap();
                sequent_core::hasura_types::Contest::from(
                    area_contest.contest.clone().unwrap(),
                )
            })
            .collect();
        let candidates: Vec<sequent_core::hasura_types::Candidate> =
            contest_ids
                .into_iter()
                .map(|contest_id| -> Vec<sequent_core::hasura_types::Candidate> {
                    let area_contest = area_contests
                        .iter()
                        .find(|area_contest| {
                            area_contest.contest_id == Some(contest_id.clone())
                        })
                        .unwrap();
                    area_contest
                        .contest
                        .clone()
                        .unwrap()
                        .candidates
                        .into_iter()
                        .map(|candidate| {
                            let _c: get_ballot_style_area::GetBallotStyleAreaSequentBackendAreaContestContestCandidates = candidate.clone();
                            sequent_core::hasura_types::Candidate::from(
                                candidate.clone(),
                            )
                        })
                        .collect()
                })
                .flatten()
                .collect();

        sequent_core::ballot_style::create_ballot_style(
            sequent_core::hasura_types::ElectionEvent::from(election_event),
            sequent_core::hasura_types::Election::from(election),
            contests,
            candidates,
        );
    }

    Ok(())
}
