// SPDX-FileCopyrightText: 2024 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Context;
use serde_json::{from_str, json, Value};
use std::{collections::HashMap, env, error::Error, fs, thread, time::Duration};

use crate::{services::loadero_service::replace_placeholder, Args};

use super::loadero_service::run_scenario_test;

#[derive(serde::Deserialize)]
pub struct EnrollmentScenarioData {
    pub election_event_id: String,
    pub otp_code: String,
}

pub fn get_enrollment_test_name_str(election_event_id: &str) -> String {
    format!("Test Enrollment - Election {}", election_event_id)
}

fn get_test_config(election_event_id: &str, script: String, test_duration: &u64) -> Value {
    json!({
        "increment_strategy": "linear",
        "mode": "load",
        "name": get_enrollment_test_name_str(&election_event_id),
        "participant_timeout": 300,
        "script": script,
        "start_interval": test_duration
    })
}

fn generate_script(url: &str, otp_code: &str) -> Result<String, Box<dyn Error>> {
    let template_path =
        "/workspaces/step/packages/e2e/enrollment/load_test/resources/enrollment_test_script.txt";
    let template_content = fs::read_to_string(template_path)?;

    // Replace placeholders with actual values
    let script = replace_placeholder(&template_content, "{url}", url);
    let script = replace_placeholder(&script, "{otpLength}", "6");
    let script = replace_placeholder(&script, "{otpCode}", otp_code);

    println!("script: {}", &script);
    Ok(script)
}

pub fn run_enrollment_test(args: &Args) -> Result<(), Box<dyn Error>> {
    let tenant_id =
        env::var("SUPER_ADMIN_TENANT_ID").with_context(|| "missing SUPER_ADMIN_TENANT_ID")?;
    let voting_portal_domain =
        env::var("VOTING_PORTAL_URL").with_context(|| "missing VOTING_PORTAL_DOMAIN")?;

    let scenario_data_json = args
        .scenario_data_json
        .as_deref()
        .ok_or("missing scenario data")?;
    let scanrio_data: EnrollmentScenarioData = from_str(scenario_data_json)
        .map_err(|e| format!("Invalid JSON for enrollment scenario: {}", e))?;

    let enrollment_election_test_name =
        get_enrollment_test_name_str(&scanrio_data.election_event_id);
    let voting_portal_url = format!(
        "{}/tenant/{}/event/{}/enroll",
        &voting_portal_domain, &tenant_id, &scanrio_data.election_event_id
    );
    let script = generate_script(&voting_portal_url, &scanrio_data.otp_code)?;
    let test_config = get_test_config(&scanrio_data.election_event_id, script, &args.test_duration);

    run_scenario_test(
        args.participants,
        test_config,
        enrollment_election_test_name,
        args.update,
    )?;

    Ok(())
}
