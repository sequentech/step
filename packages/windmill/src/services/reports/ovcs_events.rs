use super::template_renderer::*;
use crate::services::database::get_hasura_pool;
use crate::{postgres::election_event::get_election_event_by_id};
use anyhow::{anyhow, Context, Ok, Result};
use async_trait::async_trait;
use deadpool_postgres::Client as DbClient;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};
use chrono::{DateTime, Utc};



/// Struct for OVCSEvents Data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserData {
    pub election_start_date: String,
    pub election_title: String,
    pub geograpic_region: String,
    pub area: String,
    pub country: String,
    pub voting_center: String,
    pub date_of_final_testing: String,
    pub date_time_initialization: String,
    pub date_time_opening_polls: String,
    pub date_time_closing_polls: String,
    pub date_time_transmission_results: String,
    pub transmission_status: String,
    pub remarks: Option<String>,  // Free text input, so it can be optional
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
pub struct OVCSEventsTemplate {
    tenant_id: String,
    election_event_id: String,
}

#[async_trait]
impl TemplateRenderer for OVCSEventsTemplate {
    type UserData = UserData;
    type SystemData = SystemData;

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

    // TODO: replace mock data with actual data
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

        // Fetch event data
        let event_data = get_election_event_by_id(
            &hasura_transaction,
            &self.tenant_id,
            &self.election_event_id,
        )
        .await
        .with_context(|| "Error fetching election event data")?;

        let temp_val: &str = "test";
        Ok(UserData {
            election_start_date: temp_val.to_string(),
            election_title: temp_val.to_string(),
            geograpic_region: "Asia".to_string(),  // Replace with actual data
            area: "Region 1".to_string(),  // Replace with actual data
            country: "Philippines".to_string(),  // Replace with actual data
            voting_center: "Manila".to_string(),  // Replace with actual data
            date_of_final_testing: "2024-10-09".to_string(),
            date_time_initialization: "2024-10-09 08:00".to_string(),
            date_time_opening_polls: "2024-10-09 09:00".to_string(),
            date_time_closing_polls: "2024-10-09 17:00".to_string(),
            date_time_transmission_results: "2024-10-09 18:00".to_string(),
            transmission_status: "Success".to_string(),
            remarks: Some("No issues reported".to_string()),
        })
    }

    /// Prepare system metadata for the report
    async fn prepare_system_data(&self, _rendered_user_template: String) -> Result<Self::SystemData> {
        let now = Utc::now();
        let date_printed = now.format("%Y-%m-%d").to_string();
        let time_printed = now.format("%H:%M:%S").to_string();

        let system_data = SystemData {
            report_hash: String::new(),  // Placeholder, should be computed
            ovcs_version: "1.0".to_string(),  // Replace with actual version
            system_hash: String::new(),  // Placeholder, should be computed
            file_logo: String::new(),  // Placeholder for file logo path
            file_qrcode_lib: String::new(),  // Placeholder for QR code file path
            date_time_printed: format!("{} {}", date_printed, time_printed),
            printing_code: String::new(),  // Placeholder, should be computed
        };

        Ok(system_data)
    }
}
