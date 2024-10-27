// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::report_variables::{
    extract_election_data, generate_voters_turnout, get_date_and_time,
    get_election_contests_area_results_and_total_ballot_counted,
};
use super::template_renderer::*;
use crate::postgres::election::get_election_by_id;
use crate::postgres::reports::ReportType;
use crate::postgres::scheduled_event::find_scheduled_event_by_election_event_id;
use crate::services::database::{get_hasura_pool, get_keycloak_pool};
use crate::services::temp_path::*;
use crate::services::users::count_keycloak_enabled_users_by_area_id;
use crate::{postgres::election_event::get_election_event_by_id, services::s3::get_minio_url};
use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{Local, TimeZone};
use deadpool_postgres::{Client as DbClient, Transaction};
use rocket::http::Status;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::types::scheduled_event::generate_voting_period_dates;
use sequent_core::types::templates::EmailConfig;
use serde::{Deserialize, Serialize};
use std::env;
use tracing::{info, instrument};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserData {
    pub date_printed: String,
    pub closing_election_datetime: String,
    pub election_date: String,
    pub election_title: String,
    pub voting_period_start: String,
    pub voting_period_end: String,
    pub geographical_region: String,
    pub post: String,
    pub area_id: String,
    pub voting_center: String,
    pub precinct_code: String,
    pub registered_voters: i64,
    pub ballots_counted: i64,
    pub voters_turnout: i64,
    pub candidates: Vec<Candidate>,
    pub chairperson_name: String,
    pub chairperson_digital_signature: String,
    pub poll_clerk_name: String,
    pub poll_clerk_digital_signature: String,
    pub third_member_name: String,
    pub third_member_digital_signature: String,
    pub report_hash: String,
    pub software_version: String,
    pub ovcs_version: String,
    pub system_hash: String,
    pub qr_codes: Vec<String>,
    pub goverment_time: String,
}

/// Struct for each candidate's data
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Candidate {
    pub position: String,
    pub position_name: String,
    pub name_in_ballot: String,
    pub votes_garnered: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SystemData {
    pub rendered_user_template: String,
}

#[derive(Debug)]
pub struct ElectoralResults {
    tenant_id: String,
    election_event_id: String,
    election_id: String,
}

#[async_trait]
impl TemplateRenderer for ElectoralResults {
    type UserData = UserData;
    type SystemData = SystemData;

    fn get_report_type() -> ReportType {
        ReportType::ELECTORAL_RESULTS
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
        "electoral_results".to_string()
    }

    fn prefix(&self) -> String {
        format!(
            "{base_name}_{election_event_id}_{election_id}",
            base_name = Self::base_name(),
            election_event_id = self.election_event_id,
            election_id = self.election_id,
        )
    }

    fn get_email_config() -> EmailConfig {
        EmailConfig {
            subject: "Sequent Online Voting - Electoral Results".to_string(),
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
        Err(anyhow::anyhow!("Unimplemented"))
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
pub async fn generate_report(
    document_id: &str,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
    mode: GenerateReportMode,
    hasura_transaction: Option<&Transaction<'_>>,
    keycloak_transaction: Option<&Transaction<'_>>,
) -> Result<()> {
    let template = ElectoralResults {
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
