// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::cloudflare::{get_cloudflare_vars, ApiResponse, CloudflareError};
use super::limit_access_by_countries::get_voting_portal_urls_prefix;
use anyhow::anyhow;
use reqwest::Client;
use rocket::futures::stream::Forward;
use sequent_core::serialization::deserialize_with_path::deserialize_str;
use sequent_core::services::generate_urls::{get_auth_url, AuthAction};
use sequent_core::services::keycloak::get_event_realm;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;
use std::str::FromStr;
use strum_macros::{Display, EnumString};
use tracing::{error, info, instrument};

const HTTPS_SCHEME: &str = "https://";
const SAML_BROKER_ENDPOINT_PATH: &str = "broker/simplesamlphp/endpoint";
const DNS_LABEL_MAX_LENGTH: usize = 63;
const DNS_RECORDS_PAGE_SIZE: usize = 100;
const DNS_RECORD_COMMENT_PREFIX: &str = "custom-url";
const DNS_RECORD_TYPE_A: &str = "A";

/// The address a custom URL redirects to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, EnumString)]
#[strum(serialize_all = "lowercase")]
pub enum CustomUrlKind {
    Login,
    Enrollment,
    Saml,
}

/// A single DNS label, lowercased, used as the custom URL subdomain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsLabel(String);

impl DnsLabel {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for DnsLabel {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let label = value.to_ascii_lowercase();
        let is_valid = !label.is_empty()
            && label.len() <= DNS_LABEL_MAX_LENGTH
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
        if is_valid {
            Ok(DnsLabel(label))
        } else {
            Err(anyhow!(
                "Invalid custom URL prefix {value:?}: expected a single DNS label"
            ))
        }
    }
}

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

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateDNSRecordRequest {
    #[serde(rename = "type")]
    record_type: String,
    name: String,
    content: String,
    ttl: u64,
    proxied: bool,
    comment: String,
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
    comment: Option<String>,
}

/// Cloudflare response that wraps a single DNS record.
#[derive(Debug, Deserialize)]
struct DnsRecordResponse {
    result: DnsRecord,
}

#[instrument]
pub async fn get_page_rule(target_value: &str) -> Result<Option<PageRule>, Box<dyn Error>> {
    info!("target_value {:?}", target_value);
    let page_rules = get_all_page_rules().await?;
    info!("get_page_rules: {:?}", page_rules);
    Ok(find_matching_target(page_rules, target_value))
}

/// Address that the custom URL of the given kind redirects to, built from the
/// tenant and the election event.
fn custom_url_redirect_to(
    kind: CustomUrlKind,
    voting_portal_url: &str,
    keycloak_public_url: &str,
    tenant_id: &str,
    election_event_id: &str,
) -> String {
    match kind {
        CustomUrlKind::Login => get_auth_url(
            voting_portal_url,
            tenant_id,
            election_event_id,
            AuthAction::Login,
        ),
        CustomUrlKind::Enrollment => get_auth_url(
            voting_portal_url,
            tenant_id,
            election_event_id,
            AuthAction::Enroll,
        ),
        CustomUrlKind::Saml => format!(
            "{keycloak_public_url}/realms/{}/{SAML_BROKER_ENDPOINT_PATH}",
            get_event_realm(tenant_id, election_event_id)
        ),
    }
}

/// Comment stored in the DNS record to tie it to a tenant, an election event
/// and a kind.
fn custom_url_dns_comment(tenant_id: &str, election_event_id: &str, kind: CustomUrlKind) -> String {
    format!("{DNS_RECORD_COMMENT_PREFIX} {tenant_id} {election_event_id} {kind}")
}

/// Host that the election event's own page rule redirects from.
fn page_rule_host(page_rule: &PageRule) -> Option<&str> {
    page_rule
        .targets
        .first()
        .and_then(|target| target.constraint.value.strip_prefix(HTTPS_SCHEME))
        .filter(|host| {
            !host.is_empty()
                && host
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
        })
}

/// Creates or updates the DNS record for `dns_prefix` and the page rule that
/// redirects it to the login, enrollment or SAML address of the election
/// event. Only the record and the page rule of that election event and kind
/// are updated, and a prefix that another host already uses is rejected.
#[instrument]
pub async fn set_custom_url(
    tenant_id: &str,
    election_event_id: &str,
    kind: CustomUrlKind,
    dns_prefix: &DnsLabel,
) -> Result<(), Box<dyn Error>> {
    let (voting_portal_url, keycloak_public_url) = get_voting_portal_urls_prefix()?;
    let redirect_to = custom_url_redirect_to(
        kind,
        &voting_portal_url,
        &keycloak_public_url,
        tenant_id,
        election_event_id,
    );
    info!("Redirect to: {:?}", redirect_to);
    info!("DNS Prefix: {:?}", dns_prefix);

    let current_page_rule = match get_page_rule(&redirect_to).await {
        Ok(page_rule) => {
            info!("Current page rule found: {:?}", page_rule);
            page_rule
        }
        Err(e) => {
            let error_message = format!("Failed to get page rule for {}: {}", redirect_to, e);
            error!("{}", error_message);
            return Err(error_message.into());
        }
    };

    let dns_records = match get_all_dns_records().await {
        Ok(dns_records) => dns_records,
        Err(e) => {
            let error_message = format!("Failed to get DNS records: {}", e);
            error!("{}", error_message);
            return Err(error_message.into());
        }
    };

    let dns_comment = custom_url_dns_comment(tenant_id, election_event_id, kind);
    let current_dns_record = find_owned_dns_record(
        &dns_records,
        &dns_comment,
        current_page_rule.as_ref().and_then(page_rule_host),
        &custom_urls_ip_dns_content(),
    );
    info!("Current DNS record found: {:?}", current_dns_record);

    let current_dns_record_id = current_dns_record.map(|dns_record| dns_record.id.as_str());
    if is_platform_host_prefix(dns_prefix, &[&voting_portal_url, &keycloak_public_url])
        || find_conflicting_dns_record(&dns_records, dns_prefix, current_dns_record_id).is_some()
    {
        let error_message = format!(
            "Custom URL prefix {} is already in use",
            dns_prefix.as_str()
        );
        error!("{}", error_message);
        return Err(error_message.into());
    }

    let dns_record = match current_dns_record_id {
        Some(dns_record_id) => {
            match update_dns_record(dns_record_id, dns_prefix.as_str(), &dns_comment).await {
                Ok(dns_record) => {
                    info!("DNS record updated successfully.");
                    dns_record
                }
                Err(e) => {
                    let error_message = format!("Failed to update DNS record: {}", e);
                    error!("{}", error_message);
                    return Err(error_message.into());
                }
            }
        }
        None => match create_dns_record(dns_prefix.as_str(), &dns_comment).await {
            Ok(dns_record) => {
                info!("DNS record created successfully.");
                dns_record
            }
            Err(e) => {
                let error_message = format!("Failed to create DNS record: {}", e);
                error!("{}", error_message);
                return Err(error_message.into());
            }
        },
    };

    let origin = format!("{HTTPS_SCHEME}{}", dns_record.name);

    match current_page_rule {
        Some(page_rule) => {
            if let Err(e) = update_page_rule(&page_rule.id, &origin, &redirect_to).await {
                let error_message = format!("Failed to update page rule: {}", e);
                error!("{}", error_message);
                return Err(error_message.into());
            }
            info!("Page rule updated successfully.");
        }
        None => {
            if let Err(e) = create_page_rule(&origin, &redirect_to).await {
                let error_message = format!("Failed to create page rule: {}", e);
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
    let mut dns_records = vec![];
    let mut page: usize = 1;

    loop {
        let response = client
            .get(format!(
                "https://api.cloudflare.com/client/v4/zones/{}/dns_records?page={}&per_page={}",
                &zone_id, page, DNS_RECORDS_PAGE_SIZE,
            ))
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .send()
            .await
            .map_err(|e| CloudflareError::new(&format!("Request error: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.map_err(|e| {
                CloudflareError::new(&format!("Failed to read error response: {}", e))
            })?;
            info!("Error response: {}", error_text);
            return Err(Box::new(CloudflareError::new(&format!(
                "Failed to get page rules: {}",
                error_text
            ))));
        }

        let response_text = response.text().await?;
        info!("Response: {}", response_text);

        let api_response: ApiResponse<Vec<DnsRecord>> = deserialize_str(&response_text)?;
        let page_length = api_response.result.len();
        dns_records.extend(api_response.result);
        if page_length < DNS_RECORDS_PAGE_SIZE {
            return Ok(dns_records);
        }
        page += 1;
    }
}

/// Record whose full name is `expected_name`, ignoring case.
fn find_matching_dns_record<'a>(
    records: &'a [DnsRecord],
    expected_name: &str,
) -> Option<&'a DnsRecord> {
    records
        .iter()
        .find(|record| record.name.eq_ignore_ascii_case(expected_name))
}

/// The record tagged for this election event and kind, or else the untagged
/// custom URL record its page rule already redirects from.
fn find_owned_dns_record<'a>(
    records: &'a [DnsRecord],
    comment: &str,
    page_rule_host: Option<&str>,
    dns_content: &str,
) -> Option<&'a DnsRecord> {
    records
        .iter()
        .find(|record| record.comment.as_deref() == Some(comment))
        .or_else(|| {
            page_rule_host
                .and_then(|host| find_matching_dns_record(records, host))
                .filter(|record| {
                    record.comment.as_deref().unwrap_or_default().is_empty()
                        && record.record_type == DNS_RECORD_TYPE_A
                        && record.content == dns_content
                })
        })
}

/// Whether `dns_prefix` is the first label of the host of one of the platform
/// URLs.
fn is_platform_host_prefix(dns_prefix: &DnsLabel, platform_urls: &[&str]) -> bool {
    platform_urls
        .iter()
        .filter_map(|url| reqwest::Url::parse(url).ok())
        .any(|url| {
            url.host_str()
                .and_then(|host| host.split('.').next())
                .is_some_and(|label| label.eq_ignore_ascii_case(dns_prefix.as_str()))
        })
}

/// Record other than `owned_record_id` whose first label is `dns_prefix`.
fn find_conflicting_dns_record<'a>(
    records: &'a [DnsRecord],
    dns_prefix: &DnsLabel,
    owned_record_id: Option<&str>,
) -> Option<&'a DnsRecord> {
    records.iter().find(|record| {
        Some(record.id.as_str()) != owned_record_id
            && record
                .name
                .split('.')
                .next()
                .is_some_and(|label| label.eq_ignore_ascii_case(dns_prefix.as_str()))
    })
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

/// Address that the custom URL DNS records point to.
fn custom_urls_ip_dns_content() -> String {
    std::env::var("CUSTOM_URLS_IP_DNS_CONTENT").unwrap_or_else(|_| "default.ip.address".to_string())
}

#[instrument]
fn create_dns_payload(origin: &str, comment: &str) -> CreateDNSRecordRequest {
    let cloudflare_ip_dns_content = custom_urls_ip_dns_content();
    info!("cloudflare_ip_dns_content: {}", cloudflare_ip_dns_content);
    CreateDNSRecordRequest {
        name: origin.to_string(),
        record_type: DNS_RECORD_TYPE_A.to_string(),
        content: cloudflare_ip_dns_content,
        ttl: 3600,
        proxied: false,
        comment: comment.to_string(),
    }
}

async fn create_dns_record(dns_prefix: &str, comment: &str) -> Result<DnsRecord, Box<dyn Error>> {
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

    let request_dns_body = create_dns_payload(dns_prefix, comment);
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
        let response_text = response.text().await?;
        let dns_record_response: DnsRecordResponse = deserialize_str(&response_text)?;
        Ok(dns_record_response.result)
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

async fn update_dns_record(
    id: &str,
    dns_prefix: &str,
    comment: &str,
) -> Result<DnsRecord, Box<dyn Error>> {
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

    let request_dns_body = create_dns_payload(dns_prefix, comment);
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
        let response_text = response.text().await?;
        let dns_record_response: DnsRecordResponse = deserialize_str(&response_text)?;
        Ok(dns_record_response.result)
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
    origin: &str,
    redirect_to: &str,
) -> Result<(), Box<dyn Error>> {
    let (zone_id, api_key) = get_cloudflare_vars()?;
    let client = Client::new();
    let request_body = create_payload(origin, redirect_to);
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

async fn create_page_rule(origin: &str, redirect_to: &str) -> Result<(), Box<dyn Error>> {
    let (zone_id, api_key) = get_cloudflare_vars()?;
    let client = Client::new();
    info!("create_page_rule");
    let request_body = create_payload(origin, redirect_to);
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

#[cfg(test)]
mod tests {
    use super::*;

    const TENANT_ID: &str = "90505c8a-23a9-4cdf-a26b-4e19f6a097d5";
    const OTHER_TENANT_ID: &str = "5e6a0c0f-2a4b-4a6e-9d7e-3f0f2c1b8a90";
    const ELECTION_EVENT_ID: &str = "a00cbd54-d7c4-4440-a614-261d5d8d573b";
    const VOTING_PORTAL_URL: &str = "https://voting.example.com";
    const KEYCLOAK_PUBLIC_URL: &str = "https://login.example.com";
    const CUSTOM_URL_DNS_CONTENT: &str = "192.0.2.1";
    const DNS_RECORD_COMMENT_MAX_LENGTH: usize = 100;

    /// An untagged A record that points to the custom URL address.
    fn dns_record(id: &str, name: &str) -> DnsRecord {
        DnsRecord {
            record_type: DNS_RECORD_TYPE_A.to_string(),
            name: name.to_string(),
            content: CUSTOM_URL_DNS_CONTENT.to_string(),
            ttl: 3600,
            proxied: false,
            id: id.to_string(),
            comment: None,
        }
    }

    /// A custom URL record tagged for the login URL of an election event.
    fn tagged_dns_record(
        id: &str,
        name: &str,
        tenant_id: &str,
        election_event_id: &str,
    ) -> DnsRecord {
        DnsRecord {
            comment: Some(custom_url_dns_comment(
                tenant_id,
                election_event_id,
                CustomUrlKind::Login,
            )),
            ..dns_record(id, name)
        }
    }

    /// A page rule that redirects `target` to `forward_url`.
    fn page_rule(id: &str, target: &str, forward_url: &str) -> PageRule {
        PageRule {
            id: id.to_string(),
            targets: vec![Target {
                target: "url".to_string(),
                constraint: Constraint {
                    operator: "matches".to_string(),
                    value: target.to_string(),
                },
            }],
            actions: vec![Action {
                id: "forwarding_url".to_string(),
                value: ActionValue::ForwardURL(ForwardURL {
                    url: forward_url.to_string(),
                    status_code: 301,
                }),
            }],
        }
    }

    /// Redirect address of a custom URL of the test election event.
    fn redirect_to(kind: CustomUrlKind, tenant_id: &str) -> String {
        custom_url_redirect_to(
            kind,
            VOTING_PORTAL_URL,
            KEYCLOAK_PUBLIC_URL,
            tenant_id,
            ELECTION_EVENT_ID,
        )
    }

    /// Only the three known keys parse; anything else is an error, not a panic.
    #[test]
    fn custom_url_kind_rejects_unknown_key() {
        assert!("foo".parse::<CustomUrlKind>().is_err());
        assert!("".parse::<CustomUrlKind>().is_err());
        assert_eq!(
            "login".parse::<CustomUrlKind>().ok(),
            Some(CustomUrlKind::Login)
        );
        assert_eq!(
            "enrollment".parse::<CustomUrlKind>().ok(),
            Some(CustomUrlKind::Enrollment)
        );
        assert_eq!(
            "saml".parse::<CustomUrlKind>().ok(),
            Some(CustomUrlKind::Saml)
        );
    }

    /// A single label is accepted and lowercased.
    #[test]
    fn dns_label_accepts_single_label() {
        assert_eq!(
            "My-Vote2".parse::<DnsLabel>().ok().map(|label| label.0),
            Some("my-vote2".to_string())
        );
        assert!("a".repeat(DNS_LABEL_MAX_LENGTH).parse::<DnsLabel>().is_ok());
    }

    /// Dots, wildcards, URLs and invalid label characters are rejected.
    #[test]
    fn dns_label_rejects_anything_but_a_single_label() {
        for value in [
            "",
            "a.b",
            "*",
            "@",
            "-vote",
            "vote-",
            "vote_1",
            "vote 1",
            "voting.example.com",
            "https://voting.example.com",
            "vöte",
        ] {
            assert!(value.parse::<DnsLabel>().is_err(), "{value:?}");
        }
        assert!("a"
            .repeat(DNS_LABEL_MAX_LENGTH + 1)
            .parse::<DnsLabel>()
            .is_err());
    }

    /// The redirect address only depends on the tenant, the election event and the kind.
    #[test]
    fn redirect_to_is_derived_from_tenant_and_event() {
        assert_eq!(
            redirect_to(CustomUrlKind::Login, TENANT_ID),
            format!("{VOTING_PORTAL_URL}/tenant/{TENANT_ID}/event/{ELECTION_EVENT_ID}/login")
        );
        assert_eq!(
            redirect_to(CustomUrlKind::Enrollment, TENANT_ID),
            format!("{VOTING_PORTAL_URL}/tenant/{TENANT_ID}/event/{ELECTION_EVENT_ID}/enroll")
        );
        assert_eq!(
            redirect_to(CustomUrlKind::Saml, TENANT_ID),
            format!(
                "{KEYCLOAK_PUBLIC_URL}/realms/tenant-{TENANT_ID}-event-{ELECTION_EVENT_ID}/broker/simplesamlphp/endpoint"
            )
        );
    }

    /// A page rule that redirects to another tenant is not picked.
    #[test]
    fn page_rule_lookup_only_matches_own_redirect() {
        let rules = vec![page_rule(
            "rule-b",
            "https://tenant-b.example.com",
            &redirect_to(CustomUrlKind::Login, OTHER_TENANT_ID),
        )];
        assert!(
            find_matching_target(rules, &redirect_to(CustomUrlKind::Login, TENANT_ID)).is_none()
        );
    }

    /// Only a plain `https://host` page rule target has a host.
    #[test]
    fn page_rule_host_reads_exact_https_target() {
        let forward_url = redirect_to(CustomUrlKind::Login, TENANT_ID);
        assert_eq!(
            page_rule_host(&page_rule("1", "https://my-vote.example.com", &forward_url)),
            Some("my-vote.example.com")
        );
        for target in [
            "https://voting.example.com/path",
            "voting.example.com",
            "http://voting.example.com",
            "https://",
        ] {
            assert!(
                page_rule_host(&page_rule("1", target, &forward_url)).is_none(),
                "{target:?}"
            );
        }
    }

    /// A record is found by its full name, not by its first label.
    #[test]
    fn dns_record_lookup_requires_exact_name() {
        let records = vec![
            dns_record("1", "voting.example.com"),
            dns_record("2", "tenant-b.example.com"),
        ];
        assert!(find_matching_dns_record(&records, "voting").is_none());
        assert!(find_matching_dns_record(&records, "tenant-b").is_none());
        assert_eq!(
            find_matching_dns_record(&records, "Tenant-B.example.com")
                .map(|record| record.id.as_str()),
            Some("2")
        );
    }

    /// The tagged record wins; an untagged one is used only when the page rule
    /// of the election event redirects from it.
    #[test]
    fn owned_dns_record_is_the_tagged_one_or_the_untagged_page_rule_host() {
        let other_event_id = "0b8c3c2e-6f0e-4e7a-8a52-6d1f6a3b9c11";
        let comment = custom_url_dns_comment(TENANT_ID, ELECTION_EVENT_ID, CustomUrlKind::Login);
        let records = vec![
            dns_record("1", "legacy-vote.example.com"),
            tagged_dns_record("2", "other-vote.example.com", TENANT_ID, other_event_id),
            tagged_dns_record("3", "my-vote.example.com", TENANT_ID, ELECTION_EVENT_ID),
        ];
        let owned_id = |records: &[DnsRecord], host: Option<&str>| {
            find_owned_dns_record(records, &comment, host, CUSTOM_URL_DNS_CONTENT)
                .map(|record| record.id.clone())
        };

        assert_eq!(owned_id(&records, None), Some("3".to_string()));
        assert_eq!(
            owned_id(&records, Some("legacy-vote.example.com")),
            Some("3".to_string())
        );
        assert_eq!(
            owned_id(&records[..2], Some("legacy-vote.example.com")),
            Some("1".to_string())
        );
        assert_eq!(
            owned_id(&records[..2], Some("other-vote.example.com")),
            None
        );
        assert_eq!(owned_id(&records[..2], None), None);
    }

    /// An untagged record is only taken over when it is an A record with the
    /// custom URL address.
    #[test]
    fn untagged_dns_record_is_owned_only_when_it_points_to_the_custom_url_address() {
        let comment = custom_url_dns_comment(TENANT_ID, ELECTION_EVENT_ID, CustomUrlKind::Login);
        let records = vec![
            DnsRecord {
                content: "198.51.100.7".to_string(),
                ..dns_record("1", "voting.example.com")
            },
            DnsRecord {
                record_type: "CNAME".to_string(),
                content: CUSTOM_URL_DNS_CONTENT.to_string(),
                ..dns_record("2", "login.example.com")
            },
        ];
        for host in ["voting.example.com", "login.example.com"] {
            assert!(
                find_owned_dns_record(&records, &comment, Some(host), CUSTOM_URL_DNS_CONTENT)
                    .is_none(),
                "{host:?}"
            );
        }
    }

    /// A record tagged with the same election event id for another tenant is
    /// not taken over.
    #[test]
    fn dns_record_tagged_for_another_tenant_is_not_owned() {
        let comment = custom_url_dns_comment(TENANT_ID, ELECTION_EVENT_ID, CustomUrlKind::Login);
        let records = vec![tagged_dns_record(
            "1",
            "vote.example.com",
            OTHER_TENANT_ID,
            ELECTION_EVENT_ID,
        )];
        for host in [None, Some("vote.example.com")] {
            assert!(
                find_owned_dns_record(&records, &comment, host, CUSTOM_URL_DNS_CONTENT).is_none(),
                "{host:?}"
            );
        }
    }

    /// The DNS record comment stays within 100 characters.
    #[test]
    fn dns_record_comment_fits_the_length_limit() {
        for kind in [
            CustomUrlKind::Login,
            CustomUrlKind::Enrollment,
            CustomUrlKind::Saml,
        ] {
            let comment = custom_url_dns_comment(TENANT_ID, ELECTION_EVENT_ID, kind);
            assert!(
                comment.len() <= DNS_RECORD_COMMENT_MAX_LENGTH,
                "{comment:?}"
            );
        }
    }

    /// The first label of a platform URL host cannot be used as a prefix.
    #[test]
    fn prefix_of_a_platform_host_is_rejected() {
        let platform_urls = [VOTING_PORTAL_URL, KEYCLOAK_PUBLIC_URL];
        for prefix in ["voting", "Login"] {
            let prefix: DnsLabel = prefix.parse().expect("valid label");
            assert!(
                is_platform_host_prefix(&prefix, &platform_urls),
                "{prefix:?}"
            );
        }
        let my_vote: DnsLabel = "my-vote".parse().expect("valid label");
        assert!(!is_platform_host_prefix(&my_vote, &platform_urls));
    }

    /// A prefix used by a record that is not the election event's own is rejected.
    #[test]
    fn prefix_used_by_another_dns_record_is_rejected() {
        let records = vec![
            dns_record("1", "voting.example.com"),
            dns_record("2", "my-vote.example.com"),
        ];
        let voting: DnsLabel = "voting".parse().expect("valid label");
        let my_vote: DnsLabel = "my-vote".parse().expect("valid label");
        let new_vote: DnsLabel = "new-vote".parse().expect("valid label");

        assert!(find_conflicting_dns_record(&records, &voting, None).is_some());
        assert!(find_conflicting_dns_record(&records, &voting, Some("2")).is_some());
        assert!(find_conflicting_dns_record(&records, &my_vote, None).is_some());
        assert!(find_conflicting_dns_record(&records, &my_vote, Some("2")).is_none());
        assert!(find_conflicting_dns_record(&records, &new_vote, Some("2")).is_none());
    }
}
