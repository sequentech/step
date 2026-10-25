// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::types::hasura_types::*;
use crate::utils::read_config::read_config;
use crate::utils::upload_file::GetUploadUrl;
use clap::{Args, ValueEnum};
use colored::Colorize;
use graphql_client::{GraphQLQuery, Response};
use serde_json::{json, Value};
use std::{
    thread::sleep,
    time::{Duration, Instant},
};

/// Preview only, or preview and apply.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Mode {
    Preview,
    Apply,
}

impl Mode {
    fn wire(self) -> &'static str {
        match self {
            Mode::Preview => "preview",
            Mode::Apply => "apply",
        }
    }
}

/// Whether the event must carry its census.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Census {
    Optional,
    Required,
}

#[derive(Args, Debug)]
#[command(
    about = "Preview or apply a whole configuration delivery (tenant configuration and settings, roles, administrators, trustees, event) to a tenant",
    long_about = None
)]
pub struct ApplyConfigurationPackage {
    /// Tenant id - the tenant to apply to
    #[arg(long)]
    tenant_id: String,

    /// The delivery zip to upload; or pass --document-id of one uploaded before
    #[arg(
        long,
        conflicts_with = "document_id",
        required_unless_present = "document_id"
    )]
    file: Option<String>,

    /// A delivery uploaded before
    #[arg(long)]
    document_id: Option<String>,

    /// Preview only, or preview and then apply
    #[arg(long, value_enum)]
    mode: Mode,

    /// A section the delivery must carry; repeat for each (tenant-configuration,
    /// tenant-settings, role-permissions, admin-accounts, trustees, election-event)
    #[arg(long = "require")]
    required_sections: Vec<String>,

    /// Whether the event must carry its census
    #[arg(long, value_enum, default_value = "optional")]
    census: Census,

    /// How long to wait for the application to finish, in seconds
    #[arg(long, default_value_t = 1800)]
    timeout_seconds: u64,
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/apply_configuration_package.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct ApplyConfigurationPackageMutation;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/get_task_execution_record.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct GetTaskExecutionRecord;

impl ApplyConfigurationPackage {
    /// Execute the command, preserving failures for shell automation.
    pub fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        let document_id = match (&self.file, &self.document_id) {
            (_, Some(id)) => id.clone(),
            (Some(file), None) => GetUploadUrl::upload(file.clone(), false)?,
            (None, None) => return Err("pass --file or --document-id".into()),
        };
        let requirements = json!({
            "sections": self.required_sections,
            "census": match self.census {
                Census::Optional => "optional",
                Census::Required => "required",
            },
        });
        let task_id = submit(&self.tenant_id, &document_id, self.mode, requirements)?;
        println!("Application task {}", task_id.cyan());
        let (status, annotations) =
            wait_for_task(&task_id, Duration::from_secs(self.timeout_seconds))?;
        for line in summary(&annotations) {
            println!("{line}");
        }
        match (status.as_str(), conclusion(&annotations)) {
            ("SUCCESS", Some(text)) => {
                println!("{}", text.green());
                Ok(())
            }
            _ => Err(format!("the application ended as {status}; see the sections above").into()),
        }
    }
}

fn submit(
    tenant_id: &str,
    document_id: &str,
    mode: Mode,
    requirements: Value,
) -> Result<String, Box<dyn std::error::Error>> {
    let config = read_config()?;
    let variables = apply_configuration_package_mutation::Variables {
        tenant_id: tenant_id.to_string(),
        document_id: document_id.to_string(),
        mode: mode.wire().to_string(),
        requirements: Some(requirements),
    };
    let response = reqwest::blocking::Client::new()
        .post(&config.endpoint_url)
        .bearer_auth(config.auth_token)
        .json(&ApplyConfigurationPackageMutation::build_query(variables))
        .send()?;
    if !response.status().is_success() {
        return Err(format!("HTTP Status: {}\n{}", response.status(), response.text()?).into());
    }
    let body: Response<apply_configuration_package_mutation::ResponseData> = response.json()?;
    match (body.data, body.errors) {
        (Some(data), _) => data
            .apply_configuration_package
            .map(|output| output.task_execution.id)
            .ok_or_else(|| "the server returned no application task".into()),
        (None, Some(errors)) => Err(errors
            .into_iter()
            .map(|e| e.message)
            .collect::<Vec<_>>()
            .join(", ")
            .into()),
        (None, None) => Err("Unknown error occurred".into()),
    }
}

fn task_record(task_id: &str) -> Result<(String, Value), Box<dyn std::error::Error>> {
    let config = read_config()?;
    let variables = get_task_execution_record::Variables {
        task_execution_id: task_id.to_string(),
    };
    let response = reqwest::blocking::Client::new()
        .post(&config.endpoint_url)
        .bearer_auth(config.auth_token)
        .json(&GetTaskExecutionRecord::build_query(variables))
        .send()?;
    let body: Response<get_task_execution_record::ResponseData> = response.json()?;
    let task = body
        .data
        .and_then(|data| data.sequent_backend_tasks_execution.into_iter().next())
        .ok_or("No task execution found")?;
    Ok((
        task.execution_status,
        task.annotations.unwrap_or(Value::Null),
    ))
}

/// Wait until the task ends; its status is what the platform recorded, never
/// assumed from the request having been accepted.
fn wait_for_task(
    task_id: &str,
    timeout: Duration,
) -> Result<(String, Value), Box<dyn std::error::Error>> {
    let start = Instant::now();
    loop {
        crate::utils::read_config::refresh_and_save_token()?;
        let (status, annotations) = task_record(task_id)?;
        if status == "SUCCESS" || status == "FAILED" {
            return Ok((status, annotations));
        }
        if start.elapsed() >= timeout {
            return Err(format!(
                "the application task {task_id} is still {status}; check it again later rather than starting another"
            )
            .into());
        }
        sleep(Duration::from_secs(3));
    }
}

/// What a finished run says about the tenant: a preview counts the sections
/// that would change; only an application says the tenant matches.
fn conclusion(annotations: &Value) -> Option<String> {
    let record = &annotations["application"];
    match record["outcome"].as_str()? {
        "applied" => Some("The tenant matches the package.".to_string()),
        "previewed" => {
            let changing = record["sections"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|s| s["status"]["preview"]["needsApply"].as_bool() == Some(true))
                .count();
            Some(format!(
                "Preview only, nothing changed: {changing} section(s) would change."
            ))
        }
        _ => None,
    }
}

/// One line per section of the application record: its state, and the task
/// that applied it when there is one.
fn summary(annotations: &Value) -> Vec<String> {
    let record = &annotations["application"];
    let mut lines = vec![format!(
        "Outcome: {}",
        record["outcome"].as_str().unwrap_or("unknown")
    )];
    for section in record["sections"].as_array().into_iter().flatten() {
        let status = &section["status"];
        let mut line = format!(
            "  {:<22} {}",
            section["section"].as_str().unwrap_or("?"),
            status["state"].as_str().unwrap_or("?")
        );
        let task = status["submission"]["taskId"]
            .as_str()
            .or_else(|| status["taskId"].as_str());
        if let Some(task) = task {
            line.push_str(&format!(" (task {task})"));
        }
        if let Some(error) = status["error"].as_str() {
            line.push_str(&format!(": {error}"));
        }
        for blocker in status["preview"]["blockers"]
            .as_array()
            .into_iter()
            .flatten()
        {
            line.push_str(&format!(
                "\n      blocked: {}",
                blocker.as_str().unwrap_or("?")
            ));
        }
        lines.push(line);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_summary_names_each_sections_state_task_and_blockers() {
        let annotations = json!({"application": {
            "outcome": "failed",
            "sections": [
                {"section": "role-permissions", "status": {"state": "applied", "submission": {"taskId": "t-1"}, "reconciled": false}},
                {"section": "admin-accounts", "status": {"state": "blocked", "preview": {"blockers": ["group vanished"]}}},
                {"section": "election-event", "status": {"state": "failed", "error": "boom", "taskId": "t-2"}},
            ]
        }});
        assert_eq!(
            summary(&annotations),
            vec![
                "Outcome: failed".to_string(),
                "  role-permissions       applied (task t-1)".to_string(),
                "  admin-accounts         blocked\n      blocked: group vanished".to_string(),
                "  election-event         failed (task t-2): boom".to_string(),
            ]
        );
    }

    #[test]
    fn a_preview_never_says_the_tenant_matches() {
        let previewed = json!({"application": {"outcome": "previewed", "sections": [
            {"section": "trustees", "status": {"state": "previewed", "preview": {"needsApply": true}}},
            {"section": "election-event", "status": {"state": "previewed", "preview": {"needsApply": false}}},
        ]}});
        assert_eq!(
            conclusion(&previewed).as_deref(),
            Some("Preview only, nothing changed: 1 section(s) would change.")
        );
        let applied = json!({"application": {"outcome": "applied", "sections": []}});
        assert_eq!(
            conclusion(&applied).as_deref(),
            Some("The tenant matches the package.")
        );
        assert_eq!(
            conclusion(&json!({"application": {"outcome": "failed"}})),
            None
        );
    }

    #[test]
    fn the_modes_are_the_servers_wire_names() {
        assert_eq!(Mode::Preview.wire(), "preview");
        assert_eq!(Mode::Apply.wire(), "apply");
    }
}
