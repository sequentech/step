// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::types::hasura_types::*;
use crate::utils::read_config::{read_config, refresh_and_save_token};
use clap::Args;
use colored::Colorize;
use electoral_log::adapters::postgres::{AuditAnnotations, AuditOutcome};
use graphql_client::{GraphQLQuery, Response};
use sequent_core::types::hasura::extra::TasksExecutionStatus;
use serde_json::Value;
use std::{
    thread::sleep,
    time::{Duration, Instant},
};

const POLLING_INTERVAL: Duration = Duration::from_secs(5);

#[derive(Args, Debug)]
#[command(
    about = "Audit an election event's electoral log",
    long_about = "Start an electoral-log audit task, wait for it and print its logs. \
                  Exits with an error when the audit reports findings or fails."
)]
pub struct AuditElectoralLogCLI {
    /// ID of the election event to audit
    #[arg(long)]
    election_event_id: String,

    /// Seconds to wait for the audit to finish
    #[arg(long, default_value_t = 1800)]
    timeout_secs: u64,
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/audit_electoral_log.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct AuditElectoralLog;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/get_task_execution_result.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct GetTaskExecutionResult;

type CliResult<T> = Result<T, Box<dyn std::error::Error>>;

impl AuditElectoralLogCLI {
    pub fn run(&self) -> CliResult<()> {
        let task_id = start_audit(&self.election_event_id)?;
        println!("{} {}", "Audit task started:".green(), task_id.cyan());
        let task = wait_for_audit(&task_id, Duration::from_secs(self.timeout_secs))?;
        for line in log_lines(&task.logs) {
            println!("{line}");
        }
        if status(&task) == Some(TasksExecutionStatus::SUCCESS) {
            println!("{}", "Electoral log audit is clean".green());
            Ok(())
        } else {
            Err(format!(
                "Electoral log audit {}: {}",
                task.execution_status,
                outcome(&task.annotations)
            )
            .into())
        }
    }
}

fn graphql_errors(errors: Option<Vec<graphql_client::Error>>) -> Box<dyn std::error::Error> {
    match errors {
        Some(errors) => errors
            .into_iter()
            .map(|error| error.message)
            .collect::<Vec<_>>()
            .join(", ")
            .into(),
        None => "Empty GraphQL response".into(),
    }
}

fn start_audit(election_event_id: &str) -> CliResult<String> {
    let config = read_config()?;
    let request_body = AuditElectoralLog::build_query(audit_electoral_log::Variables {
        election_event_id: election_event_id.to_string(),
    });
    let response = reqwest::blocking::Client::new()
        .post(&config.endpoint_url)
        .bearer_auth(config.auth_token)
        .json(&request_body)
        .send()?;
    if !response.status().is_success() {
        return Err(format!("HTTP Status: {}\n{}", response.status(), response.text()?).into());
    }
    let body: Response<audit_electoral_log::ResponseData> = response.json()?;
    match body.data {
        Some(data) => Ok(data.audit_electoral_log.task_execution.id),
        None => Err(graphql_errors(body.errors)),
    }
}

type TaskResult = get_task_execution_result::GetTaskExecutionResultSequentBackendTasksExecution;

fn fetch_task(task_id: &str) -> CliResult<TaskResult> {
    let config = read_config()?;
    let request_body = GetTaskExecutionResult::build_query(get_task_execution_result::Variables {
        task_execution_id: task_id.to_string(),
    });
    let body: Response<get_task_execution_result::ResponseData> = reqwest::blocking::Client::new()
        .post(&config.endpoint_url)
        .bearer_auth(config.auth_token)
        .json(&request_body)
        .send()?
        .json()?;
    match body.data {
        Some(data) => data
            .sequent_backend_tasks_execution
            .into_iter()
            .next()
            .ok_or_else(|| "Audit task not found".into()),
        None => Err(graphql_errors(body.errors)),
    }
}

fn wait_for_audit(task_id: &str, timeout: Duration) -> CliResult<TaskResult> {
    let start = Instant::now();
    loop {
        if let Err(error) = refresh_and_save_token() {
            eprintln!("Warning: failed to refresh auth token: {error}");
        }
        let task = fetch_task(task_id)?;
        if status(&task) != Some(TasksExecutionStatus::IN_PROGRESS) {
            return Ok(task);
        }
        if start.elapsed() >= timeout {
            return Err(format!("Timeout while waiting for audit task {task_id}").into());
        }
        sleep(POLLING_INTERVAL);
    }
}

fn log_lines(logs: &Option<Value>) -> Vec<String> {
    logs.as_ref()
        .and_then(Value::as_array)
        .map(|entries| {
            entries
                .iter()
                .filter_map(|entry| {
                    let text = entry.get("log_text")?.as_str()?;
                    let date = entry
                        .get("created_date")
                        .and_then(Value::as_str)
                        .unwrap_or("");
                    Some(format!("{date} {text}").trim().to_string())
                })
                .collect()
        })
        .unwrap_or_default()
}

fn status(task: &TaskResult) -> Option<TasksExecutionStatus> {
    task.execution_status.parse().ok()
}

fn outcome(annotations: &Option<Value>) -> String {
    let recorded = annotations
        .clone()
        .and_then(|value| serde_json::from_value::<AuditAnnotations>(value).ok());
    match recorded {
        Some(AuditAnnotations {
            outcome: AuditOutcome::Findings,
            findings,
            ..
        }) => format!(
            "{} findings",
            findings.map(|count| count.to_string()).unwrap_or_default()
        ),
        Some(annotations) => annotations.outcome.to_string(),
        None => "no result recorded".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn prints_task_logs_in_order() {
        let logs = Some(json!([
            {"created_date": "2026-10-04T10:00:00Z", "log_text": "Task started"},
            {"created_date": "2026-10-04T10:00:01Z", "log_text": "Finding: record 7 was changed"},
            {"log_text": "no date"},
            {"created_date": "2026-10-04T10:00:02Z"},
        ]));
        assert_eq!(
            log_lines(&logs),
            vec![
                "2026-10-04T10:00:00Z Task started",
                "2026-10-04T10:00:01Z Finding: record 7 was changed",
                "no date",
            ]
        );
        assert!(log_lines(&None).is_empty());
        assert!(log_lines(&Some(json!({"not": "a list"}))).is_empty());
    }

    #[test]
    fn describes_the_recorded_outcome() {
        assert_eq!(
            outcome(&Some(json!({"outcome": "findings", "findings": 3}))),
            "3 findings"
        );
        assert_eq!(outcome(&Some(json!({"outcome": "error"}))), "error");
        assert_eq!(
            outcome(&Some(json!({"outcome": "unknown"}))),
            "no result recorded"
        );
        assert_eq!(outcome(&None), "no result recorded");
    }
}
