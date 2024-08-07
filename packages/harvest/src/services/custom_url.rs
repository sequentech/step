// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use reqwest::blocking::Client;
use rocket::futures::stream::Forward;
use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Debug, Deserialize)]
pub struct PageRule {
    pub id: String,
    pub targets: Vec<Target>,
    pub actions: Vec<Action>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Target {
    pub target: String,
    pub constraint: Constraint,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Constraint {
    pub operator: String,
    pub value: String,
}

#[derive(Debug, Deserialize)]
struct ApiResponse<T> {
    success: bool,
    result: T,
    errors: Vec<String>,
    messages: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CreatePageRuleRequest {
    targets: Vec<Target>,
    actions: Vec<Action>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Action {
    id: String,
    value: ForwardURL,
}

#[derive(Debug, Serialize, Deserialize)]
struct ForwardURL {
    url: String,
    status_code: u64,
}

pub fn get_page_rule(
    target_value: &str,
) -> Result<Option<PageRule>, Box<dyn Error>> {
    let page_rules = get_all_page_rules()?;
    Ok(find_matching_target(page_rules, target_value))
}

pub fn set_custom_url(
    redirect_to: &str,
    origin: &str,
) -> Result<(), Box<dyn Error>> {
    // Check environment
    if let Ok(_cloudflare_env) = std::env::var("CLOUDFLARE_ENV") {
        // Check if exists
        let current_page_rule = get_page_rule(&redirect_to)?;

        match current_page_rule {
            Some(page_rule) => {
                // If exists Update
                update_page_rule(&page_rule.id, redirect_to, origin)?;
                return Ok(());
            }
            None => {
                // If not create
                create_page_rule(redirect_to, origin)?;
                return Ok(());
            }
        }
    }
    Ok(())
}

fn get_cloudflare_vars() -> Result<(String, String, String), Box<dyn Error>> {
    let cloudflare_zone = std::env::var("CLOUDFLARE_ZONE")
        .map_err(|_e| "Missing cloudflare env variable".to_string())?;
    let cloudflare_api_email = std::env::var("CLOUDFLARE_API_EMAIL")
        .map_err(|_e| "Missing cloudflare env variable".to_string())?;
    let cloudflare_api_key = std::env::var("CLOUDFLARE_API_KEY")
        .map_err(|_e| "Missing cloudflare env variable".to_string())?;

    Ok((cloudflare_zone, cloudflare_api_email, cloudflare_api_key))
}

fn get_all_page_rules() -> Result<Vec<PageRule>, Box<dyn Error>> {
    let (zone_id, api_email, api_key) = get_cloudflare_vars()?;
    let client = Client::new();
    let response = client
        .get(&format!(
            "https://api.cloudflare.com/client/v4/zones/{}/pagerules",
            &zone_id,
        ))
        .header("X-Auth-Email", api_email)
        .header("X-Auth-Key", api_key)
        .header("Content-Type", "application/json")
        .send()?;
    if response.status().is_success() {
        let api_response: ApiResponse<Vec<PageRule>> = response.json()?;
        Ok(api_response.result)
    } else {
        let error_text = response.text()?;
        Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to get page rules: {}", error_text),
        )))
    }
}

fn find_matching_target(
    rules: Vec<PageRule>,
    expected_redirect_url: &str,
) -> Option<PageRule> {
    for rule in rules {
        for action in &rule.actions {
            let forward = &action.value;
            if forward.url == expected_redirect_url {
                return Some(rule);
            }
        }
    }
    None
}

fn create_payload(redirect_to: &str, origin: &str) -> CreatePageRuleRequest {
    let targets = vec![Target {
        constraint: Constraint {
            operator: "matches".to_string(),
            value: origin.to_string(),
        },
        target: "url".to_string(),
    }];

    let actions = vec![Action {
        id: "forwarding_url".to_string(),
        value: ForwardURL {
            url: redirect_to.to_string(),
            status_code: 301,
        },
    }];

    CreatePageRuleRequest { targets, actions }
}

fn update_page_rule(
    rule_id: &str,
    redirect_to: &str,
    origin: &str,
) -> Result<(), Box<dyn Error>> {
    let (zone_id, api_email, api_key) = get_cloudflare_vars()?;
    let client = Client::new();
    let request_body = create_payload(redirect_to, origin);

    let response = client
        .put(&format!(
            "https://api.cloudflare.com/client/v4/zones/{}/pagerules/{}",
            &zone_id, rule_id
        ))
        .header("X-Auth-Email", api_email)
        .header("X-Auth-Key", api_key)
        .json(&request_body)
        .send()?;
    if response.status().is_success() {
        println!("Page rule updated successfully");
        Ok(())
    } else {
        let error_text = response.text()?;
        Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to update page rule: {}", error_text),
        )))
    }
}

fn create_page_rule(
    redirect_to: &str,
    origin: &str,
) -> Result<(), Box<dyn Error>> {
    let (zone_id, api_email, api_key) = get_cloudflare_vars()?;
    let client = Client::new();

    let request_body = create_payload(redirect_to, origin);

    let response = client
        .post(&format!(
            "https://api.cloudflare.com/client/v4/zones/{}/pagerules",
            &zone_id,
        ))
        .header("X-Auth-Email", api_email)
        .header("X-Auth-Key", api_key)
        .json(&request_body)
        .send()?;
    if response.status().is_success() {
        println!("Page rule created successfully");
        Ok(())
    } else {
        let error_text = response.text()?;
        Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to create page rule: {}", error_text),
        )))
    }
}
