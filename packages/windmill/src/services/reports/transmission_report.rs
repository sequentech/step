// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::report_variables::{
    extract_area_data, extract_election_data, extract_election_event_annotations,
    generate_election_area_votes_data, get_app_hash, get_app_version, get_date_and_time,
    get_report_hash, get_results_hash, ExecutionAnnotations, InspectorData,
};
use super::template_renderer::*;
use crate::postgres::area::get_areas_by_election_id;
use crate::postgres::election::get_election_by_id;
use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::reports::ReportType;
use crate::postgres::scheduled_event::find_scheduled_event_by_election_event_id;
use crate::services::cast_votes::count_ballots_by_area_id;
use crate::services::consolidation::eml_generator::ValidateAnnotations;
use crate::services::election_dates::get_election_dates;
use crate::services::temp_path::PUBLIC_ASSETS_QRCODE_LIB;
use crate::services::transmission::{
    get_transmission_data_from_tally_session_by_area, get_transmission_servers_data, ServerData,
};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use deadpool_postgres::Transaction;
use sequent_core::ballot::StringifiedPeriodDates;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::services::pdf;
use sequent_core::services::s3::get_minio_url;
use sequent_core::types::scheduled_event::generate_voting_period_dates;
use sequent_core::util::temp_path::*;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserData {
    pub areas: Vec<UserDataArea>,
    pub election_dates: StringifiedPeriodDates,
    pub execution_annotations: ExecutionAnnotations,
}

/// Struct for Transition Report Data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserDataArea {
    pub election_title: String,
    pub geographical_region: String,
    pub post: String,
    pub country: String,
    pub voting_center: String,
    pub station_id: String,
    pub station_name: String,
    pub registered_voters: Option<i64>,
    pub ballots_counted: Option<i64>,
    pub voters_turnout: Option<f64>,
    pub servers: Vec<ServerData>,
    pub inspectors: Vec<InspectorData>,
}

/// Struct for System Data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemData {
    pub rendered_user_template: String,
    pub file_qrcode_lib: String,
}

#[derive(Debug)]
pub struct TransmissionReport {
    ids: ReportOrigins,
}

impl TransmissionReport {
    pub fn new(ids: ReportOrigins) -> Self {
        TransmissionReport { ids }
    }
}

#[async_trait]
impl TemplateRenderer for TransmissionReport {
    type UserData = UserData;
    type SystemData = SystemData;

    fn get_report_type(&self) -> ReportType {
        ReportType::TRANSMISSION_REPORT
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
        "transmission_report".to_string()
    }

    fn prefix(&self) -> String {
        format!(
            "transmission_report_{}_{}_{}",
            self.ids.tenant_id,
            self.ids.election_event_id,
            self.ids.election_id.clone().unwrap_or_default()
        )
    }

    #[instrument(err, skip_all)]
    /// Prepare user data by fetching the relevant details
    async fn prepare_user_data(
        &self,
        hasura_transaction: &Transaction<'_>,
        keycloak_transaction: &Transaction<'_>,
    ) -> Result<Self::UserData> {
        let Some(election_id) = &self.ids.election_id else {
            return Err(anyhow!("Empty election_id"));
        };

        let realm: String = get_event_realm(
            self.ids.tenant_id.as_str(),
            self.ids.election_event_id.as_str(),
        );
        // Fetch election event data
        let election_event = get_election_event_by_id(
            hasura_transaction,
            &self.ids.tenant_id,
            &self.ids.election_event_id,
        )
        .await
        .with_context(|| "Error obtaining election event")?;

        let election_event_annotations = extract_election_event_annotations(&election_event)
            .await
            .map_err(|err| anyhow!("Error extract election event annotations {err}"))?;

        // Fetch areas associated with the election
        let election_areas = get_areas_by_election_id(
            &hasura_transaction,
            &self.ids.tenant_id,
            &self.ids.election_event_id,
            &election_id,
        )
        .await
        .map_err(|err| anyhow!("Error at get_areas_by_election_id: {err:?}"))?;

        if election_areas.is_empty() {
            return Err(anyhow!("No areas found for the given election"));
        }

        let mut areas: Vec<UserDataArea> = Vec::new();

        // Fetch election event data
        let scheduled_events = find_scheduled_event_by_election_event_id(
            &hasura_transaction,
            &self.ids.tenant_id,
            &self.ids.election_event_id,
        )
        .await?;

        let date_printed = get_date_and_time();
        let election_title = election_event.name.clone();

        // get election instace
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

        let app_hash = get_app_hash();
        let app_version = get_app_version();
        let results_hash = get_results_hash(
            &hasura_transaction,
            &self.ids.tenant_id,
            &self.ids.election_event_id,
            &election_id,
        )
        .await
        .unwrap_or("-".to_string());

        let report_hash = get_report_hash(&ReportType::TRANSMISSION_REPORT.to_string())
            .await
            .unwrap_or("-".to_string());

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

        for area in election_areas.iter() {
            let country = area.clone().name.unwrap_or('-'.to_string());

            let votes_data = generate_election_area_votes_data(
                &hasura_transaction,
                &self.ids.tenant_id,
                &self.ids.election_event_id,
                election.id.as_str(),
                &area.id,
                None,
            )
            .await
            .map_err(|e| anyhow!(format!("Error generating election area votes data {e:?}")))?;

            // get area instace's general data (post, area, etc...)
            let area_general_data =
                extract_area_data(&area, election_event_annotations.sbei_users.clone())
                    .await
                    .map_err(|err| anyhow!("Error extract area data {err}"))?;

            let area_annotations = area.get_annotations_or_empty_values()?;

            let tally_session_data = get_transmission_data_from_tally_session_by_area(
                &hasura_transaction,
                &self.ids.tenant_id,
                &self.ids.election_event_id,
                &area.id,
                self.ids.tally_session_id.clone(),
            )
            .await
            .map_err(|err| {
                anyhow!("Error get_transmission_data_from_tally_session_by_area: {err:?}")
            })?;

            let transmission_data = get_transmission_servers_data(&tally_session_data, &area)
                .await
                .map_err(|err| anyhow!("Error get_transmission_servers_data: {err:?}"))?;

            let ballots_counted = count_ballots_by_area_id(
                &hasura_transaction,
                &self.ids.tenant_id,
                &self.ids.election_event_id,
                &election_id,
                &area.id,
            )
            .await
            .map_err(|err| anyhow!("Error getting counted ballots: {err}"))?;

            let area_data = UserDataArea {
                election_title: election_title.clone(),
                geographical_region: election_general_data.geographical_region.clone(),
                post: election_general_data.post.clone(),
                country: country,
                voting_center: election_general_data.voting_center.clone(),
                station_id: area_annotations.station_id.clone(),
                station_name: area_annotations.station_name.clone(),
                registered_voters: votes_data.registered_voters,
                ballots_counted: Some(ballots_counted),
                voters_turnout: votes_data.voters_turnout,
                servers: transmission_data.servers,
                inspectors: area_general_data.inspectors.clone(),
            };

            areas.push(area_data);
        }

        Ok(UserData {
            areas,
            election_dates,
            execution_annotations: ExecutionAnnotations {
                date_printed,
                report_hash,
                app_version: app_version.clone(),
                software_version: app_version.clone(),
                app_hash,
                executer_username: self.ids.executer_username.clone(),
                results_hash: Some(results_hash),
            },
        })
    }

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
