// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The Transmission Report: for each area (country) of one election (Post),
//! its registered voters, the ballots counted and the turnout, whether each
//! of its transmission destinations was sent and received the ER, and the
//! SBEI members who sign it. Restored from the 2025 implementation.

use super::report_variables::{
    calc_voters_turnout, extract_area_data, extract_election_data,
    extract_election_event_annotations, get_app_hash, get_app_version, get_date_and_time,
    get_report_hash, get_results_hash, ElectionData, ExecutionAnnotations, InspectorData,
};
use super::template_renderer::*;
use crate::postgres::area::get_areas_by_election_id;
use crate::postgres::election::get_election_by_id;
use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::reports::ReportType;
use crate::postgres::scheduled_event::find_scheduled_event_by_election_event_id;
use crate::services::ballot_box_reads::get_event_ballot_box;
use crate::services::consolidation::eml_generator::{MiruAreaAnnotations, ValidateAnnotations};
use crate::services::election_dates::get_election_dates;
use crate::services::temp_path::PUBLIC_ASSETS_QRCODE_LIB;
use crate::services::transmission::{
    get_transmission_data_from_tally_session_by_area, get_transmission_servers_data, ServerData,
};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use deadpool_postgres::Transaction;
use sequent_core::ballot::StringifiedPeriodDates;
use sequent_core::services::pdf;
use sequent_core::services::s3::get_minio_url;
use sequent_core::services::translations::Name;
use sequent_core::util::temp_path::*;
use serde::{Deserialize, Serialize};
use tracing::instrument;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserData {
    pub areas: Vec<UserDataArea>,
    pub election_dates: StringifiedPeriodDates,
    pub execution_annotations: ExecutionAnnotations,
}

/// One area's page of the report.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemData {
    pub rendered_user_template: String,
    pub file_qrcode_lib: String,
}

/// What an area's page says, from the election's and the area's
/// annotations, the voters whose ballots were counted in the area, its
/// destinations' transmission status and its SBEI members.
pub fn area_report(
    election_title: &str,
    election: &ElectionData,
    area_name: Option<&str>,
    annotations: &MiruAreaAnnotations,
    ballots_counted: i64,
    servers: Vec<ServerData>,
    inspectors: Vec<InspectorData>,
) -> Result<UserDataArea> {
    // The country is the area's annotation; areas configured without it
    // are named after their country, as in 2025.
    let country = Some(annotations.country.trim())
        .filter(|country| !country.is_empty())
        .map(str::to_string)
        .or_else(|| area_name.map(str::to_string))
        .unwrap_or_else(|| "-".to_string());
    Ok(UserDataArea {
        election_title: election_title.to_string(),
        geographical_region: election.geographical_region.clone(),
        post: election.post.clone(),
        country,
        voting_center: election.voting_center.clone(),
        station_id: annotations.station_id.clone(),
        station_name: annotations.station_name.clone(),
        registered_voters: Some(annotations.registered_voters),
        ballots_counted: Some(ballots_counted),
        voters_turnout: calc_voters_turnout(ballots_counted, annotations.registered_voters)?,
        servers,
        inspectors,
    })
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
    async fn prepare_user_data(
        &self,
        hasura_transaction: &Transaction<'_>,
        _keycloak_transaction: &Transaction<'_>,
    ) -> Result<Self::UserData> {
        let Some(election_id) = &self.ids.election_id else {
            return Err(anyhow!(
                "The transmission report is generated for one election"
            ));
        };
        let election_event = get_election_event_by_id(
            hasura_transaction,
            &self.ids.tenant_id,
            &self.ids.election_event_id,
        )
        .await
        .with_context(|| "Error obtaining election event")?;
        let election_title = election_event.get_name(&election_event.get_default_language());
        let sbei_users = extract_election_event_annotations(&election_event)
            .await
            .map_err(|err| anyhow!("Error extract election event annotations {err}"))?
            .sbei_users;

        let election = get_election_by_id(
            hasura_transaction,
            &self.ids.tenant_id,
            &self.ids.election_event_id,
            election_id,
        )
        .await
        .with_context(|| "Error getting election by id")?
        .ok_or_else(|| anyhow!("Election not found"))?;
        let election_data = extract_election_data(&election)
            .await
            .map_err(|err| anyhow!("Error extract election annotations {err}"))?;

        let election_areas = get_areas_by_election_id(
            hasura_transaction,
            &self.ids.tenant_id,
            &self.ids.election_event_id,
            election_id,
        )
        .await
        .map_err(|err| anyhow!("Error at get_areas_by_election_id: {err:?}"))?;
        if election_areas.is_empty() {
            return Err(anyhow!("No areas found for the given election"));
        }

        let scheduled_events = find_scheduled_event_by_election_event_id(
            hasura_transaction,
            &self.ids.tenant_id,
            &self.ids.election_event_id,
        )
        .await
        .map_err(|e| anyhow!("Error getting scheduled events by election event_id: {e}"))?;
        let election_dates = get_election_dates(&election, scheduled_events)
            .map_err(|e| anyhow!("Error getting election dates {e}"))?;

        let ballot_box = get_event_ballot_box(
            hasura_transaction,
            &self.ids.tenant_id,
            &self.ids.election_event_id,
        )
        .await?;

        let mut areas = Vec::with_capacity(election_areas.len());
        for area in election_areas.iter() {
            let annotations = area.get_annotations_or_empty_values()?;
            let inspectors = extract_area_data(area, sbei_users.clone())
                .await
                .map_err(|err| anyhow!("Error extract area data {err}"))?
                .inspectors;
            let tally_session_data = get_transmission_data_from_tally_session_by_area(
                hasura_transaction,
                &self.ids.tenant_id,
                &self.ids.election_event_id,
                &area.id,
                self.ids.tally_session_id.clone(),
            )
            .await
            .map_err(|err| {
                anyhow!("Error get_transmission_data_from_tally_session_by_area: {err:?}")
            })?;
            let servers = get_transmission_servers_data(&tally_session_data, area)
                .await
                .map_err(|err| anyhow!("Error get_transmission_servers_data: {err:?}"))?
                .servers;
            // The voters whose valid ballot in this area the tally counts.
            let ballots_counted = ballot_box
                .store
                .participation(
                    &self.ids.election_event_id,
                    Some(election_id.as_str()),
                    Some(area.id.as_str()),
                )
                .await
                .map_err(|err| anyhow!("Error getting counted ballots: {err}"))?
                .voters;
            areas.push(area_report(
                &election_title,
                &election_data,
                area.name.as_deref(),
                &annotations,
                ballots_counted,
                servers,
                inspectors,
            )?);
        }

        let app_version = get_app_version();
        let results_hash = get_results_hash(
            hasura_transaction,
            &self.ids.tenant_id,
            &self.ids.election_event_id,
            election_id,
        )
        .await
        .unwrap_or("-".to_string());
        let report_hash = get_report_hash(&ReportType::TRANSMISSION_REPORT.to_string())
            .await
            .unwrap_or("-".to_string());

        Ok(UserData {
            areas,
            election_dates,
            execution_annotations: ExecutionAnnotations {
                date_printed: get_date_and_time(),
                report_hash,
                app_version: app_version.clone(),
                software_version: app_version,
                app_hash: get_app_hash(),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn election() -> ElectionData {
        ElectionData {
            geographical_region: "EUROPE".to_string(),
            voting_center: "MADRID PE".to_string(),
            precinct_code: "90010001".to_string(),
            pollcenter_code: "9001A".to_string(),
            post: "MADRID PE".to_string(),
        }
    }

    fn annotations(country: &str, registered_voters: i64) -> MiruAreaAnnotations {
        MiruAreaAnnotations {
            ccs_servers: vec![],
            station_id: "90010001".to_string(),
            station_name: "SPAIN".to_string(),
            threshold: 2,
            sbei_ids: vec![],
            country: country.to_string(),
            registered_voters,
        }
    }

    fn server() -> ServerData {
        ServerData {
            server_code: "ccs-manila".to_string(),
            transmitted: "Not Transmitted".to_string(),
            server_name: "CCS Manila (SBOC)".to_string(),
            received: "Not Received".to_string(),
            date_transmitted: String::new(),
            date_received: String::new(),
        }
    }

    fn chairperson() -> InspectorData {
        InspectorData {
            role: "Chairperson".to_string(),
            name: "Maria L. Santos".to_string(),
        }
    }

    #[test]
    fn an_area_page_names_its_post_country_station_counts_destinations_and_signers() {
        let page = area_report(
            "2028 Overseas Voting",
            &election(),
            Some("A"),
            &annotations("Spain", 1250),
            5,
            vec![server()],
            vec![chairperson()],
        )
        .unwrap();
        assert_eq!(
            page,
            UserDataArea {
                election_title: "2028 Overseas Voting".to_string(),
                geographical_region: "EUROPE".to_string(),
                post: "MADRID PE".to_string(),
                country: "Spain".to_string(),
                voting_center: "MADRID PE".to_string(),
                station_id: "90010001".to_string(),
                station_name: "SPAIN".to_string(),
                registered_voters: Some(1250),
                ballots_counted: Some(5),
                voters_turnout: Some(0.4),
                servers: vec![server()],
                inspectors: vec![chairperson()],
            }
        );
    }

    #[test]
    fn an_area_without_a_country_annotation_is_named_after_the_area() {
        let named = area_report(
            "Event",
            &election(),
            Some("Spain"),
            &annotations(" ", 10),
            0,
            vec![],
            vec![],
        )
        .unwrap();
        assert_eq!(named.country, "Spain");
        let unnamed = area_report(
            "Event",
            &election(),
            None,
            &annotations("", 10),
            0,
            vec![],
            vec![],
        )
        .unwrap();
        assert_eq!(unnamed.country, "-");
    }

    #[test]
    fn an_area_without_registered_voters_has_no_turnout_to_divide_by() {
        let page = area_report(
            "Event",
            &election(),
            Some("Spain"),
            &annotations("Spain", 0),
            3,
            vec![],
            vec![],
        )
        .unwrap();
        assert_eq!(
            (
                page.registered_voters,
                page.ballots_counted,
                page.voters_turnout
            ),
            (Some(0), Some(3), Some(0.0))
        );
    }

    #[test]
    fn the_report_is_named_transmission_report_and_scoped_to_its_election() {
        let report = TransmissionReport::new(ReportOrigins {
            tenant_id: "tenant".to_string(),
            election_event_id: "event".to_string(),
            election_id: Some("election".to_string()),
            template_alias: Some("comelec".to_string()),
            voter_id: None,
            report_origin: ReportOriginatedFrom::ReportsTab,
            executer_username: Some("admin".to_string()),
            tally_session_id: Some("session".to_string()),
        });
        assert_eq!(report.get_report_type(), ReportType::TRANSMISSION_REPORT);
        assert_eq!(report.base_name(), "transmission_report");
        assert_eq!(report.prefix(), "transmission_report_tenant_event_election");
        assert_eq!(report.get_election_id().as_deref(), Some("election"));
        assert_eq!(
            report.get_initial_template_alias().as_deref(),
            Some("comelec")
        );
    }
}
