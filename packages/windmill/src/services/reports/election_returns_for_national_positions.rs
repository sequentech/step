use super::template_renderer::*;
use crate::services::database::get_hasura_pool;
use crate::{postgres::election_event::get_election_event_by_id, services::s3::get_minio_url};
use crate::{postgres::scheduled_event::find_scheduled_event_by_election_event_id_and_event_processor};
use crate::services::temp_path::*;
use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{Local, TimeZone};
use serde::{Deserialize, Serialize};
use std::env;
use tracing::{info, instrument};
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use sequent_core::types::templates::EmailConfig;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserData {
    pub election_start_date: String,
    pub election_title: String,
    pub geograpic_region: String,
    pub area: String,
    pub country: String,
    pub voting_center: String,
    pub elective_position_name: String,
    pub total_registered_voters: u32,
    pub total_ballots_counted: u32,
    pub voter_turnout: f32,
    pub candidate_data: Vec<CandidateData>,
}

/// Struct for each candidate's data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CandidateData {
    pub name_appearing_on_ballot: String,
    pub acronym: String,
    pub votes_garnered: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemData {
    pub report_hash: String,
    pub ovcs_version: String,
    pub system_hash: String,
    pub file_logo: String,
    pub file_qrcode_lib: String,
    pub date_time_printed: String,
    pub printing_code: String,
}

#[derive(Debug)]
pub struct ElectionReturnsForNationalPostionTemplate {
    tenant_id: String,
    election_event_id: String,
    elective_position: String,
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
        format!("election_returns_for_national_positions_{}", self.election_event_id)
    }

    fn get_email_config() -> EmailConfig {
        EmailConfig {
            subject: "Sequent Online Voting - Election Returns For National Positions".to_string(),
            plaintext_body: "".to_string(),
            html_body: None,
        }
    }

    async fn prepare_user_data(&self) -> Result<Self::UserData> {
        let mut hasura_db_client: DbClient = get_hasura_pool()
            .await
            .get()
            .await
            .with_context(|| "Error getting Hasura db pool")?;

        let hasura_transaction = hasura_db_client
            .transaction()
            .await
            .with_context(|| "Error starting Hasura transaction")?;

        // Fetch election event data
        let election_event = get_election_event_by_id(&hasura_transaction, &self.tenant_id, &self.election_event_id)
            .await
            .with_context(|| "Error obtaining election event")?;

        // Extract the elective position before "/"
        let elective_position_name = self.elective_position.split('/').next().unwrap_or("").to_string();

        // Fetch total registered voters and ballots counted
        let total_registered_voters: u32 = 10000; // Replace with DB query
        let total_ballots_counted: u32 = 8000; // Replace with DB query

        // Calculate voter turnout percentage
        let voter_turnout = (total_ballots_counted as f32 / total_registered_voters as f32) * 100.0;

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

        let election_title = election_event.name.clone();
        
        let temp_val: &str = "test";
        Ok(UserData {
            election_start_date: temp_val.to_string(),
            election_title,
            elective_position_name,
            total_registered_voters,
            total_ballots_counted,
            voter_turnout,
            candidate_data,
            geograpic_region: temp_val.to_string(),
            area: temp_val.to_string(),
            country: temp_val.to_string(),
            voting_center: temp_val.to_string(),
        })
    }

    async fn prepare_system_data(&self, _: String) -> Result<Self::SystemData> {
        let date_time_printed = Local::now().with_timezone(&chrono::FixedOffset::east(8 * 3600)).format("%Y-%m-%d %H:%M:%S").to_string();
        let printing_code = "XYZ123".to_string(); // Example placeholder for a real printing code

        Ok(SystemData {
            report_hash: String::new(),
            ovcs_version: String::new(),
            system_hash: String::new(),
            file_logo: String::new(),
            file_qrcode_lib: String::new(),
            date_time_printed: String::new(),
            printing_code: String::new(),
        })
    }
}
