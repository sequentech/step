// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::report_variables::{
    extract_election_event_annotations, get_app_hash, get_app_version, get_date_and_time,
    get_report_hash, ExecutionAnnotations,
};
use super::voters::{
    count_applications_by_status_and_roles, count_voters_by_area_id, get_voters_data,
    EnrollmentFilters, FilterListVoters,
};
use super::{report_variables::extract_election_data, template_renderer::*};
use crate::postgres::application::count_applications;
use crate::postgres::area::get_areas_by_election_id;
use crate::postgres::election::{get_election_by_id, get_elections};
use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::reports::ReportType;
use crate::postgres::scheduled_event::find_scheduled_event_by_election_event_id;
use crate::services::election_dates::get_election_dates;
use crate::services::keycloak_events::count_keycloak_password_reset_event_by_area;
use crate::services::temp_path::*;
use crate::types::application::{ApplicationStatus, ApplicationType};
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
    pub regions: Vec<Region>,
    pub ofov_disapproved: i64,
    pub sbei_disapproved: i64,
    pub system_disapproved: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserElectionData {
    pub election_dates: StringifiedPeriodDates,
    pub election_title: String,
}
// Struct to hold system data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemData {
    pub rendered_user_template: String,
    pub file_qrcode_lib: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Stat {
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
    pub stats: Vec<Stat>,
}

#[derive(Debug)]
pub struct OVCSStatisticsTemplate {
    ids: ReportOrigins,
}

impl OVCSStatisticsTemplate {
    pub fn new(ids: ReportOrigins) -> Self {
        OVCSStatisticsTemplate { ids }
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
            self.ids.tenant_id,
            self.ids.election_event_id,
            self.ids.election_id.clone().unwrap_or_default()
        )
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
        let report_hash = get_report_hash(&ReportType::OVCS_STATISTICS.to_string())
            .await
            .unwrap_or("-".to_string());
        let mut elections_data = vec![];

        let mut region_map: HashMap<String, Vec<Stat>> = HashMap::new();

        for election in elections {
            let election_dates = get_election_dates(&election, scheduled_events.clone())
                .map_err(|e| anyhow::anyhow!("Error getting election dates {e}"))?;

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

            for area in election_areas.iter() {
                let area_name = area.clone().name.unwrap_or("-".to_string());

                let enrolled_filter = EnrollmentFilters {
                    status: ApplicationStatus::ACCEPTED,
                    verification_type: None,
                };

                let filtered_voters = FilterListVoters {
                    enrolled: Some(enrolled_filter),
                    has_voted: None,
                    voters_sex: None,
                    post: Some(election_general_data.post.clone()),
                    landbased_or_seafarer: None,
                    verified: None,
                };

                let (enrolled_voters_data, _next_cursor) = get_voters_data(
                    &hasura_transaction,
                    &keycloak_transaction,
                    &realm,
                    &self.ids.tenant_id,
                    &self.ids.election_event_id,
                    &election.id,
                    &area.id,
                    true,
                    filtered_voters,
                    None,
                    None,
                )
                .await
                .map_err(|err| anyhow!("Error get_voters_data {err}"))?;

                let total_not_pre_enrolled = count_voters_by_area_id(
                    &keycloak_transaction,
                    &realm,
                    &area.id,
                    Some(election_general_data.post.clone()),
                    Some(false),
                )
                .await
                .map_err(|err| {
                    anyhow!("Error at count_voters_by_area_id not pre enrolled {err}")
                })?;

                let total_voters = count_voters_by_area_id(
                    &keycloak_transaction,
                    &realm,
                    &area.id,
                    Some(election_general_data.post.clone()),
                    None,
                )
                .await
                .map_err(|err| anyhow!("Error at count_voters_by_area_id by post {err}"))?;

                let total_password_reset_events = count_keycloak_password_reset_event_by_area(
                    &keycloak_transaction,
                    &realm,
                    &area.id,
                )
                .await
                .map_err(|err| {
                    anyhow!("Error at count_keycloak_password_reset_event_by_area {err}")
                })?;

                let area_stat = Stat {
                    post: election_general_data.post.clone(),
                    country: area_name,
                    total: total_voters,
                    not_pre_enrolled: total_not_pre_enrolled,
                    pre_enrolled: enrolled_voters_data.total_voters,
                    pre_enrolled_not_voted: enrolled_voters_data.total_not_voted,
                    pre_enrolled_voted: enrolled_voters_data.total_voted,
                    voted: enrolled_voters_data.total_voted, //TODO: what the difference between pre_enrolled_voted and voted?
                    password_reset_request: total_password_reset_events,
                    remarks: "-".to_string(),
                };

                region_map
                    .entry(election_general_data.geographical_region.clone())
                    .or_insert_with(Vec::new)
                    .push(area_stat);
            }
            elections_data.push(UserElectionData {
                election_dates,
                election_title: election.alias.unwrap_or(election.name).clone(),
            });
        }

        let regions: Vec<Region> = region_map
            .into_iter()
            .map(|(name, stats)| Region { name, stats })
            .collect();

        let (total_disapproved, total_ofov_disapproved, total_sbei_disapproved) =
            count_applications_by_status_and_roles(
                &hasura_transaction,
                &self.ids.tenant_id,
                &self.ids.election_event_id,
                true,
                None,
            )
            .await
            .map_err(|err| anyhow!("Error at counting all disapproved applications: {err}"))?;

        Ok(UserData {
            election_event_title: election_event.alias.unwrap_or(election_event.name).clone(),
            execution_annotations: ExecutionAnnotations {
                date_printed,
                report_hash,
                app_version: app_version.clone(),
                software_version: app_version.clone(),
                app_hash,
                executer_username: self.ids.executer_username.clone(),
                results_hash: None,
                user_timezone: None,
            },
            elections: elections_data,
            regions,
            ofov_disapproved: total_ofov_disapproved,
            sbei_disapproved: total_sbei_disapproved,
            system_disapproved: total_disapproved,
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
