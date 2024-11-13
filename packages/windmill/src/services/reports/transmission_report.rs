// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::report_variables::{
    extract_area_data, extract_election_data, extract_election_event_annotations,
    generate_voters_turnout, get_app_hash, get_app_version, get_date_and_time, get_report_hash,
    get_results_hash, get_total_number_of_registered_voters_for_area_id, InspectorData,
};
use super::template_renderer::*;
use crate::postgres::area::get_areas_by_election_id;
use crate::postgres::election::get_election_by_id;
use crate::postgres::reports::ReportType;
use crate::postgres::scheduled_event::find_scheduled_event_by_election_event_id;
use crate::services::cast_votes::count_ballots_by_area_id;
use crate::services::temp_path::*;
use crate::services::transmission::{
    get_transmission_data_from_tally_session, get_transmission_servers_data, ServerData,
};
use crate::{postgres::election_event::get_election_event_by_id, services::s3::get_minio_url};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use deadpool_postgres::Transaction;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::types::scheduled_event::generate_voting_period_dates;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserData {
    pub areas: Vec<UserDataArea>,
}

/// Struct for Transition Report Data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserDataArea {
    pub date_printed: String,
    pub election_date: String,
    pub election_title: String,
    pub voting_period_start: String,
    pub voting_period_end: String,
    pub geographical_region: String,
    pub post: String,
    pub country: String,
    pub voting_center: String,
    pub precinct_code: String,
    pub registered_voters: i64,
    pub ballots_counted: i64,
    pub voters_turnout: f64,
    pub report_hash: String,
    pub software_version: String,
    pub ovcs_version: String,
    pub system_hash: String,
    pub results_hash: String,
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
    pub tenant_id: String,
    pub election_event_id: String,
    pub election_id: Option<String>,
}

impl TransmissionReport {
    pub fn new(tenant_id: String, election_event_id: String, election_id: Option<String>) -> Self {
        TransmissionReport {
            tenant_id,
            election_event_id,
            election_id,
        }
    }
}

#[async_trait]
impl TemplateRenderer for TransmissionReport {
    type UserData = UserData;
    type SystemData = SystemData;

    fn get_report_type(&self) -> ReportType {
        ReportType::TRANSMISSION_REPORTS
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

    fn base_name(&self) -> String {
        "transmission_report".to_string()
    }

    fn prefix(&self) -> String {
        format!(
            "transmission_report_{}_{}_{}",
            self.tenant_id,
            self.election_event_id,
            self.election_id.clone().unwrap_or_default()
        )
    }

    #[instrument(err, skip(self, hasura_transaction, keycloak_transaction))]
    /// Prepare user data by fetching the relevant details
    async fn prepare_user_data(
        &self,
        hasura_transaction: &Transaction<'_>,
        keycloak_transaction: &Transaction<'_>,
    ) -> Result<Self::UserData> {
        let Some(election_id) = &self.election_id else {
            return Err(anyhow!("Empty election_id"));
        };

        let realm: String =
            get_event_realm(self.tenant_id.as_str(), self.election_event_id.as_str());
        // Fetch election event data
        let election_event =
            get_election_event_by_id(hasura_transaction, &self.tenant_id, &self.election_event_id)
                .await
                .with_context(|| "Error obtaining election event")?;

        let election_event_annotations = extract_election_event_annotations(&election_event)
            .await
            .map_err(|err| anyhow!("Error extract election event annotations {err}"))?;

        // Fetch areas associated with the election
        let election_areas = get_areas_by_election_id(
            &hasura_transaction,
            &self.tenant_id,
            &self.election_event_id,
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
            &self.tenant_id,
            &self.election_event_id,
        )
        .await?;

        // Fetch election's voting periods
        let voting_period_dates = generate_voting_period_dates(
            scheduled_events,
            &self.tenant_id,
            &self.election_event_id,
            Some(&election_id),
        )?;

        // extract start date from voting period
        let voting_period_start_date = voting_period_dates.start_date.unwrap_or_default();
        // extract end date from voting period
        let voting_period_end_date = voting_period_dates.end_date.unwrap_or_default();

        let election_date = &voting_period_start_date.to_string();

        let date_printed = get_date_and_time();
        let election_title = election_event.name.clone();

        // get election instace
        let election = match get_election_by_id(
            &hasura_transaction,
            &self.tenant_id,
            &self.election_event_id,
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
            &self.tenant_id,
            &self.election_event_id,
        )
        .await
        .unwrap_or("-".to_string());

        let report_hash = get_report_hash(&ReportType::TRANSMISSION_REPORTS.to_string())
            .await
            .unwrap_or("-".to_string());

        for area in election_areas.iter() {
            let country = area.clone().name.unwrap_or('-'.to_string());

            // get area instace's general data (post, area, etc...)
            let area_general_data =
                extract_area_data(&area, election_event_annotations.sbei_users.clone())
                    .await
                    .map_err(|err| anyhow!("Error extract area data {err}"))?;

            let registered_voters = get_total_number_of_registered_voters_for_area_id(
                &keycloak_transaction,
                &realm,
                &area.id,
            )
            .await
            .map_err(|err| anyhow!("Error counting registered voters: {err}"))?;
            let ballots_counted = count_ballots_by_area_id(
                &hasura_transaction,
                &self.tenant_id,
                &self.election_event_id,
                &election_id,
                &area.id,
            )
            .await
            .map_err(|err| anyhow!("Error getting counted ballots: {err}"))?;

            let voters_turnout = generate_voters_turnout(&ballots_counted, &registered_voters)
                .await
                .map_err(|err| anyhow!("Error generate voters turnout {err}"))?;

            let tally_session_data = get_transmission_data_from_tally_session(
                &hasura_transaction,
                &self.tenant_id,
                &self.election_event_id,
                &area.id,
            )
            .await
            .map_err(|err| anyhow!("Error get_transmission_data_from_tally_session: {err:?}"))?;

            let transmission_data = get_transmission_servers_data(&tally_session_data, &area)
                .await
                .map_err(|err| anyhow!("Error get_transmission_servers_data: {err:?}"))?;

            let area_data = UserDataArea {
                date_printed: date_printed.clone(),
                election_title: election_title.clone(),
                election_date: election_date.clone(),
                voting_period_start: voting_period_start_date.clone(),
                voting_period_end: voting_period_end_date.clone(),
                geographical_region: election_general_data.geographical_region.clone(),
                post: election_general_data.post.clone(),
                country: country,
                voting_center: election_general_data.voting_center.clone(),
                precinct_code: election_general_data.precinct_code.clone(),
                registered_voters,
                ballots_counted,
                voters_turnout,
                report_hash: report_hash.clone(),
                software_version: app_version.clone(),
                ovcs_version: app_version.clone(),
                system_hash: app_hash.clone(),
                results_hash: results_hash.clone(),
                servers: transmission_data.servers,
                inspectors: area_general_data.inspectors.clone(),
            };

            areas.push(area_data);
        }

        Ok(UserData { areas })
    }

    #[instrument]
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
