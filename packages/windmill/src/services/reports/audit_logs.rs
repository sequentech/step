// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::report_variables::{
    extract_election_data, generate_voters_turnout, get_date_and_time,
    get_election_contests_area_results_and_total_ballot_counted,
    get_total_number_of_registered_voters_for_country,
};
use super::template_renderer::*;
use crate::postgres::election::get_election_by_id;
use crate::postgres::reports::ReportType;
use crate::postgres::scheduled_event::find_scheduled_event_by_election_event_id;
use crate::services::database::get_hasura_pool;
use crate::services::database::{get_keycloak_pool, PgConfig};
use crate::services::electoral_log::{list_electoral_log, GetElectoralLogBody};
use crate::services::insert_cast_vote::CastVoteError;
use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Local};
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::services::date::ISO8601;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::types::scheduled_event::generate_voting_period_dates;
use sequent_core::types::templates::EmailConfig;
use serde::{Deserialize, Serialize};
use tracing::instrument;

/// Struct for Audit Logs User Data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserData {
    pub election_date: String,
    pub election_title: String,
    pub voting_period: String,
    pub geographical_region: String,
    pub post: String,
    pub country: String,
    pub voting_center: String,
    pub precinct_code: String,
    pub registered_voters: i64,
    pub ballots_counted: i64,
    pub voters_turnout: String,
    pub sequences: Vec<AuditLogEntry>,
    pub goverment_time: String,
    pub chairperson_name: String,
    pub chairperson_digital_signature: String,
    pub poll_clerk_name: String,
    pub poll_clerk_digital_signature: String,
    pub third_member_name: String,
    pub third_member_digital_signature: String,
    pub report_hash: String,
    pub ovcs_version: String,
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
}

// TODO: this is per election but the logs are actually at the election event
// level
#[derive(Debug)]
pub struct AuditLogsTemplate {
    tenant_id: String,
    election_event_id: String,
    election_id: String,
}

#[async_trait]
impl TemplateRenderer for AuditLogsTemplate {
    type UserData = UserData;
    type SystemData = SystemData;

    fn get_report_type() -> ReportType {
        ReportType::AUDIT_LOGS
    }

    fn get_tenant_id(&self) -> String {
        self.tenant_id.clone()
    }

    fn get_election_event_id(&self) -> String {
        self.election_event_id.clone()
    }

    fn get_election_id(&self) -> Option<String> {
        Some(self.election_id.clone())
    }

    fn base_name() -> String {
        "audit_logs".to_string()
    }

    fn prefix(&self) -> String {
        format!("audit_logs_{}", self.election_event_id)
    }

    fn get_email_config() -> EmailConfig {
        EmailConfig {
            subject: "Sequent Online Voting - Audit Logs".to_string(),
            plaintext_body: "".to_string(),
            html_body: None,
        }
    }

    #[instrument]
    async fn prepare_user_data(
        &self,
        hasura_transaction: Option<&Transaction<'_>>,
        keycloak_transaction: Option<&Transaction<'_>>,
    ) -> Result<Self::UserData> {
        let realm_name: String =
            get_event_realm(self.tenant_id.as_str(), self.election_event_id.as_str());
        // get election instace
        let election = if let Some(transaction) = hasura_transaction {
            match get_election_by_id(
                &transaction, // Use the unwrapped transaction reference
                &self.get_tenant_id(),
                &self.get_election_event_id(),
                &self.get_election_id().unwrap(),
            )
            .await
            .with_context(|| "Error getting election by id")?
            {
                Some(election) => election,
                None => return Err(anyhow::anyhow!("Election not found")),
            }
        } else {
            return Err(anyhow::anyhow!("Transaction is missing"));
        };

        // get election instace's general data (post, country, etc...)
        let election_general_data = match extract_election_data(&election).await {
            Ok(data) => data, // Extracting the ElectionData struct out of Ok
            Err(err) => {
                return Err(anyhow::anyhow!(format!(
                    "Error fetching election data: {}",
                    err
                )));
            }
        };

        // Fetch election event data
        let start_election_event = if let Some(transaction) = hasura_transaction {
            find_scheduled_event_by_election_event_id(
                &transaction,
                &self.get_tenant_id(),
                &self.get_election_event_id(),
            )
            .await
            .map_err(|e| {
                anyhow::anyhow!("Error getting scheduled event by election event_id: {}", e)
            })?
        } else {
            return Err(anyhow::anyhow!("Transaction is missing"));
        };

        // Fetch election's voting periods
        // TODO: we should decide if this is the actual start time, or the 
        // scheduled time
        let voting_period_dates = generate_voting_period_dates(
            start_election_event,
            &self.get_tenant_id(),
            &self.get_election_event_id(),
            Some(&self.get_election_id().unwrap()),
        )?;

        // extract start date from voting period
        let voting_period_start_date = match voting_period_dates.start_date {
            Some(voting_period_start_date) => voting_period_start_date,
            None => {
                return Err(anyhow::anyhow!(format!(
                    "Error fetching election start date: "
                )))
            }
        };
        // extract end date from voting period
        let voting_period_end_date = match voting_period_dates.end_date {
            Some(voting_period_end_date) => voting_period_end_date,
            None => {
                return Err(anyhow::anyhow!(format!(
                    "Error fetching election end date: "
                )))
            }
        };

        let election_date: &String = &voting_period_start_date;
        let datetime_printed: String = get_date_and_time();

        // Fetch list of audit logs
        let mut sequences: Vec<AuditLogEntry> = Vec::new();
        let electoral_logs = list_electoral_log(GetElectoralLogBody {
            tenant_id: String::from(&self.get_tenant_id()),
            election_event_id: String::from(&self.election_event_id),
            limit: None,
            offset: None,
            filter: None,
            order_by: None,
        })
        .await
        .map_err(|e| {
            anyhow::anyhow!(format!("Error in fetching list of electoral logs {:?}", e))
        })?;

        // itarate on list of audit logs and create array
        for item in &electoral_logs.items {
            let created_datetime: DateTime<Local> = if let Ok(created_datetime_parsed) =
                ISO8601::timestamp_ms_utc_to_date_opt(item.created)
            {
                created_datetime_parsed
            } else {
                return Err(anyhow::anyhow!(format!("Invalid item created timestamp: ")));
            };
            let formatted_datetime: String = created_datetime.to_string();

            // Set default username if user_id is None
            let username = item
                .user_id
                .clone()
                .unwrap_or_else(|| "Unknown User".to_string());

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

        // Fetch total of registered voters
        let registered_voters = if let Some(transaction) = keycloak_transaction {
            get_total_number_of_registered_voters_for_country(
                &transaction, // Pass the actual reference to the transaction
                &realm_name,
                &election_general_data.country,
            )
            .await
            .map_err(|e| {
                anyhow::anyhow!(
                    "Error fetching the number of registered voters for country '{}': {}",
                    &election_general_data.country,
                    e
                )
            })?
        } else {
            return Err(anyhow::anyhow!("Keycloak Transaction is missing"));
        };

        let (ballots_counted, results_area_contests, contests) = if let Some(transaction) =
            hasura_transaction
        {
            get_election_contests_area_results_and_total_ballot_counted(
                &transaction,
                &self.get_tenant_id(),
                &self.get_election_event_id(),
                &self.get_election_id().unwrap(),
            )
            .await
            .map_err(|e| anyhow::anyhow!("Error getting election contests area results: {}", e))?
        } else {
            return Err(anyhow::anyhow!("Transaction is missing"));
        };

        let voters_turnout = generate_voters_turnout(&ballots_counted, &registered_voters)
            .await
            .map_err(|e| anyhow::anyhow!(format!("Error in generating voters turnout {:?}", e)))?;

        // Fetch necessary data (dummy placeholders for now)
        let chairperson_name = "John Doe".to_string();
        let poll_clerk_name = "Jane Smith".to_string();
        let third_member_name = "Alice Johnson".to_string();
        let chairperson_digital_signature = "DigitalSignatureABC".to_string();
        let poll_clerk_digital_signature = "DigitalSignatureDEF".to_string();
        let third_member_digital_signature = "DigitalSignatureGHI".to_string();
        let goverment_time = "18:00".to_string();
        let report_hash = "dummy_report_hash".to_string();
        let ovcs_version = "1.0".to_string();
        let system_hash = "dummy_system_hash".to_string();
        Ok(UserData {
            election_date: election_date.to_string(),
            election_title: election.name.clone(),
            date_printed: datetime_printed,
            voting_period: format!("{} - {}", voting_period_start_date, voting_period_end_date),
            geographical_region: election_general_data.geographical_region,
            post: election_general_data.post,
            country: election_general_data.country,
            voting_center: election_general_data.voting_center,
            precinct_code: election_general_data.clustered_precinct_id,
            registered_voters,
            ballots_counted,
            voters_turnout: format!("{}%", voters_turnout),
            sequences,
            goverment_time,
            chairperson_name,
            chairperson_digital_signature,
            poll_clerk_name,
            poll_clerk_digital_signature,
            third_member_name,
            third_member_digital_signature,
            report_hash,
            ovcs_version,
            system_hash,
        })
    }

    #[instrument]
    async fn prepare_system_data(
        &self,
        rendered_user_template: String,
    ) -> Result<Self::SystemData> {
        Ok(SystemData {
            rendered_user_template,
        })
    }
}

#[instrument]
pub async fn generate_audit_logs_report(
    document_id: &str,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
    mode: GenerateReportMode,
    hasura_transaction: Option<&Transaction<'_>>,
    keycloak_transaction: Option<&Transaction<'_>>,
) -> Result<()> {
    let template = AuditLogsTemplate {
        tenant_id: tenant_id.to_string(),
        election_event_id: election_event_id.to_string(),
        election_id: election_id.to_string(),
    };
    template
        .execute_report(
            document_id,
            tenant_id,
            election_event_id,
            false,
            None,
            None,
            mode,
            hasura_transaction,
            keycloak_transaction,
        )
        .await
}
