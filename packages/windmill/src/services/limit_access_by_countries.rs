// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::cloudflare::{
    create_ruleset, create_ruleset_rule, get_cloudflare_vars, get_ruleset_by_phase,
    update_ruleset_rule, CreateCustomRuleRequest, Rule, Ruleset, WAF_RULESET_PHASE,
};
use anyhow::Result;
use rocket::{form::validate::Contains, http::Status};
use std::error::Error;
use tracing::instrument;

#[instrument]
fn get_voting_portal_url() -> Result<String, Box<dyn Error>> {
    let voting_portal_url = std::env::var("VOTING_PORTAL_URL")
        .map_err(|_e| "Missing voting portal env variable".to_string())?;
    Ok(voting_portal_url)
}

#[instrument]
fn create_limit_ip_by_countries_rule(
    tenant_id: String,
    countries: Vec<String>,
) -> CreateCustomRuleRequest {
    let voting_portal_base_url = get_voting_portal_url();

    let countries_str = countries
        .iter()
        .map(|country| format!("\"{}\"", country))
        .collect::<Vec<_>>()
        .join(", ");

    //TODO: check
    let rule_expression = format!(
            "((http.request.uri.full starts_with \"{:?}\"  or http.request.uri.path contain \"/registration?client_id=voting-portal\") and http.request.uri.path contains \"{}\") and (ip.geoip.country in {{{}}})",
            voting_portal_base_url,tenant_id, countries_str
        );

    CreateCustomRuleRequest {
        action: "block".to_string(),
        description: format!(
            "Block access in tenant {} from countries: {}",
            tenant_id,
            countries.join(",")
        )
        .to_string(),
        expression: rule_expression,
    }
}

#[instrument]
async fn update_or_create_limit_ip_by_countries_rule(
    api_key: &str,
    zone_id: &str,
    ruleset: &Ruleset,
    tenant_id: String,
    countries: Vec<String>,
) -> Result<CreateCustomRuleRequest, (Status, String)> {
    let existing_rules: Vec<Rule> = ruleset.rules.clone();
    let ruleset_id = ruleset.id.clone();
    let rule = create_limit_ip_by_countries_rule(tenant_id.clone(), countries.clone());

    let rule_id = existing_rules
        .iter()
        .find(|rule| rule.expression.contains(tenant_id.as_str()))
        .and_then(|rule| rule.id.clone());

    match rule_id {
        Some(id) => update_ruleset_rule(&api_key, &zone_id, &ruleset_id, &id, rule.clone())
            .await
            .map_err(|err| (Status::InternalServerError, format!("{:?}", err)))?,
        None => create_ruleset_rule(&api_key, &zone_id, &ruleset_id, rule.clone())
            .await
            .map_err(|err| (Status::InternalServerError, format!("{:?}", err)))?,
    };

    Ok(rule)
}

#[instrument]
async fn create_limit_ip_by_countries_ruleset(
    api_key: &str,
    zone_id: &str,
    tenant_id: String,
    countries: Vec<String>,
    ruleset_phase: &str,
) -> Result<CreateCustomRuleRequest, (Status, String)> {
    let rule: CreateCustomRuleRequest =
        create_limit_ip_by_countries_rule(tenant_id.clone(), countries.clone());

    create_ruleset(&api_key, &zone_id, ruleset_phase, rule.clone())
        .await
        .map_err(|err| (Status::InternalServerError, format!("{:?}", err)))?;

    Ok(rule)
}

#[instrument]
pub async fn handle_limit_ip_access_by_countries(
    tenant_id: String,
    countries: Vec<String>,
) -> Result<(), (Status, String)> {
    let (zone_id, api_key) =
        get_cloudflare_vars().map_err(|err| (Status::InternalServerError, format!("{:?}", err)))?;

    let ruleset = get_ruleset_by_phase(&api_key, &zone_id, WAF_RULESET_PHASE)
        .await
        .map_err(|err| (Status::InternalServerError, format!("{:?}", err)))
        .unwrap();

    match ruleset {
        Some(ruleset) => update_or_create_limit_ip_by_countries_rule(
            &api_key,
            &zone_id,
            &ruleset,
            tenant_id.clone(),
            countries,
        )
        .await
        .map_err(|err| (Status::InternalServerError, format!("{:?}", err)))?,
        None => create_limit_ip_by_countries_ruleset(
            &api_key,
            &zone_id,
            tenant_id,
            countries,
            WAF_RULESET_PHASE,
        )
        .await
        .map_err(|err| (Status::InternalServerError, format!("{:?}", err)))?,
    };

    Ok(())
}
