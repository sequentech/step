// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::{fs, thread, time::Duration};

use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde_json::{json, Value};
use tracing::{error, info};

/// Initialize Loadero tests
pub fn init_loadero_tests(
    election_event_id: &str,
    voting_portal_url: &str,
    voter_count: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let loadero_url = std::env::var("LOADERO_URL")?;
    let participant_count = std::env::var("LOADERO_PARTICIPANT_COUNT").unwrap_or("1".to_string()); // Fallback is 1
    let participant_count: u64 = participant_count.parse()?;
    let loadero_interval_polling_sec =
        std::env::var("LOADERO_INTERVAL_POLLING_TIME").unwrap_or("30".to_string()); // Fallback is 30 sec
    let loadero_interval_polling_sec: u64 = loadero_interval_polling_sec.parse()?;

    // Step 1: Create Test
    let test_id = create_test(
        &loadero_url,
        election_event_id,
        voting_portal_url,
        voter_count,
        participant_count,
    )?;

    // Step 1.5: Add participant to test
    create_test_paricipants(&loadero_url, &test_id, participant_count)?;

    // Step 2: Launch Test
    let run_id = launch_test(&loadero_url, &test_id)?;

    // Step 3: Poll for test result
    let polling_interval = Duration::from_secs(loadero_interval_polling_sec);
    loop {
        match check_test_status(&loadero_url, &test_id, &run_id) {
            Ok((pass, fail)) => {
                info!("Test {test_id} (run ID {run_id}): Passed {pass} times, Failed {fail} times",);
                break; // Exit the loop when test is done
            }
            Err(e) => {
                if e.to_string().contains("HTTP Status") {
                    error!("HTTP Error checking status for test {test_id}: {e}");
                    break; // Exit the loop on HTTP errors
                }
                // Wait before retrying
                thread::sleep(polling_interval);
            }
        }
    }

    Ok(())
}

/// Create a header for the Loadero API
fn create_header() -> Result<HeaderMap, Box<dyn std::error::Error>> {
    let api_key = std::env::var("LOADERO_API_KEY")?;

    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("LoaderoAuth {api_key}"))?,
    );
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    Ok(headers)
}

/// Create a test in Loadero
fn create_test(
    loadero_url: &str,
    election_event_id: &str,
    voting_portal_url: &str,
    voter_count: u64,
    participant_count: u64,
) -> Result<String, Box<dyn std::error::Error>> {
    let client = reqwest::blocking::Client::new();
    let headers = create_header()?;

    let script = generate_script(voting_portal_url, voter_count)?; //TODO: Add randomizing candidate selection - Currently the first candidate is selected

    let loadero_interval_sec = std::env::var("LOADERO_INTERVAL_TIME").unwrap_or("3.3".to_string()); // Fallback is 3.3 sec
    let loadero_interval_sec: f64 = loadero_interval_sec.parse()?;

    #[allow(clippy::cast_precision_loss)]
    let start_interval_time = (participant_count as f64) * loadero_interval_sec;

    let json_body = json!({
        "increment_strategy": "linear",
        "mode": "load",
        "name": format!("Test Voting Portal - Election {}", election_event_id),
        "participant_timeout": 300,
        "script": script,
        "start_interval": start_interval_time
    });

    let response = client
        .post(format!("{loadero_url}/tests"))
        .headers(headers)
        .json(&json_body)
        .send()?;

    if response.status().is_success() {
        let response_json: Value = response.json()?;

        if let Some(run_id) = response_json.get("id").and_then(serde_json::Value::as_i64) {
            Ok(run_id.to_string())
        } else {
            Err(Box::from("No run id found"))
        }
    } else {
        let status = response.status();
        let error_message = response.text()?;
        let error = format!("HTTP Status: {status}\nError Message: {error_message}");
        Err(Box::from(error))
    }
}

/// Create test participants in Loadero
fn create_test_paricipants(
    loadero_url: &str,
    test_id: &str,
    participant_count: u64,
) -> Result<String, Box<dyn std::error::Error>> {
    let client = reqwest::blocking::Client::new();
    let headers = create_header()?;

    let json_body = json!({
        "browser": "chromeLatest",
        "compute_unit": "g1",
        "count": participant_count,
        "location": "us-west-2",
        "media_type": "custom",
        "name": "participant",
        "network": "default",
        "record_audio": false
    });

    let response = client
        .post(format!("{loadero_url}/tests/{test_id}/participants"))
        .headers(headers)
        .json(&json_body)
        .send()?;

    if response.status().is_success() {
        let response_json: Value = response.json()?;

        if let Some(participant_id) = response_json.get("id").and_then(serde_json::Value::as_i64) {
            Ok(participant_id.to_string())
        } else {
            Err(Box::from("No id found"))
        }
    } else {
        let status = response.status();
        let error_message = response.text()?;
        let error = format!("HTTP Status: {status}\nError Message: {error_message}");
        Err(Box::from(error))
    }
}

/// Get tests from Loadero
fn get_tests(loadero_url: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let client = reqwest::blocking::Client::new();
    let headers = create_header()?;
    let response = client
        .get(format!("{loadero_url}/tests"))
        .headers(headers)
        .send()?;
    if response.status().is_success() {
        let response_json: Value = response.json()?;
        let mut test_ids = Vec::new();

        if let Some(results) = response_json
            .get("results")
            .and_then(serde_json::Value::as_array)
        {
            for result in results {
                if let Some(id) = result.get("id").and_then(serde_json::Value::as_i64) {
                    test_ids.push(id.to_string());
                }
            }
        }

        Ok(test_ids)
    } else {
        let status = response.status();
        let error_message = response.text()?;
        let error = format!("HTTP Status: {status}\nError Message: {error_message}");
        Err(Box::from(error))
    }
}

/// Launch a test in Loadero
fn launch_test(loadero_url: &str, test_id: &str) -> Result<String, Box<dyn std::error::Error>> {
    let client = reqwest::blocking::Client::new();
    let headers = create_header()?;

    let response = client
        .post(format!("{loadero_url}/tests/{test_id}/runs/"))
        .headers(headers)
        .send()?;

    if response.status().is_success() {
        let response_json: Value = response.json()?;

        if let Some(run_id) = response_json.get("id").and_then(serde_json::Value::as_i64) {
            Ok(run_id.to_string())
        } else {
            Err(Box::from("No run id found"))
        }
    } else {
        let status = response.status();
        let error_message = response.text()?;
        let error = format!("HTTP Status: {status}\nError Message: {error_message}");
        Err(Box::from(error))
    }
}

/// Check the status of a test in Loadero
fn check_test_status(
    loadero_url: &str,
    test_id: &str,
    run_id: &str,
) -> Result<(usize, usize), Box<dyn std::error::Error>> {
    let client = reqwest::blocking::Client::new();
    let headers = create_header()?;

    let response = client
        .get(format!("{loadero_url}/tests/{test_id}/runs/{run_id}/",))
        .headers(headers)
        .send()?;

    let response_status = response.status();
    let response_text = response.text()?;

    if response_status.is_success() {
        let response_json: Value = serde_json::from_str(&response_text)?;
        if let Some(status) = response_json
            .get("status")
            .and_then(serde_json::Value::as_str)
        {
            if status == "done" {
                if let Some(participant_results) = response_json
                    .get("participant_results")
                    .and_then(serde_json::Value::as_object)
                {
                    let pass = usize::try_from(
                        participant_results
                            .get("pass")
                            .and_then(Value::as_u64)
                            .unwrap_or(0),
                    )?;
                    let fail = usize::try_from(
                        participant_results
                            .get("fail")
                            .and_then(Value::as_u64)
                            .unwrap_or(0),
                    )?;
                    return Ok((pass, fail));
                }
            } else {
                return Err(Box::from("Test is not yet done"));
            }
        }
    }

    let error = format!("HTTP Status: {response_status}\nError Message: {response_text}");
    Err(Box::from(error))
}

/// Replace placeholders in a template
fn replace_placeholder(template: &str, placeholder: &str, replacement: &str) -> String {
    template.replace(placeholder, replacement)
}

/// Generate a script for a test
fn generate_script(url: &str, voter_count: u64) -> Result<String, Box<dyn std::error::Error>> {
    // Read the template file
    let template_path = "/workspaces/step/packages/step-cli/src/tests/template_script.txt";
    let template_content = fs::read_to_string(template_path)?;

    // Replace placeholders with actual values
    let script = replace_placeholder(&template_content, "{url}", url);
    let script = replace_placeholder(&script, "{voter_count}", voter_count.to_string().as_str());

    Ok(script)
}
