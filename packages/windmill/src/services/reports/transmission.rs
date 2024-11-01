// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::report_variables::{
    extract_election_data, generate_voters_turnout,
    get_election_contests_area_results_and_total_ballot_counted,
};
use super::template_renderer::*;
use crate::postgres::area::get_areas_by_election_id;
use crate::postgres::election::get_election_by_id;
use crate::postgres::reports::ReportType;
use crate::postgres::scheduled_event::find_scheduled_event_by_election_event_id;
use crate::postgres::tally_session::get_tally_sessions_by_election_event_id;
use crate::services::consolidation::eml_generator::{
    find_miru_annotation, prepend_miru_annotation, ValidateAnnotations, MIRU_AREA_CCS_SERVERS,
    MIRU_TALLY_SESSION_DATA,
};
use crate::services::database::{get_hasura_pool, get_keycloak_pool};
use crate::services::temp_path::*;
use crate::services::users::count_keycloak_enabled_users_by_area_id;
use crate::types::miru_plugin::MiruTransmissionPackageData;
use crate::{postgres::election_event::get_election_event_by_id, services::s3::get_minio_url};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use deadpool_postgres::{Client as DbClient, Transaction};
use rocket::form::validate::Contains;
use rocket::http::Status;
use sequent_core::serialization::deserialize_with_path::deserialize_str;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::types::scheduled_event::generate_voting_period_dates;
use sequent_core::types::templates::EmailConfig;
use serde::{Deserialize, Serialize};
use std::env;
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
    pub voters_turnout: i64,
    pub chairperson_name: String,
    pub chairperson_digital_signature: String,
    pub poll_clerk_name: String,
    pub poll_clerk_digital_signature: String,
    pub third_member_name: String,
    pub third_member_digital_signature: String,
    pub report_hash: String,
    pub software_version: String,
    pub ovcs_version: String,
    pub system_hash: String,
    pub servers: Vec<ServerData>,
}

/// Struct for System Data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemData {
    pub rendered_user_template: String,
    pub file_qrcode_lib: String,
}

#[derive(Debug)]
pub struct TransmissionReport {
    tenant_id: String,
    election_event_id: String,
    election_id: String,
}

#[async_trait]
impl TemplateRenderer for TransmissionReport {
    type UserData = UserData;
    type SystemData = SystemData;

    fn get_report_type() -> ReportType {
        ReportType::TRANSMISSION_REPORTS
    }

    fn get_tenant_id(&self) -> String {
        self.tenant_id.clone()
    }

    fn get_election_event_id(&self) -> String {
        self.election_event_id.clone()
    }

    fn base_name() -> String {
        "transmission_report".to_string()
    }

    fn prefix(&self) -> String {
        format!("transmission_report_{}", self.election_event_id)
    }

    fn get_email_config() -> EmailConfig {
        EmailConfig {
            subject: "Sequent Online Voting - Transitions".to_string(),
            plaintext_body: "".to_string(),
            html_body: None,
        }
    }

    #[instrument(err, skip(self, hasura_transaction, keycloak_transaction))]
    /// Prepare user data by fetching the relevant details
    async fn prepare_user_data(
        &self,
        hasura_transaction: &Transaction<'_>,
        keycloak_transaction: &Transaction<'_>,
    ) -> Result<Self::UserData> {
        let realm: String =
            get_event_realm(self.tenant_id.as_str(), self.election_event_id.as_str());
        // Fetch election event data
        let election_event =
            get_election_event_by_id(hasura_transaction, &self.tenant_id, &self.election_event_id)
                .await
                .with_context(|| "Error obtaining election event")?;

        // Fetch areas associated with the election
        let election_areas = get_areas_by_election_id(
            &hasura_transaction,
            &self.tenant_id,
            &self.election_event_id,
            &self.election_id,
        )
        .await
        .map_err(|err| anyhow!("Error at get_areas_by_election_id: {err:?}"))?;

        if election_areas.is_empty() {
            return Err(anyhow!("No areas found for the given election"));
        }

        println!("election_areas Data: {:?}", election_areas);

        let mut areas: Vec<UserDataArea> = Vec::new();

        // Fetch election event data
        let scheduled_events = find_scheduled_event_by_election_event_id(
            &hasura_transaction,
            &self.get_tenant_id(),
            &self.get_election_event_id(),
        )
        .await?;

        // Fetch election's voting periods
        let voting_period_dates = generate_voting_period_dates(
            scheduled_events,
            &self.get_tenant_id(),
            &self.get_election_event_id(),
            Some(&self.get_election_id().unwrap_or_default()),
        )?;

        // extract start date from voting period
        let voting_period_start_date = voting_period_dates.start_date.unwrap_or_default();
        // extract end date from voting period
        let voting_period_end_date = voting_period_dates.end_date.unwrap_or_default();

        // get election instace
        let election = match get_election_by_id(
            &hasura_transaction,
            &self.get_tenant_id(),
            &self.get_election_event_id(),
            &self.election_id,
        )
        .await
        .with_context(|| "Error getting election by id")?
        {
            Some(election) => election,
            None => return Err(anyhow::anyhow!("Election not found")),
        };

        for area in election_areas.iter() {
            let country = area.clone().name.unwrap_or('-'.to_string());

            // get election instace's general data (post, area, etc...)
            let election_general_data = match extract_election_data(&election).await {
                Ok(data) => data, // Extracting the ElectionData struct out of Ok
                Err(err) => {
                    return Err(anyhow::anyhow!(format!(
                        "Error fetching election data: {}",
                        err
                    )));
                }
            };
            // fetch total of registerd voters
            let registered_voters =
                count_keycloak_enabled_users_by_area_id(&keycloak_transaction, &realm, &area.id)
                    .await
                    .map_err(|err| anyhow!("Error counting registered voters: {err}"))?;

            let tally_sessions = get_tally_sessions_by_election_event_id(
                &hasura_transaction,
                &self.tenant_id,
                &self.election_event_id,
            )
            .await
            .map_err(|err| anyhow!("Error getting the tally sessions: {err:?}"))?;

            let tally_session_data_parsed: Vec<MiruTransmissionPackageData> = if let Some(
                tally_session,
            ) =
                tally_sessions.iter().find(|session| {
                    session.election_event_id == self.election_event_id
                        && session.area_ids.contains(&area.id)
                }) {
                let tally_annotation = tally_session
                    .get_valid_annotations()
                    .map_err(|err| anyhow!("Error getting valid annotations: {err}"))?;

                let tally_session_data =
                    find_miru_annotation(MIRU_TALLY_SESSION_DATA, &tally_annotation.clone())
                        .with_context(|| {
                            format!("Missing area annotation: '{}'", MIRU_TALLY_SESSION_DATA)
                        })?;

                deserialize_str(&tally_session_data)
                    .map_err(|err| anyhow!("Error deserializing tally session data: {err}"))?
            } else {
                info!("Tally session not found for the given election event and area, setting default transmission status for area: {:?}", area.id);
                vec![]
            };

            let annotations = area.clone().get_valid_annotations()?;

            let servers: Vec<AnnotationServerData> =
                find_miru_annotation(MIRU_AREA_CCS_SERVERS, &annotations)
                    .with_context(|| {
                        format!("Missing area annotation: '{}'", MIRU_AREA_CCS_SERVERS)
                    })
                    .map(|area_data_js| {
                        serde_json::from_str(&area_data_js).map_err(|err| anyhow!("{}", err))
                    })
                    .flatten()
                    .unwrap_or(vec![]);

            let servers = servers
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

            // Fetch necessary data (dummy placeholders for now)
            let chairperson_name = "John Doe".to_string();
            let poll_clerk_name = "Jane Smith".to_string();
            let third_member_name = "Alice Johnson".to_string();
            let chairperson_digital_signature = "DigitalSignatureABC".to_string();
            let poll_clerk_digital_signature = "DigitalSignatureDEF".to_string();
            let third_member_digital_signature = "DigitalSignatureGHI".to_string();
            let report_hash = "dummy_report_hash".to_string();
            let ovcs_version = "1.0".to_string();
            let software_version = "1.0".to_string();
            let system_hash = "dummy_system_hash".to_string();

            let area_data = UserDataArea {
                date_printed: "2024-10-09T14:30:00-04:00".to_string(),
                election_date: "2024-05-10T14:30:00-04:00".to_string(),
                election_title: election_event.name.clone(),
                voting_period_start: voting_period_start_date.clone(),
                voting_period_end: voting_period_end_date.clone(),
                geographical_region: election_general_data.geographical_region,
                post: election_general_data.post,
                country: country,
                voting_center: election_general_data.voting_center,
                precinct_code: election_general_data.precinct_code,
                registered_voters,
                ballots_counted: 0,
                voters_turnout: 0,
                chairperson_name,
                chairperson_digital_signature,
                poll_clerk_name,
                poll_clerk_digital_signature,
                third_member_name,
                third_member_digital_signature,
                report_hash,
                ovcs_version,
                system_hash,
                software_version,
                servers: servers,
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

#[instrument]
pub async fn generate_transmission_report(
    document_id: &str,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
    mode: GenerateReportMode,
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
) -> Result<()> {
    let template = TransmissionReport {
        tenant_id: tenant_id.to_string(),
        election_event_id: election_event_id.to_string(),
        election_id: election_id.to_string(),
    };
    template
        .execute_report(
            document_id,
            tenant_id,
            election_event_id,
            false,
            None,
            None,
            mode,
            hasura_transaction,
            keycloak_transaction,
        )
        .await
}
