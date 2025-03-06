// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::loadero_service::{run_scenario_test, TestConfig};
use crate::services::loadero_service::replace_placeholder;
use crate::Args;
use anyhow::Context;
use anyhow::Result;
use std::fs;
use std::fs::File;

#[derive(serde::Deserialize, Debug)]
pub struct GenerateReportsScenarioData {
    pub election_event_id: String,
}

pub fn get_login_test_name_str(election_event_id: &str) -> String {
    format!("Generate Report - Election {}", election_event_id)
}

fn generate_script(url: &str) -> Result<String> {
    let template_path =
        "/workspaces/step/packages/e2e/src/scenarios/data-quering/reports/reports_test_script.js";
    let template_content = fs::read_to_string(template_path)?;

    let script = replace_placeholder(&template_content, "{url}", url);
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

fn get_test_data() -> Result<GenerateReportsScenarioData> {
    print!("getting test data");
    let json_file =
        File::open("/workspaces/step/packages/e2e/src/scenarios/data-quering/reports/data.json")
            .map_err(|e| anyhow::anyhow!("Failed to open voting data file: {}", e))?;
    let scenario_data: GenerateReportsScenarioData =
        serde_json::from_reader(json_file).with_context(|| "Invalid JSON for voting scenario")?;
    print!("test data: {:?}", scenario_data);
    Ok(scenario_data)
}

pub fn run_reports_test(args: &Args) -> Result<()> {
    print!("init reports test");
    let scanrio_data = get_test_data()?;
    //https://voting-portal-dev.sequent.vote/tenant/{tenant_id}/event/{election_event_id}/login'
    let enrollment_election_test_name = get_login_test_name_str(&scanrio_data.election_event_id);
    let admin_portal_url = format!(
        "https://admin-portal-qa.sequent.vote/sequent_backend_election_event/{}",
        &scanrio_data.election_event_id
    );
    let script = generate_script(&admin_portal_url)?;
    let test_config = get_test_config(&scanrio_data.election_event_id, script, &args.test_duration);
    print!("running reports test");
    run_scenario_test(
        args.participants,
        test_config,
        enrollment_election_test_name,
        args.update,
    )?;

    Ok(())
}
