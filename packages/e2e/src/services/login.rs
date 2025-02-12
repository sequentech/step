// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::loadero_service::{run_scenario_test, TestConfig};
use crate::services::loadero_service::replace_placeholder;
use crate::Args;
use anyhow::Context;
use anyhow::Result;
use core::num;
use serde_json::{from_str, json, Value};
use std::fs::File;
use std::{env, fs};

#[derive(serde::Deserialize, Debug)]
pub struct LoginScenarioData {
    pub election_event_id: String,
    pub password: String,
}

pub fn get_login_test_name_str(election_event_id: &str) -> String {
    format!("Test Login - Election {}", election_event_id)
}

fn generate_script(url: &str, password: &str, number_of_votes: &u64) -> Result<String> {
    let template_path = "/workspaces/step/packages/e2e/src/scenarios/login/login_test_script.js";
    let template_content = fs::read_to_string(template_path)?;

    let script = replace_placeholder(&template_content, "{url}", url);
    let script = replace_placeholder(&script, "{password}", password);
    let script = replace_placeholder(&script, "{numberOfVoters}", &number_of_votes.to_string());

    println!("script: {}", &script);
    Ok(script)
}

fn get_test_config(election_event_id: &str, script: String, test_duration: &u64) -> TestConfig {
    TestConfig {
        increment_strategy: "linear".to_string(),
        mode: "load".to_string(),
        name: get_login_test_name_str(&election_event_id),
        participant_timeout: 600,
        script: script,
        start_interval: *test_duration,
    }
}

fn get_test_data() -> Result<LoginScenarioData> {
    print!("getting test data");
    let json_file = File::open("/workspaces/step/packages/e2e/src/scenarios/voting/data.json")
        .map_err(|e| anyhow::anyhow!("Failed to open voting data file: {}", e))?;
    let scenario_data: LoginScenarioData =
        serde_json::from_reader(json_file).with_context(|| "Invalid JSON for voting scenario")?;
    print!("test data: {:?}", scenario_data);
    Ok(scenario_data)
}

pub fn run_login_test(args: &Args) -> Result<()> {
    let tenant_id =
        env::var("SUPER_ADMIN_TENANT_ID").with_context(|| "missing SUPER_ADMIN_TENANT_ID")?;

    print!("running voting test");

    let scanrio_data = get_test_data()?;
    //https://voting-portal-dev.sequent.vote/tenant/{tenant_id}/event/{election_event_id}/login'
    let enrollment_election_test_name = get_login_test_name_str(&scanrio_data.election_event_id);
    let voting_portal_url = format!(
        "https://voting-portal-comelecprod2.sequent.vote/tenant/{}/event/{}/login",
        &tenant_id, &scanrio_data.election_event_id
    );
    let script = generate_script(
        &voting_portal_url,
        &scanrio_data.password,
        &args.participants,
    )?;
    let test_config = get_test_config(&scanrio_data.election_event_id, script, &args.test_duration);
    print!("running login test");
    run_scenario_test(
        args.participants,
        test_config,
        enrollment_election_test_name,
        args.update,
    )?;

    Ok(())
}
