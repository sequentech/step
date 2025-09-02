// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::report_variables::{
    extract_area_data, extract_election_data, extract_election_event_annotations, get_app_hash,
    get_app_version, get_date_and_time, get_report_hash, ExecutionAnnotations, InspectorData,
};
use super::template_renderer::*;
use super::voters::{calc_percentage, get_voters_data, FilterListVoters, FEMALE_VALE, MALE_VALE};
use crate::postgres::area::get_areas_by_election_id;
use crate::postgres::election::get_election_by_id;
use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::reports::ReportType;
use crate::postgres::scheduled_event::find_scheduled_event_by_election_event_id;
use crate::services::election_dates::get_election_dates;
use crate::services::temp_path::*;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use chrono::offset::TimeZone;
use deadpool_postgres::Transaction;
use sequent_core::ballot::StringifiedPeriodDates;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::services::pdf;
use sequent_core::services::s3::get_minio_url;
use sequent_core::types::scheduled_event::generate_voting_period_dates;
use sequent_core::util::temp_path::*;
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
pub struct UserDataElection {
    pub election_dates: StringifiedPeriodDates,
    pub election_title: String,
    pub post: String,
    pub overall_total: UserDataStats,
    pub inspectors: Vec<InspectorData>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserDataStats {
    pub total_male_registered: i64,
    pub total_female_registered: i64,
    pub total_registered: i64,
    pub total_male_voted: i64,
    pub total_female_voted: i64,
    pub total_voted: i64,
    pub percentage_male: f64,
    pub percentage_female: f64,
    pub percentage_total: f64,
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

/// Main struct for generating Voters Report
#[derive(Debug)]
pub struct VotersTurnoutPercentageReport {
    ids: ReportOrigins,
}

impl VotersTurnoutPercentageReport {
    pub fn new(ids: ReportOrigins) -> Self {
        VotersTurnoutPercentageReport { ids }
    }
}

#[async_trait]
impl TemplateRenderer for VotersTurnoutPercentageReport {
    type UserData = UserData;
    type SystemData = SystemData;

    fn get_report_type(&self) -> ReportType {
        ReportType::VOTERS_TURNOUT_PERCENTAGE
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
        "voters_turnout_percentage".to_string()
    }

    fn prefix(&self) -> String {
        format!(
            "voters_turnout_percentage{}_{}_{}",
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

        let Some(election_id) = &self.ids.election_id else {
            return Err(anyhow!("Empty election_id"));
        };

        let election_event = get_election_event_by_id(
            &hasura_transaction,
            &self.ids.tenant_id,
            &self.ids.election_event_id,
        )
        .await
        .map_err(|e| anyhow::anyhow!("Error getting election event by id: {}", e))?;

        let election_event_annotations = extract_election_event_annotations(&election_event)
            .await
            .map_err(|err| anyhow!("Error extract election event annotations {err}"))?;

        let election = match get_election_by_id(
            &hasura_transaction,
            &self.ids.tenant_id,
            &self.ids.election_event_id,
            &election_id,
        )
        .await
        .with_context(|| "Error getting election by id")?
        {
            Some(election) => election,
            None => return Err(anyhow::anyhow!("Election not found")),
        };

        let election_general_data = extract_election_data(&election)
            .await
            .map_err(|err| anyhow!("Error extract election annotations {err}"))?;

        // Fetch election event data
        let scheduled_events = find_scheduled_event_by_election_event_id(
            &hasura_transaction,
            &self.ids.tenant_id,
            &self.ids.election_event_id,
        )
        .await
        .map_err(|e| {
            anyhow::anyhow!("Error getting scheduled events by election event_id: {}", e)
        })?;

        let election_dates = get_election_dates(&election, scheduled_events)
            .map_err(|e| anyhow::anyhow!("Error getting election dates {e}"))?;

        let election_areas = get_areas_by_election_id(
            &hasura_transaction,
            &self.ids.tenant_id,
            &self.ids.election_event_id,
            &election_id,
        )
        .await
        .map_err(|err| anyhow!("Error at get_areas_by_election_id: {err:?}"))?;

        let app_hash = get_app_hash();
        let app_version = get_app_version();
        let report_hash = get_report_hash(&ReportType::VOTERS_TURNOUT_PERCENTAGE.to_string())
            .await
            .unwrap_or("-".to_string());

        let mut areas: Vec<UserDataArea> = vec![];

        let mut overall_total_male_registered: i64 = 0;
        let mut overall_total_female_registered: i64 = 0;
        let mut overall_total_registered: i64 = 0;
        let mut overall_total_male_voted: i64 = 0;
        let mut overall_total_female_voted: i64 = 0;
        let mut overall_total_voted: i64 = 0;

        for area in election_areas.iter() {
            let area_general_data =
                extract_area_data(&area, election_event_annotations.sbei_users.clone())
                    .await
                    .map_err(|err| anyhow!("Error extract area data {err}"))?;
            let area_name = area.clone().name.unwrap_or("-".to_string());

            let mut filtered_voters = FilterListVoters {
                enrolled: None,
                has_voted: None,
                voters_sex: Some(FEMALE_VALE.to_string()),
                post: None,
                landbased_or_seafarer: None,
                verified: None,
            };

            let (female_voters_data, _next_cursor) = get_voters_data(
                &hasura_transaction,
                &keycloak_transaction,
                &realm,
                &self.ids.tenant_id,
                &self.ids.election_event_id,
                &election_id,
                &area.id,
                true,
                filtered_voters.clone(),
                None,
                None,
            )
            .await
            .map_err(|err| anyhow!("Error get_voters_data {err}"))?;

            filtered_voters.voters_sex = Some(MALE_VALE.to_string());

            let (male_voters_data, _next_cursor) = get_voters_data(
                &hasura_transaction,
                &keycloak_transaction,
                &realm,
                &self.ids.tenant_id,
                &self.ids.election_event_id,
                &election_id,
                &area.id,
                true,
                filtered_voters.clone(),
                None,
                None,
            )
            .await
            .map_err(|err| anyhow!("Error get_voters_data {err}"))?;

            filtered_voters.voters_sex = None;

            let (voters_data, _next_cursor) = get_voters_data(
                &hasura_transaction,
                &keycloak_transaction,
                &realm,
                &self.ids.tenant_id,
                &self.ids.election_event_id,
                &election_id,
                &area.id,
                true,
                filtered_voters.clone(),
                None,
                None,
            )
            .await
            .map_err(|err| anyhow!("Error get_voters_data {err}"))?;

            overall_total_male_registered += male_voters_data.total_voters;
            overall_total_female_registered += female_voters_data.total_voters;
            overall_total_registered += voters_data.total_voters;
            overall_total_male_voted = male_voters_data.total_voted;
            overall_total_female_voted += female_voters_data.total_voted;
            overall_total_voted += voters_data.total_voted;

            let percentage_male =
                calc_percentage(male_voters_data.total_voted, male_voters_data.total_voters);
            let percentage_female = calc_percentage(
                female_voters_data.total_voted,
                female_voters_data.total_voters,
            );
            let percentage_total =
                calc_percentage(voters_data.total_voted, voters_data.total_voters);

            areas.push(UserDataArea {
                area_name,
                stats: UserDataStats {
                    total_male_registered: male_voters_data.total_voters,
                    total_female_registered: female_voters_data.total_voters,
                    total_registered: voters_data.total_voters,
                    total_male_voted: male_voters_data.total_voted,
                    total_female_voted: female_voters_data.total_voted,
                    total_voted: voters_data.total_voted,
                    percentage_male: percentage_male,
                    percentage_female: percentage_female,
                    percentage_total: percentage_total,
                },
            })
        }

        let percentage_male =
            calc_percentage(overall_total_male_voted, overall_total_male_registered);
        let percentage_female =
            calc_percentage(overall_total_female_voted, overall_total_female_registered);
        let percentage_total = calc_percentage(overall_total_voted, overall_total_registered);

        Ok(UserData {
            areas,
            election: UserDataElection {
                election_dates,
                election_title: election.alias.unwrap_or(election.name).clone(),
                post: election_general_data.post,
                inspectors: vec![],
                overall_total: UserDataStats {
                    total_male_registered: overall_total_male_registered,
                    total_female_registered: overall_total_female_registered,
                    total_registered: overall_total_registered,
                    total_male_voted: overall_total_male_voted,
                    total_female_voted: overall_total_female_voted,
                    total_voted: overall_total_voted,
                    percentage_male,
                    percentage_female,
                    percentage_total,
                },
            },
            execution_annotations: ExecutionAnnotations {
                date_printed,
                report_hash,
                software_version: app_version.clone(),
                app_version,
                app_hash,
                executer_username: self.ids.executer_username.clone(),
                results_hash: None,
            },
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
