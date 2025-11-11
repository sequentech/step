// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Result};
use deadpool_postgres::Transaction;
use sequent_core::types::ceremonies::{TallyExecutionStatus, TallyType};
use sequent_core::types::hasura::core::{Election, TallySession};
use sequent_core::types::keycloak::AREA_ID_ATTR_NAME;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use tracing::{info, instrument};

use crate::postgres::application::{count_applications, EnrollmentFilters};
use crate::postgres::area::get_areas_by_election_id;
use crate::postgres::election::get_elections;
use crate::postgres::tally_session::get_tally_sessions_by_election_event_id;
use crate::types::application::{ApplicationStatus, ApplicationType};

use super::cast_votes::{count_ballots_by_election, count_cast_votes_election_event};
use super::consolidation::eml_generator::ValidateAnnotations;
use super::keycloak_events::{
    count_keycloak_events_by_type, LOGIN_ERR_EVENT_TYPE, LOGIN_EVENT_TYPE,
};
use super::reports::report_variables::{VALIDATE_ID_ATTR_NAME, VALIDATE_ID_REGISTERED_VOTER};
use super::transmission::{
    get_transmission_data_from_tally_session_by_area, get_transmission_servers_data,
};
use super::users::{
    count_keycloak_enabled_users_by_attrs, AttributesFilterBy, AttributesFilterOption,
};
use super::voting_status::get_election_status_info;

#[derive(Serialize, Deserialize, Debug)]
pub struct ElectionEventMonitoring {
    pub total_enrolled_voters: i64,
    pub total_elections: i64,
    pub total_started_votes: i64,
    pub total_not_started_votes: i64,
    pub total_open_votes: i64,
    pub total_not_open_votes: i64,
    pub total_closed_votes: i64,
    pub total_not_closed_votes: i64,
    pub total_start_counting_votes: i64,
    pub total_not_start_counting_votes: i64,
    pub total_initialize: i64,
    pub total_not_initialize: i64,
    pub total_genereated_tally: i64,
    pub total_not_genereated_tally: i64,
    pub authentication_stats: MonitoringAuthentication,
    pub voting_stats: MonitoringVotingStatus,
    pub approval_stats: MonitoringApproval,
    pub transmission_stats: MonitoringTransmissionStatus,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ElectionMonitoring {
    pub total_eligible_voters: i64,
    pub total_enrolled_voters: i64,
    pub total_voted: i64,
    pub authentication_stats: MonitoringAuthentication,
    pub approval_stats: MonitoringApproval,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MonitoringTransmissionStatus {
    pub total_transmitted_results: i64,
    pub total_half_transmitted_results: i64,
    pub total_not_transmitted_results: i64,
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
    let mut total_not_started_votes: i64 = 0;
    let mut total_closed_votes: i64 = 0;
    let mut total_started_votes: i64 = 0;

    let mut total_initialize: i64 = 0;
    let mut total_start_counting_votes: i64 = 0;
    let mut total_genereated_tally: i64 = 0;

    let mut total_transmitted_results: i64 = 0;
    let mut total_half_transmitted_results: i64 = 0;
    let mut total_not_transmitted_results: i64 = 0;

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

    let mut attributes: HashMap<String, AttributesFilterOption> = HashMap::new();
    attributes.insert(
        VALIDATE_ID_ATTR_NAME.to_string(),
        AttributesFilterOption {
            value: VALIDATE_ID_REGISTERED_VOTER.to_string(),
            filter_by: AttributesFilterBy::IsEqual,
        },
    );

    let total_enrolled_voters =
        count_keycloak_enabled_users_by_attrs(&keycloak_transaction, realm, Some(attributes))
            .await
            .map_err(|err| anyhow!("Error at getting total enrolled voters: {err}"))?;

    for election in elections {
        let election_status = get_election_status_info(&election);
        total_open_votes += election_status.total_open_votes;
        total_not_started_votes += election_status.total_not_started_votes;
        total_closed_votes += election_status.total_closed_votes;
        total_started_votes += election_status.total_started_votes;

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

        let transmission_status = get_monitoring_transmission_status(
            &hasura_transaction,
            &tenant_id,
            &election_event_id,
            &election,
        )
        .await
        .map_err(|err| anyhow!("Error at getting get_monitoring_transmission_status: {err}"))?;

        match transmission_status {
            TransmissionStatus::Transmitted => total_transmitted_results += 1,
            TransmissionStatus::HalfTransmitted => total_half_transmitted_results += 1,
            TransmissionStatus::NotTransmitted => total_not_transmitted_results += 1,
        }
    }

    let authentication_stats = get_monitoring_authentication(
        &keycloak_transaction,
        &realm,
        total_enrolled_voters.clone(),
        None,
    )
    .await
    .map_err(|err| anyhow!("Error at get_monitoring_authentication: {err}"))?;

    let voting_stats = get_election_event_monitoring_voting_status(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
    )
    .await
    .map_err(|err| anyhow!("Error at get_election_event_monitoring_voting_status: {err}"))?;

    let approval_stats =
        get_monitoring_approval_stats(&hasura_transaction, &tenant_id, &election_event_id, None)
            .await
            .map_err(|err| anyhow!("Error at get_monitoring_approval_stats: {err}"))?;

    Ok(ElectionEventMonitoring {
        total_enrolled_voters,
        total_elections: total_elections,
        total_started_votes,
        total_not_started_votes,
        total_open_votes,
        total_not_open_votes: total_elections - total_open_votes,
        total_closed_votes,
        total_not_closed_votes: total_elections - total_closed_votes,
        total_genereated_tally,
        total_not_genereated_tally: total_elections - total_genereated_tally,
        total_initialize,
        total_not_initialize: total_elections - total_initialize,
        total_start_counting_votes,
        total_not_start_counting_votes: total_elections - total_start_counting_votes,
        authentication_stats,
        voting_stats,
        approval_stats,
        transmission_stats: MonitoringTransmissionStatus {
            total_transmitted_results,
            total_half_transmitted_results,
            total_not_transmitted_results,
        },
    })
}

#[instrument(skip(hasura_transaction, keycloak_transaction), err)]
pub async fn get_election_monitoring(
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    tenant_id: &str,
    realm: &str,
    election_event_id: &str,
    election_id: &str,
) -> Result<ElectionMonitoring> {
    let mut total_enrolled_voters: i64 = 0;
    let mut total_eligible_voters: i64 = 0;
    let mut authentication_stats = MonitoringAuthentication {
        total_authenticated: 0,
        total_not_authenticated: 0,
        total_invalid_users_errors: 0,
        total_invalid_password_errors: 0,
    };
    let mut approval_stats = MonitoringApproval {
        total_approved: 0,
        total_disapproved: 0,
        total_manual_approved: 0,
        total_manual_disapproved: 0,
        total_automated_approved: 0,
        total_automated_disapproved: 0,
    };

    let areas = get_areas_by_election_id(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        &election_id,
    )
    .await
    .map_err(|err| anyhow!("Error at getting areas by election is: {err}"))?;

    for area in areas {
        let mut attributes: HashMap<String, AttributesFilterOption> = HashMap::new();

        attributes.insert(
            AREA_ID_ATTR_NAME.to_string(),
            AttributesFilterOption {
                value: area.id.clone(),
                filter_by: AttributesFilterBy::IsEqual,
            },
        );

        let area_eligible_voters = count_keycloak_enabled_users_by_attrs(
            &keycloak_transaction,
            realm,
            Some(attributes.clone()),
        )
        .await
        .map_err(|err| anyhow!("Error at getting total enrolled voters: {err}"))?;

        attributes.insert(
            VALIDATE_ID_ATTR_NAME.to_string(),
            AttributesFilterOption {
                value: VALIDATE_ID_REGISTERED_VOTER.to_string(),
                filter_by: AttributesFilterBy::IsEqual,
            },
        );
        let area_enrolled_voters =
            count_keycloak_enabled_users_by_attrs(&keycloak_transaction, realm, Some(attributes))
                .await
                .map_err(|err| anyhow!("Error at getting total enrolled voters: {err}"))?;

        total_eligible_voters += area_eligible_voters;
        total_enrolled_voters += area_enrolled_voters;

        let area_authentication_stats = get_monitoring_authentication(
            &keycloak_transaction,
            &realm,
            area_enrolled_voters, // Total VERIFIED voters in this area
            Some(&area.id),
        )
        .await
        .map_err(|err| anyhow!("Error at get_monitoring_authentication: {err}"))?;
        authentication_stats.total_authenticated += area_authentication_stats.total_authenticated;
        authentication_stats.total_not_authenticated +=
            area_authentication_stats.total_not_authenticated;
        authentication_stats.total_invalid_password_errors +=
            area_authentication_stats.total_invalid_password_errors;
        authentication_stats.total_invalid_users_errors +=
            area_authentication_stats.total_invalid_users_errors;

        let area_approval_stats = get_monitoring_approval_stats(
            &hasura_transaction,
            &tenant_id,
            &election_event_id,
            Some(&area.id),
        )
        .await
        .map_err(|err| anyhow!("Error at get_monitoring_approval_stats: {err}"))?;
        approval_stats.total_approved += area_approval_stats.total_approved;
        approval_stats.total_disapproved += area_approval_stats.total_disapproved;
        approval_stats.total_manual_approved += area_approval_stats.total_manual_approved;
        approval_stats.total_manual_disapproved += area_approval_stats.total_manual_disapproved;
        approval_stats.total_automated_approved += area_approval_stats.total_automated_approved;
        approval_stats.total_automated_disapproved +=
            area_approval_stats.total_automated_disapproved;
    }

    let total_voted = count_ballots_by_election(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        &election_id,
    )
    .await
    .map_err(|err| anyhow!("Error at count ballots by election: {err}"))?;

    Ok(ElectionMonitoring {
        total_eligible_voters,
        total_enrolled_voters,
        total_voted,
        authentication_stats,
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
        .filter(|tally| tally.tally_type == Some(TallyType::ELECTORAL_RESULTS.to_string()))
        .and_then(|session| session.execution_status.as_ref())
        .and_then(|status_str| TallyExecutionStatus::from_str(status_str).ok())
}

#[instrument(skip(keycloak_transaction), err)]
pub async fn get_monitoring_authentication(
    keycloak_transaction: &Transaction<'_>,
    realm: &str,
    total_enrolled_voters: i64,
    area_id: Option<&str>,
) -> Result<MonitoringAuthentication> {
    let total_login = count_keycloak_events_by_type(
        &keycloak_transaction,
        &realm,
        LOGIN_EVENT_TYPE,
        None,
        true,
        area_id.clone(),
    )
    .await
    .map_err(|err| anyhow!("Error at count LOGIN keycloak events: {err}"))?;

    let total_not_login = total_enrolled_voters - total_login;

    let total_invalid_users = count_keycloak_events_by_type(
        &keycloak_transaction,
        &realm,
        LOGIN_ERR_EVENT_TYPE,
        Some("user_not_found"),
        false,
        area_id.clone(),
    )
    .await
    .map_err(|err| anyhow!("Error at count LOGIN keycloak events: {err}"))?;

    let total_invalid_password = count_keycloak_events_by_type(
        &keycloak_transaction,
        &realm,
        LOGIN_ERR_EVENT_TYPE,
        Some("invalid_user_credentials"),
        false,
        area_id.clone(),
    )
    .await
    .map_err(|err| anyhow!("Error at count LOGIN keycloak events: {err}"))?;
    Ok(MonitoringAuthentication {
        total_authenticated: total_login,
        total_not_authenticated: total_not_login,
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
        verification_type: None,
    };
    let total_approved = count_applications(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        area_id,
        Some(&filter),
        None,
    )
    .await
    .map_err(|err| anyhow!("Error at count total applocation approved: {err}"))?;

    filter.verification_type = Some(ApplicationType::MANUAL);

    let total_manual_approved = count_applications(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        area_id,
        Some(&filter),
        None,
    )
    .await
    .map_err(|err| anyhow!("Error at count total manual applocation approved: {err}"))?;

    filter.verification_type = Some(ApplicationType::AUTOMATIC);

    let total_automated_approved = count_applications(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        area_id,
        Some(&filter),
        None,
    )
    .await
    .map_err(|err| anyhow!("Error at count total automated applocation approved: {err}"))?;

    filter.status = ApplicationStatus::REJECTED;
    filter.verification_type = None;

    let total_disapproved = count_applications(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        area_id,
        Some(&filter),
        None,
    )
    .await
    .map_err(|err| anyhow!("Error at count total applocation disapproved: {err}"))?;

    filter.verification_type = Some(ApplicationType::MANUAL);

    let total_manual_disapproved = count_applications(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        area_id,
        Some(&filter),
        None,
    )
    .await
    .map_err(|err| anyhow!("Error at count total manual applocation disapproved: {err}"))?;

    filter.verification_type = Some(ApplicationType::AUTOMATIC);

    let total_automated_disapproved = count_applications(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        area_id,
        Some(&filter),
        None,
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
pub struct MonitoringVotingStatus {
    pub total_voted: i64,
    pub total_voted_tests_elections: i64,
}

#[instrument(skip(hasura_transaction), err)]
pub async fn get_election_event_monitoring_voting_status(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<MonitoringVotingStatus> {
    let total_voted =
        count_cast_votes_election_event(&hasura_transaction, &tenant_id, &election_event_id, None)
            .await
            .map_err(|err| anyhow!("Error at count cast votes elections: {err}"))?;

    let total_voted_tests_elections = count_cast_votes_election_event(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        Some(true),
    )
    .await
    .map_err(|err| anyhow!("Error at count cast votes elections: {err}"))?;

    Ok(MonitoringVotingStatus {
        total_voted,
        total_voted_tests_elections,
    })
}

#[derive(Serialize, Deserialize, Debug)]
pub enum TransmissionStatus {
    Transmitted,
    HalfTransmitted,
    NotTransmitted,
}

#[instrument(skip(hasura_transaction, election), err)]
pub async fn get_monitoring_transmission_status(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election: &Election,
) -> Result<TransmissionStatus> {
    let election_areas = get_areas_by_election_id(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        &election.id,
    )
    .await
    .map_err(|err| anyhow!("Error at get_areas_by_election_id: {err:?}"))?;
    let mut areas_half_transmitted_results: i64 = 0;
    let mut areas_not_transmitted_results: i64 = 0;
    for area in election_areas {
        let area_id = area.id.clone();
        let tally_session_data = get_transmission_data_from_tally_session_by_area(
            &hasura_transaction,
            &tenant_id,
            &election_event_id,
            &area_id,
            None,
        )
        .await
        .map_err(|err| anyhow!("{err}"))?;
        let transmission_data = get_transmission_servers_data(&tally_session_data, &area).await?;
        if transmission_data.total_not_transmitted == transmission_data.servers.len() as i64 {
            areas_not_transmitted_results += 1;
        } else if transmission_data.total_transmitted != transmission_data.servers.len() as i64 {
            areas_half_transmitted_results += 1;
        }
    }

    if areas_half_transmitted_results > 0 {
        Ok(TransmissionStatus::HalfTransmitted)
    } else if areas_not_transmitted_results == 0 {
        Ok(TransmissionStatus::Transmitted)
    } else {
        Ok(TransmissionStatus::NotTransmitted)
    }
}
