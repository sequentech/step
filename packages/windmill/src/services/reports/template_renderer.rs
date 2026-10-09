// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::document_visibility::DocumentVisibility;
use super::generation::{
    attach_report_manifest, bundle, copy_template_data, delivered_manifest, manifest_log,
    report_copies, task_with_log, write_report_manifest, Delivery, GeneratedFile, ReportCopy,
    ReportRequester,
};
use super::pdf_copies::merge_pdfs;
use super::report_variables::{configuration_stamp, stamp_template_data};
use super::template_time::{
    insert_template_time_variables, load_i18n_defaults, load_template_time_variables,
};
use super::utils::get_public_asset_template;
use crate::postgres::reports::{get_template_alias_for_report, Report, ReportFormat, ReportType};
use crate::postgres::signing_report_release::{
    ReleaseEncryption, ReleaseTarget, ReportEmail, ReportRelease,
};
use crate::postgres::{election_event, template};
use crate::services::celery_app::get_worker_threads;
use crate::services::consolidation::aes_256_cbc_encrypt::encrypt_file_aes_256_cbc;
use crate::services::consolidation::zip::compress_folder_to_zip;
use crate::services::database::get_hasura_pool;
use crate::services::documents::upload_and_return_document_with_annotations;
use crate::services::providers::email_sender::{Attachment, EmailSender};
use crate::services::reports_vault::get_report_secret_key;
use crate::services::serialize_tasks_logs::append_general_log;
use crate::services::signing::pdf::{
    report_delivery, report_signature_page, report_signing_action, ReportDelivery, SigningBase,
};
use crate::services::tasks_execution::{update as update_task, update_complete, update_fail};
use crate::services::temp_path::PUBLIC_ASSETS_QRCODE_LIB;
use crate::services::vault;
use crate::services::voter_secret_attributes::{
    decrypt_user_attributes, get_secret_attribute_config, strip_undeclared_secret_attributes,
};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use deadpool_postgres::Transaction;
use futures::executor::block_on;
use futures::future::join_all;
use once_cell::sync::Lazy;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use rayon::ThreadPoolBuilder;
use sequent_core::election_config::manifest::ConfigurationStamp;
use sequent_core::serialization::deserialize_with_path::{deserialize_str, deserialize_value};
use sequent_core::services::keycloak::{self, get_event_realm, KeycloakAdminClient};
use sequent_core::services::reports::template_time_variables;
use sequent_core::services::{pdf, reports};
use sequent_core::signing::SigningAction;
use sequent_core::types::hasura::core::{DocumentAnnotations, TasksExecution};
use sequent_core::types::hasura::extra::TasksExecutionStatus;
use sequent_core::types::templates::{
    CommunicationTemplatesExtraConfig, EmailConfig, PrintToPdfOptionsLocal, ReportExtraConfig,
    ReportOptions, SendTemplateBody, SmsConfig,
};
use sequent_core::types::to_map::ToMap;
use sequent_core::util::temp_path::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fmt::Debug;
use std::fs;
use std::path::{Path, PathBuf};
use strum_macros::{Display, EnumString, IntoStaticStr};
use tempfile::tempdir;
use tempfile::{NamedTempFile, TempPath};
use tokio::runtime::Runtime;
use tracing::{debug, info, instrument, warn};
use uuid::Uuid;

const PDF_MEDIA_TYPE: &str = "application/pdf";

/// What became of a generated report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReportOutcome {
    /// Uploaded as the report's document (and mailed when due).
    Released,
    /// Its action needs signatures: kept as a signing document only, to
    /// start a signing request with. The executed request releases the
    /// signed document as `ReportRelease` says (and sends the scheduled
    /// email it holds).
    AwaitingSignatures(SigningBase, ReportRelease),
}

static GLOBAL_RT: Lazy<Runtime> = Lazy::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to build global Tokio runtime")
});

/// Return the encrypted voter attributes explicitly declared by the template
/// currently assigned to a report type. Callers use this before enqueueing a
/// report so permission checks happen in the authenticated request, while the
/// worker independently validates the declaration against the realm profile.
pub async fn get_declared_report_secret_attribute_names(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    report_type: &ReportType,
    election_id: Option<&str>,
) -> Result<HashSet<String>> {
    let Some(template_alias) = get_template_alias_for_report(
        hasura_transaction,
        tenant_id,
        election_event_id,
        report_type,
        election_id,
    )
    .await
    .context("Error getting template alias for report")?
    else {
        return Ok(HashSet::new());
    };
    let Some(template) =
        template::get_template_by_alias(hasura_transaction, tenant_id, &template_alias)
            .await
            .context("Error getting report template")?
    else {
        return Ok(HashSet::new());
    };
    let body: SendTemplateBody =
        deserialize_value(template.template).context("Error deserializing report template")?;
    Ok(body.secret_attribute_names.into_iter().collect())
}
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

// Moved to sequent_core::election_config so the import writers and the importer
// agree on the wire form. Re-exported: windmill refers to it by this path.
pub use sequent_core::election_config::EReportEncryption;

pub const DEFAULT_ITEMS_PER_REPORT_LIMIT: usize = 1000;

/// The formats a generation in `mode` writes of those its report asks for.
/// A preview shows the report's design with sample data: its PDF only.
pub fn generation_formats_in(
    mode: &GenerateReportMode,
    formats: Vec<ReportFormat>,
) -> Vec<ReportFormat> {
    match mode {
        GenerateReportMode::PREVIEW => vec![ReportFormat::Pdf],
        GenerateReportMode::REAL => formats,
    }
}

/// The copies a generation prints: those of its report, and one for a
/// generation that has no report, such as an export.
pub fn generation_copies(report: Option<&Report>) -> u32 {
    report.map(Report::copy_count).unwrap_or(1)
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
    fn get_report_origin(&self) -> ReportOriginatedFrom;

    fn contains_sensitive_data(&self) -> bool {
        false
    }

    /// The protected action whose signatures this report needs when the
    /// event's rule asks for them; the report then gets a signature page.
    /// Who asked for the report, when its task does not say.
    fn requested_by(&self) -> Option<String> {
        None
    }

    fn signing_action(&self) -> Option<SigningAction> {
        report_signing_action(&self.get_report_type())
    }

    /// Can be None when a report is generated with no template assigned to it,
    /// or from other place than the reports TAB.
    fn get_initial_template_alias(&self) -> Option<String>;

    async fn count_items(&self, hasura_transaction: &Transaction<'_>) -> Result<Option<i64>> {
        Ok(None)
    }
    async fn prepare_user_data_batch(
        &self,
        hasura_transaction: &Transaction<'_>,
        keycloak_transaction: &Transaction<'_>,
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

    /// Add the canonical `user` variable object for a true per-voter report.
    /// Stored ciphertext is never added to the rendering map: undeclared
    /// fields are absent and declared fields are decrypted only here.
    #[instrument(err, skip_all)]
    async fn inject_voter_secret_variables(
        &self,
        user_data_map: &mut Map<String, Value>,
        declared_names: &HashSet<String>,
        may_read_secret_attributes: bool,
    ) -> Result<()> {
        if declared_names.is_empty() {
            return Ok(());
        }
        if !may_read_secret_attributes {
            return Err(anyhow!(
                "Generating this voter report requires voter-secret-attribute-read"
            ));
        }
        let voter_id = self.get_voter_id().ok_or_else(|| {
            anyhow!("Encrypted voter attributes are only supported by per-voter reports")
        })?;
        let tenant_id = self.get_tenant_id();
        let election_event_id = self.get_election_event_id();
        let realm = get_event_realm(&tenant_id, &election_event_id);
        let configured_names = get_secret_attribute_config(&tenant_id, &election_event_id)
            .await
            .context("Error reading the secret-attribute configuration for voter report")?
            .validated_names()?;
        let client = KeycloakAdminClient::new()
            .await
            .context("Error initializing Keycloak client for voter report")?;
        if let Some(name) = declared_names
            .iter()
            .find(|name| !configured_names.contains(*name))
        {
            return Err(anyhow!(
                "Report declares `{name}`, which is not configured as an encrypted voter attribute"
            ));
        }

        let mut voter = client
            .get_user(&realm, &voter_id)
            .await
            .context("Error reading voter for voter report")?;
        decrypt_user_attributes(&mut voter, &tenant_id, &election_event_id, declared_names).await?;
        strip_undeclared_secret_attributes(&mut voter, &configured_names, declared_names);
        let attributes = voter.attributes.unwrap_or_default();
        let mut user_variables = Map::new();
        user_variables.insert("first_name".to_string(), json!(voter.first_name));
        user_variables.insert("last_name".to_string(), json!(voter.last_name));
        user_variables.insert("username".to_string(), json!(voter.username));
        user_variables.insert("email".to_string(), json!(voter.email));
        for (name, values) in &attributes {
            if let Some(value) = values.first() {
                user_variables
                    .entry(name.clone())
                    .or_insert_with(|| json!(value));
            }
        }
        user_variables.insert("attributes".to_string(), json!(attributes));
        user_data_map.insert("user".to_string(), Value::Object(user_variables));
        Ok(())
    }

    #[instrument(err, skip(self))]
    /// The timezone variables of the report's event and election
    /// (`electionEventTimezone`, `electionTimezone`, `timezoneTexts`).
    async fn inject_time_variables(
        &self,
        hasura_transaction: &Transaction<'_>,
        user_data_map: &mut Map<String, Value>,
    ) -> Result<()> {
        let election_event_id = self.get_election_event_id();
        let time_variables = if election_event_id.is_empty() {
            template_time_variables(None, None, &*load_i18n_defaults().await)
        } else {
            load_template_time_variables(
                hasura_transaction,
                &self.get_tenant_id(),
                &election_event_id,
                self.get_election_id().as_deref(),
            )
            .await?
        };
        insert_template_time_variables(user_data_map, time_variables);
        Ok(())
    }

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
        let data: ReportExtraConfig = deserialize_str(&json_data)?;

        Ok(data)
    }

    #[instrument(err, skip_all)]
    async fn generate_report_inner(
        &self,
        generate_mode: GenerateReportMode,
        hasura_transaction: &Transaction<'_>,
        keycloak_transaction: &Transaction<'_>,
        user_tpl_document: &str,
        declared_secret_names: &HashSet<String>,
        may_read_secret_attributes: bool,
        stamp: Option<&ConfigurationStamp>,
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

        let mut user_data_map = user_data
            .to_map()
            .map_err(|e| anyhow!("Error converting user data to map: {e:?}"))?;
        self.inject_time_variables(hasura_transaction, &mut user_data_map)
            .await?;
        if generate_mode == GenerateReportMode::REAL {
            self.inject_voter_secret_variables(
                &mut user_data_map,
                declared_secret_names,
                may_read_secret_attributes,
            )
            .await?;
        }
        if let Some(stamp) = stamp {
            stamp_template_data(&mut user_data_map, stamp);
        }
        let rendered_user_template =
            reports::render_template_text(&user_tpl_document, user_data_map)
                .map_err(|e| anyhow!("Error rendering user template: {e:?}"))?;

        // Prepare system data
        let mut system_data = self
            .prepare_system_data(rendered_user_template)
            .await
            .map_err(|e| anyhow!("Error preparing system data: {e:?}"))?
            .to_map()
            .map_err(|e| anyhow!("Error converting system data to map: {e:?}"))?;
        if let Some(stamp) = stamp {
            stamp_template_data(&mut system_data, stamp);
        }
        let system_template = self
            .get_system_template()
            .await
            .map_err(|e| anyhow!("Error getting the system template: {e:?}"))?;

        let rendered_system_template = reports::render_template_text(&system_template, system_data)
            .map_err(|e| anyhow!("Error rendering system template: {e:?}"))?;

        Ok(rendered_system_template)
    }

    /// The data a report's user template is drawn with: the report's own,
    /// the time variables, a voter's declared secrets and the stamp. It is
    /// read once for a generation, however many copies it prints.
    #[instrument(err, skip_all)]
    async fn prepare_report_data(
        &self,
        generate_mode: GenerateReportMode,
        hasura_transaction: &Transaction<'_>,
        keycloak_transaction: &Transaction<'_>,
        declared_secret_names: &HashSet<String>,
        may_read_secret_attributes: bool,
        stamp: Option<&ConfigurationStamp>,
        offset: &mut Option<i64>,
        limit: Option<i64>,
    ) -> Result<Map<String, Value>> {
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
                self.prepare_user_data_batch(hasura_transaction, keycloak_transaction, o, l)
                    .await
                    .map_err(|e| anyhow!("Error preparing batched user data: {e:?}"))?
            } else {
                self.prepare_user_data(hasura_transaction, keycloak_transaction)
                    .await
                    .map_err(|e| anyhow!("Error preparing user data: {e:?}"))?
            }
        };

        let mut user_data_map = user_data
            .to_map()
            .map_err(|e| anyhow!("Error converting user data to map: {e:?}"))?;
        self.inject_time_variables(hasura_transaction, &mut user_data_map)
            .await?;
        if generate_mode == GenerateReportMode::REAL {
            self.inject_voter_secret_variables(
                &mut user_data_map,
                declared_secret_names,
                may_read_secret_attributes,
            )
            .await?;
        }
        if let Some(stamp) = stamp {
            stamp_template_data(&mut user_data_map, stamp);
        }
        Ok(user_data_map)
    }

    /// The report's HTML: the user template drawn with `user_data_map`
    /// inside the system template. A report printed in several copies is
    /// drawn once for each, with the copy named in its data.
    #[instrument(err, skip_all)]
    async fn render_report_data(
        &self,
        user_tpl_document: &str,
        mut user_data_map: Map<String, Value>,
        stamp: Option<&ConfigurationStamp>,
        copy: Option<ReportCopy>,
    ) -> Result<String> {
        if let Some(copy) = copy {
            copy_template_data(&mut user_data_map, copy);
        }
        let rendered_user_template =
            reports::render_template_text(user_tpl_document, user_data_map)
                .map_err(|e| anyhow!("Error rendering user template: {e:?}"))?;

        // Prepare system data
        let mut system_data = self
            .prepare_system_data(rendered_user_template)
            .await
            .map_err(|e| anyhow!("Error preparing system data: {e:?}"))?
            .to_map()
            .map_err(|e| anyhow!("Error converting system data to map: {e:?}"))?;
        if let Some(stamp) = stamp {
            stamp_template_data(&mut system_data, stamp);
        }
        if let Some(copy) = copy {
            copy_template_data(&mut system_data, copy);
        }

        let system_template = self
            .get_system_template()
            .await
            .map_err(|e| anyhow!("Error getting default user template: {e:?}"))?;

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
        declared_secret_names: &HashSet<String>,
        may_read_secret_attributes: bool,
        stamp: Option<&ConfigurationStamp>,
        offset: &mut Option<i64>,
        limit: Option<i64>,
    ) -> Result<String> {
        let user_data_map = self
            .prepare_report_data(
                generate_mode,
                hasura_transaction,
                keycloak_transaction,
                declared_secret_names,
                may_read_secret_attributes,
                stamp,
                offset,
                limit,
            )
            .await?;
        self.render_report_data(user_tpl_document, user_data_map, stamp, None)
            .await
    }

    /// The report as one PDF: its `copies` one after another, each drawn
    /// from the same data.
    #[instrument(err, skip_all)]
    async fn render_report_pdf(
        &self,
        user_tpl_document: &str,
        user_data_map: Map<String, Value>,
        stamp: Option<&ConfigurationStamp>,
        copies: u32,
        pdf_options: &PrintToPdfOptionsLocal,
        sensitive: bool,
    ) -> Result<Vec<u8>> {
        let mut rendered: Vec<Vec<u8>> = Vec::new();
        for copy in report_copies(copies) {
            let html = self
                .render_report_data(user_tpl_document, user_data_map.clone(), stamp, copy)
                .await?;
            rendered.push(
                pdf::PdfRenderer::render_pdf_with_sensitivity(
                    html,
                    Some(pdf_options.to_print_to_pdf_options()),
                    sensitive,
                )
                .await
                .map_err(|err| anyhow!("Error rendering report to pdf: {err:?}"))?,
            );
        }
        merge_pdfs(&rendered)
    }

    /// The formats one generation writes: those its report asks for, the
    /// type's default when it asks for none. A format the type cannot be
    /// generated in is refused.
    fn output_formats(&self, report: Option<&Report>) -> Result<Vec<ReportFormat>> {
        self.get_report_type()
            .generation_formats(report.and_then(|report| report.output_formats.as_deref()))
            .map_err(|refusal| anyhow!(refusal))
    }

    /// Writes the report in a format other than PDF, in `directory`.
    /// `None`: this report is not written in that format.
    async fn write_format(
        &self,
        format: ReportFormat,
        hasura_transaction: &Transaction<'_>,
        stamp: Option<&ConfigurationStamp>,
        directory: &Path,
    ) -> Result<Option<GeneratedFile>> {
        Ok(None)
    }

    /// Provides the User template String and the ReportExtraConfig, encapsulating the logic that gets either the custom or default.
    /// Tries to get first the custom template and extra config, if any value is not available then its default is set.
    #[instrument(err, skip_all)]
    async fn user_tpl_and_extra_cfg_provider(
        &self,
        hasura_transaction: &Transaction<'_>,
    ) -> Result<(String, ReportExtraConfig, HashSet<String>)> {
        // Do the query to get the user template data
        let template_data_opt: Option<SendTemplateBody> = self
            .get_custom_user_template_data(hasura_transaction)
            .await
            .map_err(|e| anyhow!("Error getting custom user template: {e:?}"))?;
        // Set the data from the user
        let (
            mut tpl_pdf_options,
            mut tpl_report_options,
            mut tpl_email,
            mut tpl_sms,
            mut declared_secret_names,
        ) = (None, None, None, None, HashSet::new());
        let user_tpl_document = match template_data_opt {
            Some(template) => {
                declared_secret_names = template.secret_attribute_names.into_iter().collect();
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
        Ok((user_tpl_document, ext_cfg, declared_secret_names))
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
        may_read_secret_attributes: bool,
    ) -> Result<()> {
        self.execute_report_outcome(
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
            may_read_secret_attributes,
        )
        .await
        .map(|_| ())
    }

    /// [`Self::execute_report_inner`], saying whether the report was
    /// released or awaits signatures (then it is kept as a signing
    /// document only: not the report's document, not mailed).
    #[instrument(err, skip_all)]
    async fn execute_report_outcome(
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
        may_read_secret_attributes: bool,
    ) -> Result<ReportOutcome> {
        let task_execution_ref = task_execution.as_ref();
        let (user_tpl_document, ext_cfg, declared_secret_names) = self
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

        // A report of an event imported from a signed configuration is drawn
        // only with the template that configuration approved, and names it.
        let stamp = configuration_stamp(
            hasura_transaction,
            tenant_id,
            election_event_id,
            &self.get_report_type(),
            &user_tpl_document,
        )
        .await
        .map_err(|error| {
            if let Some(task) = task_execution_ref {
                block_on(update_fail(task, &error.to_string())).ok();
            }
            error
        })?;

        // The copies and formats are the report's: the Reports step set them.
        let formats = self.output_formats(report.as_ref()).map_err(|error| {
            if let Some(task) = task_execution_ref {
                block_on(update_fail(task, &error.to_string())).ok();
            }
            error
        })?;
        let formats = generation_formats_in(&generate_mode, formats);
        let copies = generation_copies(report.as_ref());

        let contains_voter_secrets =
            generate_mode == GenerateReportMode::REAL && !declared_secret_names.is_empty();
        let document_visibility =
            DocumentVisibility::for_report(&self.get_report_type(), contains_voter_secrets);
        let is_real = generate_mode == GenerateReportMode::REAL;
        let items_count = self.count_items(&hasura_transaction).await?.unwrap_or(0);
        let report_options = ext_cfg.report_options.clone();
        let per_report_limit = report_options
            .max_items_per_report
            .unwrap_or(DEFAULT_ITEMS_PER_REPORT_LIMIT) as i64;

        info!("Items count: {items_count}, per report limit: {per_report_limit}");
        let zip_temp_dir = tempdir()?;
        let zip_temp_dir_path = zip_temp_dir.path();

        // TODO: move this out of template_renderer, because that's why we have
        // execute_report_inner() separated from execute_report()
        let rendered_pdf = if !formats.contains(&ReportFormat::Pdf) {
            None
        } else if self.get_report_type() == ReportType::ACTIVITY_LOGS
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
                        let batch_data = GLOBAL_RT
                            .block_on(async {
                                self.prepare_report_data(
                                    generate_mode.clone(),
                                    hasura_transaction,
                                    keycloak_transaction,
                                    &declared_secret_names,
                                    may_read_secret_attributes,
                                    stamp.as_ref(),
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
                                self.render_report_pdf(
                                    &user_tpl_document,
                                    batch_data,
                                    stamp.as_ref(),
                                    copies,
                                    &ext_cfg.pdf_options,
                                    self.contains_sensitive_data(),
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

            Some(GeneratedFile {
                name: zip_filename,
                path: dst_zip.to_string_lossy().to_string(),
                media_type: "application/zip".to_string(),
            })
        } else {
            // All other report types
            Some(
                self.generate_single_report(
                    hasura_transaction,
                    keycloak_transaction,
                    &user_tpl_document,
                    &declared_secret_names,
                    may_read_secret_attributes,
                    stamp.as_ref(),
                    generate_mode,
                    task_execution.clone(),
                    &ext_cfg,
                    copies,
                )
                .await
                .map_err(|e| anyhow::anyhow!("Error in generate_single_report: {}", e))?,
            )
        };

        // The report in each of its formats: its PDF, then the others.
        let mut generated: Vec<GeneratedFile> = rendered_pdf.into_iter().collect();
        for format in formats
            .iter()
            .filter(|format| **format != ReportFormat::Pdf)
        {
            generated.push(
                self.write_format(
                    *format,
                    hasura_transaction,
                    stamp.as_ref(),
                    zip_temp_dir_path,
                )
                .await?
                .ok_or_else(|| {
                    anyhow!(
                        "The {} report cannot be generated in {format}",
                        self.get_report_type()
                    )
                })?,
            );
        }

        // For an event imported from a signed configuration, the hash
        // manifest of what was just written.
        let report_manifest = stamp
            .as_ref()
            .map(|stamp| write_report_manifest(&self.get_report_type(), stamp, &generated))
            .transpose()?;
        let delivered = bundle(
            &self.prefix(),
            &generated,
            report_manifest.as_ref(),
            zip_temp_dir_path,
        )?;
        let (final_file_path, final_report_name, mimetype) = (
            delivered.path.clone(),
            delivered.name.clone(),
            delivered.media_type.clone(),
        );
        let requester = ReportRequester::named(
            self.requested_by()
                .or_else(|| task_execution_ref.map(|task| task.executed_by_user.clone())),
        );
        let file_size =
            get_file_size(&final_file_path).with_context(|| "Error obtaining the report's size")?;

        let mut annotations = if contains_voter_secrets {
            DocumentAnnotations::voter_secret_export()
        } else {
            DocumentAnnotations::default()
        };

        // A report whose action needs signatures gets its signature page,
        // and is kept as a signing document only.
        let signing_action = self
            .signing_action()
            .filter(|_| is_real && mimetype == PDF_MEDIA_TYPE);
        let signing_base = match signing_action {
            Some(action) => {
                let rendered = fs::read(&final_file_path)
                    .with_context(|| "Error reading the report to add its signature page")?;
                let tenant_uuid = Uuid::parse_str(tenant_id).context("Invalid tenant id")?;
                let event_uuid = Uuid::parse_str(election_event_id).context("Invalid event id")?;
                report_signature_page(
                    hasura_transaction,
                    tenant_uuid,
                    event_uuid,
                    action,
                    &rendered,
                )
                .await?
                .map(|base| (action, base))
            }
            None => None,
        };
        let delivery = report_delivery(
            signing_base.is_some(),
            report.as_ref().is_some_and(|report| {
                report.encryption_policy == EReportEncryption::ConfiguredPassword
            }),
            self.should_send_email(is_scheduled_task),
        );
        let (encrypt, send_email) = match (delivery, signing_base) {
            (
                ReportDelivery::Release {
                    encrypt,
                    send_email,
                },
                _,
            ) => (encrypt, send_email),
            (ReportDelivery::AwaitSignatures, Some((action, base))) => {
                let name = format!("{}-to-sign.pdf", self.prefix());
                let (_temp_path, path, size) = write_into_named_temp_file(
                    &base,
                    &format!("{}-to-sign-", self.prefix()),
                    ".pdf",
                )?;
                // Its hash manifest lists the file kept to be signed. The
                // signed report gets its own at its release.
                if let Some(generated) = &report_manifest {
                    let to_sign = GeneratedFile {
                        name: name.clone(),
                        path: path.clone(),
                        media_type: PDF_MEDIA_TYPE.to_string(),
                    };
                    let held =
                        delivered_manifest(generated, &delivered, Delivery::ToSign(&to_sign))?;
                    attach_report_manifest(
                        hasura_transaction,
                        tenant_id,
                        election_event_id,
                        &held,
                        &requester,
                        &mut annotations,
                    )
                    .await?;
                }
                // Its own document, never the report's: nothing is
                // released, mailed or printed before the signatures. A
                // document with voter secrets keeps its annotation, which
                // the signing guard refuses.
                let document = upload_and_return_document_with_annotations(
                    hasura_transaction,
                    &path,
                    size,
                    PDF_MEDIA_TYPE,
                    tenant_id,
                    Some(election_event_id.to_string()),
                    &name,
                    None,
                    false,
                    &annotations,
                )
                .await
                .map_err(|err| anyhow!("Error uploading the document to sign: {err:?}"))?;
                info!(
                    "Report {} awaits {action} signatures before its release",
                    self.prefix()
                );
                // The task succeeds once the report's signing request
                // started (the report task does that).
                // The e-mail a released report would send waits for the
                // signatures with it.
                let email = if self.should_send_email(is_scheduled_task) {
                    let email_config = ext_cfg.communication_templates.email_config.clone();
                    Some(ReportEmail {
                        recipients: self
                            .get_email_recipients(recipients, tenant_id, election_event_id)
                            .await
                            .map_err(|err| anyhow!("Error getting email receiver: {err:?}"))?,
                        subject: email_config.subject,
                        plaintext_body: email_config.plaintext_body,
                        html_body: email_config.html_body,
                    })
                } else {
                    None
                };
                let release = ReportRelease {
                    report_id: report
                        .as_ref()
                        .map(|report| Uuid::parse_str(&report.id))
                        .transpose()
                        .context("The report has no UUID")?,
                    target: ReleaseTarget::Report {
                        document_id: Uuid::parse_str(document_id)
                            .context("The report's document id is no UUID")?,
                    },
                    file_name: final_report_name.clone(),
                    is_public: document_visibility.is_public(),
                    // Released wrapped in its password, as it would have been.
                    encryption: if report.as_ref().is_some_and(|report| {
                        report.encryption_policy == EReportEncryption::ConfiguredPassword
                    }) {
                        ReleaseEncryption::ConfiguredPassword
                    } else {
                        ReleaseEncryption::NoEncryption
                    },
                    email,
                };
                return Ok(ReportOutcome::AwaitingSignatures(
                    SigningBase {
                        action,
                        document_id: Uuid::parse_str(&document.id)
                            .context("The document to sign has no UUID")?,
                        sha256: hex::encode(Sha256::digest(&base)),
                    },
                    release,
                ));
            }
            (ReportDelivery::AwaitSignatures, None) => {
                return Err(anyhow!(
                    "A report awaits signatures without its signature page"
                ));
            }
        };

        info!(
            "Final file info: path = {}, size = {}, name = {}, mimetype = {}",
            final_file_path, file_size, final_report_name, mimetype
        );

        let encrypted_temp_data: Option<TempPath> = if let Some(report) = &report {
            if encrypt {
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

                // Bind the actual password used to this document. Editing the report's
                // configured password later must not break downloads of earlier reports.
                let password_secret_id = crate::services::document_password::save_password(
                    hasura_transaction,
                    tenant_id,
                    Some(election_event_id),
                    document_id,
                    &encryption_password,
                )
                .await?;
                annotations
                    .access
                    .get_or_insert_with(Default::default)
                    .password_secret_id = Some(password_secret_id);

                Some(enc_temp_path)
            } else {
                None
            }
        } else {
            None
        };

        // The hash manifest is of the file that is stored: the encrypted
        // one, when the report is wrapped in its password.
        let enc_report_name: String = format!("{}.epdf", self.prefix());
        let encrypted_file = encrypted_temp_data.as_ref().map(|path| GeneratedFile {
            name: enc_report_name.clone(),
            path: path.to_string_lossy().to_string(),
            media_type: mimetype.clone(),
        });
        let report_manifest = report_manifest
            .as_ref()
            .map(|generated| {
                delivered_manifest(
                    generated,
                    &delivered,
                    match &encrypted_file {
                        Some(stored) => Delivery::Encrypted(stored),
                        None => Delivery::AsGenerated,
                    },
                )
            })
            .transpose()?;
        if let Some(written) = &report_manifest {
            attach_report_manifest(
                hasura_transaction,
                tenant_id,
                election_event_id,
                written,
                &requester,
                &mut annotations,
            )
            .await?;
        }

        if let Some(enc_temp_path) = encrypted_temp_data {
            let encrypted_temp_path = enc_temp_path.to_string_lossy().to_string();
            let enc_temp_size = get_file_size(encrypted_temp_path.as_str())
                .with_context(|| "Error obtaining file size")?;
            let _document = upload_and_return_document_with_annotations(
                hasura_transaction,
                &encrypted_temp_path,
                enc_temp_size,
                &mimetype,
                tenant_id,
                Some(election_event_id.to_string()),
                &enc_report_name,
                Some(document_id.to_string()),
                document_visibility.is_public(),
                &annotations,
            )
            .await
            .map_err(|err| anyhow!("Error uploading document: {err:?}"))?;

            if send_email {
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
            let _document = upload_and_return_document_with_annotations(
                hasura_transaction,
                &final_file_path,
                file_size,
                &mimetype,
                tenant_id,
                Some(election_event_id.to_string()),
                &final_report_name,
                Some(document_id.to_string()),
                document_visibility.is_public(),
                &annotations,
            )
            .await
            .map_err(|err| anyhow!("Error uploading document: {err:?}"))?;

            if send_email {
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
            let task = match &report_manifest {
                Some(written) => task_with_log(task, &manifest_log(written))?,
                None => task.clone(),
            };
            update_complete(&task, Some(document_id.to_string()))
                .await
                .context("Failed to update task execution status to COMPLETED")?;
        }

        Ok(ReportOutcome::Released)
    }

    async fn generate_single_report(
        &self,
        hasura_transaction: &Transaction<'_>,
        keycloak_transaction: &Transaction<'_>,
        user_tpl_document: &str,
        declared_secret_names: &HashSet<String>,
        may_read_secret_attributes: bool,
        stamp: Option<&ConfigurationStamp>,
        generate_mode: GenerateReportMode,
        task_execution: Option<TasksExecution>,
        ext_cfg: &ReportExtraConfig,
        copies: u32,
    ) -> Result<GeneratedFile> {
        let rendered = async {
            let user_data_map = self
                .prepare_report_data(
                    generate_mode,
                    hasura_transaction,
                    keycloak_transaction,
                    declared_secret_names,
                    may_read_secret_attributes,
                    stamp,
                    &mut None,
                    None,
                )
                .await?;
            self.render_report_pdf(
                user_tpl_document,
                user_data_map,
                stamp,
                copies,
                &ext_cfg.pdf_options,
                self.contains_sensitive_data() || !declared_secret_names.is_empty(),
            )
            .await
        }
        .await;
        let content_bytes = match rendered {
            Ok(bytes) => bytes,
            Err(err) => {
                if let Some(task) = task_execution.as_ref() {
                    update_fail(task, &format!("Failed to generate report {err:?}"))
                        .await
                        .ok();
                }
                return Err(anyhow!("Error rendering report: {err:?}"));
            }
        };

        let report_name = format!("{}.{}", self.prefix(), ReportFormat::Pdf.extension());

        let final_path = format!("/tmp/{}", report_name);
        fs::write(&final_path, &content_bytes)?;

        Ok(GeneratedFile {
            name: report_name,
            path: final_path,
            media_type: PDF_MEDIA_TYPE.to_string(),
        })
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
        may_read_secret_attributes: bool,
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
            may_read_secret_attributes,
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

#[cfg(test)]
mod copies_and_formats_tests {
    use super::*;

    const USER_TEMPLATE: &str = "<h1>{{title}}</h1>\
        {{#if execution_annotations.configuration_revision}}<p>Configuration revision \
        {{execution_annotations.configuration_revision}}, manifest SHA-256 \
        {{execution_annotations.configuration_manifest_sha256}}</p>{{/if}}\
        {{#if execution_annotations.copy_total}}<p>Copy {{execution_annotations.copy_number}} \
        of {{execution_annotations.copy_total}}</p>{{/if}}";
    const SYSTEM_TEMPLATE: &str = "<main>{{{rendered_user_template}}}</main>\
        {{#if execution_annotations.copy_total}}<footer>{{execution_annotations.copy_number}}/\
        {{execution_annotations.copy_total}}</footer>{{/if}}";

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct Data {
        title: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct System {
        rendered_user_template: String,
    }

    #[derive(Debug)]
    struct Drawn(ReportType);

    #[async_trait]
    impl TemplateRenderer for Drawn {
        type UserData = Data;
        type SystemData = System;

        fn base_name(&self) -> String {
            "drawn".to_string()
        }
        fn get_report_type(&self) -> ReportType {
            self.0.clone()
        }
        fn prefix(&self) -> String {
            "drawn".to_string()
        }
        fn get_tenant_id(&self) -> String {
            String::new()
        }
        fn get_election_event_id(&self) -> String {
            String::new()
        }
        fn get_report_origin(&self) -> ReportOriginatedFrom {
            ReportOriginatedFrom::ReportsTab
        }
        fn get_initial_template_alias(&self) -> Option<String> {
            None
        }
        async fn prepare_user_data(
            &self,
            _hasura_transaction: &Transaction<'_>,
            _keycloak_transaction: &Transaction<'_>,
        ) -> Result<Data> {
            Ok(data())
        }
        async fn prepare_system_data(&self, rendered_user_template: String) -> Result<System> {
            Ok(System {
                rendered_user_template,
            })
        }
        async fn get_system_template(&self) -> Result<String> {
            Ok(SYSTEM_TEMPLATE.to_string())
        }
    }

    fn data() -> Data {
        Data {
            title: "Election Returns".to_string(),
        }
    }

    fn stamp() -> ConfigurationStamp {
        ConfigurationStamp {
            external_id: "ov-2028".to_string(),
            revision: 3,
            manifest_sha256: "ab".repeat(32),
            template_sha256: "cd".repeat(32),
        }
    }

    fn report(report_type: &ReportType, formats: Option<Vec<ReportFormat>>) -> Report {
        Report {
            id: "report".to_string(),
            election_event_id: "event".to_string(),
            tenant_id: "tenant".to_string(),
            election_id: None,
            report_type: report_type.to_string(),
            template_alias: None,
            encryption_policy: EReportEncryption::Unencrypted,
            cron_config: None,
            created_at: chrono::Utc::now(),
            permission_label: None,
            copies: None,
            output_formats: formats,
        }
    }

    #[test]
    fn a_generation_prints_the_copies_of_its_report_and_one_without_a_report() {
        let mut seven = report(&ReportType::ELECTORAL_RESULTS, None);
        seven.copies = Some(7);
        assert_eq!(generation_copies(Some(&seven)), 7);
        assert_eq!(
            generation_copies(Some(&report(&ReportType::ELECTORAL_RESULTS, None))),
            1
        );
        assert_eq!(generation_copies(None), 1);
    }

    #[test]
    fn the_task_names_who_asked_unless_the_report_does() {
        assert_eq!(Drawn(ReportType::ELECTORAL_RESULTS).requested_by(), None);
    }

    async fn drawn(stamp: Option<&ConfigurationStamp>, copy: Option<ReportCopy>) -> String {
        let renderer = Drawn(ReportType::ELECTORAL_RESULTS);
        let mut user_data = data().to_map().unwrap();
        if let Some(stamp) = stamp {
            stamp_template_data(&mut user_data, stamp);
        }
        renderer
            .render_report_data(USER_TEMPLATE, user_data, stamp, copy)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn a_report_printed_once_is_drawn_as_before() {
        assert_eq!(
            drawn(None, None).await,
            "<main><h1>Election Returns</h1></main>"
        );
        let stamped = drawn(Some(&stamp()), None).await;
        assert!(stamped.contains(&format!(
            "Configuration revision 3, manifest SHA-256 {}",
            "ab".repeat(32)
        )));
        assert!(!stamped.contains("Copy"));
        assert!(!stamped.contains("<footer>"));
    }

    #[tokio::test]
    async fn each_copy_names_itself_in_the_report_and_around_it() {
        let stamp = stamp();
        let copies = report_copies(3);
        assert_eq!(copies.len(), 3);
        for (index, copy) in copies.into_iter().enumerate() {
            let html = drawn(Some(&stamp), copy).await;
            let number = index + 1;
            assert!(
                html.contains(&format!("<p>Copy {number} of 3</p>")),
                "{html}"
            );
            assert!(
                html.contains(&format!("<footer>{number}/3</footer>")),
                "{html}"
            );
            assert!(html.contains("Configuration revision 3"), "{html}");
        }
    }

    #[test]
    fn a_generation_writes_what_its_report_asks_for() {
        let logs = Drawn(ReportType::ACTIVITY_LOGS);
        assert_eq!(logs.output_formats(None).unwrap(), vec![ReportFormat::Pdf]);
        let asked = report(
            &ReportType::ACTIVITY_LOGS,
            Some(vec![
                ReportFormat::Csv,
                ReportFormat::Sql,
                ReportFormat::Pdf,
            ]),
        );
        assert_eq!(
            logs.output_formats(Some(&asked)).unwrap(),
            vec![ReportFormat::Csv, ReportFormat::Sql, ReportFormat::Pdf]
        );

        let returns = Drawn(ReportType::ELECTORAL_RESULTS);
        let with_xml = report(
            &ReportType::ELECTORAL_RESULTS,
            Some(vec![ReportFormat::Pdf, ReportFormat::Xml]),
        );
        assert_eq!(
            returns.output_formats(Some(&with_xml)).unwrap(),
            vec![ReportFormat::Pdf]
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    #[ignore = "renders PDFs: needs a browser"]
    async fn the_pdf_holds_the_copies_one_after_another() {
        let renderer = Drawn(ReportType::ELECTORAL_RESULTS);
        let stamp = stamp();
        let mut user_data = data().to_map().unwrap();
        stamp_template_data(&mut user_data, &stamp);
        let pdf_of = |copies: u32| {
            let (renderer, stamp, user_data) = (&renderer, &stamp, user_data.clone());
            async move {
                renderer
                    .render_report_pdf(
                        USER_TEMPLATE,
                        user_data,
                        Some(stamp),
                        copies,
                        &PrintToPdfOptionsLocal::default(),
                        false,
                    )
                    .await
                    .unwrap()
            }
        };

        let once = lopdf::Document::load_mem(&pdf_of(1).await).unwrap();
        assert_eq!(once.get_pages().len(), 1);
        assert!(!once.extract_text(&[1]).unwrap().contains("Copy"));

        let copies = lopdf::Document::load_mem(&pdf_of(3).await).unwrap();
        assert_eq!(copies.get_pages().len(), 3);
        for page in 1..=3u32 {
            let text = copies.extract_text(&[page]).unwrap();
            assert!(text.contains(&format!("Copy {page} of 3")), "{text}");
            assert!(text.contains("revision 3, manifest SHA-256"), "{text}");
        }
    }

    #[test]
    fn a_preview_draws_the_pdf_whatever_the_report_asks_for() {
        let asked = vec![ReportFormat::Csv, ReportFormat::Sql];
        assert_eq!(
            generation_formats_in(&GenerateReportMode::PREVIEW, asked.clone()),
            vec![ReportFormat::Pdf]
        );
        assert_eq!(
            generation_formats_in(&GenerateReportMode::REAL, asked.clone()),
            asked
        );
    }

    #[test]
    fn a_format_the_report_cannot_be_generated_in_is_refused() {
        let participation = Drawn(ReportType::PARTICIPATION_REPORT);
        let asked = report(
            &ReportType::PARTICIPATION_REPORT,
            Some(vec![ReportFormat::Pdf, ReportFormat::Csv]),
        );
        let refused = participation.output_formats(Some(&asked)).unwrap_err();
        assert_eq!(
            refused.to_string(),
            "the PARTICIPATION_REPORT report cannot be generated in csv: it supports pdf"
        );
    }
}
