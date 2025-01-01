// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::report_variables::{
    extract_election_data, get_app_hash, get_app_version, get_date_and_time, get_report_hash,
    ExecutionAnnotations,
};
use super::template_renderer::*;
use super::voters::{get_voters_data, EnrollmentFilters, FilterListVoters, Voter};
use crate::postgres::area::get_areas_by_election_id;
use crate::postgres::election::get_election_by_id;
use crate::postgres::reports::ReportType;
use crate::postgres::scheduled_event::find_scheduled_event_by_election_event_id;
use crate::services::election_dates::get_election_dates;
use crate::services::s3::get_minio_url;
use crate::services::temp_path::{get_public_assets_path_env_var, PUBLIC_ASSETS_QRCODE_LIB};
use crate::types::application::ApplicationStatus;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use deadpool_postgres::Transaction;
use sequent_core::ballot::StringifiedPeriodDates;
use sequent_core::services::keycloak::get_event_realm;
use serde::{Deserialize, Serialize};
use tracing::instrument;

/// Struct for Pre-Enrolled User Data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserDataArea {
    pub election_title: String,
    pub election_dates: StringifiedPeriodDates,
    pub post: String,
    pub area_name: String,
    pub voters: Vec<Voter>,
    pub voted: i64,
    pub not_voted: i64,
    pub number_of_ovs_approved_by_system: i64,
    pub number_of_ovs_approved_by_sbei: i64,
    pub number_of_ovs_approved_by_ofov: i64,
    pub total: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserData {
    pub areas: Vec<UserDataArea>,
    pub execution_annotations: ExecutionAnnotations,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemData {
    pub rendered_user_template: String,
    pub file_qrcode_lib: String,
}

/// Implement the `TemplateRenderer` trait for PreEnrolledUserTemplate
#[derive(Debug)]
pub struct PreEnrolledVoterTemplate {
    ids: ReportOrigins,
}

impl PreEnrolledVoterTemplate {
    pub fn new(ids: ReportOrigins) -> Self {
        PreEnrolledVoterTemplate { ids }
    }
}

#[async_trait]
impl TemplateRenderer for PreEnrolledVoterTemplate {
    type UserData = UserData;
    type SystemData = SystemData;

    fn get_report_type(&self) -> ReportType {
        ReportType::LIST_OF_OV_WHO_PRE_ENROLLED_APPROVED
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

    fn base_name(&self) -> String {
        "ov_who_pre_enrolled".to_string()
    }

    fn prefix(&self) -> String {
        format!(
            "ov_who_pre_enrolled_{}_{}_{}",
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
        let Some(election_id) = &self.ids.election_id else {
            return Err(anyhow!("Empty election_id"));
        };

        let realm = get_event_realm(&self.ids.tenant_id, &self.ids.election_event_id);

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

        let date_printed = get_date_and_time();

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
        let report_hash =
            get_report_hash(&ReportType::LIST_OF_OV_WHO_PRE_ENROLLED_APPROVED.to_string())
                .await
                .unwrap_or("-".to_string());

        let mut areas: Vec<UserDataArea> = vec![];

        for area in election_areas.iter() {
            let enrollment_filters = EnrollmentFilters {
                status: ApplicationStatus::ACCEPTED,
                verification_type: None,
            };

            let voters_filters = FilterListVoters {
                enrolled: Some(enrollment_filters),
                has_voted: None,
                voters_sex: None,
                post: None,
                landbased_or_seafarer: None,
                verified: None,
            };

            let voters_data = get_voters_data(
                hasura_transaction,
                keycloak_transaction,
                &realm,
                &self.ids.tenant_id,
                &self.ids.election_event_id,
                &election_id,
                &area.id,
                true,
                voters_filters,
            )
            .await
            .map_err(|e| anyhow!("Error getting voters data: {}", e))?;

            let area_name = area.clone().name.unwrap_or("-".to_string());

            areas.push(UserDataArea {
                election_title: election.alias.clone().unwrap_or(election.name.clone()),
                election_dates: election_dates.clone(),
                post: election_general_data.post.clone(),
                area_name,
                voted: voters_data.total_voted.clone(),
                not_voted: voters_data.total_not_voted.clone(),
                voters: voters_data.voters.clone(),
                number_of_ovs_approved_by_system: 0, //TODO: fix mock data
                number_of_ovs_approved_by_sbei: 0,   //TODO: fix mock data
                number_of_ovs_approved_by_ofov: 0,   //TODO: fix mock data
                total: voters_data.total_voters.clone(),
            })
        }

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
