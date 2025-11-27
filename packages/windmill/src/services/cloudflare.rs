// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use reqwest::Client;
use sequent_core::serialization::deserialize_with_path::deserialize_str;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;
use tracing::{info, instrument};

pub const WAF_RULESET_PHASE: &str = "http_request_firewall_custom";

#[derive(Debug, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub result: T,
    pub errors: Vec<String>,
    pub messages: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct Ruleset {
    description: String,
    pub id: String,
    last_updated: String,
    name: String,
    version: String,
    kind: String,
    phase: String,
    pub rules: Vec<Rule>,
}

#[derive(Debug, Deserialize)]
pub struct GetRulesetsResponse {
    description: String,
    id: String,
    last_updated: String,
    name: String,
    version: String,
    kind: String,
    phase: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateRulesetRequest {
    description: String,
    name: String,
    kind: String,
    phase: String,
    rules: Vec<CreateCustomRuleRequest>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Rule {
    pub id: Option<String>,
    pub expression: String,
    pub description: Option<String>,
    pub enabled: Option<bool>,
    pub action: String,
    pub action_parameters: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreateCustomRuleRequest {
    pub expression: String,
    pub description: String,
    pub action: String,
}

#[derive(Debug)]
pub struct CloudflareError {
    pub details: String,
}

impl CloudflareError {
    pub fn new(msg: &str) -> CloudflareError {
        CloudflareError {
            details: msg.to_string(),
        }
    }
}

impl fmt::Display for CloudflareError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CloudflareError: {}", self.details)
    }
}

impl Error for CloudflareError {}

#[instrument]
pub fn get_cloudflare_vars() -> Result<(String, String), Box<dyn Error>> {
    let cloudflare_zone = std::env::var("CLOUDFLARE_ZONE")
        .map_err(|_e| "Missing cloudflare env variable".to_string())?;
    let cloudflare_api_key = std::env::var("CLOUDFLARE_API_KEY")
        .map_err(|_e| "Missing cloudflare env variable".to_string())?;
    print!("cloudflare_zone:: {}", cloudflare_zone);
    Ok((cloudflare_zone, cloudflare_api_key))
}

#[instrument]
pub async fn list_rulesets(
    api_key: &str,
    zone_id: &str,
) -> Result<Vec<GetRulesetsResponse>, Box<dyn Error>> {
    let client = Client::new();

    let response = client
        .get(&format!(
            "https://api.cloudflare.com/client/v4/zones/{}/rulesets",
            zone_id
        ))
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await?;

    if response.status().is_success() {
        let response_text = response.text().await?;
        info!("Response: {}", response_text);

        let api_response: ApiResponse<Vec<GetRulesetsResponse>> = deserialize_str(&response_text)?;
        Ok(api_response.result)
    } else {
        let error_text = response
            .text()
            .await
            .map_err(|e| CloudflareError::new(&format!("Failed to read error response: {}", e)))?;
        info!("Error response: {}", error_text);
        Err(Box::new(CloudflareError::new(&format!(
            "Failed to get rulesets: {}",
            error_text
        ))))
    }
}

#[instrument]
pub async fn get_ruleset_by_id(
    api_key: &str,
    zone_id: &str,
    ruleset_id: &str,
) -> Result<Ruleset, Box<dyn Error>> {
    let client = Client::new();

    let response = client
        .get(&format!(
            "https://api.cloudflare.com/client/v4/zones/{}/rulesets/{}",
            zone_id, ruleset_id
        ))
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await?;

    if response.status().is_success() {
        let response_text = response.text().await?;
        info!("Response: {}", response_text);

        let api_response: ApiResponse<Ruleset> = deserialize_str(&response_text)?;
        Ok(api_response.result)
    } else {
        let error_text = response
            .text()
            .await
            .map_err(|e| CloudflareError::new(&format!("Failed to read error response: {}", e)))?;
        info!("Error response: {}", error_text);
        Err(Box::new(CloudflareError::new(&format!(
            "Failed to get ruleset: {}",
            error_text
        ))))
    }
}

#[instrument]
pub async fn get_ruleset_by_phase(
    api_key: &str,
    zone_id: &str,
    ruleset_phase: &str,
) -> Result<Option<Ruleset>, Box<dyn Error>> {
    let rulesets = list_rulesets(&api_key, &zone_id).await?;

    let ruleset_id = match rulesets
        .into_iter()
        .find(|ruleset| ruleset.phase == ruleset_phase && ruleset.kind == "zone")
    {
        Some(ruleset) => Some(ruleset.id.to_string()),
        None => None,
    };

    let ruleset: Option<Ruleset> = match ruleset_id {
        Some(id) => {
            let ruleset = get_ruleset_by_id(&api_key, &zone_id, &id).await?;
            Some(ruleset)
        }
        None => None,
    };
    Ok(ruleset)
}

#[instrument]
pub async fn create_ruleset(
    api_key: &str,
    zone_id: &str,
    phase: &str,
    rule: CreateCustomRuleRequest,
) -> Result<Ruleset, Box<dyn Error>> {
    let client = Client::new();
    let body = CreateRulesetRequest {
        name: phase.to_string(),
        description: "A ruleset for country-based access control".to_string(),
        rules: vec![rule],
        phase: phase.to_string(),
        kind: "zone".to_string(),
    };

    let response = client
        .post(&format!(
            "https://api.cloudflare.com/client/v4/zones/{}/rulesets",
            zone_id
        ))
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await?;

    if response.status().is_success() {
        let response_text = response.text().await?;
        info!("Response: {}", response_text);

        let api_response: ApiResponse<Ruleset> = deserialize_str(&response_text)?;
        Ok(api_response.result)
    } else {
        let error_text = response
            .text()
            .await
            .map_err(|e| CloudflareError::new(&format!("Failed to read error response: {}", e)))?;
        info!("Error response: {}", error_text);
        Err(Box::new(CloudflareError::new(&format!(
            "Failed to create ruleset: {}",
            error_text
        ))))
    }
}

#[instrument]
pub async fn create_ruleset_rule(
    api_key: &str,
    zone_id: &str,
    ruleset_id: &str,
    rule: CreateCustomRuleRequest,
) -> Result<(), Box<dyn Error>> {
    let client = Client::new();

    let response = client
        .post(&format!(
            "https://api.cloudflare.com/client/v4/zones/{}/rulesets/{}/rules",
            zone_id, ruleset_id
        ))
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&rule)
        .send()
        .await?;

    if response.status().is_success() {
        Ok(())
    } else {
        let error_text = response
            .text()
            .await
            .map_err(|e| CloudflareError::new(&format!("Failed to read error response: {}", e)))?;
        info!("Error response: {}", error_text);
        Err(Box::new(CloudflareError::new(&format!(
            "Failed to create rule: {}",
            error_text
        ))))
    }
}

#[instrument]
pub async fn update_ruleset_rule(
    api_key: &str,
    zone_id: &str,
    ruleset_id: &str,
    rule_id: &str,
    rule: CreateCustomRuleRequest,
) -> Result<(), Box<dyn Error>> {
    let client = Client::new();

    let response = client
        .patch(&format!(
            "https://api.cloudflare.com/client/v4/zones/{}/rulesets/{}/rules/{}",
            zone_id, ruleset_id, rule_id
        ))
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&rule)
        .send()
        .await?;

    if response.status().is_success() {
        Ok(())
    } else {
        let error_text = response
            .text()
            .await
            .map_err(|e| CloudflareError::new(&format!("Failed to read error response: {}", e)))?;
        info!("Error response: {}", error_text);
        Err(Box::new(CloudflareError::new(&format!(
            "Failed to update rule: {}",
            error_text
        ))))
    }
}

#[instrument]
pub async fn delete_ruleset_rule(
    api_key: &str,
    zone_id: &str,
    ruleset_id: &str,
    rule_id: &str,
) -> Result<(), Box<dyn Error>> {
    let client = Client::new();

    let response = client
        .delete(&format!(
            "https://api.cloudflare.com/client/v4/zones/{}/rulesets/{}/rules/{}",
            zone_id, ruleset_id, rule_id
        ))
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .send()
        .await?;

    if response.status().is_success() {
        Ok(())
    } else {
        let error_text = response
            .text()
            .await
            .map_err(|e| CloudflareError::new(&format!("Failed to read error response: {}", e)))?;
        info!("Error response: {}", error_text);
        Err(Box::new(CloudflareError::new(&format!(
            "Failed to delete rule: {}",
            error_text
        ))))
    }
}
