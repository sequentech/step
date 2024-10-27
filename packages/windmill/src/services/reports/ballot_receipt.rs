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
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::serialization::deserialize_with_path::deserialize_value;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::types::date_time::{DateFormat, TimeZone};
use sequent_core::types::scheduled_event::generate_voting_period_dates;
use sequent_core::{ballot::ElectionStatus, ballot::VotingStatus, types::templates::EmailConfig};
use serde::{Deserialize, Serialize};
use serde_json::value::Value;
use tracing::{info, instrument};

/// Wrapper struct for data specific to the ballot and the voter
/// which won't be needed in the preview mode.
#[derive(Serialize, Deserialize, Debug)]
pub struct BallotData {
    pub area_id: String,
    pub voter_id: String,
    pub ballot_id: String,
    pub ballot_tracker_url: String,
    pub time_zone: Option<TimeZone>,
    pub date_format: Option<DateFormat>,
}

#[derive(Debug)]
pub struct BallotTemplate {
    pub tenant_id: String,
    pub election_event_id: String,
    pub election_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserData {
    pub ballot_id: String,
    pub ballot_tracker_url: String,
    pub qrcode: String,
    pub logo: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemData {
    pub rendered_user_template: String,
    pub title: String,
    pub file_logo: String,
    pub file_qrcode_lib: String,
    pub ballot_tracker_url: String,
    pub timestamp: String,
}

#[derive(Debug)]
pub struct BallotTemplate {
    pub tenant_id: String,
    pub election_event_id: String,
    pub election_id: String,
}

#[async_trait]
impl TemplateRenderer for BallotTemplate {
    type UserData = UserData;
    type SystemData = SystemData;

    fn get_report_type() -> ReportType {
        ReportType::BALLOT_RECEIPT
    }

    fn base_name() -> String {
        "ballot_receipt".to_string()
    }

    fn prefix(&self) -> String {
        format!("ballot_receipt_{}", self.election_event_id)
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
            subject: "Sequent Online Voting - Ballot Receipt".to_string(),
            plaintext_body: "".to_string(),
            html_body: None,
        }
    }

    #[instrument]
    async fn prepare_user_data(
        &self,
        hasura_transaction: Option<&Transaction<'_>>,
        keycloak_transaction: Option<&Transaction<'_>>,
    ) -> Result<Self::UserData> {
        let realm_name = get_event_realm(self.tenant_id.as_str(), self.election_event_id.as_str());
        // Fetch election event data
        let election_event = if let Some(transaction) = hasura_transaction {
            get_election_event_by_id(&transaction, &self.tenant_id, &self.election_event_id)
                .await
                .with_context(|| "Error obtaining election event")?
        } else {
            return Err(anyhow::anyhow!("Transaction is missing"));
        };

        let election = if let Some(transaction) = hasura_transaction {
            match get_election_by_id(
                &transaction, // Use the unwrapped transaction reference
                &self.get_tenant_id(),
                &self.get_election_event_id(),
                &self.get_election_id().unwrap(),
            )
            .await
            .with_context(|| "Error getting election by id")?
            {
                Some(election) => election,
                None => return Err(anyhow::anyhow!("Election not found")),
            }
        } else {
            return Err(anyhow::anyhow!("Transaction is missing"));
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
        let start_election_event = if let Some(transaction) = hasura_transaction {
            find_scheduled_event_by_election_event_id(
                &transaction,
                &self.get_tenant_id(),
                &self.get_election_event_id(),
            )
            .await
            .map_err(|e| {
                anyhow::anyhow!("Error getting scheduled event by election event_id: {}", e)
            })?
        } else {
            return Err(anyhow::anyhow!("Transaction is missing"));
        };

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

        // Fetch total of registered voters
        let registered_voters = if let Some(transaction) = keycloak_transaction {
            get_total_number_of_registered_voters_for_country(
                &transaction, // Pass the actual reference to the transaction
                &realm_name,
                &election_general_data.country,
            )
            .await
            .map_err(|e| {
                anyhow::anyhow!(
                    "Error fetching the number of registered voters for country '{}': {}",
                    &election_general_data.country,
                    e
                )
            })?
        } else {
            return Err(anyhow::anyhow!("Transaction is missing"));
        };

        let (ballots_counted, results_area_contests, contests) = if let Some(transaction) =
            hasura_transaction
        {
            get_election_contests_area_results_and_total_ballot_counted(
                &transaction,
                &self.get_tenant_id(),
                &self.get_election_event_id(),
                &self.get_election_id().unwrap(),
            )
            .await
            .map_err(|e| anyhow::anyhow!("Error getting election contests area results: {}", e))?
        } else {
            return Err(anyhow::anyhow!("Transaction is missing"));
        };

        let date_printed = get_date_and_time();
        let election_date = &voting_period_start_date.to_string();
        // Format the date to the desired format
        let status_str: &'static str = status.voting_status.into();
        let ovcs_status = status_str.to_string();
        let chairperson_name = "John Doe".to_string();
        let poll_clerk_name = "Jane Smith".to_string();
        let third_member_name = "Alice Johnson".to_string();
        let report_hash = "dummy_report_hash".to_string();
        let ovcs_version = "1.0".to_string();
        let system_hash = "dummy_system_hash".to_string();

        Ok(UserData {
            election_date: election_date.to_string(),
            election_title: election_event.name.clone(),
            voting_period_start: voting_period_start_date,
            voting_period_end: voting_period_end_date,
            geographical_region: election_general_data.geographical_region,
            post: election_general_data.post,
            country: election_general_data.country,
            voting_center: election_general_data.voting_center,
            precinct_code: election_general_data.clustered_precinct_id,
            registered_voters: registered_voters,
            ballots_counted: ballots_counted,
            ovcs_status: ovcs_status,
            date_printed: date_printed,
            chairperson_name,
            poll_clerk_name,
            third_member_name,
            report_hash,
            ovcs_version,
            system_hash,
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
            file_qrcode_lib: temp_val.to_string(),
        })
    }
}

pub fn get_election_status(status_json_opt: Option<Value>) -> Option<ElectionStatus> {
    status_json_opt.and_then(|status_json| deserialize_value(status_json).ok())
}

#[instrument]
pub async fn generate_ballot_receipt_report(
    document_id: &str,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
    mode: GenerateReportMode,
    hasura_transaction: Option<&Transaction<'_>>,
    keycloak_transaction: Option<&Transaction<'_>>,
    ballot_data: Option<BallotData>,
) -> Result<()> {
    let template = BallotTemplate {
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
            hasura_transaction,
            keycloak_transaction,
        )
        .await
}
