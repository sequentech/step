// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::template_renderer::*;
use anyhow::{anyhow, Context, Ok, Result};
use async_trait::async_trait;
use deadpool_postgres::Client as DbClient;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};
use chrono::{DateTime, Utc};
use sequent_core::types::templates::EmailConfig;
use crate::postgres::reports::ReportType;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Event {
    post: String,
    country: String,
    testing_date: String,
    initialization_date: String,
    initialization_time: String,
    opening_date: String,
    opening_time: String,
    closing_date: String,
    closing_time: String,
    transmission_date: String,
    transmission_time: String,
    transmission_status: String,
    remarks: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Region {
    name: String,
    events: Vec<Event>,
}
/// Struct for OVCSEvents Data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserData {
   
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemData {
    pub date_printed: String,
    pub time_printed: String,
    pub election_date: String,
    pub election_title: String,
    pub voting_period: String,
    pub regions: Vec<Region>,
    pub precinct_id: String,
    pub goverment_date: String,
    pub goverment_time: String,
    pub local_date: String,
    pub local_time: String,
    pub ovcs_downtime: u32,
    pub software_version: String,
    pub qr_codes: Vec<String>,
    pub report_hash: String,
    pub ovcs_version: String,
    pub system_hash: String,
}

#[derive(Debug)]
pub struct OVCSEventsTemplate {
    pub tenant_id: String,
    pub election_event_id: String,
}

#[async_trait]
impl TemplateRenderer for OVCSEventsTemplate {
    type UserData = UserData;
    type SystemData = SystemData;


    fn get_report_type() -> ReportType {
        ReportType::OVCS_EVENTS
    }

    fn base_name() -> String {
        "ovcs_events".to_string()
    }

    fn prefix(&self) -> String {
        format!("ovcs_events_{}", self.election_event_id)
    }

    fn get_tenant_id(&self) -> String {
        self.tenant_id.clone()
    }

    fn get_election_event_id(&self) -> String {
        self.election_event_id.clone()
    }

    fn get_email_config() -> EmailConfig {
        EmailConfig {
            subject: "Sequent Online Voting - OVCS Events".to_string(),
            plaintext_body: "".to_string(),
            html_body: None,
        }
    }

    // TODO: replace mock data with actual data
    async fn prepare_user_data(&self) -> Result<Option<Self::UserData>> {
        Ok(None)
    }

    /// Prepare system metadata for the report
    async fn prepare_system_data(&self, _rendered_user_template: String) -> Result<Self::SystemData> {
        let data: SystemData = self.prepare_preview_data().await?;
        Ok(data)
    }
}


#[instrument] 
pub async fn generate_ovcs_report(
    document_id: &str,
    tenant_id: &str,
    election_event_id: &str,
    mode: GenerateReportMode,
) -> Result<()> {
    let template = OVCSEventsTemplate {
        tenant_id: tenant_id.to_string(),
        election_event_id: election_event_id.to_string(),
    };
    template
        .execute_report(document_id, tenant_id, election_event_id, false, None, mode)
        .await
}
