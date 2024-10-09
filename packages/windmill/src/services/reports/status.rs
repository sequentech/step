use super::template_renderer::*;
use crate::services::database::get_hasura_pool;
use crate::postgres::election_event::get_election_event_by_id;
use crate::services::s3::get_minio_url;
use crate::postgres::scheduled_event::find_scheduled_event_by_election_event_id_and_event_processor;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::info;
use deadpool_postgres::Client as DbClient;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserData {
    pub election_start_date: String,
    pub election_title: String,
    pub geograpic_region: String,
    pub area: String,
    pub country: String,
    pub voting_center: String,
    pub num_of_registered_voters: u32,
    pub total_ballots_counted: u32,
    pub ovcs_status: String,
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
pub struct StatusTemplate {
    pub tenant_id: String,
    pub election_event_id: String,
    pub voter_id: String,
}

#[async_trait]
impl TemplateRenderer for StatusTemplate {
    type UserData = UserData;
    type SystemData = SystemData;

    fn base_name() -> String {
        "ovcs_information".to_string()
    }

    fn prefix(&self) -> String {
        format!("ovcs_information_{}", self.voter_id)
    }

    fn get_tenant_id(&self) -> String {
        self.tenant_id.clone()
    }

    fn get_election_event_id(&self) -> String {
        self.election_event_id.clone()
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

        // Fetch election event data
        let election_event = get_election_event_by_id(
            &hasura_transaction,
            &self.tenant_id,
            &self.election_event_id
        )
        .await
        .with_context(|| "Error obtaining election event")?;

        // Fetch start date, registered voters, ballots counted
        let start_election_event = find_scheduled_event_by_election_event_id_and_event_processor(
            &hasura_transaction,
            &self.tenant_id,
            &self.election_event_id,
            "START_VOTING_PERIOD"
        )
        .await
        .map_err(|e| anyhow!("Error fetching scheduled election event: {:?}", e))?;

        let election_start_date = "2024-10-15".to_string(); // Placeholder, adapt according to real fetched data
        let ovcs_status = "Active".to_string();  // Fetch the real status from DB

        Ok(UserData {
            election_start_date,
            election_title: election_event.name.clone(),
            geograpic_region: "Region 1".to_string(),
            area: "Area A".to_string(),
            country: "Country X".to_string(),
            voting_center: "Center 1".to_string(),
            num_of_registered_voters: 10000,  // Fetch from DB
            total_ballots_counted: 8000,  // Fetch from DB
            ovcs_status,  // Fetch from DB
        })
    }

    async fn prepare_system_data(
        &self,
        _rendered_user_template: String,
    ) -> Result<Self::SystemData> {
        Ok(SystemData {
            report_hash: "hash123".to_string(),
            ovcs_version: "1.0".to_string(),
            system_hash: "sys_hash123".to_string(),
            file_logo: "logo.png".to_string(),
            file_qrcode_lib: "qrcode.png".to_string(),
            date_time_printed: "2024-10-09T12:00:00Z".to_string(),
            printing_code: "print123".to_string(),
        })
    }
}
