// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::report_variables::{
    get_app_hash, get_app_version, get_date_and_time, get_report_hash, process_elections,
    UserDataElection,
};
use super::template_renderer::*;
use super::voters::{set_up_region_voters_data, RegionData, VotersStatsData};
use crate::postgres::election::{get_election_by_id, get_elections};
use crate::postgres::reports::ReportType;
use crate::postgres::scheduled_event::find_scheduled_event_by_election_event_id;
use crate::services::s3::get_minio_url;
use crate::services::temp_path::*;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use deadpool_postgres::Transaction;
use sequent_core::ballot::StringifiedPeriodDates;
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
pub struct NumOVNotPreEnrolledReport {
    ids: ReportOrigins,
}

impl NumOVNotPreEnrolledReport {
    pub fn new(ids: ReportOrigins) -> Self {
        NumOVNotPreEnrolledReport { ids }
    }
}

#[async_trait]
impl TemplateRenderer for NumOVNotPreEnrolledReport {
    type UserData = UserData;
    type SystemData = SystemData;

    fn get_report_type(&self) -> ReportType {
        ReportType::NUMBER_OF_OV_WHO_HAVE_NOT_YET_PRE_ENROLLED
    }

    fn get_tenant_id(&self) -> String {
        self.ids.tenant_id.clone()
    }

    fn get_election_event_id(&self) -> String {
        self.ids.election_event_id.clone()
    }

    fn get_initial_template_alias(&self) -> Option<String> {
        self.ids.template_alias.clone()
    }


    fn get_report_origin(&self) -> ReportOriginatedFrom {
        self.ids.report_origin
    }

    fn get_election_id(&self) -> Option<String> {
        self.ids.election_id.clone()
    }

    fn base_name(&self) -> String {
        "num_of_ov_not_yet_pre_enrolled".to_string()
    }

    fn prefix(&self) -> String {
        format!(
            "num_of_ov_not_yet_pre_enrolled_{}_{}_{}",
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
        let report_hash =
            get_report_hash(&ReportType::NUMBER_OF_OV_WHO_HAVE_NOT_YET_PRE_ENROLLED.to_string())
                .await
                .unwrap_or("-".to_string());

        let mut overall_total_male_landbased: i64 = 0;
        let mut overall_total_female_landbased: i64 = 0;
        let mut overall_total_landbased: i64 = 0;
        let mut overall_total_male_seafarer: i64 = 0;
        let mut overall_total_female_seafarer: i64 = 0;
        let mut overall_total_seafarer: i64 = 0;
        let mut overall_total_male: i64 = 0;
        let mut overall_total_female: i64 = 0;
        let mut overall_total: i64 = 0;

        let mut regions: Vec<RegionData> = vec![];
        for region in elections_data.regions {
            let region_name = region.0.clone();
            let posts = region.1.clone();
            let region_data = set_up_region_voters_data(
                &keycloak_transaction,
                &realm,
                &region_name,
                posts.clone(),
                true,
            )
            .await
            .map_err(|err| anyhow!("Error set_up_region_voters_data {err}"))?;

            regions.push(region_data.clone());

            let region_overall_total = region_data.stats.clone();

            overall_total_male_landbased += region_overall_total.total_male_landbased;
            overall_total_female_landbased += region_overall_total.total_female_landbased;
            overall_total_landbased += region_overall_total.total_landbased;
            overall_total_male_seafarer += region_overall_total.total_male_seafarer;
            overall_total_female_seafarer += region_overall_total.total_female_seafarer;
            overall_total_seafarer += region_overall_total.total_seafarer;

            overall_total_male += region_overall_total.total_male;
            overall_total_female += region_overall_total.total_female;
            overall_total += region_overall_total.overall_total;
        }

        Ok(UserData {
            regions: regions,
            elections: elections_data.elections,
            execution_annotations: ExecutionAnnotations {
                date_printed,
                report_hash,
                software_version: app_version.clone(),
                app_version,
                app_hash,
            },
            overall_total: VotersStatsData {
                total_male_landbased: overall_total_male_landbased,
                total_female_landbased: overall_total_female_landbased,
                total_landbased: overall_total_landbased,
                total_male_seafarer: overall_total_male_seafarer,
                total_female_seafarer: overall_total_female_seafarer,
                total_seafarer: overall_total_seafarer,
                total_male: overall_total_male,
                total_female: overall_total_female,
                overall_total,
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
