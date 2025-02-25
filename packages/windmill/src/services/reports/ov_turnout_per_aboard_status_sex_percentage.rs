// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::report_variables::{
    extract_election_data, get_app_hash, get_app_version, get_date_and_time, get_report_hash,
    ExecutionAnnotations,
};
use super::template_renderer::*;
use super::voters::{
    calc_percentage, get_voters_data, FilterListVoters, FEMALE_VALE, LANDBASED_VALUE, MALE_VALE,
    SEAFARER_VALUE,
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

// Struct to hold user data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserData {
    pub election_event_title: String,
    pub execution_annotations: ExecutionAnnotations,
    pub elections: Vec<UserElectionData>,
    pub regions: Vec<RegionData>,
    pub overall_total: VotersStatsData,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserElectionData {
    pub election_dates: StringifiedPeriodDates,
    pub election_name: String,
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

impl VotersStatsData {
    pub fn sum(&mut self, other: &VotersStatsData) {
        self.total_male_landbased += other.total_male_landbased;
        self.total_female_landbased += other.total_female_landbased;
        self.total_landbased += other.total_landbased;
        self.total_voted_male_landbased += other.total_voted_male_landbased;
        self.total_voted_female_landbased += other.total_voted_female_landbased;
        self.total_voted_landbased += other.total_voted_landbased;
        self.total_male_seafarer += other.total_male_seafarer;
        self.total_female_seafarer += other.total_female_seafarer;
        self.total_seafarer += other.total_seafarer;
        self.total_voted_male_seafarer += other.total_voted_male_seafarer;
        self.total_voted_female_seafarer += other.total_voted_female_seafarer;
        self.total_voted_seafarer += other.total_voted_seafarer;

        self.percentage_male_landbased =
            calc_percentage(self.total_voted_male_landbased, self.total_male_landbased);
        self.percentage_female_landbased = calc_percentage(
            self.total_voted_female_landbased,
            self.total_female_landbased,
        );
        self.percentage_landbased =
            calc_percentage(self.total_voted_landbased, self.total_landbased);

        self.percentage_male_seafarer =
            calc_percentage(self.total_voted_male_seafarer, self.total_male_seafarer);
        self.percentage_female_seafarer =
            calc_percentage(self.total_voted_female_seafarer, self.total_female_seafarer);
        self.percentage_seafarer = calc_percentage(self.total_voted_seafarer, self.total_seafarer);

        self.percentage_male = calc_percentage(
            self.total_voted_male_landbased + self.total_voted_male_seafarer,
            self.total_male_landbased + self.total_male_seafarer,
        );
        self.percentage_female = calc_percentage(
            self.total_voted_female_landbased + self.total_voted_female_seafarer,
            self.total_female_landbased + self.total_female_seafarer,
        );
        self.percentage_total = calc_percentage(
            self.total_voted_landbased + self.total_voted_seafarer,
            self.total_landbased + self.total_seafarer,
        );
    }
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
        ReportType::OV_TURNOUT_PER_ABOARD_STATUS_SEX_PERCENTAGE
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
        "ov_turnout_per_aboard_status_sex_percentage".to_string()
    }

    fn prefix(&self) -> String {
        format!(
            "ov_turnout_per_aboard_status_sex_percentage_{}_{}_{}",
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
            get_report_hash(&ReportType::OV_TURNOUT_PER_ABOARD_STATUS_SEX_PERCENTAGE.to_string())
                .await
                .unwrap_or("-".to_string());

        let mut elections_data = vec![];
        let mut region_map: HashMap<String, RegionData> = HashMap::new();

        let mut overall_stats = VotersStatsData {
            total_male_landbased: 0,
            total_female_landbased: 0,
            total_landbased: 0,
            total_voted_male_landbased: 0,
            total_voted_female_landbased: 0,
            total_voted_landbased: 0,
            percentage_male_landbased: 0.0,
            percentage_female_landbased: 0.0,
            percentage_landbased: 0.0,
            total_male_seafarer: 0,
            total_female_seafarer: 0,
            total_seafarer: 0,
            total_voted_male_seafarer: 0,
            total_voted_female_seafarer: 0,
            total_voted_seafarer: 0,
            percentage_male_seafarer: 0.0,
            percentage_female_seafarer: 0.0,
            percentage_seafarer: 0.0,
            percentage_male: 0.0,
            percentage_female: 0.0,
            percentage_total: 0.0,
        };

        for election in elections {
            let election_dates = get_election_dates(&election, scheduled_events.clone())
                .map_err(|e| anyhow::anyhow!("Error getting election dates {e}"))?;

            let election_cloned = election.clone();
            let election_name = election_cloned.alias.unwrap_or(election_cloned.name);

            let election_general_data = extract_election_data(&election)
                .await
                .map_err(|err| anyhow!("Error extract election annotations {err}"))?;

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

            for area in election_areas {
                let area_name = area.clone().name.unwrap_or("-".to_string());

                let area_stats = get_voters_per_aboard_and_sex_data_by_area(
                    &hasura_transaction,
                    &keycloak_transaction,
                    &realm,
                    &self.ids.tenant_id,
                    &self.ids.election_event_id,
                    &election.id,
                    &area.id,
                    &election_general_data.post,
                )
                .await
                .map_err(|err| {
                    anyhow!("Error get_voters_per_aboard_and_sex_data_by_area for area {err}")
                })?;

                let post_area_data = PostData {
                    post: election_general_data.post.clone(),
                    area_name,
                    stats: area_stats.clone(),
                };

                region_map
                    .entry(election_general_data.geographical_region.clone())
                    .and_modify(|region| {
                        region.stats.sum(&area_stats);

                        region.posts.push(post_area_data.clone());
                    })
                    .or_insert_with(|| RegionData {
                        geographical_region: election_general_data.geographical_region.clone(),
                        stats: area_stats.clone(),
                        posts: vec![post_area_data],
                    });

                overall_stats.sum(&area_stats);
            }
        }

        let regions: Vec<RegionData> = region_map.into_values().collect();

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

#[instrument(err, skip_all)]
async fn get_voters_per_aboard_and_sex_data_by_area(
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    realm: &str,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
    area_id: &str,
    post: &str,
) -> Result<VotersStatsData> {
    let mut voters_filters = FilterListVoters {
        enrolled: None,
        has_voted: None,
        voters_sex: None,
        post: Some(post.to_string()),
        landbased_or_seafarer: Some(LANDBASED_VALUE.to_string()),
        verified: Some(true),
    };

    let (landbased_voters_data, _next_cursor) = get_voters_data(
        hasura_transaction,
        keycloak_transaction,
        &realm,
        &tenant_id,
        &election_event_id,
        &election_id,
        &area_id,
        true,
        voters_filters.clone(),
        None,
        None,
    )
    .await
    .map_err(|e| anyhow!("Error getting voters data: {}", e))?;

    voters_filters.voters_sex = Some(MALE_VALE.to_string());

    let (male_landbased_voters_data, _next_cursor) = get_voters_data(
        hasura_transaction,
        keycloak_transaction,
        &realm,
        &tenant_id,
        &election_event_id,
        &election_id,
        &area_id,
        true,
        voters_filters.clone(),
        None,
        None,
    )
    .await
    .map_err(|e| anyhow!("Error getting voters data: {}", e))?;

    voters_filters.voters_sex = Some(FEMALE_VALE.to_string());

    let (female_landbased_voters_data, _next_cursor) = get_voters_data(
        hasura_transaction,
        keycloak_transaction,
        &realm,
        &tenant_id,
        &election_event_id,
        &election_id,
        &area_id,
        true,
        voters_filters.clone(),
        None,
        None,
    )
    .await
    .map_err(|e| anyhow!("Error getting voters data: {}", e))?;

    let percentage_male_landbased = calc_percentage(
        male_landbased_voters_data.total_voted.clone(),
        male_landbased_voters_data.total_voters.clone(),
    );
    let percentage_female_landbased = calc_percentage(
        female_landbased_voters_data.total_voted.clone(),
        female_landbased_voters_data.total_voters.clone(),
    );
    let percentage_landbased = calc_percentage(
        landbased_voters_data.total_voted.clone(),
        landbased_voters_data.total_voters.clone(),
    );

    voters_filters.landbased_or_seafarer = Some(SEAFARER_VALUE.to_string());

    let (female_seafarer_voters_data, _next_cursor) = get_voters_data(
        hasura_transaction,
        keycloak_transaction,
        &realm,
        &tenant_id,
        &election_event_id,
        &election_id,
        &area_id,
        true,
        voters_filters.clone(),
        None,
        None,
    )
    .await
    .map_err(|e| anyhow!("Error getting voters data: {}", e))?;

    voters_filters.voters_sex = Some(MALE_VALE.to_string());

    let (male_seafarer_voters_data, _next_cursor) = get_voters_data(
        hasura_transaction,
        keycloak_transaction,
        &realm,
        &tenant_id,
        &election_event_id,
        &election_id,
        &area_id,
        true,
        voters_filters.clone(),
        None,
        None,
    )
    .await
    .map_err(|e| anyhow!("Error getting voters data: {}", e))?;

    voters_filters.voters_sex = None;

    let (seafarer_voters_data, _next_cursor) = get_voters_data(
        hasura_transaction,
        keycloak_transaction,
        &realm,
        &tenant_id,
        &election_event_id,
        &election_id,
        &area_id,
        true,
        voters_filters.clone(),
        None,
        None,
    )
    .await
    .map_err(|e| anyhow!("Error getting voters data: {}", e))?;

    let percentage_male_seafarer = calc_percentage(
        male_seafarer_voters_data.total_voted.clone(),
        male_seafarer_voters_data.total_voters.clone(),
    );
    let percentage_female_seafarer = calc_percentage(
        female_seafarer_voters_data.total_voted.clone(),
        female_seafarer_voters_data.total_voters.clone(),
    );
    let percentage_seafarer = calc_percentage(
        seafarer_voters_data.total_voted.clone(),
        seafarer_voters_data.total_voters.clone(),
    );

    let total_male = male_seafarer_voters_data.total_voters.clone()
        + male_landbased_voters_data.total_voters.clone();
    let total_voted_male = male_seafarer_voters_data.total_voted.clone()
        + male_landbased_voters_data.total_voted.clone();
    let percentage_male = calc_percentage(total_voted_male.clone(), total_male.clone());

    let total_female = female_seafarer_voters_data.total_voters.clone()
        + female_landbased_voters_data.total_voters.clone();
    let total_voted_female = female_seafarer_voters_data.total_voted.clone()
        + female_landbased_voters_data.total_voted.clone();
    let percentage_female = calc_percentage(total_voted_female.clone(), total_female.clone());

    let total_voters = total_male + total_female;
    let total_voted = total_voted_male + total_voted_female;
    let percentage_total = calc_percentage(total_voted, total_voters);

    Ok(VotersStatsData {
        total_male_landbased: male_landbased_voters_data.total_voters,
        total_female_landbased: female_landbased_voters_data.total_voters,
        total_landbased: landbased_voters_data.total_voters,
        total_voted_male_landbased: male_landbased_voters_data.total_voted,
        total_voted_female_landbased: female_landbased_voters_data.total_voted,
        total_voted_landbased: landbased_voters_data.total_voted,
        percentage_male_landbased: percentage_male_landbased,
        percentage_female_landbased: percentage_female_landbased,
        percentage_landbased: percentage_landbased,
        total_male_seafarer: male_seafarer_voters_data.total_voters,
        total_female_seafarer: female_seafarer_voters_data.total_voters,
        total_seafarer: seafarer_voters_data.total_voters,
        total_voted_male_seafarer: male_seafarer_voters_data.total_voted,
        total_voted_female_seafarer: female_seafarer_voters_data.total_voted,
        total_voted_seafarer: seafarer_voters_data.total_voted,
        percentage_male_seafarer: percentage_male_seafarer,
        percentage_female_seafarer: percentage_female_seafarer,
        percentage_seafarer: percentage_seafarer,
        percentage_male: percentage_male,
        percentage_female: percentage_female,
        percentage_total: percentage_total,
    })
}
