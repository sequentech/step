// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::report_variables::{
    extract_election_data, get_app_hash, get_app_version, get_date_and_time, get_report_hash,
};
use super::template_renderer::*;
use super::voters::{
    count_voters_by_their_sex, FilterListVoters, FEMALE_VALE, LANDBASED_VALUE, MALE_VALE,
    SEAFARER_VALUE,
};
use crate::postgres::area::get_areas_by_election_id;
use crate::postgres::election::get_election_by_id;
use crate::postgres::reports::ReportType;
use crate::postgres::scheduled_event::find_scheduled_event_by_election_event_id;
use crate::services::election_dates::get_election_dates;
use crate::services::s3::get_minio_url;
use crate::services::temp_path::*;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use deadpool_postgres::Transaction;
use sequent_core::ballot::StringifiedPeriodDates;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::types::scheduled_event::generate_voting_period_dates;
use serde::{Deserialize, Serialize};
use tracing::instrument;

// Struct to hold user data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserData {
    pub execution_annotations: ExecutionAnnotations,
    pub election: UserDataElection,
    pub areas: Vec<UserDataArea>,
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
pub struct UserDataElection {
    pub election_dates: StringifiedPeriodDates,
    pub election_title: String,
    pub post: String,
    pub overall_total: UserDataStats,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserDataStats {
    pub total_male_landbased: i64,
    pub total_female_landbased: i64,
    pub total_landbased: i64,
    pub total_male_seafarer: i64,
    pub total_female_seafarer: i64,
    pub total_seafarer: i64,
    pub total_male: i64,
    pub total_female: i64,
    pub overall_total: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserDataArea {
    pub area_name: String,
    pub stats: UserDataStats,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemData {
    pub rendered_user_template: String,
    pub file_qrcode_lib: String,
}

/// Main struct for generating Overseas Voters Report
#[derive(Debug)]
pub struct NumOVNotPreEnrolledReport {
    pub tenant_id: String,
    pub election_event_id: String,
    pub election_id: Option<String>,
}

impl NumOVNotPreEnrolledReport {
    pub fn new(tenant_id: String, election_event_id: String, election_id: Option<String>) -> Self {
        NumOVNotPreEnrolledReport {
            tenant_id,
            election_event_id,
            election_id,
        }
    }
}

#[async_trait]
impl TemplateRenderer for NumOVNotPreEnrolledReport {
    type UserData = UserData;
    type SystemData = SystemData;

    fn get_report_type(&self) -> ReportType {
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

    fn base_name(&self) -> String {
        "num_of_ov_not_yet_pre_enrolled".to_string()
    }

    fn prefix(&self) -> String {
        format!(
            "num_of_ov_not_yet_pre_enrolled_{}_{}_{}",
            self.tenant_id,
            self.election_event_id,
            self.election_id.clone().unwrap_or_default()
        )
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
        let scheduled_events = find_scheduled_event_by_election_event_id(
            &hasura_transaction,
            &self.tenant_id,
            &self.election_event_id,
        )
        .await
        .map_err(|e| {
            anyhow::anyhow!("Error getting scheduled events by election event_id: {}", e)
        })?;

        let election_dates = get_election_dates(&election, scheduled_events)
            .map_err(|e| anyhow::anyhow!("Error getting election dates {e}"))?;

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

        let mut overall_total_male_landbased: i64 = 0;
        let mut overall_total_female_landbased: i64 = 0;
        let mut overall_total_landbased: i64 = 0;
        let mut overall_total_male_seafarer: i64 = 0;
        let mut overall_total_female_seafarer: i64 = 0;
        let mut overall_total_seafarer: i64 = 0;
        let mut overall_total_male: i64 = 0;
        let mut overall_total_female: i64 = 0;
        let mut overall_total: i64 = 0;

        for area in election_areas.iter() {
            let area_name = area.clone().name.unwrap_or("-".to_string());

            let landbased = count_voters_by_their_sex(
                &keycloak_transaction,
                &realm,
                &area.id,
                Some(LANDBASED_VALUE),
            )
            .await
            .map_err(|err| anyhow!("Error count_voters_by_their_sex, landbase {err}"))?;
            let seafarer = count_voters_by_their_sex(
                &keycloak_transaction,
                &realm,
                &area.id,
                Some(SEAFARER_VALUE),
            )
            .await
            .map_err(|err| anyhow!("Error count_voters_by_their_sex, landbase {err}"))?;
            let general = count_voters_by_their_sex(&keycloak_transaction, &realm, &area.id, None)
                .await
                .map_err(|err| anyhow!("Error count_voters_by_their_sex, landbase {err}"))?;

            overall_total_male_landbased += landbased.total_male;
            overall_total_female_landbased += landbased.total_female;
            overall_total_landbased += landbased.overall_total;
            overall_total_male_seafarer = seafarer.total_male;
            overall_total_female_seafarer += seafarer.total_female;
            overall_total_seafarer += seafarer.overall_total;

            overall_total_male += general.total_male;
            overall_total_female += general.total_female;
            overall_total += general.overall_total;

            areas.push(UserDataArea {
                area_name,
                stats: UserDataStats {
                    total_male_landbased: landbased.total_male,
                    total_female_landbased: landbased.total_female,
                    total_landbased: landbased.overall_total,
                    total_male_seafarer: seafarer.total_male,
                    total_female_seafarer: seafarer.total_female,
                    total_seafarer: seafarer.overall_total,
                    total_male: general.total_male,
                    total_female: general.total_female,
                    overall_total: general.overall_total,
                },
            })
        }

        Ok(UserData {
            areas,
            election: UserDataElection {
                election_dates,
                election_title,
                post: election_general_data.post,
                overall_total: UserDataStats {
                    total_male_landbased: overall_total_male_landbased,
                    total_female_landbased: overall_total_female_landbased,
                    total_landbased: overall_total_landbased,
                    total_male_seafarer: overall_total_male_seafarer,
                    total_female_seafarer: overall_total_female_seafarer,
                    total_seafarer: overall_total_seafarer,
                    total_male: overall_total_male,
                    total_female: overall_total_female,
                    overall_total: overall_total,
                },
            },
            execution_annotations: ExecutionAnnotations {
                date_printed,
                report_hash,
                software_version: app_version.clone(),
                app_version,
                app_hash,
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
