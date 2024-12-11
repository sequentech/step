// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::report_variables::{
    extract_election_data, get_app_hash, get_app_version, get_date_and_time, get_report_hash,
    process_elections,
};
use super::template_renderer::*;
use super::voters::{count_voters_by_their_sex, LANDBASED_VALUE, SEAFARER_VALUE};
use crate::postgres::area::get_areas_by_election_id;
use crate::postgres::election::{get_election_by_id, get_elections};
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
use std::collections::HashMap;
use tracing::instrument;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserElectionData {
    pub election_dates: StringifiedPeriodDates,
    pub election_title: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PostAreaData {
    pub area_name: String,
    pub stats: VotersStatsData,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PostData {
    pub post: String,
    pub areas: Vec<PostAreaData>,
    pub stats: VotersStatsData,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RegionDataComputed {
    pub geographical_region: String,
    pub stats: VotersStatsData,
    pub posts: Vec<PostData>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RegionData {
    pub geographical_region: String,
    pub stats: VotersStatsData,
    pub posts: HashMap<String, PostData>,
}

// Struct to hold user data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserData {
    pub execution_annotations: ExecutionAnnotations,
    pub elections: Vec<UserElectionData>,
    pub regions: Vec<RegionDataComputed>,
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
pub struct RegionUserData {
    pub geographical_region: String,
    pub stats: VotersStatsData,
    pub posts: HashMap<String, PostData>, // Changed to HashMap
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VotersStatsData {
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

impl VotersStatsData {
    pub fn sum(&mut self, other: &VotersStatsData) {
        self.total_male_landbased += other.total_male_landbased;
        self.total_female_landbased += other.total_female_landbased;
        self.total_landbased += other.total_landbased;
        self.total_male_seafarer += other.total_male_seafarer;
        self.total_female_seafarer += other.total_female_seafarer;
        self.total_seafarer += other.total_seafarer;
        self.total_male += other.total_male;
        self.total_female += other.total_female;
        self.overall_total += other.overall_total;
    }
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
        ReportType::OVERSEAS_VOTERS_TURNOUT_PER_ABOARD_STATUS_AND_SEX
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
        "ov_turnout_per_aboard_and_sex".to_string()
    }

    fn prefix(&self) -> String {
        format!(
            "ov_turnout_per_aboard_and_sex_{}_{}_{}",
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

        let elections_data = process_elections(elections, scheduled_events.clone())
            .await
            .map_err(|err| anyhow!("Error process_elections {err}"))?;

        let app_hash = get_app_hash();
        let app_version = get_app_version();
        let report_hash = get_report_hash(
            &ReportType::OVERSEAS_VOTERS_TURNOUT_PER_ABOARD_STATUS_AND_SEX.to_string(),
        )
        .await
        .unwrap_or("-".to_string());

        // let mut overall_total_male_landbased: i64 = 0;
        // let mut overall_total_female_landbased: i64 = 0;
        // let mut overall_total_landbased: i64 = 0;
        // let mut overall_total_male_seafarer: i64 = 0;
        // let mut overall_total_female_seafarer: i64 = 0;
        // let mut overall_total_seafarer: i64 = 0;
        // let mut overall_total_male: i64 = 0;
        // let mut overall_total_female: i64 = 0;
        // let mut overall_total: i64 = 0;

        let mut elections_data = vec![];
        let mut region_map: HashMap<String, RegionData> = HashMap::new();

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

            let election_title = election.name.clone();

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
                election_title,
            });

            for area in election_areas {
                let area_name = area.clone().name.unwrap_or("-".to_string());

                let area_stats = get_voters_per_aboard_and_sex_data_by_area(
                    &keycloak_transaction,
                    &realm,
                    &election.id,
                    &area.id,
                )
                .await
                .map_err(|err| {
                    anyhow!("Error get_voters_per_aboard_and_sex_data_by_area for area {err}")
                })?;

                let post_name = election_general_data.post.clone();
                let geographical_region = election_general_data.geographical_region.clone();

                // Insert or update the region in the map
                region_map
                    .entry(geographical_region.clone())
                    .and_modify(|region| {
                        // Update region stats
                        region.stats.sum(&area_stats);
                        overall_stats.sum(&area_stats);

                        // Insert or update the post in the region
                        region
                            .posts
                            .entry(post_name.clone())
                            .and_modify(|post| {
                                // Update post stats
                                post.stats.sum(&area_stats);

                                // Add area data to the post
                                post.areas.push(PostAreaData {
                                    area_name: area_name.clone(),
                                    stats: area_stats.clone(),
                                });
                            })
                            .or_insert_with(|| PostData {
                                post: post_name.clone(),
                                areas: vec![PostAreaData {
                                    area_name: area_name.clone(),
                                    stats: area_stats.clone(),
                                }],
                                stats: area_stats.clone(),
                            });
                    })
                    .or_insert_with(|| {
                        let mut posts = HashMap::new();
                        posts.insert(
                            post_name.clone(),
                            PostData {
                                post: post_name.clone(),
                                areas: vec![PostAreaData {
                                    area_name: area_name.clone(),
                                    stats: area_stats.clone(),
                                }],
                                stats: area_stats.clone(),
                            },
                        );

                        RegionData {
                            geographical_region: geographical_region.clone(),
                            stats: area_stats.clone(),
                            posts,
                        }
                    });
            }
        }

        let regions: Vec<RegionDataComputed> = region_map
            .into_iter()
            .map(|(_, region_data)| RegionDataComputed {
                geographical_region: region_data.geographical_region,
                stats: region_data.stats,
                posts: region_data.posts.into_values().collect(),
            })
            .collect();

        // let mut regions: Vec<RegionData> = vec![];
        // for region in elections_data.regions {
        //     let region_name = region.0.clone();
        //     let posts = region.1.clone();
        //     let region_data = set_up_region_voters_data(
        //         &keycloak_transaction,
        //         &realm,
        //         &region_name,
        //         posts.clone(),
        //         false,
        //     )
        //     .await
        //     .map_err(|err| anyhow!("Error set_up_region_voters_data {err}"))?;

        //     regions.push(region_data.clone());

        //     let region_overall_total = region_data.stats.clone();

        //     overall_total_male_landbased += region_overall_total.total_male_landbased;
        //     overall_total_female_landbased += region_overall_total.total_female_landbased;
        //     overall_total_landbased += region_overall_total.total_landbased;
        //     overall_total_male_seafarer += region_overall_total.total_male_seafarer;
        //     overall_total_female_seafarer += region_overall_total.total_female_seafarer;
        //     overall_total_seafarer += region_overall_total.total_seafarer;

        //     overall_total_male += region_overall_total.total_male;
        //     overall_total_female += region_overall_total.total_female;
        //     overall_total += region_overall_total.overall_total;
        // }

        Ok(UserData {
            regions: regions,
            elections: elections_data,
            execution_annotations: ExecutionAnnotations {
                date_printed,
                report_hash,
                software_version: app_version.clone(),
                app_version,
                app_hash,
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

async fn get_voters_per_aboard_and_sex_data_by_area(
    keycloak_transaction: &Transaction<'_>,
    realm: &str,
    area_id: &str,
    post: &str,
) -> Result<VotersStatsData> {
    let landbased = count_voters_by_their_sex(
        &keycloak_transaction,
        &realm,
        &post,
        Some(LANDBASED_VALUE),
        false,
        Some(&area_id.clone()),
    )
    .await
    .map_err(|err| anyhow!("Error count_voters_by_their_sex, landbase {err}"))?;
    let seafarer = count_voters_by_their_sex(
        &keycloak_transaction,
        &realm,
        &post,
        Some(SEAFARER_VALUE),
        false,
        Some(&area_id.clone()),
    )
    .await
    .map_err(|err| anyhow!("Error count_voters_by_their_sex, landbase {err}"))?;
    let general = count_voters_by_their_sex(
        &keycloak_transaction,
        &realm,
        &post,
        None,
        false,
        Some(&area_id.clone()),
    )
    .await
    .map_err(|err| anyhow!("Error count_voters_by_their_sex, landbase {err}"))?;

    Ok(VotersStatsData {
        total_male_landbased: landbased.total_male,
        total_female_landbased: landbased.total_female,
        total_landbased: landbased.overall_total,
        total_male_seafarer: seafarer.total_male,
        total_female_seafarer: seafarer.total_female,
        total_seafarer: seafarer.overall_total,
        total_male: general.total_male,
        total_female: general.total_female,
        overall_total: general.overall_total,
    })
}
