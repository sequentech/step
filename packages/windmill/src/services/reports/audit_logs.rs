use super::report_variables::{extract_eleciton_data, get_total_number_of_registered_voters_for_country, genereate_voters_turnout};
use super::template_renderer::*;
use crate::postgres::scheduled_event::find_scheduled_event_by_election_event_id;
use sequent_core::types::scheduled_event::generate_voting_period_dates;
use crate::services::database::{get_keycloak_pool, PgConfig};
use crate::services::electoral_log::{list_electoral_log, GetElectoralLogBody};
use crate::services::database::get_hasura_pool;
use crate::postgres::election::get_election_by_id;
use sequent_core::services::keycloak::get_event_realm;
use anyhow::{Context, Result};
use async_trait::async_trait;
use deadpool_postgres::Client as DbClient;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use sequent_core::types::templates::EmailConfig;
use chrono::{NaiveDate, TimeZone, Utc};
use rocket::http::Status;

/// Struct for Audit Logs User Data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserData {
    pub election_date: String,
    pub election_title: String,
    pub voting_period: String,
    pub geographic_region: String,
    pub post: String,
    pub country: String,
    pub voting_center: String,
    pub precinct_code: String,
    pub registered_voters: i64,
    pub ballots_counted: u32,
    pub voters_turnout: String,
    pub sequences: Vec<AuditLogEntry>,
    pub goverment_time: String,
    pub chairperson_name: String,
    pub chairperson_digital_signature: String,
    pub poll_clerk_name: String,
    pub poll_clerk_digital_signature: String,
    pub third_member_name: String,
    pub third_member_digital_signature: String,
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
    pub report_hash: String,
    pub ovcs_version: String,
    pub system_hash: String,
    pub file_logo: String,
    pub file_qrcode_lib: String,
    pub time_printed: String,
    pub date_printed: String,
    pub printing_code: String,
}
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
    async fn prepare_user_data(&self) -> Result<Self::UserData> {
        // Fetch the database client from the pool
        let mut db_client: DbClient = get_hasura_pool()
            .await
            .get()
            .await
            .with_context(|| "Error getting DB pool")?;

        let hasura_transaction = db_client
            .transaction()
            .await
            .with_context(|| "Error starting transaction")?;

        let realm_name = get_event_realm(self.tenant_id.as_str(), self.election_event_id.as_str());
        let mut keycloak_db_client = get_keycloak_pool()
        .await
        .get()
        .await
        .with_context(|| "Error acquiring Keycloak DB pool")?;

        let keycloak_transaction = keycloak_db_client
        .transaction()
        .await
        .with_context(|| "Error starting Keycloak transaction")?;

        let election = match get_election_by_id(
            &hasura_transaction,
            &self.get_tenant_id(),
            &self.get_election_event_id(),
            &self.get_election_id().unwrap()
        )
        .await
        .with_context(|| "Error getting election by id")? 
        {
            Some(election) => election,
            None => return Err(anyhow::anyhow!("Election not found")),
        };   

        let election_general_data = match extract_eleciton_data(&election).await {
            Ok(data) => data,  // Extracting the ElectionData struct out of Ok
            Err(err) => {
                return Err(anyhow::anyhow!(format!("Error fetching election data: {}", err)));
            }
        };

        // Fetch election event data
        let start_election_event = find_scheduled_event_by_election_event_id(
            &hasura_transaction,
            &self.get_tenant_id(),
            &self.get_election_event_id(),
        )
        .await?;

        let voting_period_dates = generate_voting_period_dates(
            start_election_event,
            &self.get_tenant_id(),
            &self.get_election_event_id(),
            Some(&self.get_election_id().unwrap()),
        )?;

        let voting_period_start_date = match voting_period_dates.start_date {
            Some(voting_period_start_date) => voting_period_start_date,
            None => return Err(anyhow::anyhow!(format!("Error fetching election start date: "))),
        };

        let voting_period_end_date = match voting_period_dates.end_date {
            Some(voting_period_end_date) => voting_period_end_date,
            None => return Err(anyhow::anyhow!(format!("Error fetching election end date: "))),
        };

        // Parse the date string into a NaiveDate
        let parsed_date = NaiveDate::parse_from_str(&voting_period_start_date, "%Y-%m-%d").expect("Failed to parse date");
        // Format the date to the desired format
        let election_date = parsed_date.format("%B %d, %Y").to_string();

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
        .await?;

        for item in &electoral_logs.items {
            // Convert the `created` timestamp from Unix time to a formatted date-time string
            let created_datetime = Utc.timestamp_opt(item.created, 0)
            .single()  // Handle the Option, get Some(T) or None
            .expect("Invalid timestamp");
            let formatted_datetime: String = created_datetime.format("%Y-%m-%d %H:%M").to_string();
        
            // Set default username if user_id is None
            let username = item.user_id.clone().unwrap_or_else(|| "Unknown User".to_string());
        
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

        let registered_voters = get_total_number_of_registered_voters_for_country(
            &keycloak_transaction,
            &realm_name,
            &election_general_data.country
        ).await?;

        // Fetch necessary data (dummy placeholders for now)
        let voting_period = "April 10 - May 10, 2024".to_string();
        let ballots_counted = 3500;
        let voters_turnout = "70%".to_string();
        let chairperson_name = "John Doe".to_string();
        let poll_clerk_name = "Jane Smith".to_string();
        let third_member_name = "Alice Johnson".to_string();
        let chairperson_digital_signature = "DigitalSignatureABC".to_string();
        let poll_clerk_digital_signature = "DigitalSignatureDEF".to_string();
        let third_member_digital_signature = "DigitalSignatureGHI".to_string();
        let goverment_time = "18:00".to_string();

        Ok(UserData {
            election_date,
            election_title: election.name,
            voting_period: format!("{} - {}", voting_period_start_date, voting_period_end_date),
            geographic_region: election_general_data.geographical_region,
            post: election_general_data.post,
            country: election_general_data.country,
            voting_center: election_general_data.voting_center,
            precinct_code: election_general_data.clustered_precinct_id,
            registered_voters,
            ballots_counted,
            voters_turnout,
            sequences,
            goverment_time,
            chairperson_name,
            chairperson_digital_signature,
            poll_clerk_name,
            poll_clerk_digital_signature,
            third_member_name,
            third_member_digital_signature
        })
    }

    #[instrument]
    async fn prepare_system_data(
        &self,
        rendered_user_template: String,
    ) -> Result<Self::SystemData> {
        let report_hash = "dummy_report_hash".to_string();
        let ovcs_version = "1.0".to_string();
        let system_hash = "dummy_system_hash".to_string();
        let file_logo = "logo.png".to_string();
        let file_qrcode_lib = "qrcode_lib.png".to_string();
        let mut date_printed = "2024-10-13".to_string();
        let time_printed = "12:10".to_string();
        let printing_code = "XYZ123".to_string();

        // Parse the date string into a NaiveDate
         let date = NaiveDate::parse_from_str(&date_printed, "%Y-%m-%d").expect("Failed to parse date");

        // Format the date to the desired format
        date_printed = date.format("%B %d, %Y").to_string();

        Ok(SystemData {
            report_hash,
            ovcs_version,
            system_hash,
            file_logo,
            file_qrcode_lib,
            date_printed,
            time_printed,
            printing_code,
        })
    }
}