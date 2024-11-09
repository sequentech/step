// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::report_variables::{get_app_hash, get_app_version, get_date_and_time};
use super::{report_variables::extract_election_data, template_renderer::*};
use crate::postgres::scheduled_event::find_scheduled_event_by_election_event_id;
use crate::postgres::{election::get_election_by_id, reports::ReportType};
use crate::services::database::get_hasura_pool;
use crate::services::s3::get_minio_url;
use crate::services::temp_path::*;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::types::scheduled_event::generate_voting_period_dates;
use sequent_core::types::templates::EmailConfig;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};

// Struct to hold user data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserData {
    pub date_printed: String,
    pub report_hash: String,
    pub ovcs_version: String,
    pub system_hash: String,
    pub election_date: String,
    pub election_title: String,
    pub voting_period_start: String,
    pub voting_period_end: String,
    pub regions: Vec<Region>,
    pub ofov_disapproved: u32,
    pub sbei_disapproved: u32,
    pub system_disapproved: u32,
    pub qr_codes: Vec<String>,
}

// Struct to hold system data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemData {
    pub rendered_user_template: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RegionData {
    pub post: String,
    pub country: String,
    pub total: u32,
    pub not_pre_enrolled: u32,
    pub pre_enrolled: u32,
    pub pre_enrolled_not_voted: u32,
    pub pre_enrolled_voted: u32,
    pub voted: u32,
    pub password_reset_request: u32,
    pub remarks: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Region {
    pub name: String,
    pub data: Vec<RegionData>,
}

#[derive(Debug)]
pub struct OVCSStatisticsTemplate {
    tenant_id: String,
    election_event_id: String,
    pub election_id: Option<String>,
}

#[async_trait]
impl TemplateRenderer for OVCSStatisticsTemplate {
    type UserData = UserData;
    type SystemData = SystemData;

    fn get_report_type() -> ReportType {
        ReportType::OVCS_STATISTICS
    }

    fn base_name() -> String {
        "ovcs_statistics".to_string()
    }

    fn prefix(&self) -> String {
        format!(
            "ovcs_statistics_{}_{}_{}",
            self.tenant_id,
            self.election_event_id,
            self.election_id.clone().unwrap_or_default()
        )
    }

    fn get_tenant_id(&self) -> String {
        self.tenant_id.clone()
    }

    fn get_election_event_id(&self) -> String {
        self.election_event_id.clone()
    }

    fn get_election_id(&self) -> Option<String> {
        self.election_id.clone()
    }

    fn get_email_config() -> EmailConfig {
        EmailConfig {
            subject: "Sequent Online Voting - OVCS Statistics".to_string(),
            plaintext_body: "".to_string(),
            html_body: None,
        }
    }

    #[instrument(err, skip(self, hasura_transaction, keycloak_transaction))]
    async fn prepare_user_data(
        &self,
        hasura_transaction: &Transaction<'_>,
        keycloak_transaction: &Transaction<'_>,
    ) -> Result<Self::UserData> {
        let Some(election_id) = &self.election_id else {
            return Err(anyhow!("Empty election_id"));
        };
        let election = match get_election_by_id(
            hasura_transaction,
            &self.tenant_id,
            &self.election_event_id,
            &election_id,
        )
        .await
        .with_context(|| "Error getting election by id")?
        {
            Some(election) => election,
            None => return Err(anyhow::anyhow!("Election not found")),
        };

        // Fetch election event data
        let start_election_event = find_scheduled_event_by_election_event_id(
            &hasura_transaction,
            &self.tenant_id,
            &self.election_event_id,
        )
        .await
        .map_err(|e| {
            anyhow::anyhow!("Error getting scheduled event by election event_id: {}", e)
        })?;

        // Fetch election's voting periods
        let voting_period_dates = generate_voting_period_dates(
            start_election_event,
            &self.tenant_id,
            &self.election_event_id,
            Some(&election_id),
        )?;

        // extract start date from voting period
        let voting_period_start_date = voting_period_dates.start_date.unwrap_or_default();
        // extract end date from voting period
        let voting_period_end_date = voting_period_dates.end_date.unwrap_or_default();
        let election_date: &String = &voting_period_start_date;

        let datetime_printed: String = get_date_and_time();
        // Mock Data
        let regions = vec![Region {
            name: "North America".to_string(),
            data: vec![
                RegionData {
                    post: "Washington DC".to_string(),
                    country: "United States".to_string(),
                    total: 5000,
                    not_pre_enrolled: 100,
                    pre_enrolled: 4900,
                    pre_enrolled_not_voted: 200,
                    pre_enrolled_voted: 4700,
                    voted: 4800,
                    password_reset_request: 50,
                    remarks: "High turnout".to_string(),
                },
                RegionData {
                    post: "New York".to_string(),
                    country: "United States".to_string(),
                    total: 4000,
                    not_pre_enrolled: 80,
                    pre_enrolled: 3920,
                    pre_enrolled_not_voted: 150,
                    pre_enrolled_voted: 3770,
                    voted: 3850,
                    password_reset_request: 40,
                    remarks: "Smooth process".to_string(),
                },
            ],
        }];

        let ovcs_version = get_app_version();
        let system_hash = get_app_hash();

        Ok(UserData {
            date_printed: datetime_printed,
            election_date: election_date.to_string(),
            election_title: election.name.clone(),
            voting_period_start: voting_period_start_date,
            voting_period_end: voting_period_end_date,
            regions,
            ofov_disapproved: 0,
            sbei_disapproved: 0,
            system_disapproved: 0,
            qr_codes: vec!["QR12345".to_string(), "QR67890".to_string()],
            report_hash: "-".to_string(),
            ovcs_version,
            system_hash,
        })
    }

    /// Prepare system metadata for the report
    #[instrument(err, skip_all)]
    async fn prepare_system_data(
        &self,
        rendered_user_template: String,
    ) -> Result<Self::SystemData> {
        Ok(SystemData {
            rendered_user_template,
        })
    }
}

#[instrument(err, skip(hasura_transaction, keycloak_transaction))]
pub async fn generate_ovcs_statistics_report(
    document_id: &str,
    tenant_id: &str,
    election_event_id: &str,
    election_id: Option<&str>,
    mode: GenerateReportMode,
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    is_scheduled_task: bool,
    email_recipients: Vec<String>,
) -> Result<()> {
    let template = OVCSStatisticsTemplate {
        tenant_id: tenant_id.to_string(),
        election_event_id: election_event_id.to_string(),
        election_id: election_id.map(|s| s.to_string()),
    };
    template
        .execute_report(
            document_id,
            tenant_id,
            election_event_id,
            is_scheduled_task,
            email_recipients,
            None,
            mode,
            hasura_transaction,
            keycloak_transaction,
        )
        .await
}
