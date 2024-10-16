// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::report_variables::{
    extract_election_data, get_date_and_time,
    get_election_contests_area_results_and_total_ballot_counted,
    get_total_number_of_registered_voters_for_country,
};
use super::template_renderer::*;
use crate::postgres::election::get_election_by_id;
use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::reports::ReportType;
use crate::postgres::results_area_contest::get_results_area_contest;
use crate::postgres::scheduled_event::find_scheduled_event_by_election_event_id;
use crate::services::database::get_hasura_pool;
use crate::services::database::{get_keycloak_pool, PgConfig};
use crate::services::s3::get_minio_url;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use deadpool_postgres::Client as DbClient;
use sequent_core::serialization::deserialize_with_path::deserialize_value;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::types::scheduled_event::generate_voting_period_dates;
use sequent_core::{ballot::ElectionStatus, ballot::VotingStatus, types::templates::EmailConfig};
use serde::{Deserialize, Serialize};
use serde_json::value::Value;
use tracing::{info, instrument};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserData {
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
    pub ovcs_status: String,
    pub chairperson_name: String,
    pub poll_clerk_name: String,
    pub third_member_name: String,
    pub report_hash: String,
    pub ovcs_version: String,
    pub system_hash: String,
    pub date_printed: String,
    pub time_printed: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemData {
    pub rendered_user_template: String,
    pub file_qrcode_lib: String,
}

#[derive(Debug)]
pub struct StatusTemplate {
    pub tenant_id: String,
    pub election_event_id: String,
    pub election_id: String,
}

#[async_trait]
impl TemplateRenderer for StatusTemplate {
    type UserData = UserData;
    type SystemData = SystemData;

    fn get_report_type() -> ReportType {
        ReportType::STATUS
    }

    fn base_name() -> String {
        "status".to_string()
    }

    fn prefix(&self) -> String {
        format!("status_{}", self.election_event_id)
    }

    fn get_tenant_id(&self) -> String {
        self.tenant_id.clone()
    }

    fn get_election_event_id(&self) -> String {
        self.election_event_id.clone()
    }

    fn get_election_id(&self) -> Option<String> {
        Some(self.election_id.clone())
    }

    fn get_email_config() -> EmailConfig {
        EmailConfig {
            subject: "Sequent Online Voting - Status".to_string(),
            plaintext_body: "".to_string(),
            html_body: None,
        }
    }

    #[instrument]
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
            &self.get_tenant_id(),
            &self.get_election_event_id(),
        )
        .await
        .with_context(|| "Error obtaining election event")?;

        let election = match get_election_by_id(
            &hasura_transaction,
            &self.tenant_id,
            &self.election_event_id,
            &self.election_id,
        )
        .await
        .with_context(|| "Error getting election by id")?
        {
            Some(election) => election,
            None => return Err(anyhow::anyhow!("Election not found")),
        };

        let mut status = get_election_status(election.status.clone()).unwrap_or(ElectionStatus {
            voting_status: VotingStatus::NOT_STARTED,
        });

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

        // Fetch election event data
        let start_election_event = find_scheduled_event_by_election_event_id(
            &hasura_transaction,
            &self.get_tenant_id(),
            &self.get_election_event_id(),
        )
        .await
        .map_err(|e| 
            anyhow::anyhow!(format!(
                "Error getting scheduled event by election_event_id {:?}", e
            )
        ))?;

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

        // fetch total of registerd voters
        let registered_voters = get_total_number_of_registered_voters_for_country(
            &keycloak_transaction,
            &realm_name,
            &election_general_data.country,
        )
        .await
        .map_err(|e| 
            anyhow::anyhow!(format!(
                "Error getting number of registered voters {:?}", e
            )
        ))?;

        let (ballots_counted, results_area_contests, contests) =
            get_election_contests_area_results_and_total_ballot_counted(
                &hasura_transaction,
                &self.get_tenant_id(),
                &self.get_election_event_id(),
                &self.get_election_id().unwrap(),
            )
            .await
        .map_err(|e| 
            anyhow::anyhow!(format!(
                "Error getting election contests area results {:?}", e
            )
        ))?;

        let (date_printed, time_printed) = get_date_and_time();
        let election_date = &voting_period_start_date.to_string();
        // Format the date to the desired format
        let status_str: &'static str = status.voting_status.into();
        let ovcs_status = status_str.to_string();
        let temp_val: &str = "test";

        Ok(UserData {
            election_date: election_date.to_string(),
            election_title: election_event.name.clone(),
            voting_period: format!("{} - {}", voting_period_start_date, voting_period_end_date),
            geographical_region: election_general_data.geographical_region,
            post: election_general_data.post,
            country: election_general_data.country,
            voting_center: election_general_data.voting_center,
            precinct_code: election_general_data.clustered_precinct_id,
            registered_voters: registered_voters,
            ballots_counted: ballots_counted,
            ovcs_status: ovcs_status,
            chairperson_name: temp_val.to_string(),
            poll_clerk_name: temp_val.to_string(),
            third_member_name: temp_val.to_string(),
            report_hash: "hash123".to_string(),
            ovcs_version: "1.0".to_string(),
            system_hash: "sys_hash123".to_string(),
            date_printed: date_printed,
            time_printed: time_printed,
        })
    }

    #[instrument]
    async fn prepare_system_data(
        &self,
        rendered_user_template: String,
    ) -> Result<Self::SystemData> {
        let temp_val: &str = "test";
        Ok(SystemData {
            rendered_user_template,
            file_qrcode_lib: temp_val.to_string()
        })
    }
}

pub fn get_election_status(status_json_opt: Option<Value>) -> Option<ElectionStatus> {
    status_json_opt.and_then(|status_json| deserialize_value(status_json).ok())
}

#[instrument]
pub async fn generate_status_report(
    document_id: &str,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
    mode: GenerateReportMode,
) -> Result<()> {
    let template = StatusTemplate {
        tenant_id: tenant_id.to_string(),
        election_event_id: election_event_id.to_string(),
        election_id: election_id.to_string(),
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
