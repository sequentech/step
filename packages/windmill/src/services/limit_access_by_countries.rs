// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::cloudflare::{
    create_ruleset, create_ruleset_rule, delete_ruleset_rule, get_cloudflare_vars,
    get_ruleset_by_phase, update_ruleset_rule, CreateCustomRuleRequest, Rule, Ruleset,
    WAF_RULESET_PHASE,
};
use anyhow::Result;
use rocket::{form::validate::Contains, http::Status};
use tracing::instrument;

#[instrument]
fn get_voting_portal_urls_prefix() -> (String, String) {
    //TODO: change default values?
    let voting_portal_url =
        std::env::var("VOTING_PORTAL_URL").unwrap_or("https://qa.sequent.vote".to_string());
    let voting_portal_keycloak_url = std::env::var("VOTING_PORTAL_KEYCLOCK_URL")
        .unwrap_or("https://keycloak-qa.sequent.vote".to_string());
    (voting_portal_url, voting_portal_keycloak_url)
}

#[instrument]
fn create_limit_ip_by_countries_rule_format(
    tenant_id: String,
    countries: Vec<String>,
) -> CreateCustomRuleRequest {
    let (voting_portal_url, voting_portal_keycloak_url) = get_voting_portal_urls_prefix();

    let countries_expression = countries
        .iter()
        .map(|country| format!("ip.geoip.country eq \"{}\"", country))
        .collect::<Vec<_>>()
        .join("or ");

    let keycloak_rule_expression = format!(
        "http.request.full_uri contains \"{}\" and http.request.uri.query contains \"voting-portal\"",
        voting_portal_keycloak_url
    );

    let rule_expression = format!(
        "(http.request.full_uri contains \"{}\" or ({})) and (http.request.uri.path contains \"{}\") and ({})",
        voting_portal_url, keycloak_rule_expression ,tenant_id, countries_expression
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
    let rule = create_limit_ip_by_countries_rule_format(tenant_id.clone(), countries.clone());

    let rule_id = existing_rules
        .iter()
        .find(|rule| rule.expression.contains(tenant_id.as_str()))
        .and_then(|rule| rule.id.clone());

    match rule_id {
        Some(id) => match countries.len() {
            0 => {
                delete_ruleset_rule(&api_key, &zone_id, &ruleset_id, &id)
                    .await
                    .map_err(|err| (Status::InternalServerError, format!("{:?}", err)))?;
            }
            _ => update_ruleset_rule(&api_key, &zone_id, &ruleset_id, &id, rule.clone())
                .await
                .map_err(|err| (Status::InternalServerError, format!("{:?}", err)))?,
        },
        None => match countries.len() {
            0 => (),
            _ => create_ruleset_rule(&api_key, &zone_id, &ruleset_id, rule.clone())
                .await
                .map_err(|err| (Status::InternalServerError, format!("{:?}", err)))?,
        },
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
        create_limit_ip_by_countries_rule_format(tenant_id.clone(), countries.clone());

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
