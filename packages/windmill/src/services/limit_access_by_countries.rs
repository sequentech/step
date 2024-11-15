// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::cloudflare::{
    create_ruleset, create_ruleset_rule, delete_ruleset_rule, get_cloudflare_vars,
    get_ruleset_by_phase, update_ruleset_rule, CreateCustomRuleRequest, Rule, Ruleset,
    WAF_RULESET_PHASE,
};
use anyhow::{anyhow, Context, Result};
use rocket::{form::validate::Contains, http::Status};
use tracing::{info, instrument};

#[instrument]
fn get_voting_portal_urls_prefix() -> Result<(String, String)> {
    //TODO: change default values?
    let voting_portal_url = std::env::var("VOTING_PORTAL_URL")
        .with_context(|| "Error fetching VOTING_PORTAL_URL env var")?;
    let voting_portal_keycloak_url = std::env::var("KEYCLOAK_PUBLIC_URL")
        .with_context(|| "Error fetching KEYCLOAK_PUBLIC_URL env var")?;
    Ok((voting_portal_url, voting_portal_keycloak_url))
}

#[instrument]
fn create_limit_ip_by_countries_rule_format(
    tenant_id: String,
    countries: Vec<String>,
    is_enrollment: bool,
) -> Result<CreateCustomRuleRequest> {
    let (voting_portal_url, voting_portal_keycloak_url) = get_voting_portal_urls_prefix()?;

    let keycloak_rule_expression_voting = format!(
        "http.request.full_uri contains \"{}\" and http.request.uri.query contains \"voting-portal\" and http.request.uri.path contains \"/auth/realms/tenant-{}-event-\" and ends_with(http.request.uri.path, \"{}\")",
        voting_portal_keycloak_url,
        tenant_id,
        if is_enrollment {
            "/protocol/openid-connect/registrations"
        } else {
            "/protocol/openid-connect/auth"
        }
    );

    let rule_expression = if !countries.is_empty() {
        let countries_expression = countries
            .iter()
            .map(|country| format!("ip.geoip.country eq \"{}\"", country))
            .collect::<Vec<_>>()
            .join(" or ");

        format!(
            "(http.request.full_uri contains \"{}\" and ({})) and ({})",
            voting_portal_url, keycloak_rule_expression_voting, countries_expression
        )
    } else {
        format!(
            "(http.request.full_uri contains \"{}\" and ({}))",
            voting_portal_url, keycloak_rule_expression_voting
        )
    };

    Ok(CreateCustomRuleRequest {
        action: "block".to_string(),
        description: format!(
            "Block access in tenant {} from countries: {}",
            tenant_id,
            if countries.is_empty() {
                "none".to_string()
            } else {
                countries.join(",")
            }
        ),
        expression: rule_expression,
    })
}

#[instrument]
async fn update_or_create_limit_ip_by_countries_rule(
    api_key: &str,
    zone_id: &str,
    ruleset: &Ruleset,
    tenant_id: String,
    countries: Vec<String>,
    is_enrollment: bool,
) -> Result<()> {
    let existing_rules: Vec<Rule> = ruleset.rules.clone();
    let ruleset_id = ruleset.id.clone();

    let rule_id = existing_rules
        .iter()
        .find(|rule| {
            rule.expression.contains(tenant_id.as_str())
                && rule.expression.contains(if is_enrollment {
                    "/protocol/openid-connect/registrations"
                } else {
                    "/protocol/openid-connect/auth"
                })
        })
        .and_then(|rule| rule.id.clone());

    match rule_id {
        Some(id) => {
            if countries.is_empty() {
                delete_ruleset_rule(&api_key, &zone_id, &ruleset_id, &id)
                    .await
                    .map_err(|err| anyhow!("{:?}", err))?;
            } else {
                let rule = create_limit_ip_by_countries_rule_format(
                    tenant_id.clone(),
                    countries.clone(),
                    is_enrollment,
                )?;
                update_ruleset_rule(&api_key, &zone_id, &ruleset_id, &id, rule)
                    .await
                    .map_err(|err| anyhow!("{:?}", err))?;
            }
        }
        None => {
            if !countries.is_empty() {
                let rule = create_limit_ip_by_countries_rule_format(
                    tenant_id.clone(),
                    countries.clone(),
                    is_enrollment,
                )?;
                create_ruleset_rule(&api_key, &zone_id, &ruleset_id, rule)
                    .await
                    .map_err(|err| anyhow!("{:?}", err))?;
            }
        }
    }

    Ok(())
}

#[instrument]
async fn create_limit_ip_by_countries_ruleset(
    api_key: &str,
    zone_id: &str,
    tenant_id: String,
    countries: Vec<String>,
    is_enrollment: bool,
    ruleset_phase: &str,
) -> Result<CreateCustomRuleRequest> {
    let rule: CreateCustomRuleRequest = create_limit_ip_by_countries_rule_format(
        tenant_id.clone(),
        countries.clone(),
        is_enrollment,
    )?;

    create_ruleset(&api_key, &zone_id, ruleset_phase, rule.clone())
        .await
        .map_err(|err| anyhow!("{:?}", err))?;

    Ok(rule)
}

#[instrument]
pub async fn handle_limit_ip_access_by_countries(
    tenant_id: String,
    voting_countries: Vec<String>,
    enroll_countries: Vec<String>,
) -> Result<()> {
    let (zone_id, api_key) = get_cloudflare_vars().map_err(|err| anyhow!("{:?}", err))?;

    info!("zone id: {:?}, api_key: {:?}", &zone_id, &api_key);

    let ruleset = get_ruleset_by_phase(&api_key, &zone_id, WAF_RULESET_PHASE)
        .await
        .map_err(|err| anyhow!("{:?}", err))?;

    match ruleset {
        Some(ruleset) => {
            update_or_create_limit_ip_by_countries_rule(
                &api_key,
                &zone_id,
                &ruleset,
                tenant_id.clone(),
                voting_countries.clone(),
                false,
            )
            .await?;

            update_or_create_limit_ip_by_countries_rule(
                &api_key,
                &zone_id,
                &ruleset,
                tenant_id.clone(),
                enroll_countries.clone(),
                true,
            )
            .await?;
        }
        None => {
            create_limit_ip_by_countries_ruleset(
                &api_key,
                &zone_id,
                tenant_id.clone(),
                voting_countries.clone(),
                false,
                WAF_RULESET_PHASE,
            )
            .await?;

            create_limit_ip_by_countries_ruleset(
                &api_key,
                &zone_id,
                tenant_id.clone(),
                enroll_countries.clone(),
                true,
                WAF_RULESET_PHASE,
            )
            .await?;
        }
    }

    Ok(())
}
