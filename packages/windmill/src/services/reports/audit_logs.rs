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
use crate::services::database::PgConfig;
use crate::services::election_dates::get_election_dates;
use crate::services::electoral_log::{
    list_electoral_log, ElectoralLogRow, GetElectoralLogBody, IMMUDB_ROWS_LIMIT,
};

use crate::postgres::reports::ReportType;
use crate::services::reports::report_variables::get_report_hash;
use crate::services::temp_path::*;
use crate::services::users::{list_users, ListUsersFilter};
use crate::types::resources::{Aggregate, DataList, TotalAggregate};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Local};
use deadpool_postgres::Transaction;
use sequent_core::ballot::StringifiedPeriodDates;
use sequent_core::services::date::ISO8601;
use sequent_core::services::keycloak::{get_event_realm, get_tenant_realm};
use sequent_core::services::pdf;
use sequent_core::services::s3::get_minio_url;
use sequent_core::types::hasura::core::Election;
use sequent_core::util::temp_path::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tracing::{info, instrument, warn};

/// Struct for Audit Logs User Data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserData {
    pub election_event_title: String,
    pub election_dates: StringifiedPeriodDates,
    pub geographical_region: String,
    pub post: String,
    pub voting_center: String,
    pub precinct_code: String,
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
    async fn count_items(&self) -> Option<i64> {
        None
    }

    #[instrument(err, skip(self, hasura_transaction, keycloak_transaction))]
    async fn prepare_user_data(
        &self,
        hasura_transaction: &Transaction<'_>,
        keycloak_transaction: &Transaction<'_>,
    ) -> Result<Self::UserData> {
        // TODO: Fix a lot clonning happening with the getters.

        let event_realm_name =
            get_event_realm(&self.get_tenant_id(), &self.get_election_event_id());
        let tenant_realm_name = get_tenant_realm(&self.get_tenant_id());
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
            .map_err(|e| anyhow!("Failed to fetch list_users: {e:?}"))?;

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
                .map_err(|e| anyhow!("Failed to fetch list_users: {e:?}"))?;

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

        let mut offset: i64 = 0;
        loop {
            let electoral_logs_batch = list_electoral_log(GetElectoralLogBody {
                tenant_id: String::from(&self.get_tenant_id()),
                election_event_id: String::from(&self.ids.election_event_id),
                limit: Some(IMMUDB_ROWS_LIMIT as i64),
                offset: Some(offset),
                filter: None,
                order_by: None,
            })
            .await
            .map_err(|e| anyhow!(format!("Error in fetching list of electoral logs {:?}", e)))?;

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
            precinct_code,
            registered_voters: votes_data.registered_voters,
            ballots_counted: Some(ballots_counted),
            voters_turnout: votes_data.voters_turnout,
            sequences,
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
}
