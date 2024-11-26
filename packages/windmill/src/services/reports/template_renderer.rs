// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::utils::get_public_asset_template;
use crate::postgres::reports::{get_template_id_for_report, Report, ReportType};
use crate::postgres::template;
use crate::services::consolidation::aes_256_cbc_encrypt::encrypt_file_aes_256_cbc;
use crate::services::documents::upload_and_return_document;
use crate::services::providers::email_sender::{Attachment, EmailSender};
use crate::services::reports_vault::get_report_secret_key;
use crate::services::tasks_execution::{update_complete, update_fail};
use crate::services::temp_path::{
    generate_temp_file, get_file_size, read_temp_path, write_into_named_temp_file,
};
use crate::services::vault;
use crate::tasks::send_template::send_template_email;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use deadpool_postgres::Transaction;
use headless_chrome::types::PrintToPdfOptions;
use sequent_core::services::keycloak::{self, get_event_realm, KeycloakAdminClient};
use sequent_core::services::{pdf, reports};
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::types::templates::ReportExtraConfig;
use sequent_core::types::to_map::ToMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt::Debug;
use strum_macros::{Display, EnumString, IntoStaticStr};
use tempfile::{NamedTempFile, TempPath};
use tracing::{debug, info, warn};

#[allow(non_camel_case_types)]
#[derive(Display, Serialize, Deserialize, Debug, PartialEq, Eq, Clone, EnumString)]
pub enum GenerateReportMode {
    PREVIEW,
    REAL,
}

#[allow(non_camel_case_types)]
#[derive(
    Display, Serialize, Deserialize, Debug, PartialEq, Eq, Clone, EnumString, IntoStaticStr,
)]
pub enum EReportEncryption {
    #[strum(serialize = "unencrypted")]
    UNENCRYPTED,
    #[strum(serialize = "configured_password")]
    CONFIGURED_PASSWORD,
}

/// Trait that defines the behavior for rendering templates
#[async_trait]
pub trait TemplateRenderer: Debug {
    type UserData: Serialize + ToMap + Send + for<'de> Deserialize<'de>;
    type SystemData: Serialize + ToMap + for<'de> Deserialize<'de>;

    fn base_name(&self) -> String;
    fn get_report_type(&self) -> ReportType;
    fn prefix(&self) -> String;
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

    async fn get_custom_user_template(
        &self,
        hasura_transaction: &Transaction<'_>,
    ) -> Result<Option<String>> {
        let report_type = &self.get_report_type();
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

        let template_data_opt =
            template::get_template_by_id(&hasura_transaction, &self.get_tenant_id(), &template_id)
                .await
                .with_context(|| "Error getting template by id")?;

        let tpl_document: Option<&str> = match &template_data_opt {
            Some(template_data) => template_data
                .template
                .get("document")
                .and_then(Value::as_str),
            None => {
                warn!("No {} template was found by id", self.base_name());
                return Ok(None);
            }
        };

        match tpl_document {
            Some(document) if !document.is_empty() => Ok(Some(document.to_string())),
            _ => Ok(None),
        }
    }

    async fn get_default_user_template(&self) -> Result<String> {
        let base_name = self.base_name();
        get_public_asset_template(format!("{base_name}_user.hbs").as_str()).await
    }

    async fn get_system_template(&self) -> Result<String> {
        let base_name = self.base_name();
        get_public_asset_template(format!("{base_name}_system.hbs").as_str()).await
    }

    async fn get_preview_data_file(&self) -> Result<String> {
        let base_name = self.base_name();
        get_public_asset_template(format!("{base_name}.json").as_str()).await
    }

    async fn get_default_extra_config_file(&self) -> Result<String> {
        let base_name = self.base_name();
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

    async fn generate_report_inner(
        &self,
        generate_mode: GenerateReportMode,
        hasura_transaction: &Transaction<'_>,
        keycloak_transaction: &Transaction<'_>,
    ) -> Result<String> {
        // Get user template (custom or default)
        let user_template = match self
            .get_custom_user_template(hasura_transaction)
            .await
            .map_err(|e| anyhow!("Error getting custom user template: {e:?}"))?
        {
            Some(template) => template,
            None => self
                .get_default_user_template()
                .await
                .map_err(|e| anyhow!("Error getting default user template: {e:?}"))?,
        };

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
            reports::render_template_text(&user_template, user_data_map)
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

    async fn generate_report(
        &self,
        generate_mode: GenerateReportMode,
        hasura_transaction: &Transaction<'_>,
        keycloak_transaction: &Transaction<'_>,
    ) -> Result<String> {
        // Get user template (custom or default)
        let user_template = match self
            .get_custom_user_template(hasura_transaction)
            .await
            .map_err(|e| anyhow!("Error getting custom user template: {e:?}"))?
        {
            Some(template) => template,
            None => self
                .get_default_user_template()
                .await
                .map_err(|e| anyhow!("Error getting default user template: {e:?}"))?,
        };

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
            reports::render_template_text(&user_template, user_data_map)
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

    // Inner implementation for `execute_report()` so that implementors of the
    // trait can reimplement the function while calling the parent default
    // implementation too when needed
    async fn execute_report_inner(
        &self,
        document_id: &str,
        tenant_id: &str,
        election_event_id: &str,
        is_scheduled_task: bool,
        recipients: Vec<String>,
        pdf_options: Option<PrintToPdfOptions>,
        generate_mode: GenerateReportMode,
        report: Option<Report>,
        hasura_transaction: &Transaction<'_>,
        keycloak_transaction: &Transaction<'_>,
        task_execution: Option<TasksExecution>,
    ) -> Result<()> {
        // Generate report in html
        let rendered_system_template = match self
            .generate_report(generate_mode, hasura_transaction, keycloak_transaction)
            .await
        {
            Ok(template) => template,
            Err(err) => {
                if let Some(task) = task_execution {
                    update_fail(&task, "Failed to generate report").await?;
                }
                return Err(anyhow!("Error rendering report: {err:?}"));
            }
        };

        debug!("Report generated: {rendered_system_template}");

        let ext_cfg: ReportExtraConfig = self
            .get_default_extra_config()
            .await
            .map_err(|e| anyhow!("Error getting default extra config: {e:?}"))?;
        debug!("Extra config read: {ext_cfg:?}");

        // Use the default pdf options only if not given
        let pdf_options = match pdf_options {
            Some(pdf_options) => Some(pdf_options),
            None => Some(ext_cfg.pdf_options),
        };

        let extension_suffix = "pdf";

        // Generate PDF
        let content_bytes = pdf::html_to_pdf(rendered_system_template.clone(), pdf_options)
            .map_err(|err| anyhow!("Error rendering report to {extension_suffix:?}: {err:?}"))?;

        let base_name = self.base_name();
        let fmt_extension = format!(".{extension_suffix}");
        let report_name: String = format!("{}{fmt_extension}", self.prefix());

        // Write temp file and upload
        let (_temp_path, temp_path_string, file_size) = write_into_named_temp_file(
            &content_bytes,
            format!("{base_name}-").as_str(),
            fmt_extension.as_str(),
        )
        .map_err(|err| anyhow!("Error writing to file: {err:?}"))?;
        let mimetype = format!("application/{}", extension_suffix);

        info!("Report details: {:?}", report);

        let encrypted_temp_data: Option<TempPath> = if let Some(report) = &report {
            if report.encryption_policy == EReportEncryption::CONFIGURED_PASSWORD {
                let secret_key =
                    get_report_secret_key(&tenant_id, &election_event_id, Some(report.id.clone()));

                let encryption_password = vault::read_secret(secret_key.clone())
                    .await?
                    .ok_or_else(|| anyhow!("Encryption password not found"))?;
                info!("Encryption password: {:?}", encryption_password);

                // Encrypt the file
                let enc_file: NamedTempFile =
                    generate_temp_file(format!("{base_name}-").as_str(), ".epdf")
                        .with_context(|| "Error creating named temp file")?;

                let enc_temp_path = enc_file.into_temp_path();
                let encrypted_temp_path = enc_temp_path.to_string_lossy().to_string();

                encrypt_file_aes_256_cbc(
                    &temp_path_string,
                    &encrypted_temp_path,
                    &encryption_password,
                )
                .map_err(|err| anyhow!("Error encrypting file: {err:?}"))?;

                Some(enc_temp_path)
            } else {
                None // No encryption needed
            }
        } else {
            None // No report, no encryption
        };

        // Upload the document
        let auth_headers = keycloak::get_client_credentials()
            .await
            .map_err(|err| anyhow!("Error getting client credentials: {err:?}"))?;
        if let Some(enc_temp_path) = encrypted_temp_data {
            let encrypted_temp_path = enc_temp_path.to_string_lossy().to_string();
            let enc_temp_size = get_file_size(encrypted_temp_path.as_str())
                .with_context(|| "Error obtaining file size")?;
            let enc_report_name: String = format!("{}.epdf", self.prefix());
            let _document = upload_and_return_document(
                encrypted_temp_path,
                enc_temp_size,
                mimetype.clone(),
                auth_headers.clone(),
                tenant_id.to_string(),
                election_event_id.to_string(),
                enc_report_name.clone(),
                Some(document_id.to_string()),
                true,
            )
            .await
            .map_err(|err| anyhow!("Error uploading document: {err:?}"))?;

            // Send email if needed
            if self.should_send_email(is_scheduled_task) {
                let email_config = ext_cfg.communication_templates.email_config;
                let email_recipients = self
                    .get_email_recipients(recipients, tenant_id, election_event_id)
                    .await
                    .map_err(|err| anyhow!("Error getting email receiver: {err:?}"))?;
                let email_sender = EmailSender::new()
                    .await
                    .map_err(|e| anyhow::anyhow!(format!("Error getting email sender {e:?}")))?;
                let enc_report_bytes = read_temp_path(&enc_temp_path)?;
                email_sender
                    .send(
                        email_recipients,
                        email_config.subject,
                        email_config.plaintext_body,
                        email_config.html_body,
                        /* attachments */
                        vec![Attachment {
                            filename: enc_report_name,
                            mimetype: "application/octet-stream".into(),
                            content: enc_report_bytes,
                        }],
                    )
                    .await
                    .map_err(|err| anyhow!("Error sending email: {err:?}"))?;
            }
        } else {
            let _document = upload_and_return_document(
                temp_path_string,
                file_size,
                mimetype.clone(),
                auth_headers.clone(),
                tenant_id.to_string(),
                election_event_id.to_string(),
                report_name.clone(),
                Some(document_id.to_string()),
                true,
            )
            .await
            .map_err(|err| anyhow!("Error uploading document: {err:?}"))?;

            // Send email if needed
            if self.should_send_email(is_scheduled_task) {
                let email_config = ext_cfg.communication_templates.email_config;
                let email_recipients = self
                    .get_email_recipients(recipients, tenant_id, election_event_id)
                    .await
                    .map_err(|err| anyhow!("Error getting email receiver: {err:?}"))?;
                let email_sender = EmailSender::new()
                    .await
                    .map_err(|e| anyhow::anyhow!(format!("Error getting email sender {e:?}")))?;
                email_sender
                    .send(
                        email_recipients,
                        email_config.subject,
                        email_config.plaintext_body,
                        email_config.html_body,
                        /* attachments */
                        vec![Attachment {
                            filename: report_name,
                            mimetype: mimetype,
                            content: content_bytes,
                        }],
                    )
                    .await
                    .map_err(|err| anyhow!("Error sending email: {err:?}"))?;
            }
        }

        if let Some(task) = task_execution {
            update_complete(&task)
                .await
                .context("Failed to update task execution status to COMPLETED")?;
        }

        Ok(())
    }

    async fn execute_report(
        &self,
        document_id: &str,
        tenant_id: &str,
        election_event_id: &str,
        is_scheduled_task: bool,
        recipients: Vec<String>,
        pdf_options: Option<PrintToPdfOptions>,
        generate_mode: GenerateReportMode,
        report: Option<Report>,
        hasura_transaction: &Transaction<'_>,
        keycloak_transaction: &Transaction<'_>,
        task_execution: Option<TasksExecution>,
    ) -> Result<()> {
        self.execute_report_inner(
            document_id,
            tenant_id,
            election_event_id,
            is_scheduled_task,
            recipients,
            pdf_options,
            generate_mode,
            report,
            hasura_transaction,
            keycloak_transaction,
            task_execution,
        )
        .await
    }

    async fn get_email_recipients(
        &self,
        recipients: Vec<String>,
        tenant_id: &str,
        election_event_id: &str,
    ) -> Result<Vec<String>> {
        if recipients.len() > 0 {
            Ok(recipients) // If recipients are provided, use them
        } else {
            // Fetch email via voter_id if recipients are not provided
            let voter_id = self
                .get_voter_id()
                .ok_or_else(|| anyhow!("Error sending email: no recipients provided"))?;

            let client = KeycloakAdminClient::new()
                .await
                .map_err(|err| anyhow!("Error initializing Keycloak client: {err}"))?;

            let realm = get_event_realm(tenant_id, election_event_id);
            let voter = client
                .get_user(&realm, &voter_id)
                .await
                .map_err(|e| anyhow::anyhow!(format!("Error getting user {e:?}")))?;
            Ok(vec![voter.email.ok_or_else(|| {
                anyhow!("Error sending email: no email provided")
            })?])
        }
    }
}
