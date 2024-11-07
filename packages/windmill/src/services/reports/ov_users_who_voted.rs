// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::report_variables::{
    extract_election_data, get_app_hash, get_app_version, get_date_and_time, get_report_hash,
};
use super::template_renderer::*;
use crate::postgres::area::get_areas_by_election_id;
use crate::postgres::election::get_election_by_id;
use crate::postgres::reports::ReportType;
use crate::postgres::scheduled_event::find_scheduled_event_by_election_event_id;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use deadpool_postgres::Transaction;
use sequent_core::types::scheduled_event::generate_voting_period_dates;
use sequent_core::types::templates::EmailConfig;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};

/// Struct for User Data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserDataArea {
    pub date_printed: String,
    pub election_date: String,
    pub election_title: String,
    pub voting_period_start: String,
    pub voting_period_end: String,
    pub post: String,
    pub area_name: String,
    pub voters: Vec<Voter>,
    pub voted: u32,
    pub not_voted: u32,
    pub not_pre_enrolled: u32,
    pub voting_privilege_voted: u32,
    pub total: u32,
    pub report_hash: String,
    pub software_version: String,
    pub ovcs_version: String,
    pub system_hash: String,
    pub qr_code: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserData {
    pub areas: Vec<UserDataArea>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemData {
    pub rendered_user_template: String,
    pub file_qrcode_lib: String,
}

/// Struct for each voter
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Voter {
    pub number: u32,
    pub last_name: String,
    pub first_name: String,
    pub middle_name: String,
    pub suffix: String,
    pub id: String,
    pub date_voted: String,
}

/// Struct for OVUsersWhoVotedTemplate
#[derive(Debug)]
pub struct OVUsersWhoVotedTemplate {
    tenant_id: String,
    election_event_id: String,
    pub election_id: Option<String>,
}

#[async_trait]
impl TemplateRenderer for OVUsersWhoVotedTemplate {
    type UserData = UserData;
    type SystemData = SystemData;

    fn get_report_type() -> ReportType {
        ReportType::OV_USERS
    }

    fn get_tenant_id(&self) -> String {
        self.tenant_id.clone()
    }

    fn get_election_event_id(&self) -> String {
        self.election_event_id.clone()
    }

    fn get_election_id(&self) -> Option<String> {
        self.election_id.clone()
    }

    fn base_name() -> String {
        "ov_users_who_voted".to_string()
    }

    fn prefix(&self) -> String {
        format!(
            "ov_users_who_voted_{}_{}_{}",
            self.tenant_id,
            self.election_event_id,
            self.election_id.clone().unwrap_or_default()
        )
    }

    fn get_email_config() -> EmailConfig {
        EmailConfig {
            subject: "Sequent Online Voting - OV Users Who Voted".to_string(),
            plaintext_body: "".to_string(),
            html_body: None,
        }
    }

    #[instrument(err, skip(self, hasura_transaction, keycloak_transaction))]
    async fn prepare_user_data(
        &self,
        hasura_transaction: &Transaction<'_>,
        keycloak_transaction: &Transaction<'_>,
    ) -> Result<Self::UserData> {
        let Some(election_id) = &self.election_id else {
            return Err(anyhow!("Empty election_id"));
        };
        let date_printed = get_date_and_time();

        let election = match get_election_by_id(
            &hasura_transaction,
            &self.tenant_id,
            &self.election_event_id,
            &election_id,
        )
        .await
        .with_context(|| "Error getting election by id")?
        {
            Some(election) => election,
            None => return Err(anyhow::anyhow!("Election not found")),
        };

        let election_general_data = extract_election_data(&election)
            .await
            .map_err(|err| anyhow!("Error extract election annotations {err}"))?;

        // Fetch election event data
        let start_election_event = find_scheduled_event_by_election_event_id(
            &hasura_transaction,
            &self.tenant_id,
            &self.election_event_id,
        )
        .await
        .map_err(|e| {
            anyhow::anyhow!("Error getting scheduled event by election event_id: {}", e)
        })?;

        // Fetch election's voting periods
        let voting_period_dates = generate_voting_period_dates(
            start_election_event,
            &self.tenant_id,
            &self.election_event_id,
            Some(&election_id),
        )?;

        // extract start date from voting period
        let voting_period_start_date = voting_period_dates.start_date.unwrap_or_default();
        // extract end date from voting period
        let voting_period_end_date = voting_period_dates.end_date.unwrap_or_default();

        let election_date: &String = &voting_period_start_date;

        let election_areas = get_areas_by_election_id(
            &hasura_transaction,
            &self.tenant_id,
            &self.election_event_id,
            &election_id,
        )
        .await
        .map_err(|err| anyhow!("Error at get_areas_by_election_id: {err:?}"))?;

        let app_hash = get_app_hash();
        let app_version = get_app_version();
        let report_hash = get_report_hash(&ReportType::OV_USERS_WHO_VOTED.to_string())
            .await
            .unwrap_or("-".to_string());

        let mut areas: Vec<UserDataArea> = vec![];

        for area in election_areas.iter() {
            // TODO: fix mock data - Use function from report_variables.rs when finished
            let voters = vec![Voter {
                number: 1,
                first_name: "Juan".to_string(),
                last_name: "Dela Cruz".to_string(),
                middle_name: "Garcia".to_string(),
                suffix: "".to_string(),
                id: "OV12345".to_string(),
                date_voted: "2024-05-09T14:30:00-04:00".to_string(),
            }];

            let area_name = area.clone().name.unwrap_or("-".to_string());

            areas.push(UserDataArea {
                date_printed: date_printed.clone(),
                election_date: election_date.to_string(),
                election_title: election.name.clone(),
                post: election_general_data.post.clone(),
                area_name,
                voting_period_start: voting_period_start_date.clone(),
                voting_period_end: voting_period_end_date.clone(),
                voted: 0,                  //TODO: fix mock data
                not_voted: 0,              //TODO: fix mock data
                not_pre_enrolled: 0,       //TODO: fix mock data
                voters,                    //TODO: fix mock data
                voting_privilege_voted: 0, //TODO: fix mock data
                total: 0,                  //TODO: fix mock data
                report_hash: report_hash.clone(),
                ovcs_version: app_version.clone(),
                system_hash: app_hash.clone(),
                software_version: app_version.clone(),
                qr_code: "code1".to_string(),
            })
        }

        Ok(UserData { areas })
    }

    // Prepare system data
    #[instrument]
    async fn prepare_system_data(
        &self,
        rendered_user_template: String,
    ) -> Result<Self::SystemData> {
        let file_qrcode_lib: &str = "test";
        Ok(SystemData {
            rendered_user_template,
            file_qrcode_lib: file_qrcode_lib.to_string(),
        })
    }
}

#[instrument(err, skip(hasura_transaction, keycloak_transaction))]
pub async fn generate_ov_users_who_voted_report(
    document_id: &str,
    tenant_id: &str,
    election_event_id: &str,
    election_id: Option<&str>,
    mode: GenerateReportMode,
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
) -> Result<()> {
    let template = OVUsersWhoVotedTemplate {
        tenant_id: tenant_id.to_string(),
        election_event_id: election_event_id.to_string(),
        election_id: election_id.map(|s| s.to_string()),
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
