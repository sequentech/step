// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::report_variables::{
    extract_election_data, generate_voters_turnout, get_app_hash, get_app_version,
    get_date_and_time, get_results_hash, get_total_number_of_registered_voters,
};
use super::template_renderer::*;
use crate::postgres::election::get_elections;
use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::scheduled_event::find_scheduled_event_by_election_event_id;
use crate::services::cast_votes::count_ballots_by_election;
use crate::services::database::{get_keycloak_pool, PgConfig};
use crate::services::electoral_log::{list_electoral_log, GetElectoralLogBody};
use crate::services::insert_cast_vote::CastVoteError;
use crate::services::temp_path::*;
use crate::{postgres::reports::ReportType, services::s3::get_minio_url};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Local};
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::services::date::ISO8601;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::types::hasura::core::TallySession;
use sequent_core::types::scheduled_event::generate_voting_period_dates;
use serde::{Deserialize, Serialize};
use tracing::{instrument, warn};

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
    pub registered_voters: i64,
    pub ballots_counted: i64,
    pub voters_turnout: f64,
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
    ids: ReportIds,
}

impl AuditLogsTemplate {
    pub fn new(ids: ReportIds) -> Self {
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

    fn get_initial_template_id(&self) -> Option<String> {
        self.ids.template_id.clone()
    }

    fn base_name(&self) -> String {
        "audit_logs".to_string()
    }

    fn prefix(&self) -> String {
        format!("audit_logs_{}", self.ids.election_event_id)
    }

    #[instrument(err, skip(self, hasura_transaction, keycloak_transaction))]
    async fn prepare_user_data(
        &self,
        hasura_transaction: &Transaction<'_>,
        keycloak_transaction: &Transaction<'_>,
    ) -> Result<Self::UserData> {
        let realm_name = get_event_realm(&self.ids.tenant_id, &self.ids.election_event_id);

        let election_event = get_election_event_by_id(
            &hasura_transaction,
            &self.ids.tenant_id,
            &self.ids.election_event_id,
        )
        .await
        .map_err(|e| anyhow!("Error getting scheduled event by election event_id: {e:?}"))?;

        // Fetch election event data
        let start_election_event = find_scheduled_event_by_election_event_id(
            &hasura_transaction,
            &self.ids.tenant_id,
            &self.ids.election_event_id,
        )
        .await
        .map_err(|e| anyhow!("Error getting scheduled event by election event_id: {e:?}"))?;

        // Fetch election event's voting periods
        let voting_period_dates = generate_voting_period_dates(
            start_election_event,
            &self.ids.tenant_id,
            &self.ids.election_event_id,
            None,
        )
        .map_err(|e| anyhow!(format!("Error generating voting period dates {e:?}")))?;

        // extract start date from voting period
        let voting_period_start_date = voting_period_dates.start_date.unwrap_or_default();
        // extract end date from voting period
        let voting_period_end_date = voting_period_dates.end_date.unwrap_or_default();

        let election_event_date: &String = &voting_period_start_date;
        let datetime_printed: String = get_date_and_time();

        // Fetch list of audit logs
        let mut sequences: Vec<AuditLogEntry> = Vec::new();
        let electoral_logs = list_electoral_log(GetElectoralLogBody {
            tenant_id: String::from(&self.get_tenant_id()),
            election_event_id: String::from(&self.ids.election_event_id),
            limit: None,
            offset: None,
            filter: None,
            order_by: None,
        })
        .await
        .map_err(|e| anyhow!(format!("Error in fetching list of electoral logs {:?}", e)))?;

        // iterate on list of audit logs and create array
        for item in &electoral_logs.items {
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
            let username = item.user_id.clone().unwrap_or_else(|| "-".to_string());

            // Map fields from `ElectoralLogRow` to `AuditLogEntry`
            let audit_log_entry = AuditLogEntry {
                number: item.id, // Increment number for each item
                datetime: formatted_datetime,
                username,
                activity: item.statement_kind.clone(), // Assuming `statement_kind` is the activity
            };

            // Push the constructed `AuditLogEntry` to the sequences array
            sequences.push(audit_log_entry);
        }

        let elections = get_elections(
            &hasura_transaction,
            &self.ids.tenant_id,
            &self.ids.election_event_id,
        )
        .await
        .map_err(|e| anyhow!(format!("Error listing elections {e:?}")))?;

        // Since this is an election level report and it should use data from
        // results, which only is accumulated at election level,
        let total_registered_voters =
            get_total_number_of_registered_voters(&keycloak_transaction, &realm_name)
                .await
                .map_err(|e| {
                    anyhow::anyhow!("Error fetching the number of registered voters: {e:?}",)
                })?;

        let mut total_ballots_counted = 0;
        for election in elections.iter() {
            // get election instace's general data (post, country, etc...)
            let election_general_data = match extract_election_data(&election).await {
                Ok(data) => data, // Extracting the ElectionData struct out of Ok
                Err(err) => {
                    return Err(anyhow!("Error fetching election data: {err}"));
                }
            };

            // Fetch ballots counted
            let ballots_counted = count_ballots_by_election(
                &hasura_transaction,
                &self.ids.tenant_id,
                &self.ids.election_event_id,
                &election.id,
            )
            .await
            .map_err(|e| {
                anyhow::anyhow!("Error fetching the number of ballot for election {e:?}",)
            })?;

            total_ballots_counted += ballots_counted;
        }

        // Calculate aggregated turnout
        let voters_turnout =
            generate_voters_turnout(&total_ballots_counted, &total_registered_voters)
                .await
                .map_err(|e| anyhow!(format!("Error in generating voters turnout {e:?}")))?;

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
            &self.ids.tenant_id,
            &self.ids.election_event_id,
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
            registered_voters: total_registered_voters,
            ballots_counted: total_ballots_counted,
            voters_turnout,
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
