// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::report_variables::{
    extract_election_data, get_app_hash, get_app_version, get_date_and_time, get_report_hash,
};
use super::template_renderer::*;
use crate::postgres::{
    area::get_areas_by_election_id,
    election::{get_election_by_id, get_elections},
    reports::ReportType,
    scheduled_event::find_scheduled_event_by_election_event_id,
};
use crate::services::consolidation::eml_generator::ValidateAnnotations;
use crate::services::election_dates::get_election_dates;
use crate::services::s3::get_minio_url;
use crate::services::transmission::{
    get_transmission_data_from_tally_session_by_area, get_transmission_servers_data,
};
use crate::{postgres::keys_ceremony::get_keys_ceremony_by_id, services::temp_path::*};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use deadpool_postgres::Transaction;
use sequent_core::{ballot::StringifiedPeriodDates, types::hasura::core::Election};
use serde::{Deserialize, Serialize};
use tracing::instrument;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Event {
    post: String,
    country: String,
    testing_date: String,
    initialization_date: Option<String>,
    opening_date: String,
    closing_date: String,
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
    pub ovcs_downtime: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemData {
    pub rendered_user_template: String,
    pub file_qrcode_lib: String,
}

#[derive(Debug)]
pub struct OVCSEventsTemplate {
    pub tenant_id: String,
    pub election_event_id: String,
    pub election_id: Option<String>,
}

impl OVCSEventsTemplate {
    pub fn new(tenant_id: String, election_event_id: String, election_id: Option<String>) -> Self {
        OVCSEventsTemplate {
            tenant_id,
            election_event_id,
            election_id,
        }
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
        format!("ovcs_events_{}", self.election_event_id)
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
        let date_printed = get_date_and_time();

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
                None,
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
        let report_hash = get_report_hash(&ReportType::OVCS_EVENTS.to_string())
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

            let mut events: Vec<Event> = vec![];

            for area in election_areas.iter() {
                let area_name = area.clone().name.unwrap_or("-".to_string());

                let tally_session_data = get_transmission_data_from_tally_session_by_area(
                    &hasura_transaction,
                    &self.tenant_id,
                    &self.election_event_id,
                    &area.id,
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

                let initialization_date: Option<String> = match &election.keys_ceremony_id {
                    Some(keys_ceremony_id) => {
                        let keys_ceremony = get_keys_ceremony_by_id(
                            &hasura_transaction,
                            &self.tenant_id,
                            &self.election_event_id,
                            &keys_ceremony_id,
                        )
                        .await
                        .map_err(|err| anyhow!("Error get_transmission_servers_data: {err:?}"))?;
                        let date = keys_ceremony
                            .created_at
                            .map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string())
                            .unwrap_or_default();
                        Some(date)
                    }
                    None => None,
                };

                events.push(Event {
                    post: election_general_data.post.clone(),
                    country: area_name,
                    testing_date: "-".to_string(),
                    initialization_date,
                    opening_date: election_dates.first_started_at.clone().unwrap_or_default(),
                    closing_date: election_dates.last_stopped_at.clone().unwrap_or_default(),
                    transmission_date: transmission_data.last_date_transmitted,
                    transmission_status,
                    remarks: None,
                });
            }
            regions.push(Region {
                name: election_general_data.geographical_region.clone(),
                events,
            });
            elections_data.push(UserElectionData {
                election_dates,
                election_title,
                regions,
                ovcs_downtime: 0,
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
