use super::template_renderer::*;
use crate::postgres::candidate::get_candidates_by_election_id;
use crate::postgres::contest::get_contest_by_election_id;
use crate::services::database::get_hasura_pool;
use crate::services::temp_path::*;
use crate::{postgres::election_event::get_election_event_by_id, services::s3::get_minio_url};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use sequent_core::types::hasura::core::{Candidate, Contest};
use sequent_core::types::templates::EmailConfig;
use serde::{Deserialize, Serialize};

/// Struct for the initialization report
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserData {
    pub election_date: String,
    pub voting_period: String,
    pub geographical_region: String,
    pub post: String,
    pub country: String,
    pub voting_center: String,
    pub precinct_code: String,
    pub registered_voters: u32,
    pub ballots_counted: u32,
    pub contests: Vec<ContestData>,
    pub chairperson_name: String,
    pub poll_clerk_name: String,
    pub third_member_name: String,
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
    pub votes_garnered: u32,
}

/// Struct for System Data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemData {
    pub report_hash: String,
    pub ovsc_version: String,
    pub system_hash: String,
    pub software_version: String,
}

#[derive(Debug)]
pub struct InitializationTemplate {
    tenant_id: String,
    election_event_id: String,
    election_id: String,
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

    async fn prepare_user_data(&self) -> Result<Self::UserData> {
        let mut hasura_db_client: DbClient = get_hasura_pool()
            .await
            .get()
            .await
            .with_context(|| "Error getting hasura db pool")?;

        let hasura_transaction = hasura_db_client
            .transaction()
            .await
            .with_context(|| "Error starting hasura transaction")?;

        let contests = get_contest_by_election_id(
            &hasura_transaction,
            &self.tenant_id,
            &self.election_event_id,
            &self.election_id,
        )
        .await
        .with_context(|| "Error obtaining contests")?;

        // All candidates for the election (several contests)
        let election_candidates = get_candidates_by_election_id(
            &hasura_transaction,
            &self.tenant_id,
            &self.election_event_id,
            &self.election_id,
        )
        .await
        .with_context(|| "Error obtaining contests")?;

        let mut contests_data: Vec<ContestData> = Vec::new();
        for contest in contests {
            let contest_name = contest.clone().name.unwrap_or_default();
            let contest_name_parts = contest_name.split('/').collect::<Vec<&str>>();
            let contest_name = contest_name_parts.get(0).unwrap_or(&"").to_string();
            let position_name = contest_name_parts.get(1).unwrap_or(&"").to_string();

            let filtered_candidates = election_candidates
                .iter()
                .filter(|candidate| {
                    candidate.contest_id.as_ref().unwrap_or(&String::new()) == &contest.id
                })
                .collect::<Vec<&Candidate>>();

            // Fetch candidates for the contest
            let candidate_data: Vec<CandidateData> = filtered_candidates
                .into_iter()
                .map(|candidate| CandidateData {
                    name_in_ballot: candidate.clone().name.unwrap_or_default(),
                    acronym: candidate
                        .clone()
                        .annotations
                        .unwrap_or_default()
                        .get("acronym")
                        .unwrap_or(&serde_json::Value::Null)
                        .to_string(),
                    votes_garnered: 0, //TODO: Get votes from the database
                })
                .collect();

            contests_data.push(ContestData {
                contest_name,
                position_name,
                candidates: candidate_data,
            });
        }

        // Placeholder values for the remaining data
        let temp_val = "test".to_string();
        let total_registered_voters = 0;
        let total_ballots_counted = 0;

        Ok(UserData {
            election_date: temp_val.clone(),
            voting_period: temp_val.clone(),
            geographical_region: temp_val.clone(),
            post: temp_val.clone(),
            country: temp_val.clone(),
            voting_center: temp_val.clone(),
            precinct_code: temp_val.clone(),
            registered_voters: total_registered_voters,
            ballots_counted: total_ballots_counted,
            contests: contests_data,
            chairperson_name: temp_val.clone(),
            poll_clerk_name: temp_val.clone(),
            third_member_name: temp_val,
        })
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
            software_version: String::new(),
        })
    }
}
