// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::template_renderer::*;
use crate::postgres::reports::ReportType;
use crate::postgres::scheduled_event::find_scheduled_event_by_election_event_id_and_event_processor;
use crate::services::database::get_hasura_pool;
use crate::services::temp_path::*;
use crate::{postgres::election_event::get_election_event_by_id, services::s3::get_minio_url};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use sequent_core::types::templates::EmailConfig;
use serde::{Deserialize, Serialize};

/// Struct for the initialization report
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserData {
    pub election_start_date: String,
    pub election_title: String,
    pub geograpic_region: String,
    pub area: String,
    pub country: String,
    pub voting_center: String,
    pub total_registered_voters: u32,
    pub total_ballots_counted: u32,
    pub elective_position_name: String,
    pub candidate_data: Vec<CandidateData>,
    pub chairperson_name: String,
    pub poll_clerk_name: String,
    pub third_member_name: String,
}

/// Struct for each candidate's data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CandidateData {
    pub name_appearing_on_ballot: String,
    pub acronym: String,
    pub votes_garnered: u32,
}

/// Struct for System Data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemData {
    pub report_hash: String,
    pub ovsc_version: String,
    pub system_hash: String,
    pub file_logo: String,
    pub file_qrcode_lib: String,
    pub date_time_printed: String,
    pub printing_code: String,
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

    async fn prepare_user_data(&self) -> Result<Option<Self::UserData>> {
        // Fetch the Hasura database client from the pool
        let mut hasura_db_client: DbClient = get_hasura_pool()
            .await
            .get()
            .await
            .with_context(|| "Error getting hasura db pool")?;

        let hasura_transaction = hasura_db_client
            .transaction()
            .await
            .with_context(|| "Error starting hasura transaction")?;

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
        for candidate in candidates {
            candidate_data.push(CandidateData {
                name_appearing_on_ballot: candidate.name_appearing_on_ballot.clone(),
                acronym: candidate.acronym.clone(), // Assuming acronym is part of the candidate structure
                votes_garnered: 0, // Default value since no votes have been cast yet
            });
        }

        // Retrieve total registered voters and total ballots counted (Placeholder for now)
        let total_registered_voters = 0; // Replace with the correct value fetched from Felix
        let total_ballots_counted = 0; // Replace with the correct value fetched from Felix

        let temp_val: &str = "test";
        Ok(Some(UserData {
            total_registered_voters,
            total_ballots_counted,
            elective_position_name,
            candidate_data,
            election_start_date: temp_val.to_string(),
            election_title: election_event.name.clone(),
            geograpic_region: temp_val.to_string(),
            area: temp_val.to_string(),
            country: temp_val.to_string(),
            voting_center: temp_val.to_string(),
            chairperson_name: temp_val.to_string(),
            poll_clerk_name: temp_val.to_string(),
            third_member_name: temp_val.to_string(),
        }))
    }

    async fn prepare_system_data(
        &self,
        rendered_user_template: String,
    ) -> Result<Self::SystemData> {
        let public_asset_path = get_public_assets_path_env_var()?;
        let minio_endpoint_base =
            get_minio_url().with_context(|| "Error getting minio endpoint")?;

        Ok(SystemData {
            report_hash: String::new(),
            ovsc_version: String::new(),
            system_hash: String::new(),
            file_logo: String::new(),
            file_qrcode_lib: String::new(),
            date_time_printed: String::new(),
            printing_code: String::new(),
        })
    }
}
