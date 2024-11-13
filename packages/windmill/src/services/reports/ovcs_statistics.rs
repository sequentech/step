// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::report_variables::{
    extract_election_event_annotations, get_app_hash, get_app_version, get_date_and_time,
    get_report_hash,
};
use super::voters::{count_not_enrolled_voters_by_area_id, get_voters_data, FilterListVoters};
use super::{report_variables::extract_election_data, template_renderer::*};
use crate::postgres::area::get_areas_by_election_id;
use crate::postgres::election::{get_election_by_id, get_elections};
use crate::postgres::election_event::get_election_event_by_id;
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
use sequent_core::types::hasura::core::Election;
use serde::{Deserialize, Serialize};
use tracing::instrument;

// Struct to hold user data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserData {
    pub execution_annotations: ExecutionAnnotations,
    pub elections: Vec<UserElectionData>,
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
pub struct UserElectionData {
    pub election_dates: StringifiedPeriodDates,
    pub election_title: String,
    pub regions: Vec<Region>,
    pub ofov_disapproved: i64,
    pub sbei_disapproved: i64,
    pub system_disapproved: i64,
}
// Struct to hold system data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemData {
    pub rendered_user_template: String,
    pub file_qrcode_lib: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RegionData {
    pub post: String,
    pub country: String,
    pub total: i64,
    pub not_pre_enrolled: i64,
    pub pre_enrolled: i64,
    pub pre_enrolled_not_voted: i64,
    pub pre_enrolled_voted: i64,
    pub voted: i64,
    pub password_reset_request: i64,
    pub remarks: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Region {
    pub name: String,
    pub data: Vec<RegionData>,
}

#[derive(Debug)]
pub struct OVCSStatisticsTemplate {
    pub tenant_id: String,
    pub election_event_id: String,
    pub election_id: Option<String>,
}

impl OVCSStatisticsTemplate {
    pub fn new(tenant_id: String, election_event_id: String, election_id: Option<String>) -> Self {
        OVCSStatisticsTemplate {
            tenant_id,
            election_event_id,
            election_id,
        }
    }
}

#[async_trait]
impl TemplateRenderer for OVCSStatisticsTemplate {
    type UserData = UserData;
    type SystemData = SystemData;

    fn get_report_type(&self) -> ReportType {
        ReportType::OVCS_STATISTICS
    }

    fn base_name(&self) -> String {
        "ovcs_statistics".to_string()
    }

    fn prefix(&self) -> String {
        format!(
            "ovcs_statistics_{}_{}_{}",
            self.tenant_id,
            self.election_event_id,
            self.election_id.clone().unwrap_or_default()
        )
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

    #[instrument(err, skip(self, hasura_transaction, keycloak_transaction))]
    async fn prepare_user_data(
        &self,
        hasura_transaction: &Transaction<'_>,
        keycloak_transaction: &Transaction<'_>,
    ) -> Result<Self::UserData> {
        let realm = get_event_realm(&self.tenant_id, &self.election_event_id);
        let date_printed = get_date_and_time();

        let election_event = get_election_event_by_id(
            &hasura_transaction,
            &self.tenant_id,
            &self.election_event_id,
        )
        .await
        .map_err(|e| anyhow::anyhow!("Error getting election event by id: {}", e))?;

        let elections: Vec<Election> = match &self.election_id {
            Some(election_id) => {
                match get_election_by_id(
                    &hasura_transaction,
                    &self.tenant_id,
                    &self.election_event_id,
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
                &self.tenant_id,
                &self.election_event_id,
            )
            .await
            .map_err(|e| anyhow::anyhow!("Error in get_elections: {}", e))?,
        };

        let scheduled_events = find_scheduled_event_by_election_event_id(
            &hasura_transaction,
            &self.tenant_id,
            &self.election_event_id,
        )
        .await
        .map_err(|e| {
            anyhow::anyhow!("Error getting scheduled events by election event_id: {}", e)
        })?;
        let app_hash = get_app_hash();
        let app_version = get_app_version();
        let report_hash = get_report_hash(&ReportType::OVERSEAS_VOTERS.to_string())
            .await
            .unwrap_or("-".to_string());
        let mut elections_data = vec![];
        for election in elections {
            let election_dates = get_election_dates(&election, scheduled_events.clone())
                .map_err(|e| anyhow::anyhow!("Error getting election dates {e}"))?;

            let mut regions = vec![];
            let election_title = election.name.clone();
            let election_general_data = extract_election_data(&election)
                .await
                .map_err(|err| anyhow!("Error extract election annotations {err}"))?;
            let election_areas = get_areas_by_election_id(
                &hasura_transaction,
                &self.tenant_id,
                &self.election_event_id,
                &election.id,
            )
            .await
            .map_err(|err| anyhow!("Error at get_areas_by_election_id: {err:?}"))?;

            let mut areas: Vec<RegionData> = vec![];

            for area in election_areas.iter() {
                let area_name = area.clone().name.unwrap_or("-".to_string());

                let filtered_voters = FilterListVoters {
                    pre_enrolled: false,
                    has_voted: None,
                    voters_sex: None,
                };

                let voters_data = get_voters_data(
                    &hasura_transaction,
                    &keycloak_transaction,
                    &realm,
                    &self.tenant_id,
                    &self.election_event_id,
                    &election.id,
                    &area.id,
                    true,
                    filtered_voters,
                )
                .await
                .map_err(|err| anyhow!("Error get_voters_data {err}"))?;

                let total_not_pre_enrolled =
                    count_not_enrolled_voters_by_area_id(&keycloak_transaction, &realm, &area.id)
                        .await
                        .map_err(|err| {
                            anyhow!("Error count_total_not_pre_enrolled_voters_by_area_id {err}")
                        })?;

                let total_pre_enrolled =
                    voters_data.total_voters.clone() - total_not_pre_enrolled.clone();
                areas.push(RegionData {
                    post: election_general_data.post.clone(),
                    country: area_name,
                    total: voters_data.total_voters,
                    not_pre_enrolled: total_not_pre_enrolled,
                    pre_enrolled: total_pre_enrolled,
                    pre_enrolled_not_voted: voters_data.total_not_voted,
                    pre_enrolled_voted: voters_data.total_voted,
                    voted: voters_data.total_voted, //TODO: what the difference between pre_enrolled_voted and voted?
                    password_reset_request: 0,
                    remarks: "-".to_string(),
                });
            }
            regions.push(Region {
                name: election_general_data.geographical_region.clone(),
                data: areas,
            });
            elections_data.push(UserElectionData {
                election_dates,
                election_title,
                regions,
                ofov_disapproved: 0,
                sbei_disapproved: 0,
                system_disapproved: 0,
            });
        }

        Ok(UserData {
            execution_annotations: ExecutionAnnotations {
                date_printed,
                report_hash,
                app_version: app_version.clone(),
                software_version: app_version.clone(),
                app_hash,
            },
            elections: elections_data,
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
