// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::{
    report_variables::{extract_election_data, get_app_hash, get_app_version, get_date_and_time},
    template_renderer::*,
};
use crate::{
    postgres::{
        election::get_election_by_id, reports::ReportType,
        scheduled_event::find_scheduled_event_by_election_event_id,
    },
    services::database::get_hasura_pool,
};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::types::{scheduled_event::generate_voting_period_dates, templates::EmailConfig};
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Event {
    post: String,
    country: String,
    testing_date: String,
    initialization_date: String,
    opening_date: String,
    closing_date: String,
    transmission_date: String,
    transmission_status: String,
    remarks: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Region {
    name: String,
    events: Vec<Event>,
}
/// Struct for OVCSEvents Data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserData {
    pub date_printed: String,
    pub election_date: String,
    pub election_title: String,
    pub voting_period_start: String,
    pub voting_period_end: String,
    pub regions: Vec<Region>,
    pub precinct_id: String,
    pub goverment_datetime: String,
    pub local_datetime: String,
    pub ovcs_downtime: u32,
    pub software_version: String,
    pub qr_codes: Vec<String>,
    pub report_hash: String,
    pub ovcs_version: String,
    pub system_hash: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemData {
    pub rendered_user_template: String,
}

#[derive(Debug)]
pub struct OVCSEventsTemplate {
    pub tenant_id: String,
    pub election_event_id: String,
    pub election_id: Option<String>,
}

#[async_trait]
impl TemplateRenderer for OVCSEventsTemplate {
    type UserData = UserData;
    type SystemData = SystemData;

    fn get_report_type() -> ReportType {
        ReportType::OVCS_EVENTS
    }

    fn base_name() -> String {
        "ovcs_events".to_string()
    }

    fn prefix(&self) -> String {
        format!("ovcs_events_{}", self.election_event_id)
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
            subject: "Sequent Online Voting - OVCS Events".to_string(),
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

        // TODO
        let regions = vec![
            Region {
                name: "Region A".to_string(),
                events: vec![
                    Event {
                        post: "Post 1".to_string(),
                        country: "Country A".to_string(),
                        testing_date: "2024-10-01T00:00:00-04:00".to_string(),
                        initialization_date: "2024-10-03T00:00:00-04:00".to_string(),
                        opening_date: "2024-11-05T06:00:00-04:00".to_string(),
                        closing_date: "2024-11-05T20:00:00-04:00".to_string(),
                        transmission_date: "2024-11-05T21:00:00-04:00".to_string(),
                        transmission_status: "Success".to_string(),
                        remarks: Some("Smooth process".to_string()),
                    },
                    Event {
                        post: "Post 2".to_string(),
                        country: "Country B".to_string(),
                        testing_date: "2024-10-02T00:00:00-04:00".to_string(),
                        initialization_date: "2024-10-04T00:00:00-04:00".to_string(),
                        opening_date: "2024-11-05T07:00:00-04:00".to_string(),
                        closing_date: "2024-11-05T19:00:00-04:00".to_string(),
                        transmission_date: "2024-11-05T20:30:00-04:00".to_string(),
                        transmission_status: "Pending".to_string(),
                        remarks: None,
                    },
                ],
            },
            Region {
                name: "Region B".to_string(),
                events: vec![Event {
                    post: "Post 3".to_string(),
                    country: "Country C".to_string(),
                    testing_date: "2024-10-05T00:00:00-04:00".to_string(),
                    initialization_date: "2024-10-07T00:00:00-04:00".to_string(),
                    opening_date: "2024-11-05T08:00:00-04:00".to_string(),
                    closing_date: "2024-11-05T18:00:00-04:00".to_string(),
                    transmission_date: "2024-11-05T19:30:00-04:00".to_string(),
                    transmission_status: "Success".to_string(),
                    remarks: Some("Minor delays".to_string()),
                }],
            },
        ];

        let ovcs_version = get_app_version();
        let system_hash = get_app_hash();
        let software_version = ovcs_version.clone();

        Ok(UserData {
            date_printed: datetime_printed,
            election_date: election_date.to_string(),
            election_title: election.name.clone(),
            voting_period_start: voting_period_start_date,
            voting_period_end: voting_period_end_date,
            regions: regions,
            precinct_id: election_general_data.precinct_code,
            goverment_datetime: "2024-05-10T18:00:00-04:00".to_string(),
            local_datetime: "2024-05-11T08:00:00-04:00".to_string(),
            ovcs_downtime: 0,
            software_version,
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
pub async fn generate_report(
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
    let template = OVCSEventsTemplate {
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
