// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::report_variables::{
    extract_area_data, extract_election_data, extract_election_event_annotations, get_app_hash,
    get_app_version, get_date_and_time, get_report_hash, InspectorData,
};
use super::template_renderer::*;
use crate::postgres::area::get_areas_by_election_id;
use crate::postgres::election::get_election_by_id;
use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::reports::ReportType;
use crate::postgres::scheduled_event::find_scheduled_event_by_election_event_id;
use crate::services::s3::get_minio_url;
use crate::services::temp_path::*;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use chrono::offset::TimeZone;
use deadpool_postgres::Transaction;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::types::scheduled_event::generate_voting_period_dates;
use sequent_core::types::templates::EmailConfig;
use serde::{Deserialize, Serialize};
use tracing::instrument;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserDataArea {
    pub date_printed: String,
    pub election_title: String,
    pub voting_period_start: String,
    pub voting_period_end: String,
    pub election_date: String,
    pub post: String,
    pub area_name: String,
    pub precinct_code: String,
    pub voters: Vec<Voter>,       // Voter list field
    pub ov_voted: u32,            // Number of overseas voters who voted
    pub ov_not_voted: u32,        // Number of overseas voters who did not vote
    pub ov_not_pre_enrolled: u32, // Number of overseas voters not pre-enrolled
    pub eb_voted: u32,            // Election board voted count
    pub ov_total: u32,            // Total overseas voters
    pub report_hash: String,
    pub ovcs_version: String,
    pub software_version: String,
    pub system_hash: String,
    pub inspectors: Vec<InspectorData>,
}

/// Struct for User Data Area
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserData {
    pub areas: Vec<UserDataArea>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Voter {
    pub number: u32,
    pub last_name: String,
    pub first_name: String,
    pub middle_name: String,
    pub suffix: String,
    pub status: String,
    pub date_voted: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemData {
    pub rendered_user_template: String,
    pub file_qrcode_lib: String,
}

/// Main struct for generating Overseas Voters Report
#[derive(Debug)]
pub struct OverseasVotersReport {
    tenant_id: String,
    election_event_id: String,
    pub election_id: Option<String>,
}

#[async_trait]
impl TemplateRenderer for OverseasVotersReport {
    type UserData = UserData;
    type SystemData = SystemData;

    fn get_report_type() -> ReportType {
        ReportType::OVERSEAS_VOTERS
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

    fn base_name() -> String {
        "overseas_voters".to_string()
    }

    fn prefix(&self) -> String {
        format!(
            "overseas_voters_{}_{}_{}",
            self.tenant_id,
            self.election_event_id,
            self.election_id.clone().unwrap_or_default()
        )
    }

    fn get_email_config() -> EmailConfig {
        EmailConfig {
            subject: "Sequent Online Voting - Overseas Voters".to_string(),
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
        let realm = get_event_realm(&self.tenant_id, &self.election_event_id);
        let date_printed = get_date_and_time();

        let Some(election_id) = &self.election_id else {
            return Err(anyhow!("Empty election_id"));
        };

        let election_event = get_election_event_by_id(
            &hasura_transaction,
            &self.tenant_id,
            &self.election_event_id,
        )
        .await
        .map_err(|e| anyhow::anyhow!("Error getting election event by id: {}", e))?;

        let election_event_annotations = extract_election_event_annotations(&election_event)
            .await
            .map_err(|err| anyhow!("Error extract election event annotations {err}"))?;

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

        let election_title = election.name.clone();

        let election_general_data = extract_election_data(&election)
            .await
            .map_err(|err| anyhow!("Error extract election annotations {err}"))?;

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
        )
        .map_err(|e| anyhow!(format!("Error generating voting period dates {e:?}")))?;

        // extract start date from voting period
        let voting_period_start_date = voting_period_dates.start_date.unwrap_or_default();
        // extract end date from voting period
        let voting_period_end_date = voting_period_dates.end_date.unwrap_or_default();

        let election_date: String = voting_period_start_date.clone();

        let election_areas = get_areas_by_election_id(
            &hasura_transaction,
            &self.tenant_id,
            &self.election_event_id,
            &election_id,
        )
        .await
        .map_err(|err| anyhow!("Error at get_areas_by_election_id: {err:?}"))?;

        let app_hash = get_app_hash();
        let app_version = get_app_version();
        let report_hash = get_report_hash(&ReportType::OVERSEAS_VOTERS.to_string())
            .await
            .unwrap_or("-".to_string());

        let mut areas: Vec<UserDataArea> = vec![];

        for area in election_areas.iter() {
            let area_general_data =
                extract_area_data(&area, election_event_annotations.sbei_users.clone())
                    .await
                    .map_err(|err| anyhow!("Error extract area data {err}"))?;
            let area_name = area.clone().name.unwrap_or("-".to_string());

            areas.push(UserDataArea {
                date_printed: date_printed.clone(),
                election_title: election_title.clone(),
                voting_period_start: voting_period_start_date.clone(),
                voting_period_end: voting_period_end_date.clone(),
                election_date: election_date.clone(),
                post: election_general_data.post.clone(),
                area_name: area_name,
                precinct_code: election_general_data.precinct_code.clone(),
                report_hash: report_hash.clone(),
                software_version: app_version.clone(),
                ovcs_version: app_version.clone(),
                system_hash: app_hash.clone(),
                inspectors: area_general_data.inspectors,
                voters: vec![],         // Voter list field
                ov_voted: 0,            // Number of overseas voters who voted
                ov_not_voted: 0,        // Number of overseas voters who did not vote
                ov_not_pre_enrolled: 0, // Number of overseas voters not pre-enrolled
                eb_voted: 0,            // Election board voted count
                ov_total: 0,            // Total overseas voters
            })
        }

        Ok(UserData { areas })
    }

    /// Prepare system metadata for the report
    #[instrument(err, skip(self))]
    async fn prepare_system_data(
        &self,
        rendered_user_template: String,
    ) -> Result<Self::SystemData> {
        let public_asset_path = get_public_assets_path_env_var()?;
        let minio_endpoint_base =
            get_minio_url().with_context(|| "Error getting minio endpoint")?;

        Ok(SystemData {
            rendered_user_template,
            file_qrcode_lib: format!(
                "{}/{}/{}",
                minio_endpoint_base, public_asset_path, PUBLIC_ASSETS_QRCODE_LIB
            ),
        })
    }
}

/// Generate Overseas Voters Report
#[instrument(err, skip(hasura_transaction, keycloak_transaction))]
pub async fn generate_overseas_voters_report(
    document_id: &str,
    tenant_id: &str,
    election_event_id: &str,
    election_id: Option<&str>,
    mode: GenerateReportMode,
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
) -> Result<()> {
    let template = OverseasVotersReport {
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
