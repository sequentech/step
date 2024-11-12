// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::{
    report_variables::{
        extract_election_data, get_app_hash, get_app_version, get_date_and_time, get_report_hash,
    },
    template_renderer::*,
};
use crate::services::temp_path::*;
use crate::services::{
    s3::get_minio_url,
    transmission::{get_transmission_data_from_tally_session, get_transmission_servers_data},
};
use crate::{
    postgres::{
        area::get_areas_by_election_id,
        election::{get_election_by_id, get_elections},
        election_event::get_election_event_by_id,
        reports::ReportType,
        scheduled_event::find_scheduled_event_by_election_event_id,
    },
    services::{database::get_hasura_pool, election_dates::get_election_dates},
};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::{
    ballot::StringifiedPeriodDates, services::keycloak::get_event_realm,
    types::hasura::core::Election,
};
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Event {
    post: String,
    country: String,
    testing_date: String,
    initialization_date: String,
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

            let mut events: Vec<Event> = vec![];

            for area in election_areas.iter() {
                let area_name = area.clone().name.unwrap_or("-".to_string());

                let tally_session_data = get_transmission_data_from_tally_session(
                    &hasura_transaction,
                    &self.tenant_id,
                    &self.election_event_id,
                    &area.id,
                )
                .await
                .map_err(|err| {
                    anyhow!("Error get_transmission_data_from_tally_session: {err:?}")
                })?;

                let transmission_data = get_transmission_servers_data(&tally_session_data, &area)
                    .await
                    .map_err(|err| anyhow!("Error get_transmission_servers_data: {err:?}"))?;
                let transmission_status = format!(
                    "{} Transmitted, {} Not transmitted",
                    transmission_data.total_transmitted, transmission_data.total_not_transmitted
                );

                events.push(Event {
                    post: election_general_data.post.clone(),
                    country: area_name,
                    testing_date: "-".to_string(),
                    initialization_date: "".to_string(), //TODO: keys ceremony created
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
