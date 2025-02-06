// SPDX-FileCopyrightText: 2024 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{Context, Result};
use serde_json::{from_str, json, Value};
use std::{collections::HashMap, env, error::Error, fs, thread, time::Duration};

use crate::{services::loadero_service::replace_placeholder, Args};
use std::fs::File;

use super::loadero_service::{run_scenario_test, TestConfig};

#[derive(serde::Deserialize, Debug)]
pub struct EnrollmentScenarioData {
    pub election_event_id: String,
}

pub fn get_enrollment_test_name_str(election_event_id: &str) -> String {
    format!("Test Enrollment - Election {}", election_event_id)
}

fn get_test_config(election_event_id: &str, script: String, test_duration: &u64) -> TestConfig {
    TestConfig {
        increment_strategy: "linear".to_string(),
        mode: "load".to_string(),
        name: get_enrollment_test_name_str(&election_event_id),
        participant_timeout: 300,
        script: script,
        start_interval: *test_duration,
    }
}

fn get_test_data() -> Result<EnrollmentScenarioData> {
    let json_file = File::open("/workspaces/step/packages/e2e/src/scanrios/enrollment/data.json")
        .map_err(|e| anyhow::anyhow!("Failed to open voting data file: {}", e))?;
    let scenario_data: EnrollmentScenarioData =
        serde_json::from_reader(json_file).with_context(|| "Invalid JSON for voting scenario")?;

    Ok(scenario_data)
}

fn generate_script(url: &str) -> Result<String> {
    let template_path =
        "/workspaces/step/packages/e2e/src/scanrios/enrollment/scripts/enrollment_test_script.js";
    let template_content = fs::read_to_string(template_path)?;

    // Replace placeholders with actual values
    let script = replace_placeholder(&template_content, "{url}", url);

    println!("script: {}", &script);
    Ok(script)
}

pub fn run_enrollment_test(args: &Args) -> Result<()> {
    let tenant_id =
        env::var("SUPER_ADMIN_TENANT_ID").with_context(|| "missing SUPER_ADMIN_TENANT_ID")?;

    print!("running enrollment test");
    let scanrio_data = get_test_data()?;
    let enrollment_election_test_name =
        get_enrollment_test_name_str(&scanrio_data.election_event_id);
    let voting_portal_url = format!(
        "https://voting-portal-dev.sequent.vote/tenant/{}/event/{}/enroll",
        &tenant_id, &scanrio_data.election_event_id
    );
    let script = generate_script(&voting_portal_url)?;
    let test_config = get_test_config(&scanrio_data.election_event_id, script, &args.test_duration);

    run_scenario_test(
        args.participants,
        test_config,
        enrollment_election_test_name,
        args.update,
    )
    .with_context(|| "unable to run test")?;

    Ok(())
}
