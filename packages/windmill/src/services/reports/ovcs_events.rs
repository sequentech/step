// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::report_variables::{
    extract_election_data, get_app_hash, get_app_version, get_date_and_time, get_report_hash,
    ExecutionAnnotations,
};
use super::template_renderer::*;
use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::tally_session::get_tally_sessions_by_election_id;
use crate::postgres::{
    area::get_areas_by_election_id,
    election::{get_election_by_id, get_elections},
    reports::ReportType,
    scheduled_event::find_scheduled_event_by_election_event_id,
};
use crate::services::consolidation::eml_generator::ValidateAnnotations;
use crate::services::election_dates::get_election_dates;
use crate::services::election_event_status::get_election_status;
use crate::services::temp_path::*;
use crate::services::transmission::{
    get_transmission_data_from_tally_session_by_area, get_transmission_servers_data,
};
use crate::{postgres::keys_ceremony::get_keys_ceremony_by_id, services::temp_path::*};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use deadpool_postgres::Transaction;
use sequent_core::services::pdf;
use sequent_core::types::ceremonies::TallyType;
use sequent_core::util::temp_path::get_public_assets_path_env_var;
use sequent_core::{
    ballot::StringifiedPeriodDates, services::s3::get_minio_url, types::hasura::core::Election,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::instrument;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Event {
    post: String,
    country: String,
    testing_date: String,
    initialization_date: Option<String>,
    opening_date: Option<String>,
    closing_date: Option<String>,
    transmission_date: String,
    transmission_status: String,
    remarks: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Region {
    name: String,
    events: Vec<Event>,
}
/// Struct for OVCSEvents Data

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserData {
    pub execution_annotations: ExecutionAnnotations,
    pub election_event_title: String,
    pub elections: Vec<UserElectionData>,
    pub ovcs_downtime: Option<i64>,
    pub regions: Vec<Region>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserElectionData {
    pub election_dates: StringifiedPeriodDates,
    pub election_name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemData {
    pub rendered_user_template: String,
    pub file_qrcode_lib: String,
}

#[derive(Debug)]
pub struct OVCSEventsTemplate {
    ids: ReportOrigins,
}

impl OVCSEventsTemplate {
    pub fn new(ids: ReportOrigins) -> Self {
        OVCSEventsTemplate { ids }
    }
}

#[async_trait]
impl TemplateRenderer for OVCSEventsTemplate {
    type UserData = UserData;
    type SystemData = SystemData;

    fn get_report_type(&self) -> ReportType {
        ReportType::OVCS_EVENTS
    }

    fn base_name(&self) -> String {
        "ovcs_events".to_string()
    }

    fn prefix(&self) -> String {
        format!("ovcs_events_{}", self.ids.election_event_id)
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
        _keycloak_transaction: &Transaction<'_>,
    ) -> Result<Self::UserData> {
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
                Some(false),
            )
            .await
            .map_err(|e| anyhow::anyhow!("Error in get_elections: {}", e))?,
        };

        // Fetch election event data
        let election_event = get_election_event_by_id(
            hasura_transaction,
            &self.ids.tenant_id,
            &self.ids.election_event_id,
        )
        .await
        .with_context(|| "Error obtaining election event")?;

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
        let report_hash = get_report_hash(&ReportType::OVCS_EVENTS.to_string())
            .await
            .unwrap_or("-".to_string());
        let mut elections_data = vec![];
        let mut region_map: HashMap<String, Vec<Event>> = HashMap::new();

        for election in elections {
            let election_dates = get_election_dates(&election, scheduled_events.clone())
                .map_err(|e| anyhow::anyhow!("Error getting election dates {e}"))?;
            let election_cloned = election.clone();
            let election_title = election_cloned.alias.unwrap_or(election_cloned.name);

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

            // Get OVCS status
            let status = get_election_status(election.status.clone()).unwrap_or_default();

            let tally_sessions = get_tally_sessions_by_election_id(
                hasura_transaction,
                &self.ids.tenant_id,
                &self.ids.election_event_id,
                &election.id,
            )
            .await
            .map_err(|e| anyhow!("Error getting tally sessions by election id: {e}"))?;

            let initialization_date = tally_sessions
                .into_iter()
                .filter(|tally: &sequent_core::types::hasura::core::TallySession| {
                    tally.tally_type == Some(TallyType::INITIALIZATION_REPORT.to_string())
                })
                .filter_map(|tally| tally.created_at)
                .max()
                .map(|dt| dt.to_rfc3339().to_string());

            let opening_date = status
                .voting_period_dates
                .first_started_at
                .map(|dt| dt.to_rfc3339().to_string());
            let closing_date = status
                .voting_period_dates
                .last_stopped_at
                .map(|dt| dt.to_rfc3339().to_string());

            for area in election_areas.iter() {
                let area_name = area.clone().name.unwrap_or("-".to_string());

                let tally_session_data = get_transmission_data_from_tally_session_by_area(
                    &hasura_transaction,
                    &self.ids.tenant_id,
                    &self.ids.election_event_id,
                    &area.id,
                    None,
                )
                .await
                .map_err(|err| {
                    anyhow!("Error get_transmission_data_from_tally_session_by_area: {err:?}")
                })?;

                let transmission_data = get_transmission_servers_data(&tally_session_data, &area)
                    .await
                    .map_err(|err| anyhow!("Error get_transmission_servers_data: {err:?}"))?;

                let transmission_status = format!(
                    "{} Transmitted, {} Not Transmitted",
                    transmission_data.total_transmitted, transmission_data.total_not_transmitted
                );

                let event = Event {
                    post: election_general_data.post.clone(),
                    country: area_name,
                    testing_date: "-".to_string(),
                    initialization_date: initialization_date.clone(),
                    opening_date: opening_date.clone(),
                    closing_date: closing_date.clone(),
                    transmission_date: transmission_data.last_date_transmitted,
                    transmission_status,
                    remarks: None,
                };

                region_map
                    .entry(election_general_data.geographical_region.clone())
                    .or_insert_with(Vec::new)
                    .push(event);
            }

            elections_data.push(UserElectionData {
                election_dates,
                election_name: election_title,
            });
        }

        let regions: Vec<Region> = region_map
            .into_iter()
            .map(|(name, events)| Region { name, events })
            .collect();

        Ok(UserData {
            election_event_title: election_event
                .alias
                .clone()
                .unwrap_or(election_event.name.clone()),
            execution_annotations: ExecutionAnnotations {
                date_printed,
                report_hash,
                app_version: app_version.clone(),
                software_version: app_version.clone(),
                app_hash,
                executer_username: self.ids.executer_username.clone(),
                results_hash: None,
                user_timezone: self.ids.user_timezone.clone(),
            },
            elections: elections_data,
            ovcs_downtime: None,
            regions,
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
