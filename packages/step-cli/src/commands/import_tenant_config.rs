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
#[command(about = "Import a Keycloak/roles config export into a tenant", long_about = None)]
pub struct ImportTenantConfig {
    /// Tenant id - the tenant to import into
    #[arg(long)]
    tenant_id: String,

    /// Document id - from a prior export-tenant-config run
    #[arg(long)]
    document_id: String,

    /// Import the Keycloak realm's clients/groups/roles
    #[arg(long, default_value_t = true)]
    include_keycloak: bool,

    /// Import role/permission-label mappings
    #[arg(long, default_value_t = true)]
    include_roles: bool,

    /// Import the tenant row itself (name/slug/settings) — usually not
    /// wanted when cloning config into an already-created tenant
    #[arg(long, default_value_t = false)]
    include_tenant: bool,
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/import_tenant_config.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct ImportTenantConfigMutation;

impl ImportTenantConfig {
    pub fn run(&self) {
        match import_tenant_config(
            &self.tenant_id,
            &self.document_id,
            self.include_keycloak,
            self.include_roles,
            self.include_tenant,
        ) {
            Ok(()) => {
                println!("{}", "Success! Imported tenant config.".green());
            }
            Err(err) => {
                eprintln!("Error! Failed to import tenant config: {}", err)
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
            Ok(status) if status == "FAILED" => return Err("Import tenant config task failed".into()),
            Ok(_) => {
                if Instant::now().duration_since(start_time) >= timeout {
                    return Err("Timeout while waiting for import tenant config task to complete".into());
                }
                sleep(polling_interval);
            }
            Err(e) => return Err(format!("Error checking task status: {}", e).into()),
        }
    }
}

fn import_tenant_config(
    tenant_id: &str,
    document_id: &str,
    include_keycloak: bool,
    include_roles: bool,
    include_tenant: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = read_config()?;
    let client = reqwest::blocking::Client::new();

    let variables = import_tenant_config_mutation::Variables {
        tenant_id: tenant_id.to_string(),
        document_id: document_id.to_string(),
        import_configurations: Some(import_tenant_config_mutation::ImportOptions {
            include_keycloak: Some(include_keycloak),
            include_roles: Some(include_roles),
            include_tenant: Some(include_tenant),
        }),
    };

    let request_body = ImportTenantConfigMutation::build_query(variables);
    let response = client
        .post(&config.endpoint_url)
        .bearer_auth(config.auth_token)
        .json(&request_body)
        .send()?;

    if response.status().is_success() {
        let response_body: Response<import_tenant_config_mutation::ResponseData> = response.json()?;
        if let Some(data) = response_body.data {
            let Some(output) = data.import_tenant_config else {
                return Err(Box::from("failed importing tenant config"));
            };
            if let Some(err) = output.error {
                Err(Box::from(err))
            } else if let Some(task_execution) = output.task_execution {
                wait_for_task(&task_execution.id)
            } else {
                Ok(())
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
