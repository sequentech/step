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
use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use tracing::{info, instrument};

use crate::postgres::application::count_applications;
use crate::postgres::election::get_elections;
use crate::postgres::tally_session::get_tally_sessions_by_election_event_id;
use crate::types::application::{ApplicationStatus, ApplicationType};

use super::cast_votes::count_cast_votes_election;
use super::keycloak_events::{
    count_keycloak_events_by_type, list_keycloak_events_by_type, LOGIN_ERR_EVENT_TYPE,
    LOGIN_EVENT_TYPE,
};
use super::reports::report_variables::{VALIDATE_ID_ATTR_NAME, VALIDATE_ID_REGISTERED_VOTER};
use super::reports::voters::EnrollmentFilters;
use super::users::count_keycloak_enabled_users_by_attrs;
use super::voting_status::get_election_status_info;

#[derive(Serialize, Deserialize, Debug)]
pub struct ElectionEventMonitoring {
    pub total_enrolled_voters: i64,
    pub total_elections: i64,
    pub total_open_votes: i64,
    pub total_not_opened_votes: i64,
    pub total_closed_votes: i64,
    pub total_not_closed_votes: i64,
    pub total_start_counting_votes: i64,
    pub total_not_start_counting_votes: i64,
    pub total_initialize: i64,
    pub total_not_initialize: i64,
    pub total_genereated_tally: i64,
    pub total_not_genereated_tally: i64,
    pub total_transmitted_results: i64,
    pub total_not_transmitted_results: i64,
    pub authentication_stats: MonitoringAuthentication,
    pub voting_stats: MonitoringVotingSatus,
    pub approval_stats: MonitoringApproval,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MonitoringAuthentication {
    pub total_authenticated: i64,
    pub total_not_authenticated: i64,
    pub total_invalid_users_errors: i64,
    pub total_invalid_password_errors: i64,
}

#[instrument(skip(hasura_transaction, keycloak_transaction), err)]
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

    let mut total_initialize: i64 = 0;
    let mut total_start_counting_votes: i64 = 0;
    let mut total_genereated_tally: i64 = 0;

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
            Some(true) => total_initialize += 1,
            _ => {}
        }

        let tally_session =
            get_election_tally_session(&tally_sessions, &election_event_id, &election.id);

        let tally_execution_status = get_election_tally_execution_status_summary(&tally_session);
        match tally_execution_status {
            Some(TallyExecutionStatus::IN_PROGRESS) => total_start_counting_votes += 1,
            Some(TallyExecutionStatus::SUCCESS) => {
                total_start_counting_votes += 1;
                total_genereated_tally += 1
            }
            _ => {}
        };
    }

    let authentication_stats =
        get_monitoring_authentication(&keycloak_transaction, &realm, total_enrolled_voters.clone())
            .await
            .map_err(|err| anyhow!("Error at get_monitoring_authentication: {err}"))?;

    let voting_stats =
        get_monitoring_voting_status(&hasura_transaction, &tenant_id, &election_event_id)
            .await
            .map_err(|err| anyhow!("Error at get_monitoring_voting_status: {err}"))?;

    let approval_stats =
        get_monitoring_approval_stats(&hasura_transaction, &tenant_id, &election_event_id, None)
            .await
            .map_err(|err| anyhow!("Error at get_monitoring_approval_stats: {err}"))?;

    Ok(ElectionEventMonitoring {
        total_enrolled_voters,
        total_elections: total_elections,
        total_open_votes,
        total_not_opened_votes,
        total_closed_votes,
        total_not_closed_votes: total_elections - total_closed_votes,
        total_genereated_tally,
        total_not_genereated_tally: total_elections - total_genereated_tally,
        total_initialize,
        total_not_initialize: total_elections - total_initialize,
        total_start_counting_votes,
        total_not_start_counting_votes: total_elections - total_start_counting_votes,
        total_transmitted_results: 0,
        total_not_transmitted_results: 0,
        authentication_stats,
        voting_stats,
        approval_stats,
    })
}

fn get_election_tally_session(
    tally_sessions: &Vec<TallySession>,
    election_event_id: &str,
    election_id: &str,
) -> Option<TallySession> {
    let tally_session = tally_sessions
        .iter()
        .filter(|session| {
            session.election_event_id == election_event_id
                && match session.election_ids {
                    Some(ref election_ids) => election_ids.contains(&election_id.to_string()),
                    None => false,
                }
        })
        .max_by_key(|session| session.created_at);
    tally_session.cloned()
}

fn get_election_tally_execution_status_summary(
    tally_session: &Option<TallySession>,
) -> Option<TallyExecutionStatus> {
    tally_session
        .as_ref()
        .and_then(|session| session.execution_status.as_ref())
        .and_then(|status_str| TallyExecutionStatus::from_str(status_str).ok())
}

#[instrument(skip(keycloak_transaction), err)]
pub async fn get_monitoring_authentication(
    keycloak_transaction: &Transaction<'_>,
    realm: &str,
    total_enrolled_voters: i64,
) -> Result<MonitoringAuthentication> {
    let total_login_events =
        count_keycloak_events_by_type(&keycloak_transaction, &realm, LOGIN_EVENT_TYPE, None, true)
            .await
            .map_err(|err| anyhow!("Error at count LOGIN keycloak events: {err}"))?;

    let total_login_error_events = total_enrolled_voters - total_login_events;

    let total_invalid_users = count_keycloak_events_by_type(
        &keycloak_transaction,
        &realm,
        LOGIN_ERR_EVENT_TYPE,
        Some("user_not_found"),
        false,
    )
    .await
    .map_err(|err| anyhow!("Error at count LOGIN keycloak events: {err}"))?;

    let total_invalid_password = count_keycloak_events_by_type(
        &keycloak_transaction,
        &realm,
        LOGIN_ERR_EVENT_TYPE,
        Some("invalid_user_credentials"),
        false,
    )
    .await
    .map_err(|err| anyhow!("Error at count LOGIN keycloak events: {err}"))?;
    Ok(MonitoringAuthentication {
        total_authenticated: total_login_events,
        total_not_authenticated: total_login_error_events,
        total_invalid_users_errors: total_invalid_users,
        total_invalid_password_errors: total_invalid_password,
    })
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MonitoringApproval {
    pub total_approved: i64,
    pub total_disapproved: i64,
    pub total_manual_approved: i64,
    pub total_manual_disapproved: i64,
    pub total_automated_approved: i64,
    pub total_automated_disapproved: i64,
}

#[instrument(skip(hasura_transaction), err)]
pub async fn get_monitoring_approval_stats(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    area_id: Option<&str>,
) -> Result<MonitoringApproval> {
    let mut filter = EnrollmentFilters {
        status: ApplicationStatus::ACCEPTED,
        approval_type: None,
    };
    let total_approved = count_applications(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        area_id,
        Some(&filter),
    )
    .await
    .map_err(|err| anyhow!("Error at count total applocation approved: {err}"))?;

    filter.approval_type = Some(ApplicationType::MANUAL.to_string());

    let total_manual_approved = count_applications(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        area_id,
        Some(&filter),
    )
    .await
    .map_err(|err| anyhow!("Error at count total manual applocation approved: {err}"))?;

    filter.approval_type = Some(ApplicationType::AUTOMATIC.to_string());

    let total_automated_approved = count_applications(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        area_id,
        Some(&filter),
    )
    .await
    .map_err(|err| anyhow!("Error at count total automated applocation approved: {err}"))?;

    filter.status = ApplicationStatus::REJECTED;
    filter.approval_type = None;

    let total_disapproved = count_applications(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        area_id,
        Some(&filter),
    )
    .await
    .map_err(|err| anyhow!("Error at count total applocation disapproved: {err}"))?;

    filter.approval_type = Some(ApplicationType::MANUAL.to_string());

    let total_manual_disapproved = count_applications(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        area_id,
        Some(&filter),
    )
    .await
    .map_err(|err| anyhow!("Error at count total manual applocation disapproved: {err}"))?;

    filter.approval_type = Some(ApplicationType::AUTOMATIC.to_string());

    let total_automated_disapproved = count_applications(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        area_id,
        Some(&filter),
    )
    .await
    .map_err(|err| anyhow!("Error at count total automated applocation disapproved: {err}"))?;

    Ok(MonitoringApproval {
        total_approved,
        total_disapproved,
        total_manual_approved,
        total_manual_disapproved,
        total_automated_approved,
        total_automated_disapproved,
    })
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MonitoringVotingSatus {
    pub total_voted: i64,
    pub total_voted_tests_elections: i64,
}

#[instrument(skip(hasura_transaction), err)]
pub async fn get_monitoring_voting_status(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<MonitoringVotingSatus> {
    let mut total_voted = 0;
    let cast_votes: Vec<crate::services::cast_votes::ElectionCastVotes> =
        count_cast_votes_election(&hasura_transaction, &tenant_id, &election_event_id, None)
            .await
            .map_err(|err| anyhow!("Error at count cast votes elections: {err}"))?;

    for cast_vote in cast_votes {
        total_voted += cast_vote.cast_votes;
    }

    let mut total_voted_tests_elections = 0;
    let cast_votes_test_elections = count_cast_votes_election(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        Some(true),
    )
    .await
    .map_err(|err| anyhow!("Error at count cast votes elections: {err}"))?;

    for cast_vote in cast_votes_test_elections {
        total_voted_tests_elections += cast_vote.cast_votes;
    }

    Ok(MonitoringVotingSatus {
        total_voted,
        total_voted_tests_elections,
    })
}
