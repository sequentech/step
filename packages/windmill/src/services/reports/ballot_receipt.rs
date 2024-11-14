// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::template_renderer::*;
use crate::postgres::reports::ReportType;
use crate::postgres::{self};
use crate::services::s3::get_minio_url;
use crate::services::temp_path::*;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use deadpool_postgres::Transaction;

use sequent_core::types::date_time::{DateFormat, TimeZone};
use sequent_core::util::date_time::generate_timestamp;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use uuid::Uuid;

/// Wrapper struct for data specific to the ballot and the voter
/// which won't be needed in the preview mode.
#[derive(Serialize, Deserialize, Debug)]
pub struct BallotData {
    pub area_id: String,
    pub voter_id: String,
    pub ballot_id: String,
    pub ballot_tracker_url: String,
    pub time_zone: Option<TimeZone>,
    pub date_format: Option<DateFormat>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserData {
    pub ballot_id: String,
    pub ballot_tracker_url: String,
    pub qrcode: String,
    pub logo: String,
    pub timestamp: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemData {
    pub rendered_user_template: String,
    pub title: String,
    pub file_logo: String,
    pub file_qrcode_lib: String,
}

#[derive(Debug)]
pub struct BallotTemplate {
    pub tenant_id: String,
    pub election_event_id: String,
    pub election_id: Option<String>,
    pub ballot_data: Option<BallotData>,
}

impl BallotTemplate {
    pub fn new(
        tenant_id: String,
        election_event_id: String,
        election_id: Option<String>,
        ballot_data: Option<BallotData>,
    ) -> Self {
        BallotTemplate {
            tenant_id,
            election_event_id,
            election_id,
            ballot_data,
        }
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
        format!("ballot_receipt_{}", self.election_event_id,)
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

    #[instrument]
    async fn prepare_user_data(
        &self,
        hasura_transaction: &Transaction<'_>,
        _keycloak_transaction: &Transaction<'_>,
    ) -> Result<Self::UserData> {
        let Some(election_id) = &self.election_id else {
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

        let tennant_uuid = Uuid::parse_str(self.get_tenant_id().as_str())
            .map_err(|err| anyhow!("Error parsing tenant id: {:?}", err))?;
        let election_event_uuid = Uuid::parse_str(self.get_election_event_id().as_str())
            .map_err(|err| anyhow!("Error parsing election event id: {:?}", err))?;

        let ballot_uui = Uuid::parse_str(election_id.as_str())
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
            cv.ballot_id.as_deref().map_or(false, |id| id == ballot_id)
                && cv.area_id.as_deref().map_or(false, |id| id == area_id)
        }) {
            return Err(anyhow!("BallotID not found in cast votes for {voter_id}"));
        }

        Ok(UserData {
            ballot_id: ballot_id.to_string(),
            ballot_tracker_url: ballot_tracker_url.to_string(),
            qrcode: QR_CODE_TEMPLATE.to_string(),
            logo: LOGO_TEMPLATE.to_string(),
            timestamp: generate_timestamp(time_zone, date_format, None),
        })
    }

    #[instrument(err, skip(self, rendered_user_template))]
    async fn prepare_system_data(
        &self,
        rendered_user_template: String,
    ) -> Result<Self::SystemData> {
        let public_assets_path = get_public_assets_path_env_var()?;
        let minio_endpoint_base = get_minio_url()?;

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
    }
}
