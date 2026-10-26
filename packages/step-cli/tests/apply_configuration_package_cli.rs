// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
#[path = "support/http_cli.rs"]
mod http_cli;
use http_cli::command_with_graphql_responses;
use serde_json::{json, Value};

const TASK: &str = "11111111-1111-4111-8111-111111111111";

fn accepted() -> Value {
    json!({"data":{"apply_configuration_package":{"task_execution":{"id":TASK,"execution_status":"IN_PROGRESS"}}}})
}

fn task(status: &str, outcome: &str) -> Value {
    json!({"data":{"sequent_backend_tasks_execution":[{
        "id":TASK,"execution_status":status,"annotations":{"application":{
            "outcome":outcome,"sections":[{"section":"election-event","status":{
                "state":"previewed","preview":{"needsApply":true}
            }}]
        }}
    }]}})
}

fn run(mode: &str, replies: Vec<Value>) -> (std::process::Output, Vec<Value>) {
    let (output, _, requests) = command_with_graphql_responses(
        &[
            "apply-configuration-package",
            "--tenant-id",
            "tenant",
            "--document-id",
            "document",
            "--mode",
            mode,
            "--timeout-seconds",
            "0",
            "--require",
            "election-event",
            "--census",
            "required",
        ],
        Some(json!({"access_token":"synthetic-access"})),
        replies,
    );
    let graphql = requests
        .into_iter()
        .filter(|(path, _)| path.starts_with("POST /graphql "))
        .map(|(_, body)| serde_json::from_slice(&body).unwrap())
        .collect();
    (output, graphql)
}

#[test]
fn successful_preview_and_apply_report_the_actual_task_outcome() {
    for (mode, outcome, expected) in [
        (
            "preview",
            "previewed",
            "Preview only, nothing changed: 1 section(s) would change.",
        ),
        ("apply", "applied", "The tenant matches the package."),
    ] {
        let (output, requests) = run(mode, vec![accepted(), task("SUCCESS", outcome)]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stdout).contains(expected));
        assert_eq!(requests.len(), 2);
        assert_eq!(
            requests[0]["operationName"],
            "ApplyConfigurationPackageMutation"
        );
        assert_eq!(
            requests[0]["variables"],
            json!({"tenantId":"tenant", "documentId":"document", "mode":mode,
            "requirements":{"sections":["election-event"],"census":"required"}})
        );
        assert_eq!(requests[1]["operationName"], "GetTaskExecutionRecord");
        assert_eq!(requests[1]["variables"]["taskExecutionId"], TASK);
    }
}

#[test]
fn rejected_or_missing_submission_never_starts_polling_or_resubmits() {
    for (reply, expected) in [
        (
            json!({"errors":[{"message":"Synthetic admission rejected"}]}),
            "Synthetic admission rejected",
        ),
        (
            json!({"data":{"apply_configuration_package":null}}),
            "the server returned no application task",
        ),
        (json!({}), "Unknown error occurred"),
    ] {
        let (output, requests) = run("apply", vec![reply]);
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains(expected));
        assert_eq!(requests.len(), 1);
    }
}

#[test]
fn failed_missing_and_unsettled_tasks_never_trigger_another_application() {
    for (reply, expected) in [
        (task("FAILED", "failed"), "the application ended as FAILED"),
        (
            task("SUCCESS", "unknown"),
            "the application ended as SUCCESS",
        ),
        (
            json!({"data":{"sequent_backend_tasks_execution":[]}}),
            "No task execution found",
        ),
        (
            task("IN_PROGRESS", "running"),
            "check it again later rather than starting another",
        ),
    ] {
        let (output, requests) = run("apply", vec![accepted(), reply]);
        assert!(!output.status.success());
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(expected),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            requests
                .iter()
                .filter(|r| r["operationName"] == "ApplyConfigurationPackageMutation")
                .count(),
            1
        );
        assert_eq!(requests.len(), 2);
    }
}
