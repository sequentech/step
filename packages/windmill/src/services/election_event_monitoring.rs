// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Result};
use deadpool_postgres::Transaction;
use sequent_core::ballot::{ElectionPresentation, ElectionStatus, VotingPeriodDates, VotingStatus};
use sequent_core::serialization::deserialize_with_path::deserialize_value;
use sequent_core::types::ceremonies::TallyExecutionStatus;
use sequent_core::types::hasura::core::TallySession;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use tracing::{info, instrument};

use crate::postgres::election::get_elections;
use crate::postgres::tally_session::get_tally_sessions_by_election_event_id;

use super::reports::report_variables::{VALIDATE_ID_ATTR_NAME, VALIDATE_ID_REGISTERED_VOTER};
use super::users::count_keycloak_enabled_users_by_attrs;
use super::voting_status::get_election_status_info;

#[derive(Serialize, Deserialize, Debug)]
pub struct ElectionEventMonitoring {
    total_enrolled_voters: i64,
    total_elections: i64,
    total_open_votes: i64,
    total_not_opened_votes: i64,
    total_closed_votes: i64,
    total_not_closed_votes: i64,
    total_transmitted_results: i64,
    total_not_transmitted_results: i64,
    total_genereated_er: i64,
    total_not_genereated_er: i64,
    total_start_counting_votes: i64,
    total_not_start_counting_votes: i64,
    total_initialization: i64,
}

#[instrument(skip(hasura_transaction), err)]
pub async fn get_election_event_monitoring(
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    tenant_id: &str,
    realm: &str,
    election_event_id: &str,
) -> Result<ElectionEventMonitoring> {
    let mut total_open_votes: i64 = 0;
    let mut total_not_opened_votes: i64 = 0;
    let mut total_closed_votes: i64 = 0;

    let mut total_initialization: i64 = 0;
    let mut total_start_counting_votes: i64 = 0;

    let elections = get_elections(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        Some(false),
    )
    .await
    .map_err(|err| anyhow!("Error at get_elections: {err}"))?;

    let tally_sessions = get_tally_sessions_by_election_event_id(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        false,
    )
    .await
    .map_err(|err| anyhow!("Error at getting tally session by eleciton event id: {err}"))?;
    let total_elections: i64 = elections.len() as i64;

    let mut attributes: HashMap<String, String> = HashMap::new();
    attributes.insert(
        VALIDATE_ID_ATTR_NAME.to_string(),
        VALIDATE_ID_REGISTERED_VOTER.to_string(),
    );

    let total_enrolled_voters =
        count_keycloak_enabled_users_by_attrs(&keycloak_transaction, realm, Some(attributes))
            .await
            .map_err(|err| anyhow!("Error at getting total enrolled voters: {err}"))?;

    for election in elections {
        let election_status = get_election_status_info(&election);
        total_open_votes += election_status.total_open_votes;
        total_not_opened_votes += election_status.total_not_opened_votes;
        total_closed_votes += election_status.total_closed_votes;

        match election.initialization_report_generated {
            Some(true) => total_initialization += 1,
            _ => {}
        }

        let is_start_counting =
            is_election_start_counting_votes(&tally_sessions, &election_event_id, &election.id);
        match is_start_counting {
            Some(true) => total_start_counting_votes += 1,
            _ => {}
        }
    }

    Ok(ElectionEventMonitoring {
        total_enrolled_voters,
        total_elections: total_elections,
        total_open_votes,
        total_not_opened_votes,
        total_closed_votes,
        total_not_closed_votes: total_elections - total_closed_votes,
        total_transmitted_results: 0,
        total_not_transmitted_results: 0,
        total_genereated_er: 0,
        total_not_genereated_er: 0,
        total_start_counting_votes: 0,
        total_not_start_counting_votes: 0,
        total_initialization,
    })
}

fn is_election_start_counting_votes(
    tally_sessions: &Vec<TallySession>,
    election_event_id: &str,
    election_id: &str,
) -> Option<bool> {
    let tally_sessions = tally_sessions.clone();
    let election_tally_session = tally_sessions
        .iter()
        .filter(|session| {
            session.election_event_id == election_event_id
                && match session.election_ids {
                    Some(ref election_ids) => election_ids.contains(&election_id.to_string()),
                    None => false,
                }
        })
        .max_by_key(|session| session.created_at);

    match election_tally_session {
        Some(tally_session) => match tally_session.execution_status.clone() {
            Some(execution_status_str) => {
                let Some(execution_status) =
                    TallyExecutionStatus::from_str(&execution_status_str).ok()
                else {
                    return None;
                };
                match execution_status {
                    TallyExecutionStatus::IN_PROGRESS | TallyExecutionStatus::SUCCESS => Some(true),
                    _ => None,
                }
            }
            None => None,
        },
        None => None,
    }
}
