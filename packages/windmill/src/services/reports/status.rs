// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::report_variables::{
    extract_area_data, extract_election_data, extract_election_event_annotations, get_app_hash,
    get_app_version, get_date_and_time, get_total_number_of_registered_voters_for_area_id,
};
use super::template_renderer::*;
use crate::postgres::area::get_areas_by_election_id;
use crate::postgres::election::get_election_by_id;
use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::reports::ReportType;
use crate::postgres::scheduled_event::find_scheduled_event_by_election_event_id;
use crate::services::cast_votes::count_ballots_by_area_id;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use deadpool_postgres::Transaction;
use sequent_core::ballot::InitReport;
use sequent_core::serialization::deserialize_with_path::deserialize_value;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::types::scheduled_event::generate_voting_period_dates;
use sequent_core::{ballot::ElectionStatus, ballot::VotingStatus, types::templates::EmailConfig};
use serde::{Deserialize, Serialize};
use serde_json::value::Value;
use tracing::instrument;
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
    pub election_id: Option<String>,
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
        self.election_id.clone()
    }

    fn get_email_config() -> EmailConfig {
        EmailConfig {
            subject: "Sequent Online Voting - Status".to_string(),
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
        let realm = get_event_realm(self.tenant_id.as_str(), self.election_event_id.as_str());

        let Some(election_id) = &self.election_id else {
            return Err(anyhow!("Empty election_id"));
        };

        // Fetch election event data
        let election_event = get_election_event_by_id(
            &hasura_transaction,
            &self.tenant_id,
            &self.election_event_id,
        )
        .await
        .with_context(|| "Error obtaining election event")?;

        let election_event_annotations = extract_election_event_annotations(&election_event)
            .await
            .map_err(|err| anyhow!("Error extract election event annotations {err}"))?;

        // Fetch election data
        // get election instace
        let election = match get_election_by_id(
            &hasura_transaction,
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

        let election_general_data = extract_election_data(&election)
            .await
            .map_err(|err| anyhow!("Error extract election annotations {err}"))?;

        // Get OVCS status
        let status = get_election_status(election.status.clone()).unwrap_or_default();

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
            &election_id,
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
            &self.tenant_id,
            &self.election_event_id,
        )
        .await
        .map_err(|e| {
            anyhow::anyhow!("Error getting scheduled event by election event_id: {}", e)
        })?;

        let voting_period_dates = generate_voting_period_dates(
            start_election_event,
            &self.tenant_id,
            &self.election_event_id,
            Some(&election_id),
        )
        .map_err(|e| anyhow!(format!("Error generating voting period dates {e:?}")))?;

        let voting_period_start_date = voting_period_dates.start_date.unwrap_or_default();
        let voting_period_end_date = voting_period_dates.end_date.unwrap_or_default();

        let election_date = &voting_period_start_date.to_string();

        let date_printed = get_date_and_time();
        let election_title = election_event.name.clone();

        let app_hash = get_app_hash();
        let app_version = get_app_version();

        // Loop over each area and collect data
        for area in election_areas.iter() {
            let country = area.clone().name.unwrap_or('-'.to_string());

            let area_general_data =
                extract_area_data(&area, election_event_annotations.sbei_users.clone())
                    .await
                    .map_err(|err| anyhow!("Error extract area data {err}"))?;

            let registered_voters = get_total_number_of_registered_voters_for_area_id(
                &keycloak_transaction,
                &realm,
                &area.id,
            )
            .await
            .map_err(|err| anyhow!("Error counting registered voters: {err}"))?;

            let ballots_counted = count_ballots_by_area_id(
                &hasura_transaction,
                &self.tenant_id,
                &self.election_event_id,
                &election_id,
                &area.id,
            )
            .await
            .map_err(|e| {
                anyhow::anyhow!("Error fetching the number of ballot for election {e:?}",)
            })?;

            let report_hash = "-".to_string();

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
                report_hash,
                ovcs_version: app_version.clone(),
                system_hash: app_hash.clone(),
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

#[instrument(err, skip(hasura_transaction, keycloak_transaction))]
pub async fn generate_status_report(
    document_id: &str,
    tenant_id: &str,
    election_event_id: &str,
    election_id: Option<&str>,
    mode: GenerateReportMode,
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
) -> Result<()> {
    let template = StatusTemplate {
        tenant_id: tenant_id.to_string(),
        election_event_id: election_event_id.to_string(),
        election_id: election_id.map(|s| s.to_string()),
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
