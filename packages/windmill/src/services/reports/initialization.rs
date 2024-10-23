// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::report_variables::{
    extract_election_data, generate_voters_turnout, get_date_and_time, get_election_contests_area_results_and_total_ballot_counted,
    get_total_number_of_ballots, get_total_number_of_registered_voters_for_country,
};
use super::template_renderer::*;
use crate::postgres::candidate::get_candidates_by_contest_id;
use crate::postgres::contest::get_contest_by_election_id;
use crate::postgres::election::get_election_by_id;
use crate::postgres::reports::ReportType;
use crate::postgres::results_area_contest_candidate::get_results_area_contest_candidates;
use crate::postgres::scheduled_event::{
    find_scheduled_event_by_election_event_id,
    find_scheduled_event_by_election_event_id_and_event_processor,
};
use crate::services::database::{get_hasura_pool, get_keycloak_pool};
use chrono::{Local, NaiveDate, NaiveDateTime};
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::types::hasura::core::{Candidate, Contest};
use crate::services::temp_path::*;
use crate::{postgres::election_event::get_election_event_by_id, services::s3::get_minio_url};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use rocket::http::Status;
use sequent_core::types::scheduled_event::generate_voting_period_dates;
use sequent_core::types::templates::EmailConfig;
use serde::{Deserialize, Serialize};

/// Struct for the initialization report data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserData {
    pub file_qrcode_lib: String,
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
    pub contests: Vec<ContestData>,
    pub chairperson_name: String,
    pub chairperson_digital_signature: String,
    pub poll_clerk_name: String,
    pub poll_clerk_digital_signature: String,
    pub third_member_name: String,
    pub report_hash: String,
    pub software_version: String,
    pub ovcs_version: String,
    pub system_hash: String,
}

/// Struct for each contest's data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ContestData {
    pub contest_name: String,
    pub position_name: String,
    pub candidates: Vec<CandidateData>,
}

/// Struct for each candidate's data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CandidateData {
    pub name_in_ballot: String,
    pub acronym: String,
    pub votes_garnered: i64,
}

/// Struct for System Data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemData {
    pub rendered_user_template: String,
}

#[derive(Debug)]
pub struct InitializationTemplate {
    pub tenant_id: String,
    pub election_event_id: String,
    pub election_id: String,
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
        let hasura_transaction = hasura_transaction
            .ok_or_else(|| anyhow::anyhow!("Hasura transaction is required"))?;
        let keycloak_transaction = keycloak_transaction
            .ok_or_else(|| anyhow::anyhow!("Keycloak transaction is required"))?;
    
        let realm_name = get_event_realm(self.tenant_id.as_str(), self.election_event_id.as_str());

        // get election instace
        let election = match get_election_by_id(
            &hasura_transaction,
            &self.get_tenant_id(),
            &self.get_election_event_id(),
            &self.election_id,
        )
        .await
        .with_context(|| "Error getting election by id")?
        {
            Some(election) => election,
            None => return Err(anyhow::anyhow!("Election not found")),
        };

        // get election instace's general data (post, country, etc...)
        let election_general_data = match extract_election_data(&election).await {
            Ok(data) => data,
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
        .await?;

        // Fetch election's voting periods
        let voting_period_dates = generate_voting_period_dates(
            start_election_event,
            &self.get_tenant_id(),
            &self.get_election_event_id(),
            Some(&self.election_id),
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

        let election_date = voting_period_start_date.to_string();

        // fetch total of registerd voters
        let registered_voters = get_total_number_of_registered_voters_for_country(
            &keycloak_transaction,
            &realm_name,
            &election_general_data.country,
        )
        .await?;

        let (ballots_counted, results_area_contests, contests) =
            get_election_contests_area_results_and_total_ballot_counted(
                &hasura_transaction,
                &self.get_tenant_id(),
                &self.get_election_event_id(),
                &self.election_id,
            )
            .await?;

        // Parse the date string into a NaiveDate
        let current_date = Local::now().date_naive();
        let date_printed_parsed = NaiveDate::parse_from_str(&current_date.to_string(), "%Y-%m-%d")
            .expect("Failed to parse date");

        // Fetch necessary data (TODO: fix dummy placeholders)
        let file_qrcode_lib = "qrcode_lib.png".to_string();
        let chairperson_name = "John Doe".to_string();
        let poll_clerk_name = "Jane Smith".to_string();
        let third_member_name = "Alice Johnson".to_string();
        let chairperson_digital_signature = "DigitalSignatureABC".to_string();
        let poll_clerk_digital_signature = "DigitalSignatureDEF".to_string();
        let third_member_digital_signature = "DigitalSignatureGHI".to_string();
        let report_hash = "dummy_report_hash".to_string();
        let software_version = "1.0".to_string();
        let ovcs_version = "1.0".to_string();
        let system_hash = "dummy_system_hash".to_string();

        Ok(UserData {
            file_qrcode_lib,
            election_date,
            election_title: election.name,
            voting_period: format!("{} - {}", voting_period_start_date, voting_period_end_date),
            geographical_region: election_general_data.geographical_region,
            post: election_general_data.post,
            country: election_general_data.country,
            voting_center: election_general_data.voting_center,
            precinct_code: election_general_data.clustered_precinct_id,
            registered_voters,
            ballots_counted,
            contests: generate_contests_data(
                hasura_transaction,
                &self.tenant_id,
                &self.election_event_id,
                &self.election_id,
                contests,
            )
            .await?,
            chairperson_name,
            chairperson_digital_signature,
            poll_clerk_name,
            poll_clerk_digital_signature,
            third_member_name,
            report_hash,
            software_version,
            ovcs_version,
            system_hash,
        })
    }

    async fn prepare_system_data(
        &self,
        rendered_user_template: String,
    ) -> Result<Self::SystemData> {
        Ok(SystemData {
            rendered_user_template,
        })
    }
}

async fn generate_contests_data(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
    contests: Vec<Contest>,
) -> Result<Vec<ContestData>> {
    let mut contests_data: Vec<ContestData> = Vec::new();
    for contest in contests {
        let contest_name = contest.clone().name.unwrap_or_default();
        let contest_name_parts = contest_name.split('/').collect::<Vec<&str>>();
        let contest_name = contest_name_parts.get(0).unwrap_or(&"").to_string();
        let position_name = contest_name_parts.get(1).unwrap_or(&"").to_string();

        // Candidates of the specific contest
        let contest_candidates = get_candidates_by_contest_id(
            &hasura_transaction,
            tenant_id,
            election_event_id,
            contest.clone().id.as_str(),
        )
        .await
        .with_context(|| "Error obtaining contests")?;

        let mut candidate_data: Vec<CandidateData> = Vec::new();
        for candidate in contest_candidates {
            let votes_garnered = if let Some(contest_id) = candidate.clone().contest_id {
                if let Some(results) = get_results_area_contest_candidates(
                    &hasura_transaction,
                    tenant_id,
                    election_event_id,
                    election_id,
                    &contest_id,
                    &candidate.clone().id,
                )
                .await
                .unwrap_or(None)
                {
                    results.cast_votes.unwrap_or(0)
                } else {
                    0
                }
            } else {
                0
            };

            candidate_data.push(CandidateData {
                name_in_ballot: candidate.clone().name.unwrap_or_default(),
                acronym: candidate
                    .clone()
                    .annotations
                    .unwrap_or_default()
                    .get("acronym")
                    .unwrap_or(&serde_json::Value::Null)
                    .to_string(),
                votes_garnered,
            });
        }

        contests_data.push(ContestData {
            contest_name,
            position_name,
            candidates: candidate_data,
        });
    }

    Ok(contests_data)
}
