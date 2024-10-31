// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::report_variables::{
    extract_election_data, get_date_and_time,
    get_election_contests_area_results_and_total_ballot_counted,
};
use super::template_renderer::*;
use crate::postgres::area::get_areas_by_election_id;
use crate::postgres::election::get_election_by_id;
use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::keys_ceremony::get_keys_ceremony_by_id;
use crate::postgres::reports::ReportType;
use crate::postgres::scheduled_event::find_scheduled_event_by_election_event_id;
use crate::services::database::get_hasura_pool;
use crate::services::database::{get_keycloak_pool, PgConfig};
use crate::services::s3::get_minio_url;
use crate::services::users::count_keycloak_enabled_users_by_area_id;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::serialization::deserialize_with_path::deserialize_value;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::types::scheduled_event::generate_voting_period_dates;
use sequent_core::{ballot::ElectionStatus, ballot::VotingStatus, types::templates::EmailConfig};
use serde::{Deserialize, Serialize};
use serde_json::value::Value;
use tracing::{info, instrument};
// UserData struct now contains a vector of areas
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserData {
    pub areas: Vec<UserDataArea>,
}

// UserDataArea struct holds area-specific data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserDataArea {
    pub date_printed: String,
    pub election_title: String,
    pub voting_period_start: String,
    pub voting_period_end: String,
    pub election_date: String,
    pub post: String,
    pub country: String,
    pub geographical_region: String,
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
    async fn prepare_user_data(
        &self,
        hasura_transaction: Option<&Transaction<'_>>,
        keycloak_transaction: Option<&Transaction<'_>>,
    ) -> Result<Self::UserData> {
        let Some(hasura_transaction) = hasura_transaction else {
            return Err(anyhow::anyhow!("Hasura Transaction is missing"));
        };

        let Some(keycloak_transaction) = keycloak_transaction else {
            return Err(anyhow::anyhow!("Keycloak Transaction is missing"));
        };

        let realm = get_event_realm(self.tenant_id.as_str(), self.election_event_id.as_str());

        // Fetch election event data
        let election_event =
            get_election_event_by_id(hasura_transaction, &self.tenant_id, &self.election_event_id)
                .await
                .with_context(|| "Error obtaining election event")?;

        // Fetch election data
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

        // Get OVCS status
        let status = get_election_status(election.status.clone()).unwrap_or(ElectionStatus {
            voting_status: VotingStatus::NOT_STARTED,
        });

        let ovcs_status = match status.voting_status {
            VotingStatus::NOT_STARTED => "NOT INITIALIZED".to_string(),
            VotingStatus::OPEN | VotingStatus::PAUSED => "OPEN".to_string(),
            VotingStatus::CLOSED => "CLOSED".to_string(),
        };

        // Fetch areas associated with the election
        let election_areas = get_areas_by_election_id(
            &hasura_transaction,
            &self.tenant_id,
            &self.election_event_id,
            &self.election_id,
        )
        .await
        .map_err(|err| anyhow!("Error at get_areas_by_election_id: {err:?}"))?;

        if election_areas.is_empty() {
            return Err(anyhow!("No areas found for the given election"));
        }

        let mut areas: Vec<UserDataArea> = Vec::new();

        // Fetch election event data
        let start_election_event = find_scheduled_event_by_election_event_id(
            &hasura_transaction,
            &self.get_tenant_id(),
            &self.get_election_event_id(),
        )
        .await?;

        let voting_period_dates = generate_voting_period_dates(
            start_election_event,
            &self.get_tenant_id(),
            &self.get_election_event_id(),
            Some(&self.get_election_id().unwrap()),
        )
        .map_err(|e| anyhow!(format!("Error generating voting period dates {e:?}")))?;

        let voting_period_start_date = voting_period_dates.start_date.unwrap_or_default();
        let voting_period_end_date = voting_period_dates.end_date.unwrap_or_default();
        let election_date = &voting_period_start_date.to_string();

        let date_printed = get_date_and_time();
        let election_title = election_event.name.clone();

        // Loop over each area and collect data
        for area in election_areas.iter() {
            let country = area.clone().name.unwrap_or('-'.to_string());

            // Get election instance's general data (post, area, etc.)
            let election_general_data = match extract_election_data(&election).await {
                Ok(data) => data,
                Err(err) => {
                    return Err(anyhow!(format!("Error fetching election data: {}", err)));
                }
            };

            // Fetch total of registered voters for the area
            let registered_voters =
                count_keycloak_enabled_users_by_area_id(&keycloak_transaction, &realm, &area.id)
                    .await
                    .map_err(|err| anyhow!("Error counting registered voters: {err}"))?;

            // Fetch ballots counted for the area
            let (ballots_counted, _results_area_contests, _contests) =
                get_election_contests_area_results_and_total_ballot_counted(
                    &hasura_transaction,
                    &self.tenant_id,
                    &self.election_event_id,
                    &self.election_id,
                )
                .await
                .map_err(|e| anyhow!("Error getting election contests area results: {}", e))?;

            // Create UserDataArea instance
            let area_data = UserDataArea {
                date_printed: date_printed.clone(),
                election_title: election_title.clone(),
                voting_period_start: voting_period_start_date.clone(),
                voting_period_end: voting_period_end_date.clone(),
                election_date: election_date.clone(),
                post: election_general_data.post.clone(),
                country,
                geographical_region: election_general_data.geographical_region.clone(),
                voting_center: election_general_data.voting_center.clone(),
                precinct_code: election_general_data.precinct_code.clone(),
                registered_voters,
                ballots_counted,
                ovcs_status: ovcs_status.clone(),
                chairperson_name: "John Doe".to_string(),
                poll_clerk_name: "Jane Smith".to_string(),
                third_member_name: "Alice Johnson".to_string(),
                report_hash: "dummy_report_hash".to_string(),
                ovcs_version: "1.0".to_string(),
                system_hash: "dummy_system_hash".to_string(),
            };

            areas.push(area_data);
        }

        // Return the UserData with areas populated
        Ok(UserData { areas })
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
pub async fn generate_status_report(
    document_id: &str,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
    mode: GenerateReportMode,
    hasura_transaction: Option<&Transaction<'_>>,
    keycloak_transaction: Option<&Transaction<'_>>,
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
            hasura_transaction,
            keycloak_transaction,
        )
        .await
}
