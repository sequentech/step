// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::utils::get_public_asset_template;
use crate::postgres::reports::{get_template_alias_for_report, Report, ReportType};
use crate::postgres::{election_event, template};
use crate::services::celery_app::get_worker_threads;
use crate::services::consolidation::aes_256_cbc_encrypt::encrypt_file_aes_256_cbc;
use crate::services::consolidation::zip::compress_folder_to_zip;
use crate::services::database::get_hasura_pool;
use crate::services::documents::upload_and_return_document;
use crate::services::providers::email_sender::{Attachment, EmailSender};
use crate::services::reports_vault::get_report_secret_key;
use crate::services::tasks_execution::{update_complete, update_fail};
use crate::services::temp_path::PUBLIC_ASSETS_QRCODE_LIB;
use crate::services::vault;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use deadpool_postgres::Transaction;
use futures::executor::block_on;
use futures::future::join_all;
use once_cell::sync::Lazy;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use rayon::ThreadPoolBuilder;
use sequent_core::serialization::deserialize_with_path::{deserialize_str, deserialize_value};
use sequent_core::services::keycloak::{self, get_event_realm, KeycloakAdminClient};
use sequent_core::services::{pdf, reports};
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::types::templates::{
    CommunicationTemplatesExtraConfig, EmailConfig, PrintToPdfOptionsLocal, ReportExtraConfig,
    ReportOptions, SendTemplateBody, SmsConfig,
};
use sequent_core::types::to_map::ToMap;
use sequent_core::util::temp_path::*;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::fs;
use std::path::{Path, PathBuf};
use strum_macros::{Display, EnumString, IntoStaticStr};
use tempfile::tempdir;
use tempfile::{NamedTempFile, TempPath};
use tokio::runtime::Runtime;
use tracing::{debug, info, instrument, warn};

static GLOBAL_RT: Lazy<Runtime> = Lazy::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to build global Tokio runtime")
});
#[allow(non_camel_case_types)]
#[derive(Display, Serialize, Deserialize, Debug, PartialEq, Eq, Clone, EnumString)]
pub enum GenerateReportMode {
    PREVIEW,
    REAL,
}

#[derive(Debug)]
pub struct ReportOrigins {
    pub tenant_id: String,
    pub election_event_id: String,
    pub election_id: Option<String>,
    pub template_alias: Option<String>,
    pub voter_id: Option<String>,
    pub report_origin: ReportOriginatedFrom,
    pub executer_username: Option<String>,
    pub tally_session_id: Option<String>,
}

// // Note: Should be implemented once types for each id are defined.
// impl ReportOrigins {
//     pub fn new(...) -> Self {
//     }
// }

/// To signify how the report generation was triggered
#[derive(Debug, Clone, Copy)]
pub enum ReportOriginatedFrom {
    VotingPortal,
    ExportFunction,
    ReportsTab,
}

#[allow(non_camel_case_types)]
#[derive(
    Display, Serialize, Deserialize, Debug, PartialEq, Eq, Clone, EnumString, IntoStaticStr,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum EReportEncryption {
    Unencrypted,
    ConfiguredPassword,
}

pub const DEFAULT_ITEMS_PER_REPORT_LIMIT: usize = 1000;
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
    fn get_report_origin(&self) -> ReportOriginatedFrom;

    /// Can be None when a report is generated with no template assigned to it,
    /// or from other place than the reports TAB.
    fn get_initial_template_alias(&self) -> Option<String>;

    async fn count_items(
        &self,
        hasura_transaction: Option<&Transaction<'_>>,
    ) -> Result<Option<i64>> {
        Ok(None)
    }
    async fn prepare_user_data_batch(
        &self,
        hasura_transaction: Option<&Transaction<'_>>,
        keycloak_transaction: Option<&Transaction<'_>>,
        offset: &mut i64,
        limit: i64,
    ) -> Result<Self::UserData> {
        Err(anyhow!(
            "prepare_user_data_batch is not implemented for this report type"
        ))
    }

    async fn prepare_user_data(
        &self,
        hasura_transaction: &Transaction<'_>,
        keycloak_transaction: &Transaction<'_>,
    ) -> Result<Self::UserData>;
    async fn prepare_system_data(&self, rendered_user_template: String)
        -> Result<Self::SystemData>;

    /// Default implementation, can be overridden but is not recommended!.
    /// Returns None only if no template was chosen and/or none was found in DB, then TemplateRenderer will use the default template.
    ///
    /// For reports generated from Reports tab:
    /// If no initial template_alias is provided at creation of the report object, then None is returned.
    ///
    /// For Report types from the voting portal (like in ballot_receipt):
    /// No template_alias is provided (because the voter cannot choose) so the first match found in DB will be used
    /// and the UI should restrict to add only one template for that type.
    ///
    /// For reports generated from a export button:
    /// No template_alias is provided from the UI at the moment, then it must be retrieved from postgres as well.

    /// Default implementation, can be overridden in specific reports that have
    /// election_id
    #[instrument(skip(self))]
    fn get_election_id(&self) -> Option<String> {
        None
    }

    /// Send email if it's a cron job (scheduled task) or if a voterId is present
    #[instrument(skip(self))]
    fn should_send_email(&self, is_scheduled_task: bool) -> bool {
        is_scheduled_task || self.get_voter_id().is_some()
    }

    // Default implementation, can be overridden in specific reports that have
    // voterId
    #[instrument(skip(self))]
    fn get_voter_id(&self) -> Option<String> {
        None
    }

    #[instrument(err, skip(self))]
    async fn prepare_preview_data(&self) -> Result<Self::UserData> {
        println!("!!!!!prepare_preview_data");
        let json_data = self
            .get_preview_data_file()
            .await
            .map_err(|e| anyhow::anyhow!(format!("Error preparing report preview {e:?}")))?;

        let data: Self::UserData = deserialize_str(&json_data)?;

        Ok(data)
    }

    #[instrument(err, skip(self, hasura_transaction))]
    async fn get_custom_user_template_data(
        &self,
        hasura_transaction: &Transaction<'_>,
    ) -> Result<Option<SendTemplateBody>> {
        let report_type = &self.get_report_type();
        let election_id = self.get_election_id();

        // Get the template by ID and return its value:
        let report_template_alias = get_template_alias_for_report(
            hasura_transaction,
            &self.get_tenant_id(),
            &self.get_election_event_id(),
            report_type,
            election_id.as_deref(),
        )
        .await
        .with_context(|| "Error getting template alias for report")?;
        info!("template_alias: {:?}", &report_template_alias);

        let template_alias = match report_template_alias {
            Some(alias) => alias,
            None => {
                warn!("No template alias was found for report type: {report_type} when trying to get the custom user template.");
                return Ok(None);
            }
        };

        let template_table_opt = template::get_template_by_alias(
            hasura_transaction,
            &self.get_tenant_id(),
            &template_alias,
        )
        .await
        .with_context(|| "Error getting template by id")?;

        // Template table has a column with the same name "Template" which stores a Value,
        // being its atributes: document, sms, pdf_options, etc.
        match template_table_opt {
            Some(template_tbl) => {
                let template_data: SendTemplateBody = deserialize_value(template_tbl.template)
                    .map_err(|e| {
                        anyhow!(format!("Error deserializing custom user template: {e:?}"))
                    })?;
                Ok(Some(template_data))
            }
            None => {
                warn!("No {} template was found by id", self.base_name());
                return Ok(None);
            }
        }
    }

    /// Get the default ReportExtraConfig from the _extra_config file and
    /// for any passed option that is None its default value is filled.
    #[instrument(err, skip_all)]
    async fn fill_extra_config_with_default(
        &self,
        tpl_pdf_options: Option<PrintToPdfOptionsLocal>,
        tpl_report_options: Option<ReportOptions>,
        tpl_email_config: Option<EmailConfig>,
        tpl_sms_config: Option<SmsConfig>,
    ) -> Result<ReportExtraConfig> {
        let (pdf_options, report_options, email_config, sms_config) = match tpl_pdf_options
            .is_none()
            || tpl_report_options.is_none()
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
                    tpl_report_options.unwrap_or(def_ext_cfg.report_options),
                    tpl_email_config.unwrap_or(def_ext_cfg.communication_templates.email_config),
                    tpl_sms_config.unwrap_or(def_ext_cfg.communication_templates.sms_config),
                )
            }
            false => (
                tpl_pdf_options.unwrap_or_default(),
                tpl_report_options.unwrap_or_default(),
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
            report_options,
        })
    }

    #[instrument(err, skip(self))]
    async fn get_default_user_template(&self) -> Result<String> {
        let base_name = self.base_name();
        get_public_asset_template(format!("{base_name}_user.hbs").as_str()).await
    }

    #[instrument(err, skip(self))]
    async fn get_system_template(&self) -> Result<String> {
        let base_name = self.base_name();
        get_public_asset_template(format!("{base_name}_system.hbs").as_str()).await
    }

    #[instrument(err, skip(self))]
    async fn get_preview_data_file(&self) -> Result<String> {
        let base_name = self.base_name();
        info!("base_name: {}", &base_name);
        get_public_asset_template(format!("{base_name}.json").as_str()).await
    }

    #[instrument(err, skip(self))]
    async fn get_default_extra_config_file(&self) -> Result<String> {
        let base_name = self.base_name();
        get_public_asset_template(format!("{base_name}_extra_config.json").as_str()).await
    }

    /// Read the default extra config for this template's type like PDF options and communication templates.
    #[instrument(err, skip(self))]
    async fn get_default_extra_config(&self) -> Result<ReportExtraConfig> {
        let json_data = self
            .get_default_extra_config_file()
            .await
            .map_err(|e| anyhow::anyhow!(format!("Error to get the extra config data {e:?}")))?;
        let data: ReportExtraConfig = serde_json::from_str(&json_data)?;

        Ok(data)
    }

    #[instrument(err, skip_all)]
    async fn generate_report_inner(
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

        debug!("user data in template renderer: {user_data_map:#?}");
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
            .map_err(|e| anyhow!("Error getting the system template: {e:?}"))?;

        let rendered_system_template = reports::render_template_text(&system_template, system_data)
            .map_err(|e| anyhow!("Error rendering system template: {e:?}"))?;

        Ok(rendered_system_template)
    }

    #[instrument(err, skip_all)]
    async fn generate_report(
        &self,
        generate_mode: GenerateReportMode,
        hasura_transaction: &Transaction<'_>,
        keycloak_transaction: &Transaction<'_>,
        user_tpl_document: &str,
        offset: &mut Option<i64>,
        limit: Option<i64>,
    ) -> Result<String> {
        // Prepare user data either preview or real
        let user_data = if generate_mode == GenerateReportMode::PREVIEW {
            // Increase offset when using batching
            if let Some(o) = offset {
                *o += 1;
            }
            self.prepare_preview_data()
                .await
                .map_err(|e| anyhow!("Error preparing preview user data: {e:?}"))?
        } else {
            if let (Some(o), Some(l)) = (offset, limit) {
                info!("Batched processing: offset = {o}, limit = {l}");
                self.prepare_user_data_batch(
                    Some(hasura_transaction),
                    Some(keycloak_transaction),
                    o,
                    l,
                )
                .await
                .map_err(|e| anyhow!("Error preparing batched user data: {e:?}"))?
            } else {
                self.prepare_user_data(hasura_transaction, keycloak_transaction)
                    .await
                    .map_err(|e| anyhow!("Error preparing user data: {e:?}"))?
            }
        };

        let user_data_map = user_data
            .to_map()
            .map_err(|e| anyhow!("Error converting user data to map: {e:?}"))?;

        debug!("user data in template renderer: {user_data_map:#?}");

        let rendered_user_template =
            reports::render_template_text(user_tpl_document, user_data_map)
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

    /// Provides the User template String and the ReportExtraConfig, encapsulating the logic that gets either the custom or default.
    /// Tries to get first the custom template and extra config, if any value is not available then its default is set.
    #[instrument(err, skip_all)]
    async fn user_tpl_and_extra_cfg_provider(
        &self,
        hasura_transaction: &Transaction<'_>,
    ) -> Result<(String, ReportExtraConfig)> {
        // Do the query to get the user template data
        let template_data_opt: Option<SendTemplateBody> = self
            .get_custom_user_template_data(hasura_transaction)
            .await
            .map_err(|e| anyhow!("Error getting custom user template: {e:?}"))?;
        // Set the data from the user
        let (mut tpl_pdf_options, mut tpl_report_options, mut tpl_email, mut tpl_sms) =
            (None, None, None, None);
        let user_tpl_document = match template_data_opt {
            Some(template) => {
                tpl_pdf_options = template.pdf_options;
                tpl_report_options = template.report_options;
                tpl_email = template.email;
                tpl_sms = template.sms;
                Some(template.document.unwrap_or_default())
            }
            None => None,
        };
        // Fill extra config if needed with default data
        let ext_cfg: ReportExtraConfig = self
            .fill_extra_config_with_default(tpl_pdf_options, tpl_report_options, tpl_email, tpl_sms)
            .await
            .map_err(|e| anyhow!("Error getting the extra config: {e:?}"))?;
        debug!("Extra config read: {ext_cfg:?}");

        // Get the default user template document if needed
        let user_tpl_document = match user_tpl_document {
            None => self
                .get_default_user_template()
                .await
                .map_err(|e| anyhow!("Error getting default user template: {e:?}"))?,
            Some(user_tpl_document) => user_tpl_document,
        };
        Ok((user_tpl_document, ext_cfg))
    }

    // Inner implementation for `execute_report()` so that implementors of the
    // trait can reimplement the function while calling the parent default
    // implementation too when needed
    #[instrument(err, skip_all)]
    async fn execute_report_inner(
        &self,
        document_id: &str,
        tenant_id: &str,
        election_event_id: &str,
        is_scheduled_task: bool,
        recipients: Vec<String>,
        generate_mode: GenerateReportMode,
        report: Option<Report>,
        hasura_transaction: &Transaction<'_>,
        keycloak_transaction: &Transaction<'_>,
        task_execution: Option<TasksExecution>,
    ) -> Result<()> {
        let task_execution_ref = task_execution.as_ref();
        let (user_tpl_document, ext_cfg) = self
            .user_tpl_and_extra_cfg_provider(hasura_transaction)
            .await
            .map_err(|e| {
                if let Some(task) = task_execution_ref {
                    // Using block_on here is acceptable since this call is outside our batch pool.
                    block_on(update_fail(
                        task,
                        &format!("Failed to provide user template and extra config: {e:?}"),
                    ))
                    .ok();
                }
                anyhow!("Error providing the user template and extra config: {e:?}")
            })?;

        let items_count = self
            .count_items(Some(&hasura_transaction))
            .await?
            .unwrap_or(0);
        let report_options = ext_cfg.report_options.clone();
        let per_report_limit = report_options
            .max_items_per_report
            .unwrap_or(DEFAULT_ITEMS_PER_REPORT_LIMIT) as i64;

        info!("Items count: {items_count}, per report limit: {per_report_limit}");
        let zip_temp_dir = tempdir()?;
        let zip_temp_dir_path = zip_temp_dir.path();

        // TODO: move this out of template_renderer, because that's why we have
        // execute_report_inner() separated from execute_report()
        let (final_file_path, file_size, final_report_name, mimetype) = if self.get_report_type()
            == ReportType::ACTIVITY_LOGS
            && generate_mode == GenerateReportMode::REAL
        {
            info!(
                "Using batched processing because it's activity log: items_count ({}) > per_report_limit ({})",
                items_count, per_report_limit
            );

            // Calculate the number of batches needed.
            let num_batches =
                std::cmp::max((items_count + per_report_limit - 1) / per_report_limit, 1);
            info!("Number of batches: {:?}", num_batches);

            // Define a temporary reports folder (this folder will later be compressed)
            let temp_dir = tempdir()?;
            let reports_folder = temp_dir.path();

            // Build a Rayon pool for batch processing.
            let batch_pool = ThreadPoolBuilder::new()
                .num_threads(report_options.max_threads.unwrap_or(get_worker_threads()))
                .build()
                .with_context(|| "Failed to build thread pool")?;

            // Process batches concurrently.
            let batch_file_paths: Vec<PathBuf> = batch_pool.install(|| {
                (0..num_batches)
                    .into_par_iter()
                    .map(|batch_index| -> Result<PathBuf, anyhow::Error> {
                        let offset = batch_index * per_report_limit;
                        let rendered_system_template = GLOBAL_RT
                            .block_on(async {
                                self.generate_report(
                                    generate_mode.clone(),
                                    hasura_transaction,
                                    keycloak_transaction,
                                    &user_tpl_document,
                                    &mut Some(offset),
                                    Some(per_report_limit),
                                )
                                .await
                            })
                            .with_context(|| {
                                format!("Error rendering report for batch {}", offset)
                            })?;

                        // Render to PDF bytes
                        let pdf_bytes = GLOBAL_RT
                            .block_on(async {
                                pdf::PdfRenderer::render_pdf(
                                    rendered_system_template,
                                    Some(ext_cfg.pdf_options.to_print_to_pdf_options()),
                                )
                                .await
                            })
                            .with_context(|| format!("Error rendering PDF for batch {}", offset))?;

                        let prefix = self.prefix();
                        let extension_suffix = "pdf";
                        let file_suffix = format!(".{}", extension_suffix);

                        let batch_file_name = format!("{}-{}{}", prefix, offset, file_suffix);
                        info!(
                            "Batch {} => batch_file_name: {}",
                            batch_index, batch_file_name
                        );

                        // Build the final path inside `reports_folder`:
                        let final_path = reports_folder.join(&batch_file_name);

                        fs::write(&final_path, &pdf_bytes)?;
                        Ok(final_path)
                    })
                    .collect::<Result<Vec<PathBuf>, anyhow::Error>>()
            })?;

            // Now you have a `Vec<PathBuf>` of all the PDFs created in parallel.
            let some_paths = batch_file_paths.into_iter().take(10).collect::<Vec<_>>();
            info!("first 10 batch_file_paths = {:?}", some_paths);

            let zip_filename = format!("{}_final.zip", self.prefix());

            let dst_zip = zip_temp_dir_path.join(&zip_filename);

            compress_folder_to_zip(reports_folder, &dst_zip)
                .with_context(|| "Error compressing folder")?;

            let zip_file_size = get_file_size(&dst_zip.to_string_lossy())
                .with_context(|| "Error obtaining file size for zip file")?;

            (
                dst_zip.to_string_lossy().to_string(),
                zip_file_size,
                zip_filename,
                "application/zip".to_string(),
            )
        } else {
            // All other report types
            self.generate_single_report(
                hasura_transaction,
                keycloak_transaction,
                &user_tpl_document,
                generate_mode,
                task_execution.clone(),
                &ext_cfg,
            )
            .await
            .map_err(|e| anyhow::anyhow!("Error in generate_single_report: {}", e))?
        };

        info!(
            "Final file info: path = {}, size = {}, name = {}, mimetype = {}",
            final_file_path, file_size, final_report_name, mimetype
        );

        let encrypted_temp_data: Option<TempPath> = if let Some(report) = &report {
            if report.encryption_policy == EReportEncryption::ConfiguredPassword {
                let secret_key =
                    get_report_secret_key(&tenant_id, &election_event_id, Some(report.id.clone()));
                let encryption_password = vault::read_secret(
                    hasura_transaction,
                    tenant_id,
                    Some(election_event_id),
                    &secret_key,
                )
                .await?
                .ok_or_else(|| anyhow!("Encryption password not found"))?;

                let enc_file: NamedTempFile =
                    generate_temp_file(self.base_name().as_str(), ".epdf")
                        .with_context(|| "Error creating named temp file")?;

                let enc_temp_path = enc_file.into_temp_path();
                let encrypted_temp_path = enc_temp_path.to_string_lossy().to_string();

                encrypt_file_aes_256_cbc(
                    &final_file_path,
                    &encrypted_temp_path,
                    &encryption_password,
                )
                .map_err(|err| anyhow!("Error encrypting file: {err:?}"))?;

                Some(enc_temp_path)
            } else {
                None
            }
        } else {
            None
        };

        if let Some(enc_temp_path) = encrypted_temp_data {
            let encrypted_temp_path = enc_temp_path.to_string_lossy().to_string();
            let enc_temp_size = get_file_size(encrypted_temp_path.as_str())
                .with_context(|| "Error obtaining file size")?;
            let enc_report_name: String = format!("{}.epdf", self.prefix());
            let _document = upload_and_return_document(
                hasura_transaction,
                &encrypted_temp_path,
                enc_temp_size,
                &mimetype,
                tenant_id,
                Some(election_event_id.to_string()),
                &enc_report_name,
                Some(document_id.to_string()),
                true,
            )
            .await
            .map_err(|err| anyhow!("Error uploading document: {err:?}"))?;

            if self.should_send_email(is_scheduled_task) {
                let email_config = ext_cfg.communication_templates.email_config;
                let email_recipients = self
                    .get_email_recipients(recipients, tenant_id, election_event_id)
                    .await
                    .map_err(|err| anyhow!("Error getting email receiver: {err:?}"))?;
                let email_sender = EmailSender::new()
                    .await
                    .map_err(|e| anyhow!(format!("Error getting email sender {e:?}")))?;
                let enc_report_bytes = read_temp_path(&enc_temp_path)?;
                email_sender
                    .send(
                        email_recipients,
                        email_config.subject,
                        email_config.plaintext_body,
                        email_config.html_body,
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
                hasura_transaction,
                &final_file_path,
                file_size,
                &mimetype,
                tenant_id,
                Some(election_event_id.to_string()),
                &final_report_name,
                Some(document_id.to_string()),
                true,
            )
            .await
            .map_err(|err| anyhow!("Error uploading document: {err:?}"))?;

            if self.should_send_email(is_scheduled_task) {
                let email_config = ext_cfg.communication_templates.email_config;
                let email_recipients = self
                    .get_email_recipients(recipients, tenant_id, election_event_id)
                    .await
                    .map_err(|err| anyhow!("Error getting email receiver: {err:?}"))?;
                let email_sender = EmailSender::new()
                    .await
                    .map_err(|e| anyhow!(format!("Error getting email sender {e:?}")))?;
                let final_file_bytes = std::fs::read(&final_file_path)
                    .map_err(|e| anyhow!("Error reading final file: {e:?}"))?;
                email_sender
                    .send(
                        email_recipients,
                        email_config.subject,
                        email_config.plaintext_body,
                        email_config.html_body,
                        vec![Attachment {
                            filename: final_report_name,
                            mimetype: mimetype,
                            content: final_file_bytes,
                        }],
                    )
                    .await
                    .map_err(|err| anyhow!("Error sending email: {err:?}"))?;
            }
        }

        if let Some(task) = task_execution_ref {
            update_complete(task, Some(document_id.to_string()))
                .await
                .context("Failed to update task execution status to COMPLETED")?;
        }

        Ok(())
    }

    async fn generate_single_report(
        &self,
        hasura_transaction: &Transaction<'_>,
        keycloak_transaction: &Transaction<'_>,
        user_tpl_document: &str,
        generate_mode: GenerateReportMode,
        task_execution: Option<TasksExecution>,
        ext_cfg: &ReportExtraConfig,
    ) -> Result<(String, u64, String, String)> {
        let rendered_system_template = match self
            .generate_report(
                generate_mode,
                hasura_transaction,
                keycloak_transaction,
                &user_tpl_document,
                &mut None,
                None,
            )
            .await
        {
            Ok(template) => template,
            Err(err) => {
                if let Some(task) = task_execution.as_ref() {
                    update_fail(task, &format!("Failed to generate report {err:?}"))
                        .await
                        .ok();
                }
                return Err(anyhow!("Error rendering report: {err:?}"));
            }
        };

        debug!("Report generated: {rendered_system_template}");
        let extension_suffix = "pdf";
        let content_bytes = pdf::PdfRenderer::render_pdf(
            rendered_system_template.clone(),
            Some(ext_cfg.pdf_options.to_print_to_pdf_options()),
        )
        .await
        .map_err(|err| anyhow!("Error rendering report to pdf: {err:?}"))?;

        let fmt_extension = format!(".{extension_suffix}");
        let report_name = format!("{}{}", self.prefix(), fmt_extension);

        let final_path = format!("/tmp/{}", report_name);
        fs::write(&final_path, &content_bytes)?;
        let file_size =
            get_file_size(&final_path).with_context(|| "Error obtaining file size for zip file")?;

        Ok((
            final_path,
            file_size,
            report_name.clone(),
            format!("application/{}", extension_suffix),
        ))
    }

    #[instrument(err, skip_all)]
    async fn execute_report(
        &self,
        document_id: &str,
        tenant_id: &str,
        election_event_id: &str,
        is_scheduled_task: bool,
        recipients: Vec<String>,
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
            generate_mode,
            report,
            hasura_transaction,
            keycloak_transaction,
            task_execution,
        )
        .await
    }

    #[instrument(err, skip(self))]
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
