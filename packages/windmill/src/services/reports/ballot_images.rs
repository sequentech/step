// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Ballot-images report template renderer.
//!
//! This report renders a PDF containing ballot images and associated computed
//! data produced by the `velvet` pipeline.

use super::template_renderer::{ReportOriginatedFrom, ReportOrigins, TemplateRenderer};
use crate::postgres::reports::ReportType;
use crate::services::temp_path::PUBLIC_ASSETS_QRCODE_LIB;
use anyhow::{Context, Result};
use async_trait::async_trait;
use deadpool_postgres::Transaction;
use sequent_core::services::pdf;
use sequent_core::services::s3::get_minio_url;
use sequent_core::util::temp_path::get_public_assets_path_env_var;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use velvet::pipes::ballot_images::ComputedTemplateData;

#[derive(Serialize, Deserialize, Debug, Clone)]
/// System-side variables used by the ballot-images system template.
pub struct SystemData {
    /// User template already rendered with `UserData`.
    pub rendered_user_template: String,
    /// URL or path to the QR-code JS library used by the PDF rendering backend.
    pub file_qrcode_lib: String,
    /// Report title to display in the report.
    pub title: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
/// Additional user-facing strings used by the user template.
pub struct UserExtraData {
    /// Report title to render in the user template.
    pub title: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
/// User-side data for ballot-images rendering.
pub struct UserData {
    /// Computed ballot image data produced by the `velvet` pipeline.
    pub data: ComputedTemplateData,
    /// Small extra strings passed alongside computed data.
    pub extra_data: UserExtraData,
}

#[derive(Debug)]
/// Renderer for the ballot-images report templates.
#[allow(missing_docs_in_private_items)]
pub struct BallotImagesTemplate {
    ids: ReportOrigins,
}

impl BallotImagesTemplate {
    /// Creates a renderer bound to a specific tenant/event (and optionally election/template).
    #[must_use]
    pub const fn new(ids: ReportOrigins) -> Self {
        BallotImagesTemplate { ids }
    }
}

#[async_trait]
impl TemplateRenderer for BallotImagesTemplate {
    type UserData = UserData;
    type SystemData = SystemData;

    fn get_report_type(&self) -> ReportType {
        ReportType::BALLOT_IMAGES
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
        "ballot_images".to_string()
    }

    fn prefix(&self) -> String {
        format!(
            "{base_name}_{election_event_id}_{election_id:?}",
            base_name = self.base_name(),
            election_event_id = self.ids.election_event_id,
            election_id = self.ids.election_id,
        )
    }

    #[instrument(err, skip_all)]
    /// Prepares the user-side data for this report type.
    ///
    /// # Errors
    ///
    /// Currently unimplemented for this report type.
    async fn prepare_user_data(
        &self,
        hasura_transaction: &Transaction<'_>,
        keycloak_transaction: &Transaction<'_>,
    ) -> Result<Self::UserData> {
        Err(anyhow::anyhow!("Unimplemented"))
    }
    #[instrument(err, skip_all)]
    /// Prepares system-side variables used by the system template.
    ///
    /// # Errors
    ///
    /// Returns an error if public-assets configuration cannot be resolved when
    /// rendering is performed in-place.
    async fn prepare_system_data(
        &self,
        rendered_user_template: String,
    ) -> Result<Self::SystemData> {
        if pdf::doc_renderer_backend() == pdf::DocRendererBackend::InPlace {
            let public_asset_path = get_public_assets_path_env_var()?;
            let minio_endpoint_base =
                get_minio_url().with_context(|| "Error getting minio endpoint")?;

            Ok(SystemData {
                title: "Ballot Images - Sequentech".to_string(),
                rendered_user_template,
                file_qrcode_lib: format!(
                    "{minio_endpoint_base}/{public_asset_path}/{PUBLIC_ASSETS_QRCODE_LIB}"
                ),
            })
        } else {
            // If we are rendering with a lambda, the QRCode lib is
            // already included in the lambda container image.
            Ok(SystemData {
                title: "Ballot Images - Sequentech".to_string(),
                rendered_user_template,
                file_qrcode_lib: "/assets/qrcode.min.js".to_string(),
            })
        }
    }
}
