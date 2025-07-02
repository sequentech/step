// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::report_variables::{
    extract_area_data, extract_election_data, extract_election_event_annotations,
    generate_election_votes_data, get_app_hash, get_app_version, get_date_and_time,
    get_results_hash, ExecutionAnnotations, InspectorData,
};
use super::template_renderer::*;
use crate::postgres::area::get_areas_by_election_id;
use crate::postgres::election::get_election_by_id;
use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::scheduled_event::find_scheduled_event_by_election_event_id;
use crate::services::cast_votes::count_ballots_by_election;
use crate::services::celery_app::get_worker_threads;
use crate::services::consolidation::aes_256_cbc_encrypt::encrypt_file_aes_256_cbc;
use crate::services::consolidation::eml_generator::ValidateAnnotations;
use crate::services::consolidation::zip::compress_folder_to_zip;
use crate::services::database::PgConfig;
use crate::services::documents::upload_and_return_document;
use crate::services::election_dates::get_election_dates;
use crate::services::electoral_log::{
    count_electoral_log, list_electoral_log, ElectoralLogRow, GetElectoralLogBody,
    IMMUDB_ROWS_LIMIT,
};
use crate::services::providers::email_sender::{Attachment, EmailSender};
use crate::services::tasks_execution::{update_complete, update_fail};
use crate::services::vault;
use std::fs;
use std::path::PathBuf;

use crate::postgres::reports::{Report, ReportType};
use crate::services::reports::report_variables::get_report_hash;
use crate::services::reports_vault::get_report_secret_key;
use crate::services::temp_path::*;
use crate::services::users::{list_users, list_users_ids, ListUsersFilter};
use crate::types::resources::{Aggregate, DataList, TotalAggregate};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Local};
use deadpool_postgres::Transaction;
use futures::executor::block_on;
use once_cell::sync::Lazy;
use rand::seq;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use rayon::ThreadPoolBuilder;
use sequent_core::ballot::StringifiedPeriodDates;
use sequent_core::services::date::ISO8601;
use sequent_core::services::keycloak::{self, get_event_realm, get_tenant_realm};
use sequent_core::services::pdf;
use sequent_core::services::s3::get_minio_url;
use sequent_core::types::hasura::core::{Election, TasksExecution};
use sequent_core::util::temp_path::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tempfile::tempdir;
use tempfile::{NamedTempFile, TempPath};
use tokio::runtime::Runtime;
use tracing::{info, instrument, warn};

static GLOBAL_RT: Lazy<Runtime> = Lazy::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to build global Tokio runtime")
});

/// Struct for Audit Logs User Data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserData {
    pub election_event_title: String,
    pub election_dates: StringifiedPeriodDates,
    pub geographical_region: String,
    pub post: String,
    pub voting_center: String,
    pub station_id: String,
    pub station_name: String,
    pub registered_voters: Option<i64>,
    pub ballots_counted: Option<i64>,
    pub voters_turnout: Option<f64>,
    pub sequences: Vec<AuditLogEntry>,
    pub signature_date: String,
    pub inspectors: Vec<InspectorData>,
    pub execution_annotations: ExecutionAnnotations,
}

/// Struct for each Audit Log Entry
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AuditLogEntry {
    pub number: i64,
    pub datetime: String,
    pub username: String,
    pub userkind: String,
    pub activity: String,
}
/// Struct for System Data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemData {
    pub rendered_user_template: String,
    pub file_qrcode_lib: String,
}

#[derive(Debug)]
pub struct AuditLogsTemplate {
    ids: ReportOrigins,
}

impl AuditLogsTemplate {
    pub fn new(ids: ReportOrigins) -> Self {
        AuditLogsTemplate { ids }
    }
}

impl AuditLogsTemplate {
    #[instrument(err, skip(self, hasura_transaction, keycloak_transaction))]
    async fn prepare_user_data_common(
        &self,
        hasura_transaction: &Transaction<'_>,
        keycloak_transaction: &Transaction<'_>,
    ) -> Result<UserData> {
        // TODO: Fix a lot cloning happening with the getters.
        // This is used to fill the user data.
        let election_event = get_election_event_by_id(
            &hasura_transaction,
            &self.get_tenant_id(),
            &self.get_election_event_id(),
        )
        .await
        .map_err(|e| anyhow!("Error getting scheduled event by election event_id: {e:?}"))?;

        let Some(election_id) = self.get_election_id() else {
            return Err(anyhow!("Empty election_id"));
        };

        info!("Preparing data of audit logs report for election_id: {election_id}");
        let election: Election = match get_election_by_id(
            &hasura_transaction,
            &self.ids.tenant_id,
            &self.ids.election_event_id,
            &election_id,
        )
        .await
        .map_err(|e| anyhow!(format!("Error getting election by id {e:?}")))?
        {
            Some(election) => election,
            None => {
                return Err(anyhow!(
                    "No election found for the given election id: {election_id}"
                ));
            }
        };

        let election_general_data = extract_election_data(&election)
            .await
            .map_err(|err| anyhow!("Error extract election data {err}"))?;

        let election_event_annotations = extract_election_event_annotations(&election_event)
            .await
            .map_err(|err| anyhow!("Error extract election event annotations {err}"))?;

        // Fetch election's voting periods
        let scheduled_events = find_scheduled_event_by_election_event_id(
            &hasura_transaction,
            &self.ids.tenant_id,
            &self.ids.election_event_id,
        )
        .await
        .map_err(|e| {
            anyhow::anyhow!("Error getting scheduled events by election event_id: {}", e)
        })?;

        let election_dates = get_election_dates(&election, scheduled_events)
            .map_err(|e| anyhow::anyhow!("Error getting election dates {e}"))?;

        let datetime_printed: String = get_date_and_time();

        // To filter log entries by election we´ll prepare a list with the user Ids that belong to this election.
        // To get the voter_ids related to this election, we need the areas.
        let election_areas = get_areas_by_election_id(
            &hasura_transaction,
            &self.ids.tenant_id,
            &self.ids.election_event_id,
            &election_id,
        )
        .await
        .map_err(|err| anyhow!("Error at get_areas_by_election_id: {err:?}"))?;

        if election_areas.is_empty() {
            return Err(anyhow!(
                "No areas found for the given election id: {election_id}"
            ));
        }

        let votes_data = generate_election_votes_data(
            &hasura_transaction,
            &self.ids.tenant_id,
            &self.ids.election_event_id,
            &election.id,
        )
        .await
        .map_err(|e| anyhow!(format!("Error generating election votes data {e:?}")))?;

        // Fetch necessary data (dummy placeholders for now)
        let post = election_general_data.post.clone();
        let geographical_region = election_general_data.geographical_region.clone();
        let voting_center = election_general_data.voting_center.clone();
        let precinct_code = election_general_data.precinct_code.clone();
        let pollcenter_code = election_general_data.pollcenter_code.clone();

        let report_hash = get_report_hash(&ReportType::AUDIT_LOGS.to_string())
            .await
            .unwrap_or("-".to_string());
        let results_hash = get_results_hash(
            &hasura_transaction,
            &self.ids.tenant_id,
            &self.ids.election_event_id,
            &election_id,
        )
        .await
        .map_err(|err| {
            warn!("Error getting results_hash: {err:?}");
            anyhow!("Error getting results_hash: {err:?}")
        })
        .unwrap_or("-".to_string());

        let app_hash = get_app_hash();
        let app_version = get_app_version();
        let signature_date = datetime_printed.clone();

        // Fetch areas associated with the election
        let election_areas = get_areas_by_election_id(
            &hasura_transaction,
            &self.ids.tenant_id,
            &self.ids.election_event_id,
            &election_id,
        )
        .await
        .map_err(|err| anyhow!("Error at get_areas_by_election_id: {err:?}"))?;

        // we need at least one area to gather the inspectors of the area
        if election_areas.is_empty() {
            return Err(anyhow!("No areas found for the given election"));
        }
        let area_general_data = extract_area_data(
            &election_areas[0],
            election_event_annotations.sbei_users.clone(),
        )
        .await
        .map_err(|err| anyhow!("Error extract area data {err}"))?;

        let area_annotations = election_areas[0].get_annotations_or_empty_values()?;

        let ballots_counted = count_ballots_by_election(
            hasura_transaction,
            &self.ids.tenant_id,
            &self.ids.election_event_id,
            &election_id,
        )
        .await
        .map_err(|err| anyhow!("Error at count_ballots_by_election: {err:?}"))?;

        Ok(UserData {
            election_event_title: election_event.name.clone(),
            election_dates,
            geographical_region,
            post,
            voting_center,
            station_id: pollcenter_code.clone(),
            station_name: precinct_code.clone(),
            registered_voters: votes_data.registered_voters,
            ballots_counted: Some(ballots_counted),
            voters_turnout: votes_data.voters_turnout,
            sequences: vec![],
            signature_date,
            inspectors: area_general_data.inspectors,
            execution_annotations: ExecutionAnnotations {
                date_printed: datetime_printed,
                report_hash,
                software_version: app_version.clone(),
                app_version,
                app_hash,
                executer_username: self.ids.executer_username.clone(),
                results_hash: Some(results_hash),
            },
        })
    }
}

#[async_trait]
impl TemplateRenderer for AuditLogsTemplate {
    type UserData = UserData;
    type SystemData = SystemData;

    fn get_report_type(&self) -> ReportType {
        ReportType::AUDIT_LOGS
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
        "audit_logs".to_string()
    }

    fn prefix(&self) -> String {
        format!("audit_logs_{}", self.ids.election_event_id)
    }
    async fn count_items(&self, hasura_transaction: &Transaction<'_>) -> Result<Option<i64>> {
        let Some(election_id) = self.get_election_id() else {
            return Err(anyhow!("Empty election_id"));
        };

        let election_areas = get_areas_by_election_id(
            &hasura_transaction,
            &self.ids.tenant_id,
            &self.ids.election_event_id,
            &election_id,
        )
        .await
        .with_context(|| "Error at get_areas_by_election_id")?;

        let area_ids: Vec<String> = election_areas.into_iter().map(|area| area.id).collect();

        let input = GetElectoralLogBody {
            tenant_id: self.ids.tenant_id.clone(),
            election_event_id: self.ids.election_event_id.clone(),
            limit: None,
            offset: None,
            filter: None,
            order_by: None,
            area_ids: Some(area_ids),
            only_with_user: Some(true),
            election_id: Some(election_id),
        };
        Ok(count_electoral_log(input).await.ok())
    }

    #[instrument(err, skip_all)]
    async fn prepare_user_data_batch(
        &self,
        hasura_transaction: &Transaction<'_>,
        keycloak_transaction: &Transaction<'_>,
        offset: &mut i64,
        limit: i64,
    ) -> Result<Self::UserData> {
        info!(
            "Preparing data of audit logs report with {} {} ",
            &offset, &limit
        );
        let mut user_data = self
            .prepare_user_data_common(hasura_transaction, keycloak_transaction)
            .await?;
        let event_realm_name =
            get_event_realm(&self.get_tenant_id(), &self.get_election_event_id());
        let tenant_realm_name = get_tenant_realm(&self.get_tenant_id());
        // This is used to fill the user data.

        let Some(election_id) = self.get_election_id() else {
            return Err(anyhow!("Empty election_id"));
        };
        info!("Preparing data of audit logs report for election_id: {election_id}");
        let election: Election = get_election_by_id(
            &hasura_transaction,
            &self.ids.tenant_id,
            &self.ids.election_event_id,
            &election_id,
        )
        .await
        .with_context(|| "Error getting election by id")?
        .ok_or(anyhow!(
            "No election found for the given election id: {election_id}"
        ))?;

        // To filter log entries by election we´ll prepare a list with the user Ids that belong to this election.
        // To get the voter_ids related to this election, we need the areas.
        let election_areas = get_areas_by_election_id(
            &hasura_transaction,
            &self.ids.tenant_id,
            &self.ids.election_event_id,
            &election_id,
        )
        .await
        .with_context(|| "Error at get_areas_by_election_id")?;

        if election_areas.is_empty() {
            return Err(anyhow!(
                "No areas found for the given election id: {election_id}"
            ));
        }

        // We need the permission_label to filter the logs by Admin users
        // This field is not mandatory so if it´s not there the admin users simply won´t be reported.
        let perm_lbl_attributes: Option<HashMap<String, String>> = match election.permission_label {
            Some(permission_label) => Some(HashMap::from([(
                "permission_labels".to_string(),
                permission_label,
            )])),
            None => {
                warn!("No permission_label found for the election, admin users won't be reported");
                None
            }
        };

        // Get all user ids
        let admins_filter = ListUsersFilter {
            tenant_id: self.get_tenant_id(),
            realm: tenant_realm_name.clone(),
            attributes: perm_lbl_attributes.clone(),
            // user_ids: Some(election_user_ids.clone()),
            ..Default::default() // Fill the options that are left to None
        };
        let users = list_users_ids(
            &hasura_transaction,
            &keycloak_transaction,
            ListUsersFilter {
                ..admins_filter.clone()
            },
        )
        .await
        .with_context(|| "Failed to fetch list_users")?;

        let election_batch_admin_ids: HashSet<String> = users.into_iter().collect();

        let voters_filter = ListUsersFilter {
            tenant_id: self.get_tenant_id(),
            realm: event_realm_name.clone(),
            election_event_id: Some(String::from(&self.get_election_event_id())),
            election_id: Some(election_id.clone()),
            // area_id: None, // To fill below
            // user_ids: Some(election_user_ids.clone()),
            ..Default::default() // Fill the options that are left to None
        };
        let users = list_users_ids(
            &hasura_transaction,
            &keycloak_transaction,
            ListUsersFilter {
                ..voters_filter.clone()
            },
        )
        .await
        .with_context(|| "Failed to fetch list_users")?;

        let election_batch_voters_ids: HashSet<String> = users.into_iter().collect();

        let area_ids: Vec<String> = election_areas.into_iter().map(|area| area.id).collect();

        let mut sequences: Vec<AuditLogEntry> = Vec::new();
        let mut current_offset = *offset;

        loop {
            let input = GetElectoralLogBody {
                tenant_id: self.ids.tenant_id.clone(),
                election_event_id: self.ids.election_event_id.clone(),
                limit: Some(limit), // request up to the full limit each time
                offset: Some(current_offset),
                filter: None,
                order_by: None,
                election_id: Some(election_id.clone()),
                area_ids: Some(area_ids.clone()),
                only_with_user: Some(true),
            };

            let electoral_logs_batch = list_electoral_log(input)
                .await
                .with_context(|| "Error fetching electoral logs")?;
            let batch_size = electoral_logs_batch.items.len();
            if batch_size == 0 {
                // No more logs available.
                break;
            }

            // Process each returned log.
            for item in electoral_logs_batch.items {
                // With filtering applied in the SQL, these should all be relevant,
                // but we still tag them as Admin or Voter.
                let userkind = match &item.user_id {
                    Some(user_id) if election_batch_admin_ids.contains(user_id) => {
                        "Admin".to_string()
                    }
                    Some(user_id) if election_batch_voters_ids.contains(user_id) => {
                        "Voter".to_string()
                    }
                    _ => continue, // skip if it doesn't match expected types
                };

                let created_datetime = ISO8601::timestamp_secs_utc_to_date_opt(item.created)
                    .map_err(|_| anyhow!("Invalid created timestamp: {:?}", item.created))?;
                let formatted_datetime = created_datetime.to_rfc3339();
                let username = item.username.clone().unwrap_or_else(|| "-".to_string());

                let audit_log_entry = AuditLogEntry {
                    number: item.id,
                    datetime: formatted_datetime,
                    username,
                    userkind,
                    activity: item
                        .statement_head_data()
                        .map(|head| head.description.clone())
                        .unwrap_or("-".to_string()),
                };
                sequences.push(audit_log_entry);

                if sequences.len() >= limit as usize {
                    break;
                }
            }

            current_offset += batch_size as i64;
            // If we reached the desired number of logs, exit.
            if sequences.len() >= limit as usize {
                break;
            }
        }

        user_data.sequences = sequences;

        Ok(user_data)
    }

    #[instrument(err, skip(self, hasura_transaction, keycloak_transaction))]
    async fn prepare_user_data(
        &self,
        hasura_transaction: &Transaction<'_>,
        keycloak_transaction: &Transaction<'_>,
    ) -> Result<Self::UserData> {
        let mut user_data = self
            .prepare_user_data_common(hasura_transaction, keycloak_transaction)
            .await?;

        let event_realm_name =
            get_event_realm(&self.get_tenant_id(), &self.get_election_event_id());
        let tenant_realm_name = get_tenant_realm(&self.get_tenant_id());
        // This is used to fill the user data.

        let Some(election_id) = self.get_election_id() else {
            return Err(anyhow!("Empty election_id"));
        };
        info!("Preparing data of audit logs report for election_id: {election_id}");
        let election: Election = get_election_by_id(
            &hasura_transaction,
            &self.ids.tenant_id,
            &self.ids.election_event_id,
            &election_id,
        )
        .await
        .with_context(|| "Error getting election by id")?
        .ok_or(anyhow!(
            "No election found for the given election id: {election_id}"
        ))?;

        // To filter log entries by election we´ll prepare a list with the user Ids that belong to this election.
        // To get the voter_ids related to this election, we need the areas.
        let election_areas = get_areas_by_election_id(
            &hasura_transaction,
            &self.ids.tenant_id,
            &self.ids.election_event_id,
            &election_id,
        )
        .await
        .with_context(|| "Error at get_areas_by_election_id")?;

        if election_areas.is_empty() {
            return Err(anyhow!(
                "No areas found for the given election id: {election_id}"
            ));
        }

        // We need the permission_label to filter the logs by Admin users
        // This field is not mandatory so if it´s not there the admin users simply won´t be reported.
        let perm_lbl_attributes: Option<HashMap<String, String>> = match election.permission_label {
            Some(permission_label) => Some(HashMap::from([(
                "permission_labels".to_string(),
                permission_label,
            )])),
            None => {
                warn!("No permission_label found for the election, admin users won't be reported");
                None
            }
        };

        let max_batch_size = PgConfig::from_env()?.default_sql_batch_size;
        let admins_filter = ListUsersFilter {
            tenant_id: self.get_tenant_id(),
            realm: tenant_realm_name.clone(),
            attributes: perm_lbl_attributes.clone(),
            limit: Some(max_batch_size),
            ..Default::default() // Fill the options that are left to None
        };

        // Fill election_admin_ids with the Admins that matches the election_permission_label
        let mut election_admin_ids: HashSet<String> = HashSet::new();
        let mut admins_offset: i32 = 0;
        while perm_lbl_attributes.is_some() {
            let (admins, total_count) = list_users(
                &hasura_transaction,
                &keycloak_transaction,
                ListUsersFilter {
                    offset: Some(admins_offset),
                    ..admins_filter.clone()
                },
            )
            .await
            .with_context(|| "Failed to fetch list_users")?;

            admins_offset += total_count;
            for adm in admins {
                election_admin_ids.insert(adm.id.unwrap_or_default());
            }
            if total_count < max_batch_size {
                break;
            }
        }

        let voters_filter = ListUsersFilter {
            tenant_id: self.get_tenant_id(),
            realm: event_realm_name.clone(),
            election_event_id: Some(String::from(&self.get_election_event_id())),
            election_id: Some(election_id.clone()),
            limit: Some(max_batch_size),
            area_id: None,        // To fill below
            ..Default::default()  // Fill the options that are left to None
        };

        let mut voters_offset: i32 = 0;
        let mut election_user_ids: HashSet<String> = HashSet::new();
        // Loop over each area to fill election_user_ids with the voters
        for area in election_areas.iter() {
            loop {
                let (users, total_count) = list_users(
                    &hasura_transaction,
                    &keycloak_transaction,
                    ListUsersFilter {
                        area_id: Some(area.id.clone()),
                        offset: Some(voters_offset),
                        ..voters_filter.clone()
                    },
                )
                .await
                .with_context(|| "Failed to fetch list_users")?;

                voters_offset += total_count;
                for user in users {
                    election_user_ids.insert(user.id.unwrap_or_default());
                }
                if total_count < max_batch_size {
                    break;
                }
            }
        }

        // Fetch list of audit logs
        let mut sequences: Vec<AuditLogEntry> = Vec::new();
        let mut electoral_logs: DataList<ElectoralLogRow> = DataList {
            items: vec![],
            total: TotalAggregate {
                aggregate: Aggregate { count: 0 },
            },
        };
        let area_ids: Vec<String> = election_areas.into_iter().map(|area| area.id).collect();

        let mut offset: i64 = 0;
        loop {
            let electoral_logs_batch = list_electoral_log(GetElectoralLogBody {
                tenant_id: String::from(&self.get_tenant_id()),
                election_event_id: String::from(&self.ids.election_event_id),
                limit: Some(IMMUDB_ROWS_LIMIT as i64),
                offset: Some(offset),
                filter: None,
                order_by: None,
                area_ids: Some(area_ids.clone()),
                only_with_user: Some(true),
                election_id: Some(election_id.clone()),
            })
            .await
            .with_context(|| "Error in fetching list of electoral logs")?;

            let batch_size = electoral_logs_batch.items.len();
            offset += batch_size as i64;
            electoral_logs.items.extend(electoral_logs_batch.items);
            electoral_logs.total.aggregate.count = electoral_logs_batch.total.aggregate.count;
            if batch_size < IMMUDB_ROWS_LIMIT {
                break;
            }
        }

        // iterate on list of audit logs and create array
        for item in &electoral_logs.items {
            // Discard the log entries that are not related to this election
            let userkind = match &item.user_id {
                Some(user_id) if election_admin_ids.contains(user_id) => "Admin".to_string(),
                Some(user_id) if election_user_ids.contains(user_id) => "Voter".to_string(),
                Some(_) => continue, // Some user_id not belonging to this election
                None => continue,    // There is no user_id, ignore log entry
            };

            let created_datetime: DateTime<Local> = if let Ok(created_datetime_parsed) =
                ISO8601::timestamp_secs_utc_to_date_opt(item.created)
            {
                created_datetime_parsed
            } else {
                return Err(anyhow!(
                    "Invalid item created timestamp: {:?}",
                    item.created
                ));
            };
            let formatted_datetime: String = created_datetime.to_rfc3339();

            // Set default username if user_id is None
            let username = item
                .username
                .clone()
                .map(|user| {
                    if user == "null" {
                        "-".to_string()
                    } else {
                        user
                    }
                })
                .unwrap_or_else(|| "-".to_string());

            // Map fields from `ElectoralLogRow` to `AuditLogEntry`
            let audit_log_entry = AuditLogEntry {
                number: item.id, // Increment number for each item
                datetime: formatted_datetime,
                username,
                userkind,
                activity: item
                    .statement_head_data()
                    .map(|head| head.description.clone())
                    .unwrap_or("-".to_string()),
            };

            // Push the constructed `AuditLogEntry` to the sequences array
            sequences.push(audit_log_entry);
        }

        user_data.sequences = sequences;

        Ok(user_data)
    }

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
            //If we are rendering with a lambda, the QRCode lib is
            //already included in the lambda container image.
            Ok(SystemData {
                rendered_user_template,
                file_qrcode_lib: "/assets/qrcode.min.js".to_string(),
            })
        }
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

        let items_count = self.count_items(&hasura_transaction).await?.unwrap_or(0);
        let report_options = ext_cfg.report_options.clone();
        let per_report_limit = report_options
            .max_items_per_report
            .unwrap_or(DEFAULT_ITEMS_PER_REPORT_LIMIT) as i64;

        info!("Items count: {items_count}, per report limit: {per_report_limit}");
        let zip_temp_dir = tempdir()?;
        let zip_temp_dir_path = zip_temp_dir.path();

        let (final_file_path, file_size, final_report_name, mimetype) = if generate_mode
            == GenerateReportMode::PREVIEW
        {
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
        } else {
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
            block_on(update_complete(task, Some(document_id.to_string())))
                .context("Failed to update task execution status to COMPLETED")?;
        }

        Ok(())
    }
}
