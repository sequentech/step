// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::types::hasura_types::*;
use crate::utils::read_config::{read_config, refresh_and_save_token};
use clap::Args;
use colored::Colorize;
use graphql_client::{GraphQLQuery, Response};
use std::{
    thread::sleep,
    time::{Duration, Instant},
};

#[derive(Args, Debug)]
#[command(about = "Delete a tenant (must have no election events left)", long_about = None)]
pub struct DeleteTenantCLI {
    /// ID of the tenant to delete
    #[arg(long)]
    tenant_id: String,
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/delete_tenant.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct DeleteTenant;

impl DeleteTenantCLI {
    pub fn run(&self) {
        match delete_tenant(&self.tenant_id) {
            Ok(id) => {
                println!(
                    "{} {}",
                    "Success! Tenant deleted successfully! ID:".green(),
                    id.cyan()
                );
            }
            Err(err) => {
                eprintln!("Error! Failed to delete tenant: {}", err)
            }
        }
    }
}

fn wait_for_task(task_execution_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let start_time = Instant::now();
    let timeout = Duration::from_secs(300);
    let polling_interval = Duration::from_secs(3);

    loop {
        // Best-effort refresh before each poll — a slow deletion can outlast
        // the access token's own lifetime. See the same fix in
        // delete_election_event.rs's wait_for_task.
        if let Err(err) = refresh_and_save_token() {
            eprintln!("Warning: failed to refresh auth token: {}", err);
        }
        match crate::utils::tasks::get_task_status(task_execution_id) {
            Ok(status) if status == "SUCCESS" => return Ok(()),
            Ok(status) if status == "FAILED" => return Err("Delete tenant task failed".into()),
            Ok(_) => {
                if Instant::now().duration_since(start_time) >= timeout {
                    return Err("Timeout while waiting for delete tenant task to complete".into());
                }
                sleep(polling_interval);
            }
            Err(e) => return Err(format!("Error checking task status: {}", e).into()),
        }
    }
}

fn delete_tenant(tenant_id: &str) -> Result<String, Box<dyn std::error::Error>> {
    let config = read_config()?;
    let client = reqwest::blocking::Client::new();

    let variables = delete_tenant::Variables {
        tenant_id: tenant_id.to_string(),
    };

    let request_body = DeleteTenant::build_query(variables);
    let response = client
        .post(&config.endpoint_url)
        .bearer_auth(config.auth_token)
        .json(&request_body)
        .send()?;

    if response.status().is_success() {
        let response_body: Response<delete_tenant::ResponseData> = response.json()?;
        if let Some(data) = response_body.data {
            let Some(output) = data.delete_tenant else {
                return Err(Box::from("failed deleting tenant"));
            };
            if let Some(err) = output.error_msg {
                Err(Box::from(err))
            } else {
                if let Some(task_execution) = output.task_execution {
                    wait_for_task(&task_execution.id)?;
                }
                Ok(output.id.unwrap_or_else(|| tenant_id.to_string()))
            }
        } else if let Some(errors) = response_body.errors {
            let error_messages: Vec<String> = errors.into_iter().map(|e| e.message).collect();
            Err(Box::from(error_messages.join(", ")))
        } else {
            Err(Box::from("Unknown error occurred"))
        }
    } else {
        let status = response.status();
        let error_message = response.text()?;
        let error = format!("HTTP Status: {}\nError Message: {}", status, error_message);
        Err(Box::from(error))
    }
}
