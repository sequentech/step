// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::template_renderer::*;
use crate::postgres::reports::{Report, ReportType};
use crate::services::database::get_hasura_pool;
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
    pub time_printed: String,
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

        // Fetch election event data
        let election_event = get_election_event_by_id(
            &hasura_transaction,
            &self.tenant_id,
            &self.election_event_id,
        )
        .await
        .with_context(|| "Error obtaining election event")?;

        // Fetch total registered voters and ballots counted
        let registered_voters: i64 = 10000; // Replace with DB query
        let ballots_counted: i64 = 8000; // Replace with DB query

        // Calculate voter turnout percentage
        let voters_turnout = (ballots_counted as f64 / registered_voters as f64) * 100.0;

        // TODO: replace mock data with actual data
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
            election_date: temp_val.to_string(),
            election_title,
            registered_voters,
            ballots_counted,
            voters_turnout: voters_turnout.to_string(),
            candidates,
            geographical_region: temp_val.to_string(),
            post: temp_val.to_string(),
            country: temp_val.to_string(),
            voting_center: temp_val.to_string(),
            voting_period: temp_val.to_string(),
            precinct_code: temp_val.to_string(),
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
            date_printed: String::new(),
            time_printed: String::new(),
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
    report: Report,
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
            Some(report),
        )
        .await
}
