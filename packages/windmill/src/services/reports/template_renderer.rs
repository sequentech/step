// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::utils::get_public_asset_template;
use crate::postgres::reports::{get_template_id_for_report, ReportType};
use crate::postgres::template;
use crate::services::documents::upload_and_return_document;
use crate::services::temp_path::write_into_named_temp_file;
use crate::tasks::send_template::EmailSender;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use deadpool_postgres::Transaction;
use sequent_core::serialization::deserialize_with_path::*;
use sequent_core::services::keycloak::{self, get_event_realm, KeycloakAdminClient};
use sequent_core::services::{pdf, reports};
use sequent_core::types::templates::{
    CommunicationTemplatesExtraConfig, EmailConfig, PrintToPdfOptionsLocal, ReportExtraConfig,
    SendTemplateBody, SmsConfig,
};
use sequent_core::types::to_map::ToMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt::Debug;
use strum_macros::{Display, EnumString};
use tracing::{debug, info, warn};

#[allow(non_camel_case_types)]
#[derive(Display, Serialize, Deserialize, Debug, PartialEq, Eq, Clone, EnumString)]
pub enum GenerateReportMode {
    PREVIEW,
    REAL,
}

/// Trait that defines the behavior for rendering templates
#[async_trait]
pub trait TemplateRenderer: Debug {
    type UserData: Serialize + ToMap + Send + for<'de> Deserialize<'de>;
    type SystemData: Serialize + ToMap + for<'de> Deserialize<'de>;

    fn base_name() -> String;
    fn prefix(&self) -> String;
    fn get_report_type() -> ReportType;
    fn get_tenant_id(&self) -> String;
    fn get_election_event_id(&self) -> String;

    /// Default implementation, can be overridden in specific reports that have
    /// election_id
    fn get_election_id(&self) -> Option<String> {
        None
    }

    /// Send email if it's a cron job (scheduled task) or if a voterId is present
    fn should_send_email(&self, is_scheduled_task: bool) -> bool {
        is_scheduled_task || self.get_voter_id().is_some()
    }

    // Default implementation, can be overridden in specific reports that have
    // voterId
    fn get_voter_id(&self) -> Option<String> {
        None
    }

    async fn prepare_preview_data(&self) -> Result<Self::UserData> {
        let json_data = self
            .get_preview_data_file()
            .await
            .map_err(|e| anyhow::anyhow!(format!("Error preparing report preview {e:?}")))?;
        let data: Self::UserData = serde_json::from_str(&json_data)?;

        Ok(data)
    }

    async fn prepare_user_data(
        &self,
        hasura_transaction: &Transaction<'_>,
        keycloak_transaction: &Transaction<'_>,
    ) -> Result<Self::UserData>;

    async fn prepare_system_data(&self, rendered_user_template: String)
        -> Result<Self::SystemData>;

    async fn get_custom_user_template_data(
        &self,
        hasura_transaction: &Transaction<'_>,
    ) -> Result<Option<SendTemplateBody>> {
        // TODO: Breaking change, fix callers
        let report_type = &Self::get_report_type();
        let election_id = self.get_election_id();

        let report_template_id = get_template_id_for_report(
            &hasura_transaction,
            &self.get_tenant_id(),
            &self.get_election_event_id(),
            report_type,
            election_id.as_deref(),
        )
        .await
        .with_context(|| "Error getting template id for report")?;
        // Get the template by ID and return its value:
        let template_id = match report_template_id {
            Some(id) => id,
            None => {
                warn!("No template id was found for report type: {report_type} when trying to get the custom user template.");
                return Ok(None);
            }
        };

        let template_table_opt =
            template::get_template_by_id(&hasura_transaction, &self.get_tenant_id(), &template_id)
                .await
                .with_context(|| "Error getting template by id")?;

        // Unfortunately Template table has a column with the same name "Template" which stores a Value,
        // being document, sms, pdf_options, etc its attributes.
        match template_table_opt {
            Some(template_tbl) => {
                let template_data: SendTemplateBody = deserialize_value(template_tbl.template)
                    .map_err(|e| {
                        anyhow!(format!("Error deserializing custom user template: {e:?}"))
                    })?;
                Ok(Some(template_data))
            }
            None => {
                warn!("No {} template was found by id", Self::base_name());
                Ok(None)
            }
        }
    }

    /// Get the custom extra config provided by the user for this template or the values by default
    /// from the _extra_config file..
    async fn get_extra_config(
        &self,
        tpl_pdf_options: Option<PrintToPdfOptionsLocal>,
        tpl_email_config: Option<EmailConfig>,
        tpl_sms_config: Option<SmsConfig>,
    ) -> Result<ReportExtraConfig> {
        let (pdf_options, email_config, sms_config) = match tpl_pdf_options.is_none()
            || tpl_email_config.is_none()
            || tpl_sms_config.is_none()
        {
            true => {
                let def_ext_cfg: ReportExtraConfig = self
                    .get_default_extra_config()
                    .await
                    .map_err(|e| anyhow!("Error getting default extra config: {e:?}"))?;
                debug!("Default extra config read: {def_ext_cfg:?}");
                (
                    tpl_pdf_options.unwrap_or(def_ext_cfg.pdf_options),
                    tpl_email_config.unwrap_or(def_ext_cfg.communication_templates.email_config),
                    tpl_sms_config.unwrap_or(def_ext_cfg.communication_templates.sms_config),
                )
            }
            false => (
                tpl_pdf_options.unwrap_or_default(),
                tpl_email_config.unwrap_or_default(),
                tpl_sms_config.unwrap_or_default(),
            ),
        };
        Ok(ReportExtraConfig {
            pdf_options,
            communication_templates: CommunicationTemplatesExtraConfig {
                email_config,
                sms_config,
            },
        })
    }

    async fn get_default_user_template(&self) -> Result<String> {
        let base_name = Self::base_name();
        get_public_asset_template(format!("{base_name}_user.hbs").as_str()).await
    }

    async fn get_system_template(&self) -> Result<String> {
        let base_name = Self::base_name();
        get_public_asset_template(format!("{base_name}_system.hbs").as_str()).await
    }

    async fn get_preview_data_file(&self) -> Result<String> {
        let base_name = Self::base_name();
        get_public_asset_template(format!("{base_name}.json").as_str()).await
    }

    async fn get_default_extra_config_file(&self) -> Result<String> {
        let base_name = Self::base_name();
        get_public_asset_template(format!("{base_name}_extra_config.json").as_str()).await
    }

    /// Read the default extra config for this template's type like PDF options and communication templates.
    async fn get_default_extra_config(&self) -> Result<ReportExtraConfig> {
        let json_data = self
            .get_default_extra_config_file()
            .await
            .map_err(|e| anyhow::anyhow!(format!("Error to get the extra config data {e:?}")))?;
        let data: ReportExtraConfig = serde_json::from_str(&json_data)?;

        Ok(data)
    }

    async fn generate_report(
        &self,
        generate_mode: GenerateReportMode,
        hasura_transaction: &Transaction<'_>,
        keycloak_transaction: &Transaction<'_>,
        user_tpl_document: &str,
    ) -> Result<String> {
        // Prepare user data either preview or real
        let user_data = if generate_mode == GenerateReportMode::PREVIEW {
            self.prepare_preview_data()
                .await
                .map_err(|e| anyhow!("Error preparing preview user data: {e:?}"))?
        } else {
            self.prepare_user_data(hasura_transaction, keycloak_transaction)
                .await
                .map_err(|e| anyhow!("Error preparing user data: {e:?}"))?
        };

        let user_data_map = user_data
            .to_map()
            .map_err(|e| anyhow!("Error converting user data to map: {e:?}"))?;

        info!("user data in template renderer: {user_data_map:#?}");

        let rendered_user_template =
            reports::render_template_text(&user_tpl_document, user_data_map)
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
        is_scheduled_task: bool,
        receiver: Option<String>,
        generate_mode: GenerateReportMode,
        hasura_transaction: &Transaction<'_>,
        keycloak_transaction: &Transaction<'_>,
    ) -> Result<()> {
        // Do the query to get the user template data
        let template_data_opt: Option<SendTemplateBody> = self
            .get_custom_user_template_data(hasura_transaction)
            .await
            .map_err(|e| anyhow!("Error getting custom user template: {e:?}"))?;

        // Set the data from the user
        let (mut tpl_pdf_options, mut tpl_email, mut tpl_sms) = (None, None, None);
        let user_tpl_document = match template_data_opt {
            Some(template) => {
                tpl_pdf_options = template.pdf_options;
                tpl_email = template.email;
                tpl_sms = template.sms;
                template.document.unwrap_or_default()
            }
            None => "".to_string(),
        };

        // Fill extra config if needed with default data
        let ext_cfg: ReportExtraConfig = self
            .get_extra_config(tpl_pdf_options, tpl_email, tpl_sms)
            .await
            .map_err(|e| anyhow!("Error getting the extra config: {e:?}"))?;
        debug!("Extra config read: {ext_cfg:?}");

        // Get the default user template document if needed
        let user_tpl_document = match user_tpl_document.is_empty() {
            true => self
                .get_default_user_template()
                .await
                .map_err(|e| anyhow!("Error getting default user template: {e:?}"))?,
            false => user_tpl_document,
        };

        // Generate report in html
        let rendered_system_template = self
            .generate_report(
                generate_mode,
                hasura_transaction,
                keycloak_transaction,
                &user_tpl_document,
            )
            .await
            .map_err(|err| anyhow!("Error rendering report: {err:?}"))?;

        debug!("Report generated: {rendered_system_template}");

        let extension_suffix = "pdf";
        // Generate PDF
        let content_bytes = pdf::html_to_pdf(
            rendered_system_template.clone(),
            Some(ext_cfg.pdf_options.to_print_to_pdf_options()),
        )
        .map_err(|err| anyhow!("Error rendering report to {extension_suffix:?}: {err:?}"))?;

        let base_name = Self::base_name();
        let fmt_extension = format!(".{extension_suffix}");
        let report_name: String = format!("{}{fmt_extension}", self.prefix());

        // Write temp file and upload
        let (_temp_path, temp_path_string, file_size) = write_into_named_temp_file(
            &content_bytes,
            format!("{base_name}-").as_str(),
            fmt_extension.as_str(),
        )
        .map_err(|err| anyhow!("Error writing to file: {err:?}"))?;

        let auth_headers = keycloak::get_client_credentials()
            .await
            .map_err(|err| anyhow!("Error getting client credentials: {err:?}"))?;
        let _document = upload_and_return_document(
            temp_path_string,
            file_size,
            format!("application/{}", extension_suffix),
            auth_headers.clone(),
            tenant_id.to_string(),
            election_event_id.to_string(),
            report_name,
            Some(document_id.to_string()),
            true,
        )
        .await
        .map_err(|err| anyhow!("Error uploading document: {err:?}"))?;

        if self.should_send_email(is_scheduled_task) {
            let email_config = ext_cfg.communication_templates.email_config;
            let email_receiever = self
                .get_email_receiver(receiver, tenant_id, election_event_id)
                .await
                .map_err(|err| anyhow!("Error getting email receiver: {err:?}"))?;
            let email_sender = EmailSender::new()
                .await
                .map_err(|e| anyhow::anyhow!(format!("Error getting email sender {e:?}")))?;
            email_sender
                .send(
                    email_receiever,
                    email_config.subject,
                    email_config.plaintext_body,
                    rendered_system_template.clone(),
                )
                .await
                .map_err(|err| anyhow!("Error sending email: {err:?}"))?;
        }

        Ok(())
    }

    async fn get_email_receiver(
        &self,
        receiver: Option<String>,
        tenant_id: &str,
        election_event_id: &str,
    ) -> Result<String> {
        match receiver {
            Some(receiver) => Ok(receiver), // If receiver is provided, use it
            None => {
                // Fetch email via voter_id if receiver is not provided
                let voter_id = self
                    .get_voter_id()
                    .ok_or_else(|| anyhow!("Error sending email: no receiver provided"))?;

                let client = KeycloakAdminClient::new()
                    .await
                    .map_err(|err| anyhow!("Error initializing Keycloak client: {err}"))?;

                let realm = get_event_realm(tenant_id, election_event_id);
                let voter = client
                    .get_user(&realm, &voter_id)
                    .await
                    .map_err(|e| anyhow::anyhow!(format!("Error getting user {e:?}")))?;
                voter
                    .email
                    .ok_or_else(|| anyhow!("Error sending email: no email provided"))
            }
        }
    }
}
