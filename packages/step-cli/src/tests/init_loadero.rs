// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::{fs, thread, time::Duration};

use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde_json::{json, Value};

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
                println!(
                    "Test {} (run ID {}): Passed {} times, Failed {} times",
                    test_id, run_id, pass, fail
                );
                break; // Exit the loop when test is done
            }
            Err(e) => {
                if e.to_string().contains("HTTP Status") {
                    eprintln!("HTTP Error checking status for test {}: {}", test_id, e);
                    break; // Exit the loop on HTTP errors
                } else {
                    // Wait before retrying
                    thread::sleep(polling_interval);
                }
            }
        }
    }

    Ok(())
}

fn create_header() -> Result<HeaderMap, Box<dyn std::error::Error>> {
    let api_key = std::env::var("LOADERO_API_KEY")?;

    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("LoaderoAuth {}", api_key))?,
    );
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    Ok(headers)
}

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
        .post(format!("{}/tests", &loadero_url))
        .headers(headers)
        .json(&json_body)
        .send()?;

    if response.status().is_success() {
        let response_json: Value = response.json()?;

        if let Some(run_id) = response_json["id"].as_i64() {
            Ok(run_id.to_string())
        } else {
            Err(Box::from("No run id found"))
        }
    } else {
        let status = response.status();
        let error_message = response.text()?;
        let error = format!("HTTP Status: {}\nError Message: {}", status, error_message);
        Err(Box::from(error))
    }
}

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
        .post(format!("{}/tests/{}/participants", &loadero_url, &test_id))
        .headers(headers)
        .json(&json_body)
        .send()?;

    if response.status().is_success() {
        let response_json: Value = response.json()?;

        if let Some(participant_id) = response_json["id"].as_i64() {
            Ok(participant_id.to_string())
        } else {
            Err(Box::from("No id found"))
        }
    } else {
        let status = response.status();
        let error_message = response.text()?;
        let error = format!("HTTP Status: {}\nError Message: {}", status, error_message);
        Err(Box::from(error))
    }
}

fn get_tests(loadero_url: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let client = reqwest::blocking::Client::new();
    let headers = create_header()?;
    let response = client
        .get(format!("{}/tests", &loadero_url))
        .headers(headers)
        .send()?;
    if response.status().is_success() {
        let response_json: Value = response.json()?;
        let mut test_ids = Vec::new();

        if let Some(results) = response_json["results"].as_array() {
            for result in results {
                if let Some(id) = result["id"].as_i64() {
                    test_ids.push(id.to_string());
                }
            }
        }

        Ok(test_ids)
    } else {
        let status = response.status();
        let error_message = response.text()?;
        let error = format!("HTTP Status: {}\nError Message: {}", status, error_message);
        Err(Box::from(error))
    }
}

fn launch_test(loadero_url: &str, test_id: &str) -> Result<String, Box<dyn std::error::Error>> {
    let client = reqwest::blocking::Client::new();
    let headers = create_header()?;

    let response = client
        .post(format!("{}/tests/{}/runs/", &loadero_url, test_id))
        .headers(headers)
        .send()?;

    if response.status().is_success() {
        let response_json: Value = response.json()?;

        if let Some(run_id) = response_json["id"].as_i64() {
            Ok(run_id.to_string())
        } else {
            Err(Box::from("No run id found"))
        }
    } else {
        let status = response.status();
        let error_message = response.text()?;
        let error = format!("HTTP Status: {}\nError Message: {}", status, error_message);
        Err(Box::from(error))
    }
}

fn check_test_status(
    loadero_url: &str,
    test_id: &str,
    run_id: &str,
) -> Result<(usize, usize), Box<dyn std::error::Error>> {
    let client = reqwest::blocking::Client::new();
    let headers = create_header()?;

    let response = client
        .get(format!(
            "{}/tests/{}/runs/{}/",
            &loadero_url, test_id, run_id
        ))
        .headers(headers)
        .send()?;

    let status = response.status();
    let response_text = response.text()?;

    if status.is_success() {
        let response_json: Value = serde_json::from_str(&response_text)?;
        if let Some(status) = response_json["status"].as_str() {
            if status == "done" {
                if let Some(participant_results) = response_json["participant_results"].as_object()
                {
                    let pass = participant_results
                        .get("pass")
                        .and_then(Value::as_u64)
                        .unwrap_or(0) as usize;
                    let fail = participant_results
                        .get("fail")
                        .and_then(Value::as_u64)
                        .unwrap_or(0) as usize;
                    return Ok((pass, fail));
                }
            } else {
                return Err(Box::from("Test is not yet done"));
            }
        }
    }

    let error = format!("HTTP Status: {}\nError Message: {}", status, response_text);
    Err(Box::from(error))
}

fn replace_placeholder(template: &str, placeholder: &str, replacement: &str) -> String {
    template.replace(placeholder, replacement)
}

fn generate_script(url: &str, voter_count: u64) -> Result<String, Box<dyn std::error::Error>> {
    // Read the template file
    let template_path = "/workspaces/step/packages/step-cli/src/tests/template_script.txt";
    let template_content = fs::read_to_string(template_path)?;

    // Replace placeholders with actual values
    let script = replace_placeholder(&template_content, "{url}", url);
    let script = replace_placeholder(&script, "{voter_count}", voter_count.to_string().as_str());

    Ok(script)
}
