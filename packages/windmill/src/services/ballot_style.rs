// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::database::get_hasura_pool;
use crate::postgres::area::get_event_areas;
use crate::postgres::area_contest::export_area_contests;
use crate::postgres::ballot_publication::{
    get_ballot_publication_by_id, update_ballot_publication_status,
};
use crate::postgres::ballot_style::insert_ballot_style;
use crate::postgres::candidate::export_candidates;
use crate::postgres::contest::export_contests;
use crate::postgres::election::export_elections;
use crate::postgres::election_event::get_election_event_by_id;
use crate::services::import_election_event::AreaContest;
use crate::types::error::Result;
use anyhow::{anyhow, Context, Result as AnyhowResult};
use chrono::Duration;
use deadpool_postgres::{Client as DbClient, Transaction};
use futures::try_join;
use sequent_core::services::connection;
use sequent_core::types::hasura::core::{
    self as hasura_type, Area, BallotPublication, Candidate, Contest, Election, ElectionEvent,
};

use std::collections::HashMap;
use std::convert::From;
use tracing::{event, instrument, Level};
use uuid::Uuid;

use crate::hasura;
use crate::hasura::ballot_style::get_ballot_style_area;
use crate::services::date::ISO8601;
use crate::services::pg_lock::PgLock;

impl From<&get_ballot_style_area::GetBallotStyleAreaSequentBackendElectionEvent>
    for hasura_type::ElectionEvent
{
    fn from(
        election_event: &get_ballot_style_area::GetBallotStyleAreaSequentBackendElectionEvent,
    ) -> Self {
        hasura_type::ElectionEvent {
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
            alias: None,
            statistics: None,
        }
    }
}

impl From<&get_ballot_style_area::GetBallotStyleAreaSequentBackendElection>
    for hasura_type::Election
{
    fn from(election: &get_ballot_style_area::GetBallotStyleAreaSequentBackendElection) -> Self {
        hasura_type::Election {
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
            alias: election.alias.clone(),
            voting_channels: election.voting_channels.clone(),
            is_kiosk: election.is_kiosk.clone(),
            image_document_id: election.image_document_id.clone(),
            statistics: election.statistics.clone(),
            receipts: election.receipts.clone(),
        }
    }
}

impl From<get_ballot_style_area::GetBallotStyleAreaSequentBackendAreaContestContest>
    for hasura_type::Contest
{
    fn from(
        contest: get_ballot_style_area::GetBallotStyleAreaSequentBackendAreaContestContest,
    ) -> Self {
        hasura_type::Contest {
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
            alias: contest.alias.clone(),
            description: contest.description.clone(),
            presentation: contest.presentation.clone(),
            min_votes: contest.min_votes.clone(),
            max_votes: contest.max_votes.clone(),
            winning_candidates_num: contest.winning_candidates_num,
            voting_type: contest.voting_type.clone(),
            counting_algorithm: contest.counting_algorithm.clone(),
            is_encrypted: contest.is_encrypted.clone(),
            tally_configuration: contest.tally_configuration.clone(),
            conditions: contest.conditions.clone(),
            image_document_id: None,
        }
    }
}

impl From<get_ballot_style_area::GetBallotStyleAreaSequentBackendAreaContestContestCandidates>
    for hasura_type::Candidate
{
    fn from(
        candidate: get_ballot_style_area::GetBallotStyleAreaSequentBackendAreaContestContestCandidates,
    ) -> Self {
        hasura_type::Candidate {
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
            alias: candidate.alias.clone(),
            description: candidate.description.clone(),
            r#type: candidate.type_.clone(),
            presentation: candidate.presentation.clone(),
            is_public: candidate.is_public,
            image_document_id: None,
        }
    }
}

impl From<&get_ballot_style_area::GetBallotStyleAreaSequentBackendArea> for hasura_type::Area {
    fn from(area: &get_ballot_style_area::GetBallotStyleAreaSequentBackendArea) -> Self {
        hasura_type::Area {
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

#[instrument(err)]
pub async fn create_ballot_style(
    auth_headers: connection::AuthHeaders,
    area_id: String,
    tenant_id: String,
    election_event_id: String,
    election_ids: Vec<String>,
    ballot_publication_id: String,
) -> Result<()> {
    let lock = PgLock::acquire(
        format!("create_ballot_style-{}-{}", tenant_id, election_event_id),
        Uuid::new_v4().to_string(),
        ISO8601::now() + Duration::seconds(60),
    )
    .await?;
    let hasura_response = hasura::ballot_style::get_ballot_style_area(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
        area_id.clone(),
    )
    .await?
    .data
    .with_context(|| "can't find election event")?;

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
            .with_context(|| format!("contest not found for area contest {}", area_contest.id))?;
        if !election_ids.contains(&contest.election_id) {
            continue;
        }
        election_contest_map
            .entry(contest.election_id.clone())
            .and_modify(|contest_ids| contest_ids.push(contest.id.clone()))
            .or_insert(vec![contest.id.clone()]);
    }

    for (election_id, contest_ids) in election_contest_map.into_iter() {
        let election = elections
            .iter()
            .find(|election| election.id == election_id)
            .with_context(|| format!("election id not found {}", election_id))?;
        let contests = contest_ids
            .clone()
            .into_iter()
            .map(|contest_id| -> Result<hasura_type::Contest> {
                let area_contest = area_contests
                    .iter()
                    .find(|area_contest| area_contest.contest_id == Some(contest_id.clone()))
                    .with_context(|| format!("contest id not found {}", contest_id))?;
                Ok(hasura_type::Contest::from(
                    area_contest.contest.clone().unwrap(),
                ))
            })
            .collect::<Result<Vec<hasura_type::Contest>>>()?;
        let candidates: Vec<hasura_type::Candidate> = contest_ids
            .into_iter()
            .map(|contest_id| -> Result<Vec<hasura_type::Candidate>> {
                let area_contest = area_contests
                    .iter()
                    .find(|area_contest| area_contest.contest_id == Some(contest_id.clone()))
                    .with_context(|| format!("contest id not found {}", contest_id))?;
                area_contest
                    .contest
                    .clone()
                    .with_context(|| {
                        format!("contest missing on area contest id {}", area_contest.id)
                    })?
                    .candidates
                    .into_iter()
                    .map(|candidate| Ok(hasura_type::Candidate::from(candidate.clone())))
                    .collect::<Result<Vec<hasura_type::Candidate>>>()
            })
            .collect::<Result<Vec<Vec<hasura_type::Candidate>>>>()?
            .into_iter()
            .flatten()
            .collect();

        let ballot_style_id = Uuid::new_v4();
        let election_dto = sequent_core::ballot_style::create_ballot_style(
            ballot_style_id.clone().to_string(),
            hasura_type::Area::from(area),
            hasura_type::ElectionEvent::from(election_event),
            hasura_type::Election::from(election),
            contests,
            candidates,
        )?;
        let election_dto_json_string = serde_json::to_string(&election_dto)?;
        let _hasura_response = hasura::ballot_style::insert_ballot_style(
            auth_headers.clone(),
            ballot_style_id.to_string(),
            tenant_id.clone(),
            election_event_id.clone(),
            election.id.clone(),
            area_id.clone(),
            Some(election_dto_json_string),
            None,
            None,
            ballot_publication_id.clone(),
        )
        .await?;
    }
    lock.release().await?;

    Ok(())
}

pub async fn create_ballot_style_postgres(
    transaction: &Transaction<'_>,
    area: &Area,
    tenant_id: &str,
    election_event: &ElectionEvent,
    ballot_publication: &BallotPublication,
    elections: &Vec<Election>,
    contests: &Vec<Contest>,
    candidates: &Vec<Candidate>,
    area_contests_input: &Vec<AreaContest>,
) -> Result<()> {
    let election_ids = ballot_publication.election_ids.clone().unwrap_or(vec![]);
    if 0 == election_ids.len() {
        event!(Level::INFO, "No election ids",);
        return Ok(());
    }
    let lock = PgLock::acquire(
        format!("create_ballot_style-{}-{}", tenant_id, election_event.id),
        Uuid::new_v4().to_string(),
        ISO8601::now() + Duration::seconds(60),
    )
    .await?;
    let area_contests: Vec<AreaContest> = area_contests_input
        .clone()
        .into_iter()
        .filter(|area_contest| area_contest.area_id.to_string() == area.id)
        .collect();

    // election_id, vec<contest_ids>
    let mut election_contest_map: HashMap<String, Vec<String>> = HashMap::new();

    for area_contest in area_contests.iter() {
        let Some(contest) = contests
            .iter()
            .find(|contest| contest.id == area_contest.contest_id.to_string())
        else {
            event!(
                Level::INFO,
                "missing contest for area contest: {}",
                area_contest.id
            );
            continue;
        };
        if !election_ids.contains(&contest.election_id) {
            continue;
        }
        election_contest_map
            .entry(contest.election_id.clone())
            .and_modify(|contest_ids| contest_ids.push(contest.id.clone()))
            .or_insert(vec![contest.id.clone()]);
    }

    for (election_id, contest_ids) in election_contest_map.into_iter() {
        let election = elections
            .iter()
            .find(|election| election.id == election_id)
            .with_context(|| format!("election id not found {}", election_id))?;
        let contests = contest_ids
            .clone()
            .into_iter()
            .map(|contest_id| -> Result<hasura_type::Contest> {
                let area_contest = area_contests
                    .iter()
                    .find(|area_contest| area_contest.contest_id.to_string() == contest_id)
                    .with_context(|| format!("contest id not found {}", contest_id))?;
                let contest = contests
                    .iter()
                    .find(|contest| contest.id == contest_id)
                    .with_context(|| format!("contest not found {}", contest_id))?;
                Ok(contest.clone())
            })
            .collect::<Result<Vec<hasura_type::Contest>>>()?;
        let candidates: Vec<hasura_type::Candidate> = contest_ids
            .into_iter()
            .map(|contest_id| -> Result<Vec<hasura_type::Candidate>> {
                let area_contest = area_contests
                    .iter()
                    .find(|area_contest| area_contest.contest_id.to_string() == contest_id)
                    .with_context(|| format!("contest id not found {}", contest_id))?;
                let area_candidates: Vec<Candidate> = candidates
                    .clone()
                    .into_iter()
                    .filter(|candidate| candidate.contest_id == Some(contest_id))
                    .collect();
                Ok(area_candidates)
            })
            .collect::<Result<Vec<Vec<hasura_type::Candidate>>>>()?
            .into_iter()
            .flatten()
            .collect();

        let ballot_style_id = Uuid::new_v4();
        let election_dto = sequent_core::ballot_style::create_ballot_style(
            ballot_style_id.clone().to_string(),
            hasura_type::Area::from(area),
            hasura_type::ElectionEvent::from(election_event),
            hasura_type::Election::from(election),
            contests,
            candidates,
        )?;
        let election_dto_json_string = serde_json::to_string(&election_dto)?;
        let _hasura_response = insert_ballot_style(
            transaction,
            &ballot_style_id.to_string(),
            tenant_id,
            &election_event.id,
            &election.id,
            &area.id,
            Some(election_dto_json_string),
            None,
            None,
            &ballot_publication.id,
        )
        .await?;
    }
    lock.release().await?;

    Ok(())
}

#[instrument(err)]
pub async fn update_election_event_ballot_styles(
    tenant_id: &str,
    election_event_id: &str,
    ballot_publication_id: &str,
) -> AnyhowResult<()> {
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .with_context(|| "Error getting hasura db pool")?;

    let transaction = hasura_db_client
        .transaction()
        .await
        .with_context(|| "Error starting hasura transaction")?;

    let Some(ballot_publication) = get_ballot_publication_by_id(
        &transaction,
        tenant_id,
        election_event_id,
        ballot_publication_id,
    )
    .await?
    else {
        return Err(anyhow!("can't find ballot publication"));
    };
    let (election_event, elections, contests, candidates, areas, area_contests) = try_join!(
        get_election_event_by_id(&transaction, tenant_id, election_event_id),
        export_elections(&transaction, tenant_id, election_event_id),
        export_contests(&transaction, tenant_id, election_event_id),
        export_candidates(&transaction, tenant_id, election_event_id),
        get_event_areas(&transaction, tenant_id, election_event_id),
        export_area_contests(&transaction, tenant_id, election_event_id),
    )?;

    for area in &areas {
        create_ballot_style_postgres(
            &transaction,
            area,
            &tenant_id,
            &election_event,
            &ballot_publication,
            &elections,
            &contests,
            &candidates,
            &area_contests,
        )
        .await?;
    }
    update_ballot_publication_status(
        &transaction,
        tenant_id,
        election_event_id,
        ballot_publication_id,
        true,
        None,
    )
    .await?;

    let _commit = transaction.commit().await.with_context(|| "Commit failed");
    Ok(())
}
