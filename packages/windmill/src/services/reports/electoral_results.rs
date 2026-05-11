// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Electoral-results report template renderer.
//!
//! This report integrates with the shared template-rendering pipeline and
//! provides system-side variables required by the PDF rendering backend.

use super::template_renderer::*;
use crate::postgres::reports::ReportType;
use crate::services::temp_path::*;
use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{Local, TimeZone};
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::services::pdf;
use sequent_core::services::s3::get_minio_url;
use sequent_core::util::temp_path::*;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};
use velvet::pipes::generate_reports::TemplateData;

#[derive(Debug, Serialize, Deserialize, Clone)]
/// System-side variables used by the electoral-results system template.
pub struct SystemData {
    /// User template already rendered with `UserData`.
    pub rendered_user_template: String,
    /// URL or path to the QR-code JS library used by the PDF rendering backend.
    pub file_qrcode_lib: String,
}

#[derive(Debug)]
/// Renderer for the electoral-results report templates.
#[allow(missing_docs_in_private_items)]
pub struct ElectoralResults {
    ids: ReportOrigins,
}

impl ElectoralResults {
    /// Creates a renderer bound to a specific tenant/event (and optionally election/template).
    #[must_use]
    pub const fn new(ids: ReportOrigins) -> Self {
        ElectoralResults { ids }
    }
}

#[async_trait]
impl TemplateRenderer for ElectoralResults {
    type UserData = TemplateData;
    type SystemData = SystemData;

    fn get_report_type(&self) -> ReportType {
        ReportType::ELECTORAL_RESULTS
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
        "electoral_results".to_string()
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
