// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::utils::read_config::read_config;
use clap::Args;
use graphql_client::Response;
use sequent_core::types::permissions::Permissions;
use serde::Deserialize;
use serde_json::{json, Value};
use std::{error::Error, time::Duration};
use uuid::Uuid;

#[derive(Args)]
#[command(about = "Compare official and tested events and emit signed readiness evidence as JSON")]
pub struct ReadinessCheck {
    /// Official election event UUID, whose voting must remain untouched
    #[arg(long)]
    official: Uuid,
    /// Closed and sealed test election event UUID
    #[arg(long)]
    tested: Uuid,
}

#[derive(Deserialize)]
struct Verdict {
    passed: bool,
    log_entry_id: Option<String>,
}

impl ReadinessCheck {
    pub fn run(&self) -> Result<(), Box<dyn Error>> {
        let config = read_config()?;
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(300))
            .build()?;
        // Readiness returns JSONB, so preserve every evidence field without
        // coupling the CLI to a generated GraphQL schema revision.
        let response: Response<Value> = client
            .post(&config.endpoint_url)
            .bearer_auth(&config.auth_token)
            .header("x-hasura-role", Permissions::READINESS_CHECK.to_string())
            .json(&json!({
                "operationName": "ReadinessCheck",
                "query": "mutation ReadinessCheck($official_event_id: String!, $tested_event_id: String!) { readiness_check(official_event_id: $official_event_id, tested_event_id: $tested_event_id) }",
                "variables": {
                    "official_event_id": self.official.to_string(),
                    "tested_event_id": self.tested.to_string(),
                },
            }))
            .send()?
            .error_for_status()?
            .json()?;
        if let Some(errors) = response.errors.filter(|errors| !errors.is_empty()) {
            return Err(errors
                .into_iter()
                .map(|error| error.message)
                .collect::<Vec<_>>()
                .join(", ")
                .into());
        }
        let report = response
            .data
            .and_then(|mut data| data.get_mut("readiness_check").map(Value::take))
            .filter(|report| !report.is_null())
            .ok_or("Readiness check returned no report")?;
        println!("{}", serde_json::to_string_pretty(&report)?);
        let verdict: Verdict = serde_json::from_value(report)?;
        if !verdict.passed {
            return Err("Readiness verification failed; inspect the JSON checks".into());
        }
        if verdict.log_entry_id.as_deref().is_none_or(str::is_empty) {
            return Err("Readiness check returned no signed log entry".into());
        }
        Ok(())
    }
}
