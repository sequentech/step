// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::report_variables::{
    extract_area_data, extract_election_data, extract_election_event_annotations, get_app_hash,
    get_app_version, get_date_and_time, get_report_hash,
    get_total_number_of_registered_voters_for_area_id,
};
use super::template_renderer::*;
use crate::postgres::area::get_areas_by_election_id;
use crate::postgres::election::get_election_by_id;
use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::reports::ReportType;
use crate::postgres::scheduled_event::find_scheduled_event_by_election_event_id;
use crate::services::s3::get_minio_url;
use crate::services::temp_path::*;
use anyhow::{anyhow, Context, Ok, Result};
use async_trait::async_trait;
use deadpool_postgres::Transaction;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::types::scheduled_event::generate_voting_period_dates;
use sequent_core::types::templates::EmailConfig;
use serde::{Deserialize, Serialize};
use tracing::instrument;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserData {
    pub areas: Vec<UserDataArea>,
}
/// Struct for User Data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserDataArea {
    pub date_printed: String,
    pub copy_number: String,
    pub election_date: String,
    pub election_title: String,
    pub voting_period_start: String,
    pub voting_period_end: String,
    pub geographical_region: String,
    pub post: String,
    pub country: String,
    pub voting_center: String,
    pub precinct_code: String,
    pub registered_voters: i64,
    pub report_hash: String,
    pub software_version: String,
    pub ovcs_version: String,
    pub system_hash: String,
    pub qr_codes: Vec<String>,
}

/// Struct for System Data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemData {
    pub rendered_user_template: String,
    pub file_qrcode_lib: String,
}

#[derive(Debug)]
pub struct OVCSInformaitionTemplate {
    pub tenant_id: String,
    pub election_event_id: String,
    pub election_id: Option<String>,
}

#[async_trait]
impl TemplateRenderer for OVCSInformaitionTemplate {
    type UserData = UserData;
    type SystemData = SystemData;

    fn get_report_type() -> ReportType {
        ReportType::OVCS_INFORMATION
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
        "ovcs_information".to_string()
    }

    fn prefix(&self) -> String {
        format!(
            "ovcs_information_{}_{}_{}",
            self.tenant_id,
            self.election_event_id,
            self.election_id.clone().unwrap_or_default()
        )
    }

    fn get_email_config() -> EmailConfig {
        EmailConfig {
            subject: "Sequent Online Voting - OVCS Information".to_string(),
            plaintext_body: "".to_string(),
            html_body: None,
        }
    }

    #[instrument]
    async fn prepare_user_data(
        &self,
        hasura_transaction: &Transaction<'_>,
        keycloak_transaction: &Transaction<'_>,
    ) -> Result<Self::UserData> {
        let realm_name = get_event_realm(self.tenant_id.as_str(), self.election_event_id.as_str());

        let Some(election_id) = &self.election_id else {
            return Err(anyhow!("Empty election_id"));
        };

        // Fetch the election data
        let election = match get_election_by_id(
            &hasura_transaction,
            &self.get_tenant_id(),
            &self.get_election_event_id(),
            &election_id,
        )
        .await
        .with_context(|| "Error getting election by id")?
        {
            Some(election) => election,
            None => return Err(anyhow::anyhow!("Election not found")),
        };

        // Fetch the start election event data
        let start_election_event = find_scheduled_event_by_election_event_id(
            &hasura_transaction,
            &self.tenant_id,
            &self.election_event_id,
        )
        .await
        .with_context(|| "Error getting scheduled event by election_event_id")?;

        // Generate voting period dates
        let voting_period_dates = generate_voting_period_dates(
            start_election_event,
            &self.tenant_id,
            &self.election_event_id,
            Some(&election_id),
        )
        .map_err(|e| anyhow!(format!("Error generating voting period dates {e:?}")))?;

        // Extract start and end dates from voting period
        let voting_period_start_date = voting_period_dates.start_date.unwrap_or_default();
        let voting_period_end_date = voting_period_dates.end_date.unwrap_or_default();

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

        let election_general_data = extract_election_data(&election)
            .await
            .map_err(|err| anyhow!("Error extract election annotations {err}"))?;

        let election_areas = get_areas_by_election_id(
            &hasura_transaction,
            &self.tenant_id,
            &self.election_event_id,
            &election_id,
        )
        .await
        .map_err(|err| anyhow!("Error at get_areas_by_election_id: {err:?}"))?;

        let date_printed = get_date_and_time();
        let election_date = voting_period_start_date.clone().to_string();
        let election_title = election_event.name.clone();

        let app_hash = get_app_hash();
        let app_version = get_app_version();
        let report_hash = get_report_hash(&ReportType::OVCS_INFORMATION.to_string())
            .await
            .unwrap_or("-".to_string());

        let mut areas: Vec<UserDataArea> = vec![];

        for area in election_areas.iter() {
            let country = area.clone().name.unwrap_or('-'.to_string());

            let area_general_data =
                extract_area_data(&area, election_event_annotations.sbei_users.clone())
                    .await
                    .map_err(|err| anyhow!("Can't extract election data: {err}"))?;

            // Fetch total of registered voters for the area
            let registered_voters = get_total_number_of_registered_voters_for_area_id(
                keycloak_transaction,
                &realm_name,
                &area.id,
            )
            .await
            .with_context(|| format!("Error counting registered voters for area {}", &area.id))?;

            let temp_val: &str = "test";

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
                copy_number: temp_val.to_string(),
                qr_codes: vec![],
                report_hash: report_hash.clone(),
                software_version: app_version.clone(),
                ovcs_version: app_version.clone(),
                system_hash: app_hash.clone(),
            };

            areas.push(area_data);
        }

        Ok(UserData { areas })
    }

    #[instrument]
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

#[instrument]
pub async fn generate_ovcs_informations_report(
    document_id: &str,
    tenant_id: &str,
    election_event_id: &str,
    election_id: Option<&str>,
    mode: GenerateReportMode,
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    is_scheduled_task: bool,
    email_recipients: Option<String>,
) -> Result<()> {
    let template = OVCSInformaitionTemplate {
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
