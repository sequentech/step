// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::report_variables::{
    generate_election_votes_data, get_app_hash, get_app_version, get_date_and_time,
    get_results_hash,
};
use super::template_renderer::*;
use crate::postgres::area::get_areas_by_election_id;
use crate::postgres::election::get_election_by_id;
use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::scheduled_event::find_scheduled_event_by_election_event_id;
use crate::services::database::PgConfig;
use crate::services::electoral_log::{
    list_electoral_log_without_null_user_ids, ElectoralLogRow, GetElectoralLogBody,
    IMMUDB_ROWS_LIMIT,
};

use crate::services::temp_path::*;
use crate::services::users::{list_users, ListUsersFilter};
use crate::types::resources::{Aggregate, DataList, TotalAggregate};
use crate::{postgres::reports::ReportType, services::s3::get_minio_url};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Local};
use deadpool_postgres::Transaction;
use sequent_core::services::date::ISO8601;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::types::{hasura::core::Election, scheduled_event::generate_voting_period_dates};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tracing::{info, instrument, warn};

/// Struct for Audit Logs User Data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserData {
    pub election_event_date: String,
    pub election_event_title: String,
    pub voting_period_start: String,
    pub voting_period_end: String,
    pub geographical_region: String,
    pub post: String,
    pub area_id: String,
    pub voting_center: String,
    pub precinct_code: String,
    pub registered_voters: Option<i64>,
    pub ballots_counted: Option<i64>,
    pub voters_turnout: Option<f64>,
    pub sequences: Vec<AuditLogEntry>,
    pub signature_date: String,
    pub chairperson_name: String,
    pub chairperson_digital_signature: String,
    pub poll_clerk_name: String,
    pub poll_clerk_digital_signature: String,
    pub third_member_name: String,
    pub third_member_digital_signature: String,
    pub results_hash: String,
    pub report_hash: String,
    pub ovcs_version: String,
    pub software_version: String,
    pub system_hash: String,
    pub date_printed: String,
}

/// Struct for each Audit Log Entry
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AuditLogEntry {
    pub number: i64,
    pub datetime: String,
    pub username: String,
    pub activity: String,
}
/// Struct for System Data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemData {
    pub rendered_user_template: String,
    pub file_qrcode_lib: String,
}

// TODO: this is per election but the logs are actually at the election event
// level
#[derive(Debug)]
pub struct AuditLogsTemplate {
    tenant_id: String,
    election_event_id: String,
}

impl AuditLogsTemplate {
    pub fn new(tenant_id: String, election_event_id: String) -> Self {
        AuditLogsTemplate {
            tenant_id,
            election_event_id,
        }
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
        self.tenant_id.clone()
    }

    fn get_election_event_id(&self) -> String {
        self.election_event_id.clone()
    }

    fn base_name(&self) -> String {
        "audit_logs".to_string()
    }

    fn prefix(&self) -> String {
        format!("audit_logs_{}", self.election_event_id)
    }

    #[instrument(err, skip(self, hasura_transaction, keycloak_transaction))]
    async fn prepare_user_data(
        &self,
        hasura_transaction: &Transaction<'_>,
        keycloak_transaction: &Transaction<'_>,
    ) -> Result<Self::UserData> {
        let realm_name = get_event_realm(&self.tenant_id, &self.election_event_id);
        // This is used to fill the user data.
        let election_event = get_election_event_by_id(
            &hasura_transaction,
            &self.tenant_id,
            &self.election_event_id,
        )
        .await
        .map_err(|e| anyhow!("Error getting scheduled event by election event_id: {e:?}"))?;

        let election_id: String = "".to_string(); // WIP: get the right election_id from self, when the topic with ReportOrigins is merged into main.
                                                  // self.get_election_id(),

        let election: Election = match get_election_by_id(
            &hasura_transaction,
            &self.tenant_id,
            &self.election_event_id,
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

        // We need the permission_label to filter the logs by Admin users
        // This field is not mandatory so if it´s not there admin users won´t be filtered or won´t be shown.
        let election_permision_label = election.permission_label;

        // Fetch election event data
        let start_election_event = find_scheduled_event_by_election_event_id(
            &hasura_transaction,
            &self.tenant_id,
            &self.election_event_id,
        )
        .await
        .map_err(|e| anyhow!("Error getting scheduled event by election event_id: {e:?}"))?;

        // Fetch election event's voting periods
        let voting_period_dates = generate_voting_period_dates(
            start_election_event,
            &self.tenant_id,
            &self.election_event_id,
            None,
        )
        .map_err(|e| anyhow!(format!("Error generating voting period dates {e:?}")))?;

        // extract start date from voting period
        let voting_period_start_date = voting_period_dates.start_date.unwrap_or_default();
        // extract end date from voting period
        let voting_period_end_date = voting_period_dates.end_date.unwrap_or_default();

        let election_event_date: &String = &voting_period_start_date;
        let datetime_printed: String = get_date_and_time();

        // To filter log entries by election we´ll prepare a list with the user Ids that belong to this election.
        // To get the voter_ids related to this election, we need the areas.
        let election_areas = get_areas_by_election_id(
            &hasura_transaction,
            &self.tenant_id,
            &self.election_event_id,
            &election_id,
        )
        .await
        .map_err(|err| anyhow!("Error at get_areas_by_election_id: {err:?}"))?;

        if election_areas.is_empty() {
            return Err(anyhow!(
                "No areas found for the given election id: {election_id}"
            ));
        }

        let max_batch_size = PgConfig::from_env()?.default_sql_batch_size;
        let mut voters_offset: i32 = 0;
        let election_filter = ListUsersFilter {
            tenant_id: self.get_tenant_id(),
            realm: realm_name.clone(),
            election_event_id: Some(String::from(&self.get_election_event_id())),
            election_id: Some(election_id),
            limit: Some(max_batch_size),
            offset: Some(voters_offset),
            area_id: None,        // To fill below
            ..Default::default()  // Fill the options that are left to None
        };

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
                        ..election_filter.clone()
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

        // WIP: Fill election_user_ids with the Admins that matches the election_permission_label
        // Problem: user_ids of the admin user_ids are empty in the immmuDB logs. Reefactor how these logs are posted?
        // Cannot lookup the permission labels without the user ids.

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
            let electoral_logs_batch =
                list_electoral_log_without_null_user_ids(GetElectoralLogBody {
                    tenant_id: String::from(&self.get_tenant_id()),
                    election_event_id: String::from(&self.election_event_id),
                    limit: Some(IMMUDB_ROWS_LIMIT as i64),
                    offset: Some(offset),
                    filter: None,
                    order_by: None,
                })
                .await
                .map_err(|e| {
                    anyhow!(format!("Error in fetching list of electoral logs {:?}", e))
                })?;

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
            // Discard the logs not related to this election
            match &item.user_id {
                Some(user_id) => {
                    if !election_user_ids.contains(user_id) {
                        continue;
                    }
                    // WIP: More filters ?...
                }
                None => continue,
            }

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
                .user_id
                .clone()
                .map(|username| {
                    if username == "null" {
                        "-".to_string()
                    } else {
                        username
                    }
                })
                .unwrap_or_else(|| "-".to_string());

            // Map fields from `ElectoralLogRow` to `AuditLogEntry`
            let audit_log_entry = AuditLogEntry {
                number: item.id, // Increment number for each item
                datetime: formatted_datetime,
                username,
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
            &self.tenant_id,
            &self.election_event_id,
            &election.id,
        )
        .await
        .map_err(|e| anyhow!(format!("Error generating election votes data {e:?}")))?;

        // Fetch necessary data (dummy placeholders for now)
        let geographical_region = "Global".to_string();
        let post = "Global".to_string();
        let area_id = "Global".to_string();
        let voting_center = "Global".to_string();
        let precinct_code = "Global".to_string();

        let chairperson_name = "".to_string();
        let poll_clerk_name = "".to_string();
        let third_member_name = "".to_string();
        let chairperson_digital_signature = "DigitalSignatureABC".to_string();
        let poll_clerk_digital_signature = "DigitalSignatureDEF".to_string();
        let third_member_digital_signature = "DigitalSignatureGHI".to_string();
        let report_hash = "-".to_string();
        let results_hash = get_results_hash(
            &hasura_transaction,
            &self.tenant_id,
            &self.election_event_id,
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

        Ok(UserData {
            election_event_date: election_event_date.to_string(),
            election_event_title: election_event.name.clone(),
            date_printed: datetime_printed.clone(),
            voting_period_start: voting_period_start_date,
            voting_period_end: voting_period_end_date,
            geographical_region,
            post,
            area_id,
            voting_center,
            precinct_code,
            registered_voters: votes_data.registered_voters,
            ballots_counted: votes_data.total_ballots,
            voters_turnout: votes_data.voters_turnout,
            sequences,
            signature_date,
            chairperson_name,
            chairperson_digital_signature,
            poll_clerk_name,
            poll_clerk_digital_signature,
            third_member_name,
            third_member_digital_signature,
            results_hash,
            report_hash,
            software_version: app_version.clone(),
            ovcs_version: app_version,
            system_hash: app_hash,
        })
    }

    #[instrument(err, skip_all)]
    async fn prepare_system_data(
        &self,
        rendered_user_template: String,
    ) -> Result<Self::SystemData> {
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
    }
}
