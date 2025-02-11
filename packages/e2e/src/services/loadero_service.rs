// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Context, Result};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{env, error::Error, thread, time::Duration};

#[derive(Serialize, Deserialize, Clone)]
pub struct TestConfig {
    pub increment_strategy: String,
    pub mode: String,
    pub name: String,
    pub participant_timeout: u64,
    pub script: String,
    pub start_interval: u64,
}

pub fn run_scenario_test(
    participants_count: u64,
    test_config: TestConfig,
    test_name: String,
    update: bool,
) -> Result<()> {
    let loadero_url: String =
        env::var("LOADERO_BASE_URL").with_context(|| "missing  LOADERO_BASE_URL")?;

    let test_id = match get_test_id_by_name(test_name)? {
        Some(id) => id,
        None => init_loadero_test(&loadero_url, &test_config, participants_count)?,
    };

    if update {
        update_script(&test_id, test_config)?;
    }

    run_test(&loadero_url, &test_id)?;

    Ok(())
}

pub fn init_loadero_test(
    loadero_url: &str,
    test_config: &TestConfig,
    participants_count: u64,
) -> Result<String> {
    let test_id =
        create_test(&loadero_url, test_config).context("Failed to create test in Loadero")?;

    create_test_participants(loadero_url, &test_id, participants_count)
        .with_context(|| format!("Failed to create participants for test ID {}", test_id))?;

    Ok(test_id)
}

pub fn run_test(loadero_url: &str, test_id: &str) -> Result<()> {
    let loadero_interval_polling_sec =
        env::var("LOADERO_INTERVAL_POLLING_TIME").unwrap_or_else(|_| "30".to_string());

    let loadero_interval_polling_sec: u64 = loadero_interval_polling_sec
        .parse()
        .context("LOADERO_INTERVAL_POLLING_TIME must be an string")?;

    let run_id = launch_test(&loadero_url, &test_id)
        .with_context(|| format!("Failed to launch test ID {}", test_id))?;

    println!("Test {} (run ID {})", test_id, run_id);

    //Poll for test result
    let polling_interval = Duration::from_secs(loadero_interval_polling_sec);
    loop {
        println!("check status:");
        match check_test_status(&loadero_url, &test_id, &run_id) {
            Ok((pass, fail)) => {
                println!(
                    "Test {} (run ID {}): Passed {} times, Failed {} times",
                    test_id, run_id, pass, fail
                );
                break;
            }
            Err(e) => {
                if e.to_string().contains("HTTP Status") {
                    eprintln!("HTTP Error checking status for test {}: {}", test_id, e);
                    break;
                } else {
                    thread::sleep(polling_interval);
                }
            }
        }
    }

    Ok(())
}

fn create_header() -> Result<HeaderMap> {
    let api_key = env::var("LOADERO_API_KEY").context("missing LOADERO_API_KEY")?;

    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("LoaderoAuth {}", api_key))
            .context("Invalid LOADERO_API_KEY format")?,
    );
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    Ok(headers)
}

fn create_test(loadero_url: &str, test_config: &TestConfig) -> Result<String> {
    let client = reqwest::blocking::Client::new();
    let headers = create_header()?;

    let response = client
        .post(format!("{}/tests", &loadero_url))
        .headers(headers)
        .json(&test_config)
        .send()?;

    if !response.status().is_success() {
        let status = response.status();
        let error_message = response
            .text()
            .unwrap_or_else(|_| "Could not read response body".to_string());
        return Err(anyhow!(
            "Create Test error: HTTP Status: {}\nError Message: {}",
            status,
            error_message
        ));
    }

    let response_json: Value = response
        .json()
        .context("Failed to parse createTest response JSON")?;

    let run_id = response_json["id"]
        .as_i64()
        .ok_or_else(|| anyhow!("No test ID found in createTest response"))?;

    Ok(run_id.to_string())
}

fn create_test_participants(
    loadero_url: &str,
    test_id: &str,
    participant_count: u64,
) -> Result<()> {
    let client = reqwest::blocking::Client::new();
    let headers = create_header().context("Failed to create headers for participants")?;

    let json_body = json!({
        "browser": "chromeLatest",
        "compute_unit": "g1",
        "count": participant_count,
        "location": "us-west-2",
        "media_type": "custom",
        "name": "omrio@moveo.co.il|Aa1234567!",
        "network": "default",
        "record_audio": false
    });

    let response = client
        .post(format!("{}/tests/{}/participants", &loadero_url, test_id))
        .headers(headers)
        .json(&json_body)
        .send()?;

    if !response.status().is_success() {
        let status = response.status();
        let error_message = response
            .text()
            .unwrap_or_else(|_| "Could not read response body".to_string());
        return Err(anyhow!(
            "Add Participants error: HTTP Status: {}\nError Message: {}",
            status,
            error_message
        ));
    }

    Ok(())
}

pub fn get_test_id_by_name(test_name: String) -> Result<Option<String>> {
    let loadero_url: String =
        env::var("LOADERO_BASE_URL").with_context(|| "missing  LOADERO_BASE_URL")?;
    let client = reqwest::blocking::Client::new();
    let headers = create_header()?;

    let response = client
        .get(format!("{}/tests", &loadero_url))
        .headers(headers)
        .send()?;

    if response.status().is_success() {
        let response_json: Value = response.json()?;
        if let Some(results) = response_json["results"].as_array() {
            for result in results {
                if let (Some(id), Some(name)) = (result["id"].as_i64(), result["name"].as_str()) {
                    if test_name == name.to_string() {
                        return Ok(Some(id.to_string()));
                    }
                }
            }
        }

        Ok(None)
    } else {
        let status = response.status();
        let error_message = response
            .text()
            .unwrap_or_else(|_| "Could not read response body".to_string());
        return Err(anyhow!(
            "HTTP Status: {}\nError Message: {}",
            status,
            error_message
        ));
    }
}

pub fn launch_test(loadero_url: &str, test_id: &str) -> Result<String> {
    let client = reqwest::blocking::Client::new();
    let headers = create_header().context("Failed to create headers in launch_test")?;

    let response = client
        .post(format!("{}/tests/{}/runs/", &loadero_url, test_id))
        .headers(headers)
        .send()?;

    if !response.status().is_success() {
        let status = response.status();
        let error_message = response
            .text()
            .unwrap_or_else(|_| "Could not read response body".to_string());
        return Err(anyhow!(
            "Launch Test error: HTTP Status: {}\nError Message: {}",
            status,
            error_message
        ));
    }

    let response_json: Value = response
        .json()
        .context("Failed to parse JSON in launch_test")?;

    let run_id = response_json["id"]
        .as_i64()
        .ok_or_else(|| anyhow!("No run id found in launch_test response"))?;

    Ok(run_id.to_string())
}

fn check_test_status(loadero_url: &str, test_id: &str, run_id: &str) -> Result<(usize, usize)> {
    let client = reqwest::blocking::Client::new();
    let headers = create_header().context("Failed to create headers in check_test_status")?;

    println!("in check status");

    let response = client
        .get(format!(
            "{}/tests/{}/runs/{}/",
            &loadero_url, test_id, run_id
        ))
        .headers(headers)
        .send()?;

    let status = response.status();
    let response_text = response
        .text()
        .context("Failed to read response text in check_test_status")?;

    if status.is_success() {
        let response_json: Value = serde_json::from_str(&response_text)
            .context("Failed to parse JSON in check_test_status")?;
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
                return Err(anyhow!("Test is not yet done"));
            }
        }
    }

    // If we got here, either status isn't success or JSON didn't contain needed data
    Err(anyhow!(
        "HTTP Status: {}. Response: {}",
        status,
        response_text
    ))
}

pub fn replace_placeholder(template: &str, placeholder: &str, replacement: &str) -> String {
    template.replace(placeholder, replacement)
}

pub fn update_script(test_id: &str, test_config: TestConfig) -> Result<()> {
    let loadero_url: String =
        env::var("LOADERO_BASE_URL").with_context(|| "missing  LOADERO_BASE_URL")?;

    let client = reqwest::blocking::Client::new();
    let headers = create_header().with_context(|| "Failed to create headers in update_script")?;

    let url = format!("{}/tests/{}/script", &loadero_url, &test_id);
    println!("url: {}", &url);

    let response = client
        .put(format!("{}/tests/{}", &loadero_url, &test_id))
        .headers(headers)
        .json(&test_config)
        .send()?;

    if response.status().is_success() {
        Ok(())
    } else {
        let status = response.status();
        let error_message = response
            .text()
            .unwrap_or_else(|_| "Could not read response body".to_string());
        return Err(anyhow!(
            "Update Test error: HTTP Status: {}\nError Message: {}",
            status,
            error_message
        ));
    }
}
