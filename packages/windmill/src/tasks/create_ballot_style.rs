// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{Context, Result};
use celery::error::TaskError;
use celery::prelude::*;
use immu_board::BoardClient;
use serde::{Deserialize, Serialize};
use sequent_core;
use std::collections::HashMap;
use std::convert::From;
use std::env;
use tracing::{event, instrument, Level};
use uuid::Uuid;

use crate::connection;
use crate::hasura;
use crate::hasura::ballot_style::get_ballot_style_area;
use crate::hasura::event_execution::insert_event_execution_with_result;
use crate::services::date::ISO8601;
use crate::types::scheduled_event::ScheduledEvent;

impl From<&get_ballot_style_area::GetBallotStyleAreaSequentBackendElectionEvent>
    for sequent_core::hasura_types::ElectionEvent
{
    fn from(
        election_event: &get_ballot_style_area::GetBallotStyleAreaSequentBackendElectionEvent,
    ) -> Self {
        sequent_core::hasura_types::ElectionEvent {
            id: election_event.id.clone(),
            created_at: election_event
                .created_at
                .clone()
                .map(|value| ISO8601::to_date(value.as_str()).unwrap()),
            updated_at: election_event
                .updated_at
                .clone()
                .map(|value| ISO8601::to_date(value.as_str()).unwrap()),
            labels: election_event.labels.clone(),
            annotations: election_event.annotations.clone(),
            tenant_id: election_event.tenant_id.clone(),
            name: election_event.name.clone(),
            description: election_event.description.clone(),
            presentation: election_event.presentation.clone(),
            bulletin_board_reference: election_event.bulletin_board_reference.clone(),
            is_archived: election_event.is_archived.clone(),
            voting_channels: election_event.voting_channels.clone(),
            dates: election_event.dates.clone(),
            status: election_event.status.clone(),
            user_boards: election_event.user_boards.clone(),
            encryption_protocol: election_event.encryption_protocol.clone(),
            is_audit: election_event.is_audit.clone(),
            audit_election_event_id: election_event.audit_election_event_id.clone(),
            public_key: election_event.public_key.clone(),
        }
    }
}

impl From<&get_ballot_style_area::GetBallotStyleAreaSequentBackendElection>
    for sequent_core::hasura_types::Election
{
    fn from(election: &get_ballot_style_area::GetBallotStyleAreaSequentBackendElection) -> Self {
        sequent_core::hasura_types::Election {
            id: election.id.clone(),
            tenant_id: election.tenant_id.clone(),
            election_event_id: election.election_event_id.clone(),
            created_at: election
                .created_at
                .clone()
                .map(|value| ISO8601::to_date(value.as_str()).unwrap()),
            last_updated_at: election
                .last_updated_at
                .clone()
                .map(|value| ISO8601::to_date(value.as_str()).unwrap()),
            labels: election.labels.clone(),
            annotations: election.annotations.clone(),
            name: election.name.clone(),
            description: election.description.clone(),
            presentation: election.presentation.clone(),
            dates: election.dates.clone(),
            status: election.status.clone(),
            eml: election.eml.clone(),
            num_allowed_revotes: election.num_allowed_revotes.clone(),
            is_consolidated_ballot_encoding: election.is_consolidated_ballot_encoding.clone(),
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
            created_at: contest
                .created_at
                .clone()
                .map(|value| ISO8601::to_date(value.as_str()).unwrap()),
            last_updated_at: contest
                .last_updated_at
                .clone()
                .map(|value| ISO8601::to_date(value.as_str()).unwrap()),
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
            created_at: candidate
                .created_at
                .clone()
                .map(|value| ISO8601::to_date(value.as_str()).unwrap()),
            last_updated_at: candidate
                .last_updated_at
                .clone()
                .map(|value| ISO8601::to_date(value.as_str()).unwrap()),
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

impl From<&get_ballot_style_area::GetBallotStyleAreaSequentBackendArea>
    for sequent_core::hasura_types::Area
{
    fn from(area: &get_ballot_style_area::GetBallotStyleAreaSequentBackendArea) -> Self {
        sequent_core::hasura_types::Area {
            id: area.id.clone(),
            tenant_id: area.tenant_id.clone(),
            election_event_id: area.election_event_id.clone(),
            created_at: area
                .created_at
                .clone()
                .map(|value| ISO8601::to_date(value.as_str()).unwrap()),
            last_updated_at: area
                .last_updated_at
                .clone()
                .map(|value| ISO8601::to_date(value.as_str()).unwrap()),
            labels: area.labels.clone(),
            annotations: area.annotations.clone(),
            name: area.name.clone(),
            description: area.description.clone(),
            r#type: area.type_.clone(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreateBallotStylePayload {
    pub area_id: String,
}

#[instrument(skip(auth_headers))]
#[celery::task]
pub async fn create_ballot_style(
    auth_headers: connection::AuthHeaders,
    event: ScheduledEvent,
    body: CreateBallotStylePayload,
) -> TaskResult<()> {
    // read tenant_id and election_event_id
    let tenant_id = event
        .tenant_id
        .clone()
        .with_context(|| "scheduled event is missing tenant_id")
        .map_err(|err| TaskError::UnexpectedError(format!("{:?}", err)))?;
    let election_event_id = event
        .election_event_id
        .clone()
        .with_context(|| "scheduled event is missing election_event_id")
        .map_err(|err| TaskError::UnexpectedError(format!("{:?}", err)))?;
    let hasura_response = hasura::ballot_style::get_ballot_style_area(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
        body.area_id.clone(),
    )
    .await
    .map_err(|err| TaskError::UnexpectedError(format!("{:?}", err)))?
    .data
    .with_context(|| "can't find election event")
    .map_err(|err| TaskError::UnexpectedError(format!("{:?}", err)))?;

    let area = &hasura_response.sequent_backend_area[0];
    let election_event: &get_ballot_style_area::GetBallotStyleAreaSequentBackendElectionEvent =
        &hasura_response.sequent_backend_election_event[0];
    let elections = &hasura_response.sequent_backend_election;
    let area_contests = &hasura_response.sequent_backend_area_contest;

    // election_id, vec<contest_ids>
    let mut election_contest_map: HashMap<String, Vec<String>> = HashMap::new();

    for area_contest in area_contests.iter() {
        if area_contest.contest.is_none() {
            event!(
                Level::INFO,
                "missing contest for area contest: {}",
                area_contest.id
            );
            continue;
        }
        let contest = area_contest
            .contest
            .clone()
            .with_context(|| format!("contest not found for area contest {}", area_contest.id))
            .map_err(|err| TaskError::UnexpectedError(format!("{:?}", err)))?;
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
            .with_context(|| format!("election id not found {}", election_id))
            .map_err(|err| TaskError::UnexpectedError(format!("{:?}", err)))?;
        let contests = contest_ids
            .clone()
            .into_iter()
            .map(
                |contest_id| -> Result<sequent_core::hasura_types::Contest> {
                    let area_contest = area_contests
                        .iter()
                        .find(|area_contest| area_contest.contest_id == Some(contest_id.clone()))
                        .with_context(|| format!("contest id not found {}", contest_id))
                        .map_err(|err| TaskError::UnexpectedError(format!("{:?}", err)))?;
                    Ok(sequent_core::hasura_types::Contest::from(
                        area_contest.contest.clone().unwrap(),
                    ))
                },
            )
            .collect::<Result<Vec<sequent_core::hasura_types::Contest>>>()
            .map_err(|err| TaskError::UnexpectedError(format!("{:?}", err)))?;
        let candidates: Vec<sequent_core::hasura_types::Candidate> = contest_ids
            .into_iter()
            .map(
                |contest_id| -> Result<Vec<sequent_core::hasura_types::Candidate>> {
                    let area_contest = area_contests
                        .iter()
                        .find(|area_contest| area_contest.contest_id == Some(contest_id.clone()))
                        .with_context(|| format!("contest id not found {}", contest_id))
                        .map_err(|err| TaskError::UnexpectedError(format!("{:?}", err)))?;
                    area_contest
                        .contest
                        .clone()
                        .with_context(|| {
                            format!("contest missing on area contest id {}", area_contest.id)
                        })
                        .map_err(|err| TaskError::UnexpectedError(format!("{:?}", err)))?
                        .candidates
                        .into_iter()
                        .map(|candidate| {
                            Ok(sequent_core::hasura_types::Candidate::from(
                                candidate.clone(),
                            ))
                        })
                        .collect::<Result<Vec<sequent_core::hasura_types::Candidate>>>()
                },
            )
            .into_iter()
            .collect::<Result<Vec<Vec<sequent_core::hasura_types::Candidate>>>>()
            .map_err(|err| TaskError::UnexpectedError(format!("{:?}", err)))?
            .into_iter()
            .flatten()
            .collect();

        let ballot_style_id = Uuid::new_v4();
        let election_dto = sequent_core::ballot_style::create_ballot_style(
            ballot_style_id.clone().to_string(),
            sequent_core::hasura_types::Area::from(area),
            sequent_core::hasura_types::ElectionEvent::from(election_event),
            sequent_core::hasura_types::Election::from(election),
            contests,
            candidates,
        );
        let election_dto_json_string = serde_json::to_string(&election_dto)
            .map_err(|err| TaskError::UnexpectedError(format!("{:?}", err)))?;
        let hasura_response = hasura::ballot_style::insert_ballot_style(
            auth_headers.clone(),
            ballot_style_id.to_string(),
            tenant_id.clone(),
            election_event_id.clone(),
            election.id.clone(),
            body.area_id.clone(),
            Some(election_dto_json_string),
            None,
            None,
        )
        .await
        .map_err(|err| TaskError::UnexpectedError(format!("{:?}", err)))?;
    }

    insert_event_execution_with_result(auth_headers, event, None)
        .await
        .map_err(|err| TaskError::ExpectedError(format!("{:?}", err)))?;

    Ok(())
}
