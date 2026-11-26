// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::utils::read_config::read_config;
use clap::{Args, ValueEnum};
use graphql_client::Response;
use sequent_core::types::permissions::Permissions;
use serde::Deserialize;
use serde_json::{json, Value};
use std::{error::Error, time::Duration};
use uuid::Uuid;

/// Which comparison to run. Only `production` can record signed evidence.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum Comparison {
    /// The strict check that records ReadinessVerified on success
    #[default]
    Production,
    /// Read-only: ERT against STL, each with its own certified census
    RehearsalErtStl,
    /// Read-only: the official event against ERT
    RehearsalOfficialErt,
    /// Read-only: the official event against STL; the subset is never verified
    RehearsalOfficialStl,
}

impl Comparison {
    fn wire(self) -> &'static str {
        match self {
            Comparison::Production => "production",
            Comparison::RehearsalErtStl => "rehearsal_ert_stl",
            Comparison::RehearsalOfficialErt => "rehearsal_official_ert",
            Comparison::RehearsalOfficialStl => "rehearsal_official_stl",
        }
    }
}

#[derive(Args)]
#[command(about = "Compare official and tested events and emit signed readiness evidence as JSON")]
pub struct ReadinessCheck {
    /// Official election event UUID, whose voting must remain untouched
    #[arg(long)]
    official: Option<Uuid>,
    /// Closed and sealed test election event UUID (production only)
    #[arg(long)]
    tested: Option<Uuid>,
    /// Comparison to run; rehearsal modes are read-only and never record evidence
    #[arg(long, value_enum, default_value_t = Comparison::Production)]
    comparison: Comparison,
    /// ERT election event UUID (rehearsal modes)
    #[arg(long)]
    ert: Option<Uuid>,
    /// STL election event UUID (rehearsal modes)
    #[arg(long)]
    stl: Option<Uuid>,
}

#[derive(Deserialize)]
struct Verdict {
    passed: bool,
    log_entry_id: Option<String>,
}

const PRODUCTION_QUERY: &str = "mutation ReadinessCheck($official_event_id: String!, $tested_event_id: String!) { readiness_check(official_event_id: $official_event_id, tested_event_id: $tested_event_id) }";
const REHEARSAL_QUERY: &str = "mutation ReadinessCheck($official_event_id: String!, $tested_event_id: String!, $comparison: ReadinessComparison!) { readiness_check(official_event_id: $official_event_id, tested_event_id: $tested_event_id, comparison: $comparison) }";

impl ReadinessCheck {
    /// The two request positions, named by role for each comparison.
    fn positions(&self) -> Result<(Uuid, Uuid), Box<dyn Error>> {
        let (first, second, unused): (_, _, [(&str, Option<Uuid>); 2]) = match self.comparison {
            Comparison::Production => (
                ("--official", self.official),
                ("--tested", self.tested),
                [("--ert", self.ert), ("--stl", self.stl)],
            ),
            Comparison::RehearsalErtStl => (
                ("--ert", self.ert),
                ("--stl", self.stl),
                [("--official", self.official), ("--tested", self.tested)],
            ),
            Comparison::RehearsalOfficialErt => (
                ("--official", self.official),
                ("--ert", self.ert),
                [("--stl", self.stl), ("--tested", self.tested)],
            ),
            Comparison::RehearsalOfficialStl => (
                ("--official", self.official),
                ("--stl", self.stl),
                [("--ert", self.ert), ("--tested", self.tested)],
            ),
        };
        let mode = self.comparison.wire();
        if let Some((flag, _)) = unused.iter().find(|(_, value)| value.is_some()) {
            return Err(format!("{flag} does not apply to the {mode} comparison").into());
        }
        let required = |(flag, value): (&str, Option<Uuid>)| {
            value.ok_or_else(|| format!("The {mode} comparison requires {flag}"))
        };
        Ok((required(first)?, required(second)?))
    }

    pub fn run(&self) -> Result<(), Box<dyn Error>> {
        let (first, second) = self.positions()?;
        let config = read_config()?;
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(300))
            .build()?;
        // The production request stays exactly as before: no mode is sent.
        let body = match self.comparison {
            Comparison::Production => json!({
                "operationName": "ReadinessCheck",
                "query": PRODUCTION_QUERY,
                "variables": {
                    "official_event_id": first.to_string(),
                    "tested_event_id": second.to_string(),
                },
            }),
            rehearsal => json!({
                "operationName": "ReadinessCheck",
                "query": REHEARSAL_QUERY,
                "variables": {
                    "official_event_id": first.to_string(),
                    "tested_event_id": second.to_string(),
                    "comparison": rehearsal.wire(),
                },
            }),
        };
        // Readiness returns JSONB, so preserve every evidence field without
        // coupling the CLI to a generated GraphQL schema revision.
        let response: Response<Value> = client
            .post(&config.endpoint_url)
            .bearer_auth(&config.auth_token)
            .header("x-hasura-role", Permissions::READINESS_CHECK.to_string())
            .json(&body)
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
        match self.comparison {
            Comparison::Production => production_verdict(report),
            rehearsal => rehearsal_verdict(rehearsal, &report),
        }
    }
}

fn production_verdict(report: Value) -> Result<(), Box<dyn Error>> {
    // A production report never names a comparison.
    if report.get("comparison").is_some() {
        return Err("Readiness check returned a rehearsal report for a production check".into());
    }
    let verdict: Verdict = serde_json::from_value(report)?;
    if !verdict.passed {
        return Err("Readiness verification failed; inspect the JSON checks".into());
    }
    if verdict.log_entry_id.as_deref().is_none_or(str::is_empty) {
        return Err("Readiness check returned no signed log entry".into());
    }
    Ok(())
}

/// Only a consistent report of the requested read-only comparison succeeds;
/// a rehearsal never carries a pass flag or signed evidence.
fn rehearsal_verdict(comparison: Comparison, report: &Value) -> Result<(), Box<dyn Error>> {
    if report.get("comparison").and_then(Value::as_str) != Some(comparison.wire()) {
        return Err("Readiness check returned a different comparison than requested".into());
    }
    if report.get("passed").is_some() || report.get("log_entry_id").is_some() {
        return Err("A rehearsal comparison returned readiness or signed evidence".into());
    }
    match report.get("verdict").and_then(Value::as_str) {
        Some("consistent") => Ok(()),
        Some("not_verified") => Err(
            "Rehearsal comparison not verified; no readiness was recorded. Inspect the JSON checks"
                .into(),
        ),
        Some("inconsistent") => Err(
            "Rehearsal comparison inconsistent; no readiness was recorded. Inspect the JSON checks"
                .into(),
        ),
        _ => Err("Rehearsal comparison returned no verdict".into()),
    }
}
