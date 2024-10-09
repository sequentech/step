// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::template_renderer::*;
use crate::services::database::get_hasura_pool;
use crate::{postgres::election_event::get_election_event_by_id, services::s3::get_minio_url};
use crate::{postgres::scheduled_event::find_scheduled_event_by_election_event_id_and_event_processor};
use crate::services::temp_path::*;
use anyhow::{anyhow, Context, Ok, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::env;
use tracing::{info, instrument};
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;


/// Struct for User Data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserData {
    pub election_start_date: String,
    pub election_title: String,
    pub geograpic_region: String,
    pub area: String,
    pub country: String,
    pub voting_center: String,
    pub chairperson_name: String,
    pub poll_clerk_name: String,
    pub third_member_name: String,
}

/// Struct for System Data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SystemData {
    pub report_hash: String,
    pub ovsc_version: String,
    pub system_hash: String,
    pub file_logo: String,
    pub file_qrcode_lib: String,
    pub date_time_printed: String,
    pub printing_code: String,
}

#[derive(Debug)]
pub struct SBEITemplate {
    tenant_id: String,
    election_event_id: String,
    voter_id: String,
}


#[async_trait]
impl TemplateRenderer for SBEITemplate {
    type UserData = UserData;
    type SystemData = SystemData;

    fn get_tenant_id(&self) -> String {
        self.tenant_id.clone()
    }

    fn get_election_event_id(&self) -> String {
        self.election_event_id.clone()
    }

    fn base_name() -> String {
        "ovcs_information".to_string()
    }

    fn prefix(&self) -> String {
        format!("ovcs_information_{}", self.voter_id)
    }

    async fn prepare_user_data(&self) -> Result<Self::UserData> {
        // Fetch the Hasura database client from the pool
        let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .with_context(|| "Error getting hasura db pool")?;

        let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .with_context(|| "Error starting hasura transaction")?;

         // Fetch election event data
        let election_event =
        get_election_event_by_id(&hasura_transaction, &self.tenant_id,  &self.election_event_id)
            .await
            .with_context(|| "Error obtaining election event")?;


        // Fetch election event data
        let start_election_event = find_scheduled_event_by_election_event_id_and_event_processor(
            &hasura_transaction,
            &self.tenant_id,
            &self.election_event_id,
            "START_VOTING_PERIOD"
        )
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)));

        // TODO: replace mock data with actual data
        let mut election_start_date: String;
        // if let Some(cron_config) = start_election_event.get(0).and_then(|event| event.cron_config.clone()) {
        //     // Now cron_config is a CronConfig, not an Option
        //     if let Some(scheduled_date) = cron_config.scheduled_date {
        //         election_start_date = scheduled_date;
        //     } 
            
        // }
        

        let temp_val: &str = "test";
        Ok(UserData{
            election_start_date: temp_val.to_string(),
            election_title: election_event.name.clone(),
            geograpic_region: temp_val.to_string(),
            area: temp_val.to_string(),
            country: temp_val.to_string(),
            voting_center: temp_val.to_string(),
            chairperson_name: temp_val.to_string(),
            poll_clerk_name: temp_val.to_string(),
            third_member_name: temp_val.to_string(),
        })
    }



    async fn prepare_system_data(
        &self,
        rendered_user_template: String,
    ) -> Result<Self::SystemData> {
        let public_asset_path = get_public_assets_path_env_var()?;
        let minio_endpoint_base =
            get_minio_url().with_context(|| "Error getting minio endpoint")?;

        Ok(SystemData {
            report_hash: String::new(),
            ovsc_version: String::new(),
            system_hash: String::new(),
            file_logo: String::new(),
            file_qrcode_lib: String::new(),
            date_time_printed: String::new(),
            printing_code: String::new(),
        })
    }
}