// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::report_variables::{
    get_app_hash, get_app_version, get_date_and_time, get_report_hash, process_elections,
    UserDataElection,
};
use super::template_renderer::*;
use crate::postgres::election::{get_election_by_id, get_elections};
use crate::postgres::reports::ReportType;
use crate::postgres::scheduled_event::find_scheduled_event_by_election_event_id;
use crate::services::s3::get_minio_url;
use crate::services::temp_path::*;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use deadpool_postgres::Transaction;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::types::hasura::core::Election;
use serde::{Deserialize, Serialize};
use tracing::instrument;

// Struct to hold user data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserData {
    pub execution_annotations: ExecutionAnnotations,
    pub elections: Vec<UserDataElection>,
    pub regions: Vec<RegionData>,
    pub overall_total: VotersStatsData,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VotersStatsData {
    pub total_male_landbased: i64,
    pub total_female_landbased: i64,
    pub total_landbased: i64,
    pub total_voted_male_landbased: i64,
    pub total_voted_female_landbased: i64,
    pub total_voted_landbased: i64,
    pub percentage_male_landbased: f64,
    pub percentage_female_landbased: f64,
    pub percentage_landbased: f64,
    pub total_male_seafarer: i64,
    pub total_female_seafarer: i64,
    pub total_seafarer: i64,
    pub total_voted_male_seafarer: i64,
    pub total_voted_female_seafarer: i64,
    pub total_voted_seafarer: i64,
    pub percentage_male_seafarer: f64,
    pub percentage_female_seafarer: f64,
    pub percentage_seafarer: f64,
    pub percentage_male: f64,
    pub percentage_female: f64,
    pub percentage_total: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PostData {
    pub post: String,
    pub area_name: String,
    pub stats: VotersStatsData,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RegionData {
    pub geographical_region: String,
    pub posts: Vec<PostData>,
    pub stats: VotersStatsData,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExecutionAnnotations {
    pub date_printed: String,
    pub report_hash: String,
    pub app_version: String,
    pub software_version: String,
    pub app_hash: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemData {
    pub rendered_user_template: String,
    pub file_qrcode_lib: String,
}

/// Main struct for generating Overseas Voters Report
#[derive(Debug)]
pub struct OVTurnoutPerAboardAndSexPercentageReport {
    ids: ReportOrigins,
}

impl OVTurnoutPerAboardAndSexPercentageReport {
    pub fn new(ids: ReportOrigins) -> Self {
        OVTurnoutPerAboardAndSexPercentageReport { ids }
    }
}

#[async_trait]
impl TemplateRenderer for OVTurnoutPerAboardAndSexPercentageReport {
    type UserData = UserData;
    type SystemData = SystemData;

    fn get_report_type(&self) -> ReportType {
        ReportType::OVERSEAS_VOTERS_TURNOUT_PER_ABOARD_STATUS_SEX_AND_WITH_PERCENTAGE
    }

    fn get_tenant_id(&self) -> String {
        self.ids.tenant_id.clone()
    }

    fn get_election_event_id(&self) -> String {
        self.ids.election_event_id.clone()
    }

    fn get_initial_template_id(&self) -> Option<String> {
        self.ids.template_id.clone()
    }

    fn get_report_origin(&self) -> ReportOriginatedFrom {
        self.ids.report_origin
    }

    fn get_election_id(&self) -> Option<String> {
        self.ids.election_id.clone()
    }

    fn base_name(&self) -> String {
        "ov_turnout_per_aboard_and_sex_percentage".to_string()
    }

    fn prefix(&self) -> String {
        format!(
            "ov_turnout_per_aboard_and_sex_percentage_{}_{}_{}",
            self.ids.tenant_id,
            self.ids.election_event_id,
            self.ids.election_id.clone().unwrap_or_default()
        )
    }

    #[instrument(err, skip(self, hasura_transaction, keycloak_transaction))]
    async fn prepare_user_data(
        &self,
        hasura_transaction: &Transaction<'_>,
        keycloak_transaction: &Transaction<'_>,
    ) -> Result<Self::UserData> {
        let realm = get_event_realm(&self.ids.tenant_id, &self.ids.election_event_id);
        let date_printed = get_date_and_time();

        let elections: Vec<Election> = match &self.ids.election_id {
            Some(election_id) => {
                match get_election_by_id(
                    &hasura_transaction,
                    &self.ids.tenant_id,
                    &self.ids.election_event_id,
                    &election_id,
                )
                .await
                .with_context(|| "Error getting election by id")?
                {
                    Some(election) => vec![election],
                    None => vec![],
                }
            }
            None => get_elections(
                &hasura_transaction,
                &self.ids.tenant_id,
                &self.ids.election_event_id,
                None,
            )
            .await
            .map_err(|e| anyhow::anyhow!("Error in get_elections: {}", e))?,
        };

        let scheduled_events = find_scheduled_event_by_election_event_id(
            &hasura_transaction,
            &self.ids.tenant_id,
            &self.ids.election_event_id,
        )
        .await
        .map_err(|e| {
            anyhow::anyhow!("Error getting scheduled events by election event_id: {}", e)
        })?;

        let elections_data = process_elections(elections, scheduled_events)
            .await
            .map_err(|err| anyhow!("Error process_elections {err}"))?;

        let app_hash = get_app_hash();
        let app_version = get_app_version();
        let report_hash = get_report_hash(
            &ReportType::OVERSEAS_VOTERS_TURNOUT_PER_ABOARD_STATUS_SEX_AND_WITH_PERCENTAGE
                .to_string(),
        )
        .await
        .unwrap_or("-".to_string());

        let regions_mock = vec![RegionData {
            geographical_region: "Asia/Pacific".to_string(),
            posts: vec![PostData {
                post: "New York PE".to_string(),
                area_name: "United States".to_string(),
                stats: VotersStatsData {
                    total_male_landbased: 20,
                    total_female_landbased: 22,
                    total_landbased: 42,
                    total_voted_male_landbased: 12,
                    total_voted_female_landbased: 17,
                    total_voted_landbased: 29,
                    percentage_male_landbased: 60.00,
                    percentage_female_landbased: 77.27,
                    percentage_landbased: 69.04,
                    total_male_seafarer: 10,
                    total_female_seafarer: 5,
                    total_seafarer: 15,
                    total_voted_male_seafarer: 4,
                    total_voted_female_seafarer: 3,
                    total_voted_seafarer: 7,
                    percentage_male_seafarer: 40.00,
                    percentage_female_seafarer: 60.00,
                    percentage_seafarer: 46.66,
                    percentage_male: 53.33,
                    percentage_female: 74.07,
                    percentage_total: 63.00,
                },
            }],
            stats: VotersStatsData {
                total_male_landbased: 20,
                total_female_landbased: 22,
                total_landbased: 42,
                total_voted_male_landbased: 12,
                total_voted_female_landbased: 17,
                total_voted_landbased: 29,
                percentage_male_landbased: 60.00,
                percentage_female_landbased: 77.27,
                percentage_landbased: 69.04,
                total_male_seafarer: 10,
                total_female_seafarer: 5,
                total_seafarer: 15,
                total_voted_male_seafarer: 4,
                total_voted_female_seafarer: 3,
                total_voted_seafarer: 7,
                percentage_male_seafarer: 40.00,
                percentage_female_seafarer: 60.00,
                percentage_seafarer: 46.66,
                percentage_male: 53.33,
                percentage_female: 74.07,
                percentage_total: 63.00,
            },
        }];

        Ok(UserData {
            regions: regions_mock,
            elections: elections_data.elections,
            execution_annotations: ExecutionAnnotations {
                date_printed,
                report_hash,
                software_version: app_version.clone(),
                app_version,
                app_hash,
            },
            overall_total: VotersStatsData {
                total_male_landbased: 0,
                total_female_landbased: 0,
                total_landbased: 0,
                total_voted_male_landbased: 0,
                total_voted_female_landbased: 0,
                total_voted_landbased: 0,
                percentage_male_landbased: 0.00,
                percentage_female_landbased: 0.00,
                percentage_landbased: 0.00,
                total_male_seafarer: 0,
                total_female_seafarer: 0,
                total_seafarer: 0,
                total_voted_male_seafarer: 0,
                total_voted_female_seafarer: 0,
                total_voted_seafarer: 0,
                percentage_male_seafarer: 0.00,
                percentage_female_seafarer: 0.00,
                percentage_seafarer: 0.00,
                percentage_male: 0.00,
                percentage_female: 0.00,
                percentage_total: 0.00,
            },
        })
    }

    /// Prepare system metadata for the report
    #[instrument(err, skip_all)]
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
