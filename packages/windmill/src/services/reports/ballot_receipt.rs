// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Ballot receipt report template renderer.
//!
//! This report is generated from the voting portal after casting a vote. It
//! verifies the ballot exists in the cast-vote records and renders a PDF receipt
//! containing the ballot tracker URL and a timestamp.

use super::template_renderer::*;
use crate::postgres::reports::ReportType;
use crate::postgres::{self};
use crate::services::temp_path::*;

use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use deadpool_postgres::Transaction;
use sequent_core::util::temp_path::*;

use sequent_core::services::pdf;
use sequent_core::services::s3::get_minio_url;
use sequent_core::services::uuid_validation::parse_uuid_v4;
use sequent_core::types::date_time::{DateFormat, TimeZone};
use sequent_core::util::date_time::get_date_and_time;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use uuid::Uuid;

/// Ballot- and voter-specific inputs required to generate a real receipt.
///
/// This data is not required for preview mode, where the renderer reads example
/// JSON data from public assets instead.
#[derive(Serialize, Deserialize, Debug)]
pub struct BallotData {
    /// Area where the ballot was cast.
    pub area_id: String,
    /// Voter identifier (Keycloak user id).
    pub voter_id: String,
    /// Ballot identifier used by cast-vote records.
    pub ballot_id: String,
    /// Public URL used to track the ballot.
    pub ballot_tracker_url: String,
    /// Optional time zone for rendering timestamps.
    pub time_zone: Option<TimeZone>,
    /// Optional date format for rendering timestamps.
    pub date_format: Option<DateFormat>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
/// User-side data rendered into the ballot-receipt user template.
pub struct UserData {
    /// Ballot identifier to display on the receipt.
    pub ballot_id: String,
    /// Public ballot tracker URL.
    pub ballot_tracker_url: String,
    /// HTML template snippet for rendering a QR code.
    pub qrcode: String,
    /// HTML template snippet for rendering the logo.
    pub logo: String,
    /// Timestamp string included in the rendered receipt.
    pub timestamp: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
/// System-side variables used by the ballot-receipt system template.
pub struct SystemData {
    /// User template already rendered with `UserData`.
    pub rendered_user_template: String,
    /// PDF title.
    pub title: String,
    /// URL or path to the logo image.
    pub file_logo: String,
    /// URL or path to the QR-code JS library.
    pub file_qrcode_lib: String,
}

#[derive(Debug)]
/// Renderer for the ballot-receipt report templates.
#[allow(missing_docs_in_private_items)]
pub struct BallotTemplate {
    ids: ReportOrigins,
    /// Only present in real mode; preview mode uses public-assets JSON.
    pub ballot_data: Option<BallotData>,
}

impl BallotTemplate {
    /// Creates a renderer for the given tenant/event and ballot inputs.
    pub fn new(ids: ReportOrigins, ballot_data: Option<BallotData>) -> Self {
        BallotTemplate { ids, ballot_data }
    }
}

#[async_trait]
impl TemplateRenderer for BallotTemplate {
    type UserData = UserData;
    type SystemData = SystemData;

    fn get_report_type(&self) -> ReportType {
        ReportType::BALLOT_RECEIPT
    }

    fn base_name(&self) -> String {
        "ballot_receipt".to_string()
    }

    fn prefix(&self) -> String {
        format!("ballot_receipt_{}", self.ids.election_event_id,)
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
    /// Validates the ballot exists in cast votes and builds receipt template data.
    ///
    /// # Errors
    ///
    /// Returns an error if required ids are missing, ids are not valid UUIDv4,
    /// cast votes cannot be fetched, or the given ballot cannot be found for the voter.
    async fn prepare_user_data(
        &self,
        hasura_transaction: &Transaction<'_>,
        _keycloak_transaction: &Transaction<'_>,
    ) -> Result<Self::UserData> {
        let Some(election_id) = &self.ids.election_id else {
            return Err(anyhow!("Empty election_id"));
        };

        let (area_id, voter_id, ballot_id, ballot_tracker_url, time_zone, date_format) =
            match &self.ballot_data {
                Some(ballot_data) => (
                    ballot_data.area_id.as_str(),
                    ballot_data.voter_id.as_str(),
                    ballot_data.ballot_id.as_str(),
                    ballot_data.ballot_tracker_url.as_str(),
                    ballot_data.time_zone.clone(),
                    ballot_data.date_format.clone(),
                ),
                None => {
                    return Err(anyhow!(
                        "Cannot verify ballot id becasue Ballot data is missing"
                    ));
                }
            };

        let tennant_uuid = parse_uuid_v4(self.get_tenant_id().as_str())
            .map_err(|err| anyhow!("Error parsing tenant id: {:?}", err))?;
        let election_event_uuid = parse_uuid_v4(self.get_election_event_id().as_str())
            .map_err(|err| anyhow!("Error parsing election event id: {:?}", err))?;

        let ballot_uui = parse_uuid_v4(election_id.as_str())
            .map_err(|err| anyhow!("Error parsing election id: {:?}", err))?;

        let cast_votes = postgres::cast_vote::get_cast_votes(
            hasura_transaction,
            &tennant_uuid,
            &election_event_uuid,
            &ballot_uui,
            voter_id,
        )
        .await?;

        // Verify that the vote has been casted
        if !cast_votes.iter().any(|cv| {
            (cv.ballot_id.as_deref() == Some(ballot_id)) && (cv.area_id.as_deref() == Some(area_id))
        }) {
            return Err(anyhow!("BallotID not found in cast votes for {voter_id}"));
        }

        Ok(UserData {
            ballot_id: ballot_id.to_string(),
            ballot_tracker_url: ballot_tracker_url.to_string(),
            qrcode: QR_CODE_TEMPLATE.to_string(),
            logo: LOGO_TEMPLATE.to_string(),
            timestamp: get_date_and_time(),
        })
    }

    #[instrument(err, skip_all)]
    /// Prepares system-side variables used by the system template.
    ///
    /// # Errors
    ///
    /// Returns an error if public-assets configuration cannot be resolved.
    async fn prepare_system_data(
        &self,
        rendered_user_template: String,
    ) -> Result<Self::SystemData> {
        let public_assets_path = get_public_assets_path_env_var()?;
        let minio_endpoint_base =
            get_minio_url().with_context(|| "Error getting minio endpoint")?;

        if pdf::doc_renderer_backend() == pdf::DocRendererBackend::InPlace {
            Ok(SystemData {
                rendered_user_template,
                file_logo: format!(
                    "{}/{}/{}",
                    minio_endpoint_base, public_assets_path, PUBLIC_ASSETS_LOGO_IMG
                ),
                file_qrcode_lib: format!(
                    "{}/{}/{}",
                    minio_endpoint_base, public_assets_path, PUBLIC_ASSETS_QRCODE_LIB
                ),
                title: "Ballot receipt - Sequentech".to_string(),
            })
        } else {
            // If we are rendering with a lambda, the QRCode lib is
            // already included in the lambda container image.
            Ok(SystemData {
                rendered_user_template,
                file_logo: format!(
                    "{}/{}/{}",
                    minio_endpoint_base, public_assets_path, PUBLIC_ASSETS_LOGO_IMG
                ),
                file_qrcode_lib: "/assets/qrcode.min.js".to_string(),
                title: "Ballot receipt - Sequentech".to_string(),
            })
        }
    }
}
