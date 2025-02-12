// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::report_variables::{
    extract_election_data, get_app_hash, get_app_version, get_date_and_time, get_report_hash,
    ExecutionAnnotations,
};
use super::template_renderer::*;
use super::voters::{get_not_enrolled_voters_by_area_id, Voter};
use crate::postgres::area::get_areas_by_election_id;
use crate::postgres::election::get_election_by_id;
use crate::postgres::election::get_elections;
use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::reports::ReportType;
use crate::postgres::scheduled_event::find_scheduled_event_by_election_event_id;
use crate::services::election_dates::get_election_dates;
use crate::services::temp_path::PUBLIC_ASSETS_QRCODE_LIB;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use deadpool_postgres::Transaction;
use sequent_core::ballot::StringifiedPeriodDates;
use sequent_core::serialization::deserialize_with_path::deserialize_value;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::services::pdf;
use sequent_core::services::s3::get_minio_url;
use sequent_core::signatures::temp_path::*;
use sequent_core::types::hasura::core::Election;
use sequent_core::{ballot::ElectionStatus, types::templates::EmailConfig};
use serde::{Deserialize, Serialize};
use serde_json::value::Value;
use tracing::instrument;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserDataArea {
    pub election_title: String,
    pub election_dates: StringifiedPeriodDates,
    pub post: String,
    pub area_name: String,
    pub voters: Vec<Voter>, // Voter list field
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserData {
    pub execution_annotations: ExecutionAnnotations,
    pub areas: Vec<UserDataArea>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemData {
    pub rendered_user_template: String,
    pub file_qrcode_lib: String,
}

#[derive(Debug)]
pub struct NotPreEnrolledListTemplate {
    ids: ReportOrigins,
}

impl NotPreEnrolledListTemplate {
    pub fn new(ids: ReportOrigins) -> Self {
        NotPreEnrolledListTemplate { ids }
    }
}

#[async_trait]
impl TemplateRenderer for NotPreEnrolledListTemplate {
    type UserData = UserData;
    type SystemData = SystemData;

    fn get_report_type(&self) -> ReportType {
        ReportType::OV_NOT_YET_PRE_ENROLLED_LIST
    }

    fn base_name(&self) -> String {
        "ov_not_yet_pre_enrolled_list".to_string()
    }

    fn prefix(&self) -> String {
        format!(
            "ov_not_yet_pre_enrolled_list_{}",
            self.ids.election_event_id
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

    #[instrument(err, skip(self, hasura_transaction, keycloak_transaction))]
    async fn prepare_user_data(
        &self,
        hasura_transaction: &Transaction<'_>,
        keycloak_transaction: &Transaction<'_>,
    ) -> Result<Self::UserData> {
        let realm = get_event_realm(
            self.ids.tenant_id.as_str(),
            self.ids.election_event_id.as_str(),
        );
        let date_printed: String = get_date_and_time();

        // Fetch election event data
        let election_event = get_election_event_by_id(
            &hasura_transaction,
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

        let mut areas: Vec<UserDataArea> = vec![];
        for election in elections {
            let election_general_data = extract_election_data(&election)
                .await
                .map_err(|err| anyhow!("Error extract election annotations {err}"))?;

            let election_id = election.id.clone();
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

            let election_dates = get_election_dates(&election, scheduled_events.clone())
                .map_err(|e| anyhow::anyhow!("Error getting election dates {e}"))?;

            // Loop over each area and collect data
            for area in election_areas.iter() {
                let area_name = area.clone().name.unwrap_or('-'.to_string());

                let voters =
                    get_not_enrolled_voters_by_area_id(&keycloak_transaction, &realm, &area.id)
                        .await
                        .map_err(|e| anyhow::anyhow!("Error getting election dates {e}"))?;

                let area_data = UserDataArea {
                    election_title: election.alias.clone().unwrap_or(election.name.clone()),
                    election_dates: election_dates.clone(),
                    area_name,
                    post: election_general_data.post.clone(),
                    voters,
                };

                areas.push(area_data);
            }
        }

        let app_hash = get_app_hash();
        let app_version = get_app_version();
        let report_hash = get_report_hash(&ReportType::STATUS.to_string())
            .await
            .unwrap_or("-".to_string());

        // Return the UserData with areas populated
        Ok(UserData {
            areas,
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

    #[instrument]
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

pub fn get_election_status(status_json_opt: Option<Value>) -> Option<ElectionStatus> {
    status_json_opt.and_then(|status_json| deserialize_value(status_json).ok())
}
