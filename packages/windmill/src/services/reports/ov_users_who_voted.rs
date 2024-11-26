// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::report_variables::{
    extract_election_data, get_app_hash, get_app_version, get_date_and_time, get_report_hash,
};
use super::template_renderer::*;
use super::voters::{get_voters_data, FilterListVoters, Voter};
use crate::postgres::area::get_areas_by_election_id;
use crate::postgres::election::get_election_by_id;
use crate::postgres::reports::ReportType;
use crate::postgres::scheduled_event::find_scheduled_event_by_election_event_id;
use crate::services::election_dates::get_election_dates;
use crate::services::s3::get_minio_url;
use crate::services::temp_path::{get_public_assets_path_env_var, PUBLIC_ASSETS_QRCODE_LIB};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use deadpool_postgres::Transaction;
use sequent_core::ballot::StringifiedPeriodDates;
use sequent_core::services::keycloak::get_event_realm;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};

/// Struct for User Data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserDataArea {
    pub date_printed: String,
    pub election_title: String,
    pub election_dates: StringifiedPeriodDates,
    pub post: String,
    pub area_name: String,
    pub voters: Vec<Voter>,
    pub voted: i64,
    pub not_voted: i64,
    pub voting_privilege_voted: i64,
    pub total: i64,
    pub report_hash: String,
    pub software_version: String,
    pub ovcs_version: String,
    pub system_hash: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserData {
    pub areas: Vec<UserDataArea>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemData {
    pub rendered_user_template: String,
    pub file_qrcode_lib: String,
}

#[derive(Debug)]
pub struct OVUsersWhoVotedTemplate {
    ids: ReportOrigins,
}

impl OVUsersWhoVotedTemplate {
    pub fn new(ids: ReportOrigins) -> Self {
        OVUsersWhoVotedTemplate { ids }
    }
}

#[async_trait]
impl TemplateRenderer for OVUsersWhoVotedTemplate {
    type UserData = UserData;
    type SystemData = SystemData;

    fn get_report_type(&self) -> ReportType {
        ReportType::OV_USERS
    }

    fn get_tenant_id(&self) -> String {
        self.ids.tenant_id.clone()
    }

    fn get_election_event_id(&self) -> String {
        self.ids.election_event_id.clone()
    }

    fn get_initial_template_id(&self) -> Option<String> {
        self.ids.template_id.clone()
    }

    fn get_report_origin(&self) -> ReportOriginatedFrom {
        self.ids.report_origin
    }

    fn get_election_id(&self) -> Option<String> {
        self.ids.election_id.clone()
    }

    fn base_name(&self) -> String {
        "ov_users_who_voted".to_string()
    }

    fn prefix(&self) -> String {
        format!(
            "ov_users_who_voted_{}_{}_{}",
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
        let date_printed = get_date_and_time();

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
        let report_hash = get_report_hash(&ReportType::OV_USERS_WHO_VOTED.to_string())
            .await
            .unwrap_or("-".to_string());

        let mut areas: Vec<UserDataArea> = vec![];

        for area in election_areas.iter() {
            let voters_filters = FilterListVoters {
                enrolled: None,
                has_voted: Some(true),
                voters_sex: None,
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
                date_printed: date_printed.clone(),
                election_title: election.name.clone(),
                election_dates: election_dates.clone(),
                post: election_general_data.post.clone(),
                area_name,
                voted: voters_data.total_voted.clone(),
                not_voted: voters_data.total_not_voted.clone(),
                voters: voters_data.voters.clone(),
                voting_privilege_voted: 0, //TODO: fix mock data
                total: voters_data.total_voters.clone(),
                report_hash: report_hash.clone(),
                ovcs_version: app_version.clone(),
                system_hash: app_hash.clone(),
                software_version: app_version.clone(),
            })
        }

        Ok(UserData { areas })
    }

    #[instrument(err, skip(self))]
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
