// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Context;
use clap::ValueEnum;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde_json::{json, Value};
use std::{env, error::Error, fs, thread, time::Duration};

pub fn run_enrollment_test(
    election_event_id: &str,
    participants_count: u64,
    enrollments_per_participant: u64,
    otp_code: &str,
    test_id: Option<String>,
) -> Result<(), Box<dyn Error>> {
    let tenant_id =
        env::var("SUPER_ADMIN_TENANT_ID").with_context(|| "missing SUPER_ADMIN_TENANT_ID")?;
    let voting_portal_domain =
        env::var("VOTING_PORTAL_URL").with_context(|| "missing VOTING_PORTAL_DOMAIN")?;
    let loadero_url: String =
        env::var("LOADERO_BASE_URL").with_context(|| "missing  LOADERO_BASE_URL")?;

    let voting_portal_url = format!(
        "{}/tenant/{}/event/{}/enroll",
        &voting_portal_domain, &tenant_id, &election_event_id
    );

    let test_id = match test_id {
        Some(id) => id,
        None => init_loadero_test(
            &loadero_url,
            &election_event_id,
            &voting_portal_url,
            participants_count,
            enrollments_per_participant,
            otp_code,
        )?,
    };

    run_test(&loadero_url, &test_id)?;

    Ok(())
}

pub fn init_loadero_test(
    loadero_url: &str,
    election_event_id: &str,
    voting_portal_url: &str,
    participants_count: u64,
    enrollments_per_participant: u64,
    otp_code: &str,
) -> Result<String, Box<dyn Error>> {
    let test_id = create_test(
        &loadero_url,
        election_event_id,
        voting_portal_url,
        enrollments_per_participant,
        otp_code,
    )?;

    create_test_participants(&loadero_url, &test_id, participants_count)?;

    Ok(test_id)
}

pub fn run_test(loadero_url: &str, test_id: &str) -> Result<(), Box<dyn Error>> {
    let loadero_interval_polling_sec =
        env::var("LOADERO_INTERVAL_POLLING_TIME").unwrap_or("30".to_string());
    let loadero_interval_polling_sec: u64 = loadero_interval_polling_sec.parse()?;

    println!("innnnnn");

    let run_id = launch_test(&loadero_url, &test_id)?;

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

pub fn get_enrollment_test_name_str(election_event_id: &str) -> String {
    format!("Test Enrollment - Election {}", election_event_id)
}

fn get_test_config(election_event_id: &str, script: String) -> Value {
    json!({
        "increment_strategy": "constant",
        "mode": "load",
        "name": get_enrollment_test_name_str(&election_event_id),
        "participant_timeout": 300,
        "script": script,
        "start_interval": 0
    })
}

fn create_test(
    loadero_url: &str,
    election_event_id: &str,
    voting_portal_url: &str,
    enrollments_per_participant: u64,
    otp_code: &str,
) -> Result<String, Box<dyn Error>> {
    let client = reqwest::blocking::Client::new();
    let headers = create_header()?;

    let script = generate_script(voting_portal_url, enrollments_per_participant, otp_code)?;

    let json_body = get_test_config(&election_event_id, script);

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
        let error = format!(
            "Creare Test error: HTTP Status: {}\nError Message: {}",
            status, error_message
        );
        Err(Box::from(error))
    }
}

fn create_test_participants(
    loadero_url: &str,
    test_id: &str,
    participant_count: u64,
) -> Result<String, Box<dyn Error>> {
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
        .post(format!("{}/tests/{}/participants", &loadero_url, test_id))
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
        let error = format!(
            "Add Participants error: HTTP Status: {}\nError Message: {}",
            status, error_message
        );
        Err(Box::from(error))
    }
}

pub fn get_test_by_name(test_name: String) -> Result<Option<String>, Box<dyn std::error::Error>> {
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
        let error_message = response.text()?;
        let error = format!("HTTP Status: {}\nError Message: {}", status, error_message);
        Err(Box::from(error))
    }
}

pub fn launch_test(loadero_url: &str, test_id: &str) -> Result<String, Box<dyn Error>> {
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
        let error = format!(
            "lunch Test error: HTTP Status: {}\nError Message: {}",
            status, error_message
        );
        Err(Box::from(error))
    }
}

fn check_test_status(
    loadero_url: &str,
    test_id: &str,
    run_id: &str,
) -> Result<(usize, usize), Box<dyn Error>> {
    let client = reqwest::blocking::Client::new();
    let headers = create_header()?;

    println!("in check status");

    let response = client
        .get(format!(
            "{}/tests/{}/runs/{}/",
            &loadero_url, test_id, run_id
        ))
        .headers(headers)
        .send()?;

    let status = response.status();
    let response_text = response.text()?;

    println!(
        "in check status, the status: {:?}, response: {}",
        status.clone(),
        &response_text
    );

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

fn generate_script(
    url: &str,
    enrollments_per_participant: u64,
    otp_code: &str,
) -> Result<String, Box<dyn Error>> {
    let template_path =
        "/workspaces/step/packages/e2e/enrollment/load_test/resources/enrollment_test_script.txt";
    let template_content = fs::read_to_string(template_path)?;

    // Replace placeholders with actual values
    let script = replace_placeholder(&template_content, "{url}", url);
    let script = replace_placeholder(&script, "{otpLength}", "6");
    let script = replace_placeholder(&script, "{otpCode}", otp_code);
    let script = replace_placeholder(
        &script,
        "{enrollmentsCount}",
        enrollments_per_participant.to_string().as_str(),
    );

    println!("script: {}", &script);
    Ok(script)
}

pub fn update_script(
    test_id: &str,
    election_event_id: &str,
    enrollments_per_participant: u64,
    otp_code: &str,
) -> Result<(), Box<dyn Error>> {
    let tenant_id =
        env::var("SUPER_ADMIN_TENANT_ID").with_context(|| "missing SUPER_ADMIN_TENANT_ID")?;
    let voting_portal_domain =
        env::var("VOTING_PORTAL_URL").with_context(|| "missing VOTING_PORTAL_DOMAIN")?;
    let loadero_url: String =
        env::var("LOADERO_BASE_URL").with_context(|| "missing  LOADERO_BASE_URL")?;

    let voting_portal_url = format!(
        "{}/tenant/{}/event/{}/enroll",
        &voting_portal_domain, &tenant_id, &election_event_id
    );

    let client = reqwest::blocking::Client::new();
    let headers = create_header()?;

    let script = generate_script(&voting_portal_url, enrollments_per_participant, otp_code)?;

    let json_body = get_test_config(&election_event_id, script);

    let url = format!("{}/tests/{}/script", &loadero_url, &test_id);
    println!("url: {}", &url);

    let response = client
        .put(format!("{}/tests/{}", &loadero_url, &test_id))
        .headers(headers)
        .json(&json_body)
        .send()?;

    if response.status().is_success() {
        Ok(())
    } else {
        let status = response.status();
        let error_message = response.text()?;
        let error = format!(
            "Update Test error: HTTP Status: {}\nError Message: {}",
            status, error_message
        );
        Err(Box::from(error))
    }
}
