// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::report_variables::{extract_election_data, get_app_hash, get_app_version};
use super::{report_variables::get_date_and_time, template_renderer::*};
use crate::postgres::election::get_election_by_id;
use crate::postgres::scheduled_event::find_scheduled_event_by_election_event_id;
use crate::services::database::get_hasura_pool;
use crate::services::s3::get_minio_url;
use crate::{postgres::reports::ReportType, services::database::get_keycloak_pool};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use deadpool_postgres::{Client as DbClient, Transaction};
use rocket::http::Status;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::types::scheduled_event::generate_voting_period_dates;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};

/// Struct for Pre-Enrolled User Data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PreEnrolledUserData {
    pub no: u32,
    pub first_name: String,
    pub last_name: String,
    pub middle_name: Option<String>,
    pub suffix: Option<String>,
    pub id: String,
    pub status: String, // Either "voted" or "not voted"
    pub date_pre_enrolled: String,
    pub approved_by: String, // OFOV/SBEI/SYSTEM
}

/// Struct for OV Count Data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserData {
    pub election_date: String,
    pub election_title: String,
    pub voting_period_start: String,
    pub voting_period_end: String,
    pub post: String,
    pub area_id: String,
    pub number_of_ovs_voted: u32,
    pub number_of_ovs_not_voted: u32,
    pub number_of_ovs_total: u32,
    pub number_of_ovs_approved_by: String, // OFOV/SBEI/SYSTEM
    pub pre_enrolled_users: Vec<PreEnrolledUserData>,
    pub chairperson_name: String,
    pub poll_clerk_name: String,
    pub third_member_name: String,
    pub report_hash: String,
    pub ovcs_version: String,
    pub system_hash: String,
    pub date_printed: String,
}

// Struct to hold system data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemData {
    pub rendered_user_template: String,
    pub file_qrcode_lib: String,
}

/// Implement the `TemplateRenderer` trait for PreEnrolledUserTemplate
#[derive(Debug)]
pub struct PreEnrolledUserTemplate {
    tenant_id: String,
    election_event_id: String,
    pre_enrolled_user_id: String,
}

#[async_trait]
impl TemplateRenderer for PreEnrolledUserTemplate {
    type UserData = UserData;
    type SystemData = SystemData;

    fn get_report_type() -> ReportType {
        ReportType::PRE_ENROLLED_USERS
    }

    fn get_tenant_id(&self) -> String {
        self.tenant_id.clone()
    }

    fn get_election_event_id(&self) -> String {
        self.election_event_id.clone()
    }

    fn base_name() -> String {
        "pre_enrolled_users".to_string()
    }

    fn prefix(&self) -> String {
        format!("pre_enrolled_user_{}", self.pre_enrolled_user_id)
    }

    // TODO: replace mock data with actual data
    // Fetch and prepare user data
    #[instrument(err, skip(self, hasura_transaction, keycloak_transaction))]
    async fn prepare_user_data(
        &self,
        hasura_transaction: &Transaction<'_>,
        keycloak_transaction: &Transaction<'_>,
    ) -> Result<Self::UserData> {
        // get election instace
        let election = match get_election_by_id(
            &hasura_transaction, // Use the unwrapped transaction reference
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
        let datetime_printed: String = get_date_and_time();

        // Mock data for pre_enrolled_users
        let pre_enrolled_users = vec![
            PreEnrolledUserData {
                no: 1,
                first_name: "John".to_string(),
                last_name: "Doe".to_string(),
                middle_name: Some("A".to_string()),
                suffix: None,
                id: "123456".to_string(),
                status: "voted".to_string(),
                date_pre_enrolled: "2024-01-01T08:30:00-04:00".to_string(),
                approved_by: "OFOV".to_string(),
            },
            PreEnrolledUserData {
                no: 2,
                first_name: "Jane".to_string(),
                last_name: "Smith".to_string(),
                middle_name: None,
                suffix: Some("Jr".to_string()),
                id: "7891011".to_string(),
                status: "not voted".to_string(),
                date_pre_enrolled: "2024-01-02T08:30:00-04:00".to_string(),
                approved_by: "SBEI".to_string(),
            },
            PreEnrolledUserData {
                no: 3,
                first_name: "Michael".to_string(),
                last_name: "Johnson".to_string(),
                middle_name: Some("B".to_string()),
                suffix: None,
                id: "987654".to_string(),
                status: "voted".to_string(),
                date_pre_enrolled: "2024-01-03T08:30:00-04:00".to_string(),
                approved_by: "SYSTEM".to_string(),
            },
        ];

        // Calculate the number of OVs who voted, didn't vote, and the total
        let number_of_ovs_voted = pre_enrolled_users
            .iter()
            .filter(|u| u.status == "voted")
            .count() as u32;
        let number_of_ovs_not_voted = pre_enrolled_users
            .iter()
            .filter(|u| u.status == "not voted")
            .count() as u32;
        let number_of_ovs_total = pre_enrolled_users.len() as u32;

        // Assuming "OFOV" approval is common, modify logic to fit your use case
        let number_of_ovs_approved_by = "OFOV".to_string();
        let chairperson_name = "John Doe".to_string();
        let poll_clerk_name = "Jane Smith".to_string();
        let third_member_name = "Alice Johnson".to_string();
        let report_hash = "-".to_string();
        let ovcs_version = get_app_version();
        let system_hash = get_app_hash();

        Ok(UserData {
            election_date: election_date.to_string(),
            election_title: election.name.clone(),
            voting_period_start: voting_period_start_date,
            voting_period_end: voting_period_end_date,
            post: election_general_data.post,
            area_id: election_general_data.area_id,
            date_printed: datetime_printed,
            number_of_ovs_voted,
            number_of_ovs_not_voted,
            number_of_ovs_total,
            number_of_ovs_approved_by,
            pre_enrolled_users,
            chairperson_name,
            poll_clerk_name,
            third_member_name,
            report_hash,
            ovcs_version,
            system_hash,
        })
    }

    /// Prepare system metadata for the report
    #[instrument(err, skip(self, rendered_user_template))]
    async fn prepare_system_data(
        &self,
        rendered_user_template: String,
    ) -> Result<Self::SystemData> {
        let file_qrcode_lib: &str = "test";
        Ok(SystemData {
            rendered_user_template,
            file_qrcode_lib: file_qrcode_lib.to_string(),
        })
    }
}
