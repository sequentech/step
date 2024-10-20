// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::template_renderer::*;
use crate::postgres::election::get_election_by_id;
use crate::postgres::reports::ReportType;
use crate::postgres::scheduled_event::find_scheduled_event_by_election_event_id;
use super::report_variables::{
    extract_election_data, generate_voters_turnout, get_date_and_time,
    get_election_contests_area_results_and_total_ballot_counted,
    get_total_number_of_registered_voters_for_country,
};
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::types::scheduled_event::generate_voting_period_dates;
use crate::services::database::{get_hasura_pool, get_keycloak_pool};
use crate::services::temp_path::*;
use crate::{postgres::election_event::get_election_event_by_id, services::s3::get_minio_url};
use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{Local, TimeZone};
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use sequent_core::types::templates::EmailConfig;
use serde::{Deserialize, Serialize};
use std::env;
use tracing::{info, instrument};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserData {
    pub date_printed: String,
    pub closing_election_datetime: String,
    pub election_date: String,
    pub election_title: String,
    pub voting_period: String,
    pub geographical_region: String,
    pub post: String,
    pub country: String,
    pub voting_center: String,
    pub precinct_code: String,
    pub registered_voters: i64,
    pub ballots_counted: i64,
    pub voters_turnout: String,
    pub candidates: Vec<Candidate>,
    pub chairperson_name: String,
    pub chairperson_digital_signature: String,
    pub poll_clerk_name: String,
    pub poll_clerk_digital_signature: String,
    pub third_member_name: String,
    pub third_member_digital_signature: String,
    pub report_hash: String,
    pub software_version: String,
    pub ovcs_version: String,
    pub system_hash: String,
    pub qr_codes: Vec<String>,
    pub goverment_time: String,
}

/// Struct for each candidate's data
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Candidate {
    pub position: String,
    pub position_name: String,
    pub name_in_ballot: String,
    pub votes_garnered: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SystemData {
    pub rendered_user_template: String,
}

#[derive(Debug)]
pub struct ElectionReturnsForNationalPostionTemplate {
    tenant_id: String,
    election_event_id: String,
}

#[async_trait]
impl TemplateRenderer for ElectionReturnsForNationalPostionTemplate {
    type UserData = UserData;
    type SystemData = SystemData;

    fn get_report_type() -> ReportType {
        ReportType::ELECTION_RETURNS_FOR_NATIONAL_POSITIONS
    }

    fn get_tenant_id(&self) -> String {
        self.tenant_id.clone()
    }

    fn get_election_event_id(&self) -> String {
        self.election_event_id.clone()
    }

    fn base_name() -> String {
        "election_returns_for_national_positions".to_string()
    }

    fn prefix(&self) -> String {
        format!(
            "election_returns_for_national_positions_{}",
            self.election_event_id
        )
    }

    fn get_email_config() -> EmailConfig {
        EmailConfig {
            subject: "Sequent Online Voting - Election Returns For National Positions".to_string(),
            plaintext_body: "".to_string(),
            html_body: None,
        }
    }

    #[instrument]
    async fn prepare_user_data(&self) -> Result<Self::UserData> {
        let date_time_printed = Local::now().to_string();
        let printing_code = "XYZ123".to_string(); // Example placeholder for a real printing code

        let mut hasura_db_client: DbClient = get_hasura_pool()
            .await
            .get()
            .await
            .with_context(|| "Error getting Hasura db pool")?;

        let hasura_transaction = hasura_db_client
            .transaction()
            .await
            .with_context(|| "Error starting Hasura transaction")?;

        let realm_name = get_event_realm(self.tenant_id.as_str(), self.election_event_id.as_str());
        let mut keycloak_db_client = get_keycloak_pool()
            .await
            .get()
            .await
            .with_context(|| "Error acquiring Keycloak DB pool")?;
    
        let keycloak_transaction = keycloak_db_client
            .transaction()
            .await
            .with_context(|| "Error starting Keycloak transaction")?;

        // Fetch election event data
        let election_event = get_election_event_by_id(
            &hasura_transaction,
            &self.tenant_id,
            &self.election_event_id,
        )
        .await
        .with_context(|| "Error obtaining election event")?;

        // get election instace
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

        // get election instace's general data (post, country, etc...)
        let election_general_data = match extract_election_data(&election).await {
            Ok(data) => data, // Extracting the ElectionData struct out of Ok
            Err(err) => {
                return Err(anyhow::anyhow!(format!(
                    "Error fetching election data: {}",
                    err
                )));
            }
        };


        // Fetch total registered voters and ballots counted
        let registered_voters = get_total_number_of_registered_voters_for_country(
            &keycloak_transaction,
            &realm_name,
            &election_general_data.country,
        )
        .await
        .map_err(|e| {
            anyhow::anyhow!(format!(
                "Error in getting the number of registered voters {:?}",
                e
            ))
        })?;

        let (ballots_counted, results_area_contests, contests) =
            get_election_contests_area_results_and_total_ballot_counted(
                &hasura_transaction,
                &self.get_tenant_id(),
                &self.get_election_event_id(),
                &self.get_election_id().unwrap(),
            )
            .await
            .map_err(|e| {
                anyhow::anyhow!(format!(
                    "Error in getting election contests area results {:?}",
                    e
                ))
            })?;

        // Calculate voter turnout percentage
        let voters_turnout = generate_voters_turnout(&ballots_counted, &registered_voters)
            .await
            .map_err(|e| anyhow::anyhow!(format!("Error in generating voters turnout {:?}", e)))?;

        // Fetch election event data
        let start_election_event = find_scheduled_event_by_election_event_id(
            &hasura_transaction,
            &self.get_tenant_id(),
            &self.get_election_event_id(),
        )
        .await
        .map_err(|e| {
            anyhow::anyhow!(format!(
                "Error getting scheduled event by election event_id {:?}",
                e
            ))
        })?;

        let date_printed = get_date_and_time();
        // Fetch election's voting periods
        let voting_period_dates = generate_voting_period_dates(
            start_election_event,
            &self.get_tenant_id(),
            &self.get_election_event_id(),
            Some(&self.get_election_id().unwrap()),
        )?;

        // extract start date from voting period
        let voting_period_start_date = match voting_period_dates.start_date {
            Some(voting_period_start_date) => voting_period_start_date,
            None => {
                return Err(anyhow::anyhow!(format!(
                    "Error fetching election start date: "
                )))
            }
        };
         // extract end date from voting period
         let voting_period_end_date = match voting_period_dates.end_date {
            Some(voting_period_end_date) => voting_period_end_date,
            None => {
                return Err(anyhow::anyhow!(format!(
                    "Error fetching election end date: "
                )))
            }
        };
        let election_date = &voting_period_start_date;
        let closing_election_datetime = "2024-10-09T12:05:00Z".to_string();
        // Extract candidate names and acronyms
        let candidates: Vec<Candidate> = Vec::new(); // Assuming the structure has candidates array
        // let mut candidate_data: Vec<CandidateData> = Vec::new();
        // for candidate in candidates {
        //     candidate_data.push(CandidateData {
        //         name_appearing_on_ballot: candidate.name_appearing_on_ballot.clone(),
        //         acronym: candidate.acronym.clone(), // Assuming acronym is part of the candidate structure
        //         votes_garnered: 0, // Default value since no votes have been cast yet
        //     });
        // }

        let election_title = election_event.name.clone();
        let temp_val: &str = "test";
        Ok(UserData {
            election_date: election_date.to_string(),
            closing_election_datetime,
            election_title,
            registered_voters,
            ballots_counted,
            voters_turnout: voters_turnout.to_string(),
            candidates,
            geographical_region: election_general_data.geographical_region,
            post: election_general_data.post,
            country: election_general_data.country,
            voting_center: election_general_data.voting_center,
            voting_period: format!("{} - {}", voting_period_start_date, voting_period_end_date),
            precinct_code: election_general_data.clustered_precinct_id,
            software_version: temp_val.to_string(),
            chairperson_name: temp_val.to_string(),
            chairperson_digital_signature: temp_val.to_string(),
            poll_clerk_name: temp_val.to_string(),
            poll_clerk_digital_signature: temp_val.to_string(),
            third_member_name: temp_val.to_string(),
            third_member_digital_signature: temp_val.to_string(),
            report_hash: String::new(),
            ovcs_version: String::new(),
            system_hash: String::new(),
            date_printed,
            qr_codes: vec![
                "String 1".to_string(),
                "String 2".to_string(),
                "String 3".to_string(),
                "String 4".to_string(),
            ],
            goverment_time: "18:00".to_string(),
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

#[instrument]
pub async fn generate_election_returns_for_national_positions_report(
    document_id: &str,
    tenant_id: &str,
    election_event_id: &str,
    mode: GenerateReportMode,
) -> Result<()> {
    let template = ElectionReturnsForNationalPostionTemplate {
        tenant_id: tenant_id.to_string(),
        election_event_id: election_event_id.to_string(),
    };
    template
        .execute_report(
            document_id,
            tenant_id,
            election_event_id,
            false,
            None,
            None,
            mode,
        )
        .await
}
