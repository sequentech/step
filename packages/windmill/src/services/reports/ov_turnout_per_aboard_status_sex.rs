// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::report_variables::{
    extract_election_data, get_app_hash, get_app_version, get_date_and_time, get_report_hash,
    ExecutionAnnotations,
};
use super::template_renderer::*;
use super::voters::{
    set_up_voters_per_aboard_and_sex_by_area_post_region, PostData, RegionData, VotersStatsData,
};
use crate::postgres::area::get_areas_by_election_id;
use crate::postgres::election::{get_election_by_id, get_elections};
use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::reports::ReportType;
use crate::postgres::scheduled_event::find_scheduled_event_by_election_event_id;
use crate::services::election_dates::get_election_dates;
use crate::services::temp_path::PUBLIC_ASSETS_QRCODE_LIB;
use crate::services::temp_path::*;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use deadpool_postgres::Transaction;
use sequent_core::ballot::StringifiedPeriodDates;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::services::pdf;
use sequent_core::services::s3::get_minio_url;
use sequent_core::types::hasura::core::Election;
use sequent_core::util::temp_path::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::instrument;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserElectionData {
    pub election_dates: StringifiedPeriodDates,
    pub election_name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RegionDataComputed {
    pub geographical_region: String,
    pub stats: VotersStatsData,
    pub posts: Vec<PostData>,
}

// Struct to hold user data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserData {
    pub election_event_title: String,
    pub execution_annotations: ExecutionAnnotations,
    pub elections: Vec<UserElectionData>,
    pub regions: Vec<RegionDataComputed>,
    pub overall_total: VotersStatsData,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemData {
    pub rendered_user_template: String,
    pub file_qrcode_lib: String,
}

/// Main struct for generating Overseas Voters Report
#[derive(Debug)]
pub struct OVTurnoutPerAboardAndSexReport {
    ids: ReportOrigins,
}

impl OVTurnoutPerAboardAndSexReport {
    pub fn new(ids: ReportOrigins) -> Self {
        OVTurnoutPerAboardAndSexReport { ids }
    }
}

#[async_trait]
impl TemplateRenderer for OVTurnoutPerAboardAndSexReport {
    type UserData = UserData;
    type SystemData = SystemData;

    fn get_report_type(&self) -> ReportType {
        ReportType::OV_TURNOUT_PER_ABOARD_STATUS_SEX
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
        "ov_turnout_per_aboard_status_sex".to_string()
    }

    fn prefix(&self) -> String {
        format!(
            "ov_turnout_per_aboard_status_sex_{}_{}_{}",
            self.ids.tenant_id,
            self.ids.election_event_id,
            self.ids.election_id.clone().unwrap_or_default()
        )
    }

    #[instrument(err, skip_all)]
    async fn prepare_user_data(
        &self,
        hasura_transaction: &Transaction<'_>,
        keycloak_transaction: &Transaction<'_>,
    ) -> Result<Self::UserData> {
        let realm = get_event_realm(&self.ids.tenant_id, &self.ids.election_event_id);
        let date_printed = get_date_and_time();

        let election_event = get_election_event_by_id(
            &hasura_transaction,
            &self.ids.tenant_id,
            &self.ids.election_event_id,
        )
        .await
        .map_err(|e| anyhow::anyhow!("Error getting election event by id: {}", e))?;

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
                Some(false),
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

        let app_hash = get_app_hash();
        let app_version = get_app_version();
        let report_hash =
            get_report_hash(&ReportType::OV_TURNOUT_PER_ABOARD_STATUS_SEX.to_string())
                .await
                .unwrap_or("-".to_string());

        let mut elections_data = vec![];

        let mut overall_stats = VotersStatsData {
            total_male_landbased: 0,
            total_female_landbased: 0,
            total_landbased: 0,
            total_male_seafarer: 0,
            total_female_seafarer: 0,
            total_seafarer: 0,
            total_male: 0,
            total_female: 0,
            overall_total: 0,
        };

        let mut region_map: HashMap<String, RegionData> = HashMap::new();

        for election in elections {
            let election_general_data = extract_election_data(&election)
                .await
                .map_err(|err| anyhow!("Error extract election annotations {err}"))?;

            let election_dates = get_election_dates(&election, scheduled_events.clone())
                .map_err(|e| anyhow::anyhow!("Error getting election dates {e}"))?;

            let election_cloned = election.clone();
            let election_name = election_cloned.alias.unwrap_or(election_cloned.name);

            let election_areas = get_areas_by_election_id(
                &hasura_transaction,
                &self.ids.tenant_id,
                &self.ids.election_event_id,
                &election.id,
            )
            .await
            .map_err(|err| anyhow!("Error at get_areas_by_election_id: {err:?}"))?;

            elections_data.push(UserElectionData {
                election_dates,
                election_name,
            });

            let post_name = election_general_data.post.clone();
            let geographical_region = election_general_data.geographical_region.clone();

            set_up_voters_per_aboard_and_sex_by_area_post_region(
                &keycloak_transaction,
                &realm,
                post_name.clone(),
                geographical_region.clone(),
                false,
                election_areas,
                &mut overall_stats,
                &mut region_map,
            )
            .await
            .map_err(|err| {
                anyhow!("Error at set_up_voters_per_aboard_and_sex_by_area_post_region: {err:?}")
            })?;
        }

        let regions: Vec<RegionDataComputed> = region_map
            .into_iter()
            .map(|(_, region_data)| RegionDataComputed {
                geographical_region: region_data.geographical_region,
                stats: region_data.stats,
                posts: region_data.posts.into_values().collect(),
            })
            .collect();

        Ok(UserData {
            election_event_title: election_event
                .alias
                .clone()
                .unwrap_or(election_event.name.clone()),
            regions: regions,
            elections: elections_data,
            execution_annotations: ExecutionAnnotations {
                date_printed,
                report_hash,
                software_version: app_version.clone(),
                app_version,
                app_hash,
                executer_username: self.ids.executer_username.clone(),
                results_hash: None,
                user_timezone: None,
            },
            overall_total: overall_stats,
        })
    }

    /// Prepare system metadata for the report
    #[instrument(err, skip_all)]
    async fn prepare_system_data(
        &self,
        rendered_user_template: String,
    ) -> Result<Self::SystemData> {
        if pdf::doc_renderer_backend() == pdf::DocRendererBackend::InPlace {
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
        } else {
            // If we are rendering with a lambda, the QRCode lib is
            // already included in the lambda container image.
            Ok(SystemData {
                rendered_user_template,
                file_qrcode_lib: "/assets/qrcode.min.js".to_string(),
            })
        }
    }
}
