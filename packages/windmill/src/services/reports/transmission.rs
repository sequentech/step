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
use crate::postgres::tally_session::get_tally_sessions_by_election_event_id;
use crate::services::cast_votes::count_ballots_by_area_id;
use crate::services::consolidation::eml_generator::{
    find_miru_annotation, ValidateAnnotations, MIRU_AREA_CCS_SERVERS, MIRU_TALLY_SESSION_DATA,
};
use crate::services::temp_path::*;
use crate::types::miru_plugin::MiruTransmissionPackageData;
use crate::{postgres::election_event::get_election_event_by_id, services::s3::get_minio_url};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use deadpool_postgres::Transaction;
use rocket::form::validate::Contains;
use sequent_core::serialization::deserialize_with_path::deserialize_str;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::types::scheduled_event::generate_voting_period_dates;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};

/// Struct for Server Data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AnnotationServerData {
    pub tag: String,
    pub public_key_pem: String,
    pub address: String,
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServerData {
    pub server_code: String,
    pub transmitted: String,
    pub server_name: String,
    pub received: String,
    pub date_transmitted: String,
    pub date_received: String,
}

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
    ids: ReportIds,
}

impl TransmissionReport {
    pub fn new(ids: ReportIds) -> Self {
        TransmissionReport { ids }
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
        self.ids.tenant_id.clone()
    }

    fn get_election_event_id(&self) -> String {
        self.ids.election_event_id.clone()
    }

    fn get_initial_template_id(&self) -> Option<String> {
        self.ids.template_id.clone()
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

    #[instrument(err, skip(self, hasura_transaction, keycloak_transaction))]
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

        // Fetch election's voting periods
        let voting_period_dates = generate_voting_period_dates(
            scheduled_events,
            &self.ids.tenant_id,
            &self.ids.election_event_id,
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
        let election_annotations = election.get_annotations()?;

        let election_general_data = extract_election_data(&election)
            .await
            .map_err(|err| anyhow!("Error extract election annotations {err}"))?;

        let app_hash = get_app_hash();
        let app_version = get_app_version();
        let results_hash = get_results_hash(
            &hasura_transaction,
            &self.ids.tenant_id,
            &self.ids.election_event_id,
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
                &self.ids.tenant_id,
                &self.ids.election_event_id,
                &election_id,
                &area.id,
            )
            .await
            .map_err(|err| anyhow!("Error getting counted ballots: {err}"))?;

            let voters_turnout = generate_voters_turnout(&ballots_counted, &registered_voters)
                .await
                .map_err(|err| anyhow!("Error generate voters turnout {err}"))?;

            let tally_sessions = get_tally_sessions_by_election_event_id(
                &hasura_transaction,
                &self.ids.tenant_id,
                &self.ids.election_event_id,
                false,
            )
            .await
            .map_err(|err| anyhow!("Error getting the tally sessions: {err:?}"))?;

            let tally_session_data_parsed: Vec<MiruTransmissionPackageData> = if let Some(
                tally_session,
            ) =
                tally_sessions.iter().find(|session| {
                    session.election_event_id == self.ids.election_event_id
                        && session.area_ids.contains(&area.id)
                }) {
                let tally_annotation = tally_session
                    .get_annotations()
                    .map_err(|err| anyhow!("Error getting valid annotations: {err}"))?;

                tally_annotation
            } else {
                info!("Tally session not found for the given election event and area, setting default transmission status for area: {:?}", area.id);
                vec![]
            };

            let annotations = area.get_annotations()?.patch(&election_annotations);

            let servers = annotations
                .ccs_servers
                .into_iter()
                .map(|server| ServerData {
                    server_code: server.tag,
                    transmitted: if tally_session_data_parsed.iter().any(|data| {
                        data.documents.iter().any(|doc| {
                            doc.servers_sent_to
                                .iter()
                                .any(|server_sent| server_sent.name == server.name)
                        })
                    }) {
                        "Transmitted".to_string()
                    } else {
                        "Not Transmitted".to_string()
                    },
                    date_transmitted: tally_session_data_parsed
                        .iter()
                        .find_map(|data| {
                            let server_name = server.name.clone();
                            data.documents.iter().find_map(|doc| {
                                doc.servers_sent_to.iter().find_map(|server_sent| {
                                    if server_sent.name == server_name {
                                        Some(server_sent.sent_at.clone())
                                    } else {
                                        None
                                    }
                                })
                            })
                        })
                        .unwrap_or_else(|| "".to_string()),
                    received: if tally_session_data_parsed.iter().any(|data| {
                        data.documents.iter().any(|doc| {
                            doc.servers_sent_to
                                .iter()
                                .any(|server_sent| server_sent.name == server.name)
                        })
                    }) {
                        "Received".to_string()
                    } else {
                        "Not Received".to_string()
                    },
                    date_received: tally_session_data_parsed
                        .iter()
                        .find_map(|data| {
                            let server_name = server.name.clone();
                            data.documents.iter().find_map(|doc| {
                                doc.servers_sent_to.iter().find_map(|server_sent| {
                                    if server_sent.name == server_name {
                                        Some(server_sent.sent_at.clone())
                                    } else {
                                        None
                                    }
                                })
                            })
                        })
                        .unwrap_or_else(|| "".to_string()),
                    server_name: server.name,
                })
                .collect();

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
                servers,
                inspectors: area_general_data.inspectors.clone(),
            };

            areas.push(area_data);
        }

        Ok(UserData { areas })
    }

    #[instrument(err, skip(self, rendered_user_template))]
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
