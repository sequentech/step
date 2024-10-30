// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::report_variables::{
    extract_election_data, generate_voters_turnout, get_app_hash, get_app_version,
    get_date_and_time, get_total_number_of_registered_voters_for_area_id,
};
use super::template_renderer::*;
use crate::postgres::election::get_election_by_id;
use crate::postgres::reports::ReportType;
use crate::postgres::scheduled_event::{
    find_scheduled_event_by_election_event_id,
    find_scheduled_event_by_election_event_id_and_event_processor,
};
use crate::services::cast_votes::count_ballots_by_area_id;
use crate::services::database::{get_hasura_pool, get_keycloak_pool};
use crate::services::temp_path::*;
use crate::{postgres::election_event::get_election_event_by_id, services::s3::get_minio_url};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use deadpool_postgres::{Client as DbClient, Transaction};
use rocket::http::Status;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::types::scheduled_event::generate_voting_period_dates;
use sequent_core::types::templates::EmailConfig;
use serde::{Deserialize, Serialize};
use tracing::instrument;

/// Struct for the initialization report
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserData {
    pub election_date: String,
    pub election_title: String,
    pub geographical_region: String,
    pub post: String,
    pub area_id: String,
    pub voting_center: String,
    pub voting_period_start: String,
    pub voting_period_end: String,
    pub registered_voters: i64,
    pub ballots_counted: i64,
    pub candidate_data: Vec<CandidateData>,
    pub acronym: String,
    pub chairperson_name: String,
    pub poll_clerk_name: String,
    pub third_member_name: String,
    pub precinct_code: String,
    pub report_hash: String,
    pub software_version: String,
    pub ovcs_version: String,
    pub system_hash: String,
}

/// Struct for each candidate's data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CandidateData {
    pub position: String,
    pub position_name: String,
    pub name_in_ballot: String,
    pub votes_garnered: u32,
}

/// Struct for System Data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemData {
    pub rendered_user_template: String,
}

#[derive(Debug)]
pub struct InitializationTemplate {
    tenant_id: String,
    election_event_id: String,
}

#[async_trait]
impl TemplateRenderer for InitializationTemplate {
    type UserData = UserData;
    type SystemData = SystemData;

    fn get_report_type() -> ReportType {
        ReportType::INITIALIZATION
    }

    fn get_tenant_id(&self) -> String {
        self.tenant_id.clone()
    }

    fn get_election_event_id(&self) -> String {
        self.election_event_id.clone()
    }

    fn base_name() -> String {
        "initialization_report".to_string()
    }

    fn prefix(&self) -> String {
        format!("initialization_report_{}", self.election_event_id)
    }

    fn get_email_config() -> EmailConfig {
        EmailConfig {
            subject: "Sequent Online Voting - Initialization".to_string(),
            plaintext_body: "".to_string(),
            html_body: None,
        }
    }

    async fn prepare_user_data(
        &self,
        hasura_transaction: Option<&Transaction<'_>>,
        keycloak_transaction: Option<&Transaction<'_>>,
    ) -> Result<Self::UserData> {
        let Some(hasura_transaction) = hasura_transaction else {
            return Err(anyhow::anyhow!("Hasura Transaction is missing"));
        };

        let realm_name = get_event_realm(self.tenant_id.as_str(), self.election_event_id.as_str());

        // Fetch election event data
        let election_event = get_election_event_by_id(
            &hasura_transaction,
            &self.tenant_id,
            &self.election_event_id,
        )
        .await
        .with_context(|| "Error obtaining election event")?;

        // Split elective position name before the '/'
        let elective_position_name = election_event
            .name
            .split('/')
            .next()
            .unwrap_or("Unknown Position")
            .to_string();

        // TODO: replace mock data with actual data
        // Extract candidate names and acronyms
        let candidates: Vec<CandidateData> = Vec::new(); // Assuming the structure has candidates array
        let mut candidate_data: Vec<CandidateData> = Vec::new();
        // for candidate in candidates {
        //     candidate_data.push(CandidateData {
        //         name_appearing_on_ballot: candidate.name_appearing_on_ballot.clone(),
        //         acronym: candidate.acronym.clone(), // Assuming acronym is part of the candidate structure
        //         votes_garnered: 0, // Default value since no votes have been cast yet
        //     });
        // }

        let election = match get_election_by_id(
            &hasura_transaction,
            &self.get_tenant_id(),
            &self.get_election_event_id(),
            &self.get_election_id().unwrap(),
        )
        .await
        .with_context(|| "Error getting election by id")?
        {
            Some(election) => election,
            None => return Err(anyhow::anyhow!("Election not found")),
        };

        // get election instace's general data (post, area, etc...)
        let election_general_data = match extract_election_data(&election).await {
            Ok(data) => data, // Extracting the ElectionData struct out of Ok
            Err(err) => {
                return Err(anyhow::anyhow!(format!(
                    "Error fetching election data: {}",
                    err
                )));
            }
        };

        // Fetch election event data
        let start_election_event = find_scheduled_event_by_election_event_id(
            &hasura_transaction,
            &self.get_tenant_id(),
            &self.get_election_event_id(),
        )
        .await
        .map_err(|e| {
            anyhow::anyhow!("Error getting scheduled event by election event_id: {}", e)
        })?;

        // Fetch election's voting periods
        let voting_period_dates = generate_voting_period_dates(
            start_election_event,
            &self.get_tenant_id(),
            &self.get_election_event_id(),
            Some(&self.get_election_id().unwrap()),
        )?;

        // extract start date from voting period
        let voting_period_start_date = voting_period_dates.start_date.unwrap_or_default();
        // extract end date from voting period
        let voting_period_end_date = voting_period_dates.end_date.unwrap_or_default();
        let election_date: &String = &voting_period_start_date;

        // fetch total of registerd voters
        let registered_voters = if let Some(transaction) = keycloak_transaction {
            get_total_number_of_registered_voters_for_area_id(
                &transaction, // Pass the actual reference to the transaction
                &realm_name,
                &election_general_data.area_id,
            )
            .await
            .map_err(|e| {
                anyhow::anyhow!(
                    "Error fetching count_keycloak_enabled_users_by_area_id '{}': {}",
                    &election_general_data.area_id,
                    e
                )
            })?
        } else {
            return Err(anyhow::anyhow!("Keycloak Transaction is missing"));
        };

        let ballots_counted = count_ballots_by_area_id(
            &hasura_transaction,
            &self.tenant_id,
            &self.election_event_id,
            &self.get_election_id().unwrap(),
            &election_general_data.area_id,
        )
        .await
        .map_err(|e| anyhow::anyhow!("Error fetching the number of ballots {e:?}",))?;

        let temp_val: &str = "test";
        let report_hash = "-".to_string();
        let ovcs_version = get_app_version();
        let system_hash = get_app_hash();
        let software_version = ovcs_version.clone();

        Ok(UserData {
            election_date: election_date.to_string(),
            election_title: election.name.clone(),
            voting_period_start: voting_period_start_date,
            voting_period_end: voting_period_end_date,
            registered_voters: registered_voters,
            ballots_counted: ballots_counted,
            candidate_data,
            geographical_region: election_general_data.geographical_region,
            acronym: "JD".to_string(),
            post: election_general_data.post,
            area_id: election_general_data.area_id,
            voting_center: election_general_data.voting_center,
            precinct_code: election_general_data.precinct_code,
            chairperson_name: temp_val.to_string(),
            poll_clerk_name: temp_val.to_string(),
            third_member_name: temp_val.to_string(),
            report_hash,
            ovcs_version,
            system_hash,
            software_version,
        })
    }

    #[instrument]
    async fn prepare_system_data(
        &self,
        rendered_user_template: String,
    ) -> Result<Self::SystemData> {
        Ok(SystemData {
            rendered_user_template,
        })
    }
}
