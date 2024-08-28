// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use reqwest::Client;
use rocket::futures::stream::Forward;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;

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
    status: String
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateDNSRecordRequest {
    #[serde(rename = "type")]
     record_type: String, 
     name: String,
     content: String,
     ttl: u64,
     proxied: bool, 
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

#[derive(Debug)]
struct CloudflareError {
    details: String,
}

#[derive(Serialize, Deserialize)]
struct DnsRecord {
    #[serde(rename = "type")]
    record_type: String,
    name: String,
    content: String,
    ttl: u32,
    proxied: bool,
}

impl CloudflareError {
    fn new(msg: &str) -> CloudflareError {
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

pub async fn get_page_rule(
    target_value: &str,
) -> Result<Option<PageRule>, Box<dyn Error>> {
    info!("target_value {:?}", target_value);
    let page_rules = get_all_page_rules().await?;
    info!("get_page_rules: {:?}", page_rules);
    Ok(find_matching_target(page_rules, target_value))
}
pub async fn set_custom_url(
    origin: &str,
    redirect_to: &str,
) -> Result<(), Box<dyn Error>> {
    info!{"origin pring {:?}", origin}
    info!{"origin redirect too {:?}", redirect_to}
    if let Ok(_cloudflare_env) = std::env::var("CLOUDFLARE_ENV") {
        let current_page_rule = get_page_rule(origin).await?;
        info!("got to set custom service - page rule {:?}", current_page_rule);

         match current_page_rule {
             Some(page_rule) => {
                 if create_dns_record(redirect_to, origin).await.is_ok() {
                     update_page_rule(&page_rule.id, redirect_to, origin).await?;
                 } else {
                     return Err("Failed to create DNS record. Page rule was not created.".into());
                 }
         info!("got updated");
             }
             None => {
                 if create_dns_record(redirect_to, origin).await.is_ok() {
                //      let redirect_to_login = format!("{}/login", redirect_to);
                //      let redirect_to_enrollment = format!("{}/enrollment", redirect_to);
                //      info!("originoriginoriginorigin{:?}", origin);
                    create_page_rule(redirect_to, origin).await?;
                 } else {
                     return Err("Failed to create DNS record. Page rule was not created.".into());
                 }
             }
         }
    } else {
        info!("CLOUDFLARE_ENV environment variable is not set.");
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

async fn get_all_page_rules() -> Result<Vec<PageRule>, Box<dyn Error>> {
    let (zone_id, api_email, api_key) = get_cloudflare_vars()?;
    info!("zone_id {:?}", zone_id);
    info!("api_email {:?}", api_email);
    info!("api_key {:?}", format!("Bearer {}", api_key));
    
    let client = Client::new();
    
    let response = client
        .get(&format!(
            "https://api.cloudflare.com/client/v4/zones/{}/pagerules",
            &zone_id,
        ))
        .header("X-Auth-Email", api_email)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .send()
        .await
        .map_err(|e| CloudflareError::new(&format!("Request error: {}", e)))?;
    
    info!("Response status: {:?}", response.status());
    info!("Response headers: {:?}", response.headers());

    if response.status().is_success() {
        info!("Successful response");
        let api_response: ApiResponse<Vec<PageRule>> = response
            .json()
            .await
            .map_err(|e| CloudflareError::new(&format!("Failed to parse response: {}", e)))?;
        Ok(api_response.result)
    } else {
        let error_text = response.text().await.map_err(|e| CloudflareError::new(&format!("Failed to read error response: {}", e)))?;
        info!("Error response: {}", error_text);
        Err(Box::new(CloudflareError::new(&format!(
            "Failed to get page rules: {}",
            error_text
        ))))
    }
}

fn find_matching_target(
    rules: Vec<PageRule>,
    expected_redirect_url: &str,
) -> Option<PageRule> {
    info!("rulesrulesrulesrules: {:?}", rules);
    info!("expected_redirect_urlexpected_redirect_url: {}", expected_redirect_url);
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

fn create_payload(origin: &str, redirect_to: &str) -> CreatePageRuleRequest {
    // let url = format!("https://{}.vaiphon.com", origin);
    let targets = vec![Target {
        constraint: Constraint {
            operator: "matches".to_string(),
            value: origin.to_string()
            // origin.to_string(),
            
        },
        target: "url".to_string(),
    }];
    info!("lets url the url {:?}", origin);
    let actions = vec![Action {
        id: "forwarding_url".to_string(),
        value: ForwardURL {
            url: redirect_to.to_string(),
            status_code: 301,
        },
    }];

    CreatePageRuleRequest { targets, actions, status:"active".to_string() }
}

fn create_dns_payload( redirect_to: &str, origin: &str,) -> CreateDNSRecordRequest {

info!("originnnnnn {:?}", origin);
    CreateDNSRecordRequest {
        name: origin.to_string(),
        record_type: "A".to_string(),
        content: "165.22.199.100".to_string(),
        ttl: 3600,
        proxied: false,
    }
}
pub async fn create_dns_record(
    redirect_to: &str,
    origin: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let (zone_id, api_email, api_key) = get_cloudflare_vars()?;
    let url = format!("https://api.cloudflare.com/client/v4/zones/{}/dns_records", zone_id);
    let request_dns_body = create_dns_payload(redirect_to, origin);

    let response = client
        .post(&url)
        .header("X-Auth-Email", api_email)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request_dns_body)
        .send()
        .await?;

    if response.status().is_success() {
        println!("DNS record created successfully");
        Ok(())
    } else {
        let body = response.text().await?;
        Err(format!("Failed to create DNS record: {}", body).into())
    }
}

async fn update_page_rule(
    rule_id: &str,
    redirect_to: &str,
    origin: &str,
) -> Result<(), Box<dyn Error>> {
    let (zone_id, api_email, api_key) = get_cloudflare_vars()?;
    let client = Client::new();
    let request_body = create_payload(redirect_to, origin);

    let page_rules = get_all_page_rules().await?;
    info!("Existing page rules: {:?}", page_rules);

    let response = client
        .put(&format!(
            "https://api.cloudflare.com/client/v4/zones/{}/pagerules/{}",
            zone_id, rule_id
        ))
        .header("X-Auth-Email", api_email)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request_body)
        .send()
        .await?;

    if response.status().is_success() {
        println!("Page rule updated successfully");
        Ok(())
    } else {
        let error_text = response.text().await?;
        info!("Failed to update page rule: {}", error_text);
        Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to update page rule: {}", error_text),
        )))
    }
}

async fn create_page_rule(
    redirect_to: &str,
    origin: &str,
) -> Result<(), Box<dyn Error>> {
    let (zone_id, api_email, api_key) = get_cloudflare_vars()?;
    let client = Client::new();
    info!("create_page_rule");
    let request_body = create_payload(redirect_to, origin);

    let response = client
        .post(&format!(
            "https://api.cloudflare.com/client/v4/zones/{}/pagerules",
            &zone_id,
        ))
        .header("X-Auth-Email", api_email)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request_body)
        .send()
        .await
        .map_err(|e| CloudflareError::new(&format!("Request error: {}", e)))?;
    
    if response.status().is_success() {
        println!("Page rule created successfully");
        Ok(())
    } else {
        let error_text = response.text().await.map_err(|e| CloudflareError::new(&format!("Failed to read error response: {}", e)))?;
        Err(Box::new(CloudflareError::new(&format!(
            "Failed to create page rule: {}",
            error_text
        ))))
    }
}

