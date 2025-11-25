// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::cloudflare::{get_cloudflare_vars, ApiResponse, CloudflareError};
use reqwest::Client;
use rocket::futures::stream::Forward;
use sequent_core::serialization::deserialize_with_path::deserialize_str;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;
use tracing::{error, info, instrument};

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

#[derive(Debug, Serialize, Deserialize)]
struct CreatePageRuleRequest {
    targets: Vec<Target>,
    actions: Vec<Action>,
    status: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PreviousCustomUrls {
    pub login: String,
    pub enrollment: String,
    pub saml: String,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ForwardURL {
    url: String,
    status_code: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum ActionValue {
    String(String),
    Integer(i64),
    ForwardURL(ForwardURL),
}
#[derive(Debug, Serialize, Deserialize)]
struct Action {
    id: String,
    value: ActionValue,
}

#[derive(Debug, Serialize, Deserialize)]
struct DnsRecord {
    #[serde(rename = "type")]
    record_type: String,
    name: String,
    content: String,
    ttl: u32,
    proxied: bool,
    id: String,
}

#[instrument]
pub async fn get_page_rule(target_value: &str) -> Result<Option<PageRule>, Box<dyn Error>> {
    info!("target_value {:?}", target_value);
    let page_rules = get_all_page_rules().await?;
    info!("get_page_rules: {:?}", page_rules);
    Ok(find_matching_target(page_rules, target_value))
}

#[instrument]
pub async fn get_dns_record(record_name: &str) -> Result<Option<DnsRecord>, Box<dyn Error>> {
    let dns_records = get_all_dns_records().await?;
    Ok(find_matching_dns_record(dns_records, record_name))
}

#[instrument]
pub async fn set_custom_url(
    origin: &str,
    redirect_to: &str,
    dns_prefix: &str,
    prev_custom_urls: &PreviousCustomUrls,
    key: &str,
) -> Result<(), Box<dyn Error>> {
    info!("Origin: {:?}", origin);
    info!("Redirect to: {:?}", redirect_to);
    info!("DNS Prefix: {:?}", dns_prefix);

    let current_prev_url = match key {
        "login" => &prev_custom_urls.login,
        "enrollment" => &prev_custom_urls.enrollment,
        "saml" => &prev_custom_urls.saml,
        _ => panic!("Invalid key provided"),
    };

    let current_dns_record = match get_dns_record(current_prev_url).await {
        Ok(dns_record) => {
            info!("Current DNS record found: {:?}", dns_record);
            dns_record
        }
        Err(e) => {
            let error_message = format!("Failed to get DNS record for {}: {}", origin, e);
            error!("{}", error_message);
            return Err(error_message.into());
        }
    };

    match current_dns_record {
        Some(dns_record) => {
            if let Err(e) = update_dns_record(&dns_record.id, redirect_to, dns_prefix).await {
                let error_message = format!("Failed to update DNS record: {}", e.to_string());
                error!("{}", error_message);
                return Err(error_message.into());
            }
            info!("DNS record updated successfully.");
        }
        None => {
            if let Err(e) = create_dns_record(redirect_to, dns_prefix).await {
                let error_message = format!("Failed to create DNS record: {}", e.to_string());
                error!("{}", error_message);
                return Err(error_message.into());
            }
            info!("DNS record created successfully.");
        }
    }

    let current_page_rule = match get_page_rule(origin).await {
        Ok(page_rule) => {
            info!("Current page rule found: {:?}", page_rule);
            page_rule
        }
        Err(e) => {
            let error_message = format!("Failed to get page rule for {}: {}", origin, e);
            error!("{}", error_message);
            return Err(error_message.into());
        }
    };

    match current_page_rule {
        Some(page_rule) => {
            if let Err(e) = update_page_rule(&page_rule.id, redirect_to, origin).await {
                let error_message = format!("Failed to update page rule: {}", e.to_string());
                error!("{}", error_message);
                return Err(error_message.into());
            }
            info!("Page rule updated successfully.");
        }
        None => {
            if let Err(e) = create_page_rule(redirect_to, origin).await {
                let error_message = format!("Failed to create page rule: {}", e.to_string());
                error!("{}", error_message);
                return Err(error_message.into());
            }
            info!("Page rule created successfully.");
        }
    }

    Ok(())
}

#[instrument]
async fn get_all_page_rules() -> Result<Vec<PageRule>, Box<dyn Error>> {
    let (zone_id, api_key) = get_cloudflare_vars()?;
    info!("zone_id {:?}", zone_id);
    info!("api_key {:?}", api_key);

    let client = Client::new();

    let response = client
        .get(&format!(
            "https://api.cloudflare.com/client/v4/zones/{}/pagerules",
            &zone_id,
        ))
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .send()
        .await
        .map_err(|e| CloudflareError::new(&format!("Request error: {}", e)))?;

    if response.status().is_success() {
        let response_text = response.text().await?;
        info!("Response: {}", response_text);

        let api_response: ApiResponse<Vec<PageRule>> = deserialize_str(&response_text)?;
        Ok(api_response.result)
    } else {
        let error_text = response
            .text()
            .await
            .map_err(|e| CloudflareError::new(&format!("Failed to read error response: {}", e)))?;
        info!("Error response: {}", error_text);
        Err(Box::new(CloudflareError::new(&format!(
            "Failed to get page rules: {}",
            error_text
        ))))
    }
}

#[instrument]
async fn get_all_dns_records() -> Result<Vec<DnsRecord>, Box<dyn Error>> {
    let (zone_id, api_key) = get_cloudflare_vars()?;
    info!("zone_id {:?}", zone_id);
    info!("api_key {:?}", api_key);

    let client = Client::new();

    let response = client
        .get(&format!(
            "https://api.cloudflare.com/client/v4/zones/{}/dns_records",
            &zone_id,
        ))
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .send()
        .await
        .map_err(|e| CloudflareError::new(&format!("Request error: {}", e)))?;

    if response.status().is_success() {
        let response_text = response.text().await?;
        info!("Response: {}", response_text);

        let api_response: ApiResponse<Vec<DnsRecord>> = deserialize_str(&response_text)?;
        Ok(api_response.result)
    } else {
        let error_text = response
            .text()
            .await
            .map_err(|e| CloudflareError::new(&format!("Failed to read error response: {}", e)))?;
        info!("Error response: {}", error_text);
        Err(Box::new(CloudflareError::new(&format!(
            "Failed to get page rules: {}",
            error_text
        ))))
    }
}

#[instrument]
fn find_matching_dns_record(records: Vec<DnsRecord>, expected_name: &str) -> Option<DnsRecord> {
    info!("find_matching_dns_record expected_name:{}", expected_name);
    for record in records {
        let name: Vec<String> = record.name.split(".").map(|s| s.to_owned()).collect();

        if let Some(name) = name.first() {
            info!("name: {}", name);

            if name == expected_name {
                return Some(record);
            }
        }
    }

    None
}

#[instrument]
fn find_matching_target(rules: Vec<PageRule>, expected_redirect_url: &str) -> Option<PageRule> {
    for rule in rules {
        for action in &rule.actions {
            if let ActionValue::ForwardURL(fwd) = action.value.clone() {
                if fwd.url == expected_redirect_url {
                    return Some(rule);
                }
            }
        }
    }
    None
}

#[instrument]
fn create_payload(origin: &str, redirect_to: &str) -> CreatePageRuleRequest {
    let targets = vec![Target {
        constraint: Constraint {
            operator: "matches".to_string(),
            value: origin.to_string(),
        },
        target: "url".to_string(),
    }];
    info!("lets url the url {:?}", origin);
    let actions = vec![Action {
        id: "forwarding_url".to_string(),
        value: ActionValue::ForwardURL(ForwardURL {
            url: redirect_to.to_string(),
            status_code: 301,
        }),
    }];

    CreatePageRuleRequest {
        targets,
        actions,
        status: "active".to_string(),
    }
}

#[instrument]
fn create_dns_payload(origin: &str) -> CreateDNSRecordRequest {
    let cloudflare_ip_dns_content = std::env::var("CUSTOM_URLS_IP_DNS_CONTENT")
        .unwrap_or_else(|_| "default.ip.address".to_string());
    info!("cloudflare_ip_dns_content: {}", cloudflare_ip_dns_content);
    CreateDNSRecordRequest {
        name: origin.to_string(),
        record_type: "A".to_string(),
        content: cloudflare_ip_dns_content,
        ttl: 3600,
        proxied: false,
    }
}

pub async fn create_dns_record(redirect_to: &str, dns_prefix: &str) -> Result<(), Box<dyn Error>> {
    let client = Client::new();
    let (zone_id, api_key) = match get_cloudflare_vars() {
        Ok(vars) => vars,
        Err(e) => {
            error!("Failed to get Cloudflare environment variables: {}", e);
            return Err(format!("Failed to get Cloudflare environment variables: {}", e).into());
        }
    };

    let url = format!(
        "https://api.cloudflare.com/client/v4/zones/{}/dns_records",
        zone_id
    );

    let request_dns_body = create_dns_payload(dns_prefix);
    info!("DNS prefix {:?}", dns_prefix);
    let response = match client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request_dns_body)
        .send()
        .await
    {
        Ok(resp) => resp,
        Err(e) => {
            error!("HTTP request failed: {}", e);
            return Err(format!("HTTP request failed: {}", e).into());
        }
    };

    if response.status().is_success() {
        println!("DNS record created successfully");
        Ok(())
    } else {
        let body = match response.text().await {
            Ok(text) => text,
            Err(e) => {
                error!("Failed to read error response: {}", e);
                return Err(format!("Failed to read error response: {}", e).into());
            }
        };
        Err(format!("Failed to create DNS record: {}", body).into())
    }
}

pub async fn update_dns_record(
    id: &str,
    redirect_to: &str,
    dns_prefix: &str,
) -> Result<(), Box<dyn Error>> {
    let client = Client::new();
    let (zone_id, api_key) = match get_cloudflare_vars() {
        Ok(vars) => vars,
        Err(e) => {
            error!("Failed to get Cloudflare environment variables: {}", e);
            return Err(format!("Failed to get Cloudflare environment variables: {}", e).into());
        }
    };

    let url = format!(
        "https://api.cloudflare.com/client/v4/zones/{}/dns_records/{}",
        zone_id, id
    );

    let request_dns_body = create_dns_payload(dns_prefix);
    info!("DNS prefix {:?}", dns_prefix);
    let response = match client
        .put(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request_dns_body)
        .send()
        .await
    {
        Ok(resp) => resp,
        Err(e) => {
            error!("HTTP request failed: {}", e);
            return Err(format!("HTTP request failed: {}", e).into());
        }
    };

    if response.status().is_success() {
        println!("DNS record created successfully");
        Ok(())
    } else {
        let body = match response.text().await {
            Ok(text) => text,
            Err(e) => {
                error!("Failed to read error response: {}", e);
                return Err(format!("Failed to read error response: {}", e).into());
            }
        };
        Err(format!("Failed to create DNS record: {}", body).into())
    }
}

async fn update_page_rule(
    rule_id: &str,
    redirect_to: &str,
    origin: &str,
) -> Result<(), Box<dyn Error>> {
    let (zone_id, api_key) = get_cloudflare_vars()?;
    let client = Client::new();
    let request_body = create_payload(redirect_to, origin);
    let page_rules = get_all_page_rules().await?;
    info!("Existing page rules: {:?}", page_rules);

    let response = client
        .put(&format!(
            "https://api.cloudflare.com/client/v4/zones/{}/pagerules/{}",
            zone_id, rule_id
        ))
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request_body)
        .send()
        .await?;

    if response.status().is_success() {
        info!("Page rule updated successfully");
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

async fn create_page_rule(redirect_to: &str, origin: &str) -> Result<(), Box<dyn Error>> {
    let (zone_id, api_key) = get_cloudflare_vars()?;
    let client = Client::new();
    info!("create_page_rule");
    let request_body = create_payload(redirect_to, origin);
    let response = client
        .post(&format!(
            "https://api.cloudflare.com/client/v4/zones/{}/pagerules",
            &zone_id,
        ))
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request_body)
        .send()
        .await
        .map_err(|e| CloudflareError::new(&format!("Request error: {}", e)))?;

    if response.status().is_success() {
        info!("Page rule created successfully");
        Ok(())
    } else {
        let error_text = response
            .text()
            .await
            .map_err(|e| CloudflareError::new(&format!("Failed to read error response: {}", e)))?;
        Err(Box::new(CloudflareError::new(&format!(
            "Failed to create page rule: {}",
            error_text
        ))))
    }
}
