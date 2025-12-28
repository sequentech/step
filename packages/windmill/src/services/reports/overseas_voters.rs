// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::report_variables::{
    extract_area_data, extract_election_data, extract_election_event_annotations, get_app_hash,
    get_app_version, get_date_and_time, get_report_hash, ExecutionAnnotations, InspectorData,
};
use super::template_renderer::*;
use super::voters::{count_voters_by_area_id, get_voters_data, FilterListVoters, Voter};
use crate::postgres::area::get_areas_by_election_id;
use crate::postgres::election::get_election_by_id;
use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::reports::{Report, ReportType};
use crate::postgres::scheduled_event::find_scheduled_event_by_election_event_id;
use crate::services::consolidation::zip::compress_folder_to_zip;
use crate::services::election_dates::get_election_dates;
use crate::services::tasks_execution::{update_complete, update_fail};
use crate::services::temp_path::PUBLIC_ASSETS_QRCODE_LIB;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use deadpool_postgres::Transaction;
use futures::executor::block_on;
use once_cell::sync::Lazy;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use rayon::ThreadPoolBuilder;
use sequent_core::ballot::StringifiedPeriodDates;
use sequent_core::services::keycloak::{self, get_event_realm};
use sequent_core::services::s3::get_minio_url;
use sequent_core::services::{pdf, reports};
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::util::temp_path::*;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, instrument};

use sequent_core::types::templates::ReportExtraConfig;
use sequent_core::types::to_map::ToMap;
use std::fmt::Debug;
use std::fs;
use std::path::Path;
use tempfile::tempdir;
use tempfile::{NamedTempFile, TempPath};
use tokio::runtime::Runtime;

use crate::services::celery_app::get_worker_threads;
use crate::services::consolidation::aes_256_cbc_encrypt::encrypt_file_aes_256_cbc;
use crate::services::documents::upload_and_return_document;
use crate::services::providers::email_sender::{Attachment, EmailSender};
use crate::services::reports::pre_enrolled_ov_but_disapproved::first_n_codepoints;
use crate::services::reports_vault::get_report_secret_key;
use crate::services::vault;

static GLOBAL_RT: Lazy<Runtime> = Lazy::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to build global Tokio runtime")
});

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserDataArea {
    pub area_id: String,
    pub election_id: String,
    pub election_title: String,
    pub election_dates: StringifiedPeriodDates,
    pub post: String,
    pub area_name: String,
    pub station_id: String,
    pub station_name: String,
    pub voters: Vec<Voter>,       // Voter list field
    pub ov_voted: i64,            // Number of overseas voters who voted
    pub ov_not_voted: i64,        // Number of overseas voters who did not vote
    pub ov_not_pre_enrolled: i64, // Number of overseas voters not pre-enrolled
    pub eb_voted: i64,            // Election board voted count
    pub ov_total: i64,            // Total overseas voters
    pub inspectors: Vec<InspectorData>,
}

/// Struct for User Data Area
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserData {
    pub areas: Vec<UserDataArea>,
    pub execution_annotations: ExecutionAnnotations,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemData {
    pub rendered_user_template: String,
    pub file_qrcode_lib: String,
}

/// Main struct for generating Overseas Voters Report
#[derive(Debug)]
pub struct OverseasVotersReport {
    ids: ReportOrigins,
}

impl OverseasVotersReport {
    pub fn new(ids: ReportOrigins) -> Self {
        OverseasVotersReport { ids }
    }

    async fn prepare_user_data_common(&self) -> Result<UserData> {
        let date_printed = get_date_and_time();
        let app_hash = get_app_hash();
        let app_version = get_app_version();
        let report_hash = get_report_hash(&ReportType::LIST_OF_OVERSEAS_VOTERS.to_string())
            .await
            .unwrap_or("-".to_string());

        Ok(UserData {
            areas: vec![],
            execution_annotations: ExecutionAnnotations {
                date_printed,
                report_hash,
                software_version: app_version.clone(),
                app_version,
                app_hash,
                executer_username: self.ids.executer_username.clone(),
                results_hash: None,
            },
        })
    }

    #[instrument(err, skip_all)]
    async fn generate_report_area(
        &self,
        generate_mode: GenerateReportMode,
        hasura_transaction: &Transaction<'_>,
        keycloak_transaction: &Transaction<'_>,
        user_tpl_document: &str,
        user_data: UserData,
        area: UserDataArea,
        per_report_limit: i64,
        batch_index: i64,
        ext_cfg: &ReportExtraConfig,
        reports_folder: &Path,
    ) -> Result<()> {
        info!("inside generate_report_area");
        // Prepare real use data
        let mut batch = 1;
        let mut offset: i64 = 0;
        loop {
            let (user_data, next_offset): (UserData, Option<i64>) = self
                .prepare_data_batch(
                    hasura_transaction,
                    keycloak_transaction,
                    area.clone(),
                    user_data.clone(),
                    per_report_limit,
                    offset,
                )
                .await
                .map_err(|e| anyhow!("Error preparing batched user data: {e:?}"))?;

            if let Some(new_offset) = next_offset {
                offset = new_offset;
            } else {
                offset = 0;
            }

            // if this is not the first batch and the offset is 0, we don't a
            // new batch
            if batch > 1 && offset == 0 {
                break;
            }

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

            let rendered_system_template =
                reports::render_template_text(&system_template, system_data)
                    .map_err(|e| anyhow!("Error rendering system template: {e:?}"))?;

            // Render to PDF bytes
            let pdf_bytes = pdf::PdfRenderer::render_pdf(
                rendered_system_template,
                Some(ext_cfg.pdf_options.to_print_to_pdf_options()),
            )
            .await
            .with_context(|| format!("Error rendering PDF for batch {}", batch_index))?;

            let prefix = first_n_codepoints(&self.prefix(), 5);
            let extension_suffix = "pdf";
            let file_suffix = format!(".{}", extension_suffix);
            let area_id = first_n_codepoints(&area.area_id, 5);

            let batch_file_name = format!(
                "{}_area_{}{}_{}{}",
                prefix, area.area_name, area_id, batch, file_suffix
            );

            info!(
                "Batch {} => batch_file_name: {}",
                batch_index, batch_file_name
            );

            // Build the final path inside `reports_folder`:
            let final_path = reports_folder.join(&batch_file_name);
            info!("final_path {:?}", &final_path);

            fs::write(&final_path, &pdf_bytes)?;
            batch += 1;

            // if this is the first batch and offset is 0, we already rendered
            // it but we don't need any more batches so we stop here
            if batch == 1 && offset == 0 {
                break;
            }
        }
        Ok(())
    }

    #[instrument(err)]
    async fn prepare_data_batch(
        &self,
        hasura_transaction: &Transaction<'_>,
        keycloak_transaction: &Transaction<'_>,
        area: UserDataArea,
        user_data: UserData,
        limit: i64,
        offset: i64,
    ) -> Result<(UserData, Option<i64>)> {
        let realm = get_event_realm(&self.ids.tenant_id, &self.ids.election_event_id);

        let filtered_voters = FilterListVoters {
            enrolled: None,
            has_voted: None,
            voters_sex: None,
            post: None,
            landbased_or_seafarer: None,
            verified: None,
        };

        let (voters_data, next_offset) = get_voters_data(
            &hasura_transaction,
            &keycloak_transaction,
            &realm,
            &self.ids.tenant_id,
            &self.ids.election_event_id,
            &area.election_id,
            &area.area_id,
            true,
            filtered_voters,
            Some(limit),
            Some(offset),
        )
        .await
        .map_err(|err| anyhow!("Error get_voters_data {err}"))?;

        let total_not_pre_enrolled = count_voters_by_area_id(
            &keycloak_transaction,
            &realm,
            &area.area_id,
            None,
            Some(false),
        )
        .await
        .map_err(|err| anyhow!("Error at count_voters_by_area_id not pre enrolled {err}"))?;

        let area_final_data = UserData {
            areas: vec![UserDataArea {
                election_id: area.election_id.clone(),
                area_id: area.area_id.clone(),
                election_title: area.election_title.clone(),
                election_dates: area.election_dates.clone(),
                station_id: area.station_id.clone(),
                station_name: area.station_name.clone(),
                post: area.post.clone(),
                inspectors: area.inspectors.clone(),
                area_name: area.area_name.clone(),
                voters: voters_data.voters,
                ov_voted: voters_data.total_voted,
                ov_not_voted: voters_data.total_not_voted,
                ov_not_pre_enrolled: total_not_pre_enrolled,
                eb_voted: 0, // Election board voted count
                ov_total: voters_data.total_voters,
            }],
            execution_annotations: user_data.execution_annotations.clone(),
        };

        Ok((area_final_data, next_offset))
    }

    fn zip_filename(&self) -> String {
        format!(
            "overseas_voters_{}_{}_final.zip",
            self.ids.election_event_id,
            self.ids.election_id.clone().unwrap_or_default()
        )
    }
}

#[async_trait]
impl TemplateRenderer for OverseasVotersReport {
    type UserData = UserData;
    type SystemData = SystemData;

    fn get_report_type(&self) -> ReportType {
        ReportType::LIST_OF_OVERSEAS_VOTERS
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
        "overseas_voters".to_string()
    }

    fn prefix(&self) -> String {
        format!(
            "overseas_voters_{}_{}_{}",
            self.ids.tenant_id,
            self.ids.election_event_id,
            self.ids.election_id.clone().unwrap_or_default()
        )
    }

    #[instrument(err, skip_all)]
    async fn prepare_user_data(
        &self,
        hasura_transaction: &Transaction<'_>,
        keycloak_transaction: &Transaction<'_>,
    ) -> Result<Self::UserData> {
        unimplemented!()
    }

    /// Prepare system metadata for the report
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
        let Some(election_id) = &self.ids.election_id else {
            return Err(anyhow!("Empty election_id"));
        };

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

        let zip_temp_dir = tempdir()?;
        let zip_temp_dir_path = zip_temp_dir.path();

        let (final_file_path, file_size, final_report_name, mimetype) = match generate_mode {
            GenerateReportMode::PREVIEW => self
                .generate_single_report(
                    hasura_transaction,
                    keycloak_transaction,
                    &user_tpl_document,
                    generate_mode,
                    task_execution.clone(),
                    &ext_cfg,
                )
                .await
                .map_err(|e| anyhow::anyhow!("Error in generate_single_report: {}", e))?,
            GenerateReportMode::REAL => {
                let election_event = get_election_event_by_id(
                    &hasura_transaction,
                    &self.ids.tenant_id,
                    &self.ids.election_event_id,
                )
                .await
                .map_err(|e| anyhow::anyhow!("Error getting election event by id: {}", e))?;

                let election = match get_election_by_id(
                    &hasura_transaction,
                    &self.ids.tenant_id,
                    &self.ids.election_event_id,
                    &election_id,
                )
                .await
                .with_context(|| "Error getting election by id")?
                {
                    Some(election) => election,
                    None => return Err(anyhow::anyhow!("Election not found")),
                };

                let scheduled_events = find_scheduled_event_by_election_event_id(
                    &hasura_transaction,
                    &self.ids.tenant_id,
                    &self.ids.election_event_id,
                )
                .await
                .map_err(|e| {
                    anyhow::anyhow!("Error getting scheduled events by election event_id: {}", e)
                })?;

                let election_event_annotations =
                    extract_election_event_annotations(&election_event)
                        .await
                        .map_err(|err| anyhow!("Error extract election event annotations {err}"))?;

                let mut areas: Vec<UserDataArea> = vec![];
                let election_general_data = extract_election_data(&election)
                    .await
                    .map_err(|err| anyhow!("Error extract election annotations {err}"))?;

                let election_dates = get_election_dates(&election, scheduled_events.clone())
                    .map_err(|e| anyhow::anyhow!("Error getting election dates {e}"))?;
                let election_id = election.id.clone();
                let election_areas = get_areas_by_election_id(
                    &hasura_transaction,
                    &self.ids.tenant_id,
                    &self.ids.election_event_id,
                    &election_id,
                )
                .await
                .map_err(|err| anyhow!("Error at get_areas_by_election_id: {err:?}"))?;

                for area in election_areas.iter() {
                    let area_general_data =
                        extract_area_data(&area, election_event_annotations.sbei_users.clone())
                            .await
                            .map_err(|err| anyhow!("Error extract area data {err}"))?;

                    areas.push(UserDataArea {
                        election_id: election_id.clone(),
                        area_id: area.id.clone(),
                        election_title: election.alias.clone().unwrap_or(election.name.clone()),
                        election_dates: election_dates.clone(),
                        station_name: election_general_data.precinct_code.clone(),
                        station_id: election_general_data.pollcenter_code.clone(),
                        post: election_general_data.post.clone(),
                        inspectors: area_general_data.inspectors,
                        area_name: area.clone().name.unwrap_or("-".to_string()),
                        voters: vec![],
                        ov_voted: 0,
                        ov_not_voted: 0,
                        ov_not_pre_enrolled: 0,
                        eb_voted: 0, // Election board voted count
                        ov_total: 0,
                    })
                }

                let common_data = self.prepare_user_data_common().await?;
                // let items_count = areas.
                let report_options = ext_cfg.report_options.clone();
                let per_report_limit = report_options
                    .max_items_per_report
                    .unwrap_or(DEFAULT_ITEMS_PER_REPORT_LIMIT)
                    as i64;

                // Calculate the number of batches needed.
                let num_batches = areas.len();
                info!("Number of batches: {:?}", num_batches);

                // Define a temporary reports folder (this folder will later be compressed)
                let temp_dir = tempdir()?;
                let reports_folder = temp_dir.path();

                // Build a Rayon pool for batch processing.
                let num_threads = report_options.max_threads.unwrap_or(get_worker_threads());
                info!("Parallelization configuration: num_threads = {num_threads}");
                let batch_pool = ThreadPoolBuilder::new()
                    .num_threads(num_threads)
                    .build()
                    .with_context(|| "Failed to build thread pool")?;

                // Process batches concurrently.
                let _ = batch_pool.install(|| {
                        (0..num_batches)
                            .into_par_iter()
                            .map(|batch_index| -> Result<(), anyhow::Error> {
                                info!("processing batch {batch_index}");
                                let area_id = &areas[batch_index].clone().area_id;
                                let election_id = &areas[batch_index].clone().election_id;
                                GLOBAL_RT
                                    .block_on(async {
                                        info!("processing batch {batch_index}: inside the GLOBAL_RT.block_on");
                                        self.generate_report_area(
                                            generate_mode.clone(),
                                            &hasura_transaction,
                                            &keycloak_transaction,
                                            &user_tpl_document,
                                            common_data.clone(),
                                            areas[batch_index].clone(),
                                            per_report_limit,
                                            batch_index as i64,
                                            &ext_cfg,
                                            &reports_folder,
                                        )
                                        .await
                                    })
                                    .with_context(|| {
                                        format!(
                                            "Error rendering report for batch election {} area {}",
                                            election_id, area_id
                                        )
                                    })?;
                                Ok(())
                            })
                            .collect::<Result<Vec<()>, anyhow::Error>>()
                    });

                let zip_file_name = self.zip_filename();

                let dst_zip = zip_temp_dir_path.join(&zip_file_name);

                compress_folder_to_zip(reports_folder, &dst_zip)
                    .with_context(|| "Error compressing folder")?;

                let zip_file_size = get_file_size(&dst_zip.to_string_lossy())
                    .with_context(|| "Error obtaining file size for zip file")?;

                let final_file_path = dst_zip.to_string_lossy().to_string();
                let mimetype = "application/zip".to_string();
                (final_file_path, zip_file_size, zip_file_name, mimetype)
            }
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
                false,
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
                false,
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
}
