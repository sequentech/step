// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::documents::upload_and_return_document;
use crate::services::temp_path::write_into_named_temp_file;
use crate::postgres::{election_event, template};
use crate::services::database::get_hasura_pool;
use super::utils::{get_public_asset_template, ToMap};
use crate::services::s3::get_minio_url;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use deadpool_postgres::Client as DbClient;
use sequent_core::services::keycloak;
use sequent_core::services::{pdf, reports};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::fmt::Debug;
use tracing::{info, instrument, warn};

/// Trait that defines the behavior for rendering templates
#[async_trait]
pub trait TemplateRenderer: Debug {
    type UserData: Serialize + ToMap;
    type SystemData: Serialize + ToMap;

    fn base_name() -> String;
    fn prefix(&self) -> String;

    fn get_tenant_id(&self) -> String;
    fn get_election_event_id(&self) -> String;

    async fn prepare_user_data(&self) -> Result<Self::UserData>;
    async fn prepare_system_data(&self, rendered_user_template: String)
        -> Result<Self::SystemData>;

    async fn get_custom_user_template(&self) -> Result<Option<String>> {
        let mut hasura_db_client: DbClient = get_hasura_pool()
            .await
            .get()
            .await
            .with_context(|| "Error getting hasura db pool")?;

        let transaction = hasura_db_client
            .transaction()
            .await
            .with_context(|| "Error starting hasura transaction")?;

        let election_event = election_event::get_election_event_by_id(
            &transaction,
            &self.get_tenant_id(),
            &self.get_election_event_id(),
        )
        .await
        .with_context(|| "Error getting the election event by id")?;

        let presentation = match election_event.presentation {
            Some(val) => val,
            None => return Err(anyhow!("Election event has no presentation")),
        };

        let active_template_ids = match presentation.get("active_template_ids") {
            Some(val) => val,
            None => {
                warn!("No active_template_ids in presentation");
                return Ok(None);
            }
        };

        let usr_verification_tpl_id = match active_template_ids
            .get("manual_verification")
            .and_then(Value::as_str)
        {
            Some(id) if !id.is_empty() => id.to_string(),
            _ => {
                info!("manual_verification id not found or empty");
                return Ok(None);
            }
        };

        // Get the template by ID and return its value:
        let template_data_opt = template::get_template_by_id(
            &transaction,
            &self.get_tenant_id(),
            &usr_verification_tpl_id,
        )
        .await
        .with_context(|| "Error getting template by id")?;

        let tpl_document: Option<&str> = match &template_data_opt {
            Some(template_data) => template_data
                .template
                .get("document")
                .and_then(Value::as_str),
            None => {
                warn!("No manual verification template was found by id");
                return Ok(None);
            }
        };

        match tpl_document {
            Some(document) if !document.is_empty() => Ok(Some(document.to_string())),
            _ => Ok(None),
        }
    }

    async fn get_default_user_template(&self) -> Result<String> {
        let base_name = Self::base_name();
        get_public_asset_template(format!("{base_name}_user.hbs").as_str()).await
    }

    async fn get_system_template(&self) -> Result<String> {
        let base_name = Self::base_name();
        get_public_asset_template(format!("{base_name}_system.hbs").as_str()).await
    }

    async fn generate_report(&self) -> Result<String> {
        // Get user template (custom or default)
        let user_template = match self
            .get_custom_user_template()
            .await
            .map_err(|e| anyhow!("Error getting custom user template: {e:?}"))?
        {
            Some(template) => template,
            None => self
                .get_default_user_template()
                .await
                .map_err(|e| anyhow!("Error getting default user template: {e:?}"))?,
        };

        // Prepare user data
        let user_data = self
            .prepare_user_data()
            .await
            .map_err(|e| anyhow!("Error preparing user data: {e:?}"))?
            .to_map()
            .map_err(|e| anyhow!("Error converting user data to map: {e:?}"))?;

        let rendered_user_template = reports::render_template_text(&user_template, user_data)
            .map_err(|e| anyhow!("Error rendering user template: {e:?}"))?;

        // Prepare system data
        let system_data = self
            .prepare_system_data(rendered_user_template)
            .await
            .map_err(|e| anyhow!("Error preparing system data: {e:?}"))?
            .to_map()
            .map_err(|e| anyhow!("Error converting system data to map: {e:?}"))?;
        let system_template = self
            .get_system_template()
            .await
            .map_err(|e| anyhow!("Error getting default user template: {e:?}"))?;

        let rendered_system_template = reports::render_template_text(&system_template, system_data)
            .map_err(|e| anyhow!("Error rendering system template: {e:?}"))?;

        Ok(rendered_system_template)
    }

    async fn execute_report(
        &self,
        document_id: &str,
        tenant_id: &str,
        election_event_id: &str,
    ) -> Result<()> {
        let rendered_system_template = self
            .generate_report()
            .await
            .map_err(|err| anyhow!("Error rendering report: {}", err))?;

        // Generate PDF
        let bytes_pdf = pdf::html_to_pdf(rendered_system_template)
            .map_err(|err| anyhow!("Error rendering report to pdf: {}", err))?;

        let base_name = Self::base_name();
        let report_prefix = self.prefix();

        // Write temp file and upload
        let (_temp_path, temp_path_string, file_size) =
            write_into_named_temp_file(&bytes_pdf, format!("{base_name}-").as_str(), ".pdf")
                .map_err(|err| anyhow!("Error writing to file: {err}"))?;

        let auth_headers = keycloak::get_client_credentials()
            .await
            .map_err(|err| anyhow!("Error getting client credentials: {err}"))?;
        let _document = upload_and_return_document(
            temp_path_string,
            file_size,
            "application/pdf".to_string(),
            auth_headers.clone(),
            tenant_id.to_string(),
            election_event_id.to_string(),
            format!("{report_prefix}.pdf"),
            Some(document_id.to_string()),
            true,
        )
        .await
        .map_err(|err| anyhow!("Error uploading document: {err}"))?;

        Ok(())
    }
}
