// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::types::hasura_types::*;
use crate::utils::read_config::read_config;
use clap::Args;
use colored::Colorize;
use graphql_client::{GraphQLQuery, Response};
use std::{
    thread::sleep,
    time::{Duration, Instant},
};

#[derive(Args, Debug)]
#[command(about = "Export a tenant's Keycloak/roles config", long_about = None)]
pub struct ExportTenantConfig {
    /// Tenant id - the tenant to export
    #[arg(long)]
    tenant_id: String,
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/export_tenant_config.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct ExportTenantConfigMutation;

impl ExportTenantConfig {
    pub fn run(&self) {
        match export_tenant_config(&self.tenant_id) {
            Ok(document_id) => {
                println!(
                    "{} {}",
                    "Success! Exported tenant config. Document ID:".green(),
                    document_id.cyan()
                );
            }
            Err(err) => {
                eprintln!("Error! Failed to export tenant config: {}", err)
            }
        }
    }
}

fn wait_for_task(task_execution_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let start_time = Instant::now();
    let timeout = Duration::from_secs(300);
    let polling_interval = Duration::from_secs(3);

    loop {
        match crate::utils::tasks::get_task_status(task_execution_id) {
            Ok(status) if status == "SUCCESS" => return Ok(()),
            Ok(status) if status == "FAILED" => return Err("Export tenant config task failed".into()),
            Ok(_) => {
                if Instant::now().duration_since(start_time) >= timeout {
                    return Err("Timeout while waiting for export tenant config task to complete".into());
                }
                sleep(polling_interval);
            }
            Err(e) => return Err(format!("Error checking task status: {}", e).into()),
        }
    }
}

fn export_tenant_config(tenant_id: &str) -> Result<String, Box<dyn std::error::Error>> {
    let config = read_config()?;
    let client = reqwest::blocking::Client::new();

    let variables = export_tenant_config_mutation::Variables {
        tenant_id: Some(tenant_id.to_string()),
    };

    let request_body = ExportTenantConfigMutation::build_query(variables);
    let response = client
        .post(&config.endpoint_url)
        .bearer_auth(config.auth_token)
        .json(&request_body)
        .send()?;

    if response.status().is_success() {
        let response_body: Response<export_tenant_config_mutation::ResponseData> = response.json()?;
        if let Some(data) = response_body.data {
            let Some(output) = data.export_tenant_config else {
                return Err(Box::from("failed exporting tenant config"));
            };
            if let Some(err) = output.error_msg {
                Err(Box::from(err))
            } else {
                wait_for_task(&output.task_execution.id)?;
                Ok(output.document_id)
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
