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
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use tracing::{info, instrument};

const COUNTRY_CODE_LENGTH: usize = 2;

/// Two-character country code, as used by Cloudflare's `ip.geoip.country`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct CountryCode(String);

impl FromStr for CountryCode {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        let is_valid = value.len() == COUNTRY_CODE_LENGTH
            && value
                .chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit());
        if is_valid {
            Ok(CountryCode(value.to_string()))
        } else {
            Err(anyhow!("Invalid country code {value:?}"))
        }
    }
}

impl TryFrom<String> for CountryCode {
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self> {
        value.parse()
    }
}

impl fmt::Display for CountryCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Cloudflare expression that matches a request from any of the countries.
fn countries_expression(countries: &[CountryCode]) -> String {
    let codes = countries
        .iter()
        .map(|country| format!("\"{country}\""))
        .collect::<Vec<_>>()
        .join(" ");
    format!("ip.geoip.country in {{{codes}}}")
}

#[instrument]
pub(crate) fn get_voting_portal_urls_prefix() -> Result<(String, String)> {
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
    countries: &[CountryCode],
    is_enrollment: bool,
) -> Result<CreateCustomRuleRequest> {
    let (voting_portal_url, voting_portal_keycloak_url) = get_voting_portal_urls_prefix()?;

    let countries_expression = countries_expression(countries);

    let keycloak_rule_expression_voting = format!(
        "http.request.full_uri contains \"{}\" and http.request.uri.query contains \"voting-portal\"",
        voting_portal_keycloak_url
    );

    let login_registration_rule_expression = format!(
        "ends_with(http.request.uri.path, \"/protocol/openid-connect/registrations\")
        or ends_with(http.request.uri.path, \"/login-actions/registration\")"
    );

    let rule_expression_enroll = format!(
        "starts_with(http.request.uri.path, \"/realms/tenant-{}-event-\") and ends_with(http.request.uri.path, \"/protocol/openid-connect/registrations\") and http.request.uri.query contains \"client_id=voting-portal\"",
        tenant_id
    );

    let rule_expression_voting = format!(
        "(http.request.full_uri contains \"{}\" or ({})) and (http.request.uri.path contains \"{}\") and ({}) and ({})",
        voting_portal_url, keycloak_rule_expression_voting, tenant_id, countries_expression, login_registration_rule_expression
    );

    Ok(CreateCustomRuleRequest {
        action: "block".to_string(),
        description: format!(
            "{} {}",
            rule_description_prefix(&tenant_id),
            countries
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(",")
        ),
        expression: if is_enrollment {
            rule_expression_enroll
        } else {
            rule_expression_voting
        },
    })
}

/// Start of the description of every country rule written for the tenant.
fn rule_description_prefix(tenant_id: &str) -> String {
    format!("Block access in tenant {tenant_id} from countries:")
}

/// Id of the tenant's existing rule, found through the description written
/// when the rule was created.
fn find_tenant_rule_id(rules: &[Rule], tenant_id: &str, is_enrollment: bool) -> Option<String> {
    let description_prefix = rule_description_prefix(tenant_id);
    rules
        .iter()
        .find(|rule| {
            rule.description
                .as_deref()
                .is_some_and(|description| description.starts_with(&description_prefix))
                && rule.expression.contains(if is_enrollment {
                    "enroll"
                } else {
                    "voting-portal"
                })
        })
        .and_then(|rule| rule.id.clone())
}

#[instrument]
async fn update_or_create_limit_ip_by_countries_rule(
    api_key: &str,
    zone_id: &str,
    ruleset: &Ruleset,
    tenant_id: String,
    countries: Vec<CountryCode>,
    is_enrollment: bool,
) -> Result<CreateCustomRuleRequest> {
    let existing_rules: Vec<Rule> = ruleset.rules.clone();
    let ruleset_id = ruleset.id.clone();
    let rule: CreateCustomRuleRequest =
        create_limit_ip_by_countries_rule_format(tenant_id.clone(), &countries, is_enrollment)?;

    let rule_id = find_tenant_rule_id(&existing_rules, &tenant_id, is_enrollment);

    match rule_id {
        Some(id) => match countries.len() {
            0 => {
                delete_ruleset_rule(&api_key, &zone_id, &ruleset_id, &id)
                    .await
                    .map_err(|err| anyhow!("{:?}", err))?;
            }
            _ => update_ruleset_rule(&api_key, &zone_id, &ruleset_id, &id, rule.clone())
                .await
                .map_err(|err| anyhow!("{:?}", err))?,
        },
        None => match countries.len() {
            0 => (),
            _ => create_ruleset_rule(&api_key, &zone_id, &ruleset_id, rule.clone())
                .await
                .map_err(|err| anyhow!("{:?}", err))?,
        },
    };

    Ok(rule)
}

#[instrument]
async fn create_limit_ip_by_countries_ruleset(
    api_key: &str,
    zone_id: &str,
    tenant_id: String,
    countries: Vec<CountryCode>,
    is_enrollment: bool,
    ruleset_phase: &str,
) -> Result<CreateCustomRuleRequest> {
    let rule: CreateCustomRuleRequest =
        create_limit_ip_by_countries_rule_format(tenant_id.clone(), &countries, is_enrollment)?;

    create_ruleset(&api_key, &zone_id, ruleset_phase, rule.clone())
        .await
        .map_err(|err| anyhow!("{:?}", err))?;

    Ok(rule)
}

#[instrument]
pub async fn handle_limit_ip_access_by_countries(
    tenant_id: String,
    voting_countries: Vec<CountryCode>,
    enroll_countries: Vec<CountryCode>,
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A country value with a quote is not a country code, so it never reaches an expression.
    #[test]
    fn country_rule_rejects_country_with_quote() {
        let value = "U\"";
        assert!(value.parse::<CountryCode>().is_err());
        assert!(
            serde_json::from_value::<Vec<CountryCode>>(serde_json::json!(["US", value])).is_err()
        );
    }

    /// Country codes are exactly two uppercase letters or digits.
    #[test]
    fn country_code_rejects_anything_but_two_characters() {
        for value in ["", "U", "USA", "us", "\"US\"", " US", "U ", "ÜS"] {
            assert!(value.parse::<CountryCode>().is_err(), "{value:?}");
        }
        for value in ["US", "FR", "T1", "XX"] {
            assert_eq!(
                value
                    .parse::<CountryCode>()
                    .ok()
                    .map(|code| code.to_string()),
                Some(value.to_string())
            );
        }
    }

    /// A tenant only finds the rules described for itself, even if another rule mentions its id.
    #[test]
    fn rule_lookup_ignores_rules_described_for_other_tenants() {
        let tenant_id = "90505c8a-23a9-4cdf-a26b-4e19f6a097d5";
        let other_tenant_id = "5e6a0c0f-2a4b-4a6e-9d7e-3f0f2c1b8a90";
        let rule = |id: &str, owner: &str, expression: String| Rule {
            id: Some(id.to_string()),
            expression,
            description: Some(format!("Block access in tenant {owner} from countries: FR")),
            enabled: Some(true),
            action: "block".to_string(),
            action_parameters: None,
        };
        let other_rule = rule(
            "other",
            other_tenant_id,
            format!(
                "(http.request.uri.query contains \"voting-portal\") and (http.request.uri.path contains \"{other_tenant_id}\" or http.request.uri.path contains \"{tenant_id}\")"
            ),
        );
        let own_rule = rule(
            "own",
            tenant_id,
            format!(
                "(http.request.uri.query contains \"voting-portal\") and (http.request.uri.path contains \"{tenant_id}\")"
            ),
        );

        assert_eq!(
            find_tenant_rule_id(&[other_rule.clone()], tenant_id, false),
            None
        );
        assert_eq!(
            find_tenant_rule_id(&[other_rule.clone(), own_rule], tenant_id, false),
            Some("own".to_string())
        );
        assert_eq!(
            find_tenant_rule_id(&[other_rule], other_tenant_id, false),
            Some("other".to_string())
        );
    }

    /// The expression uses the `in` set syntax with one entry per country.
    #[test]
    fn countries_expression_lists_codes_as_a_set() {
        let countries: Vec<CountryCode> =
            serde_json::from_value(serde_json::json!(["US", "FR"])).expect("valid country codes");
        assert_eq!(
            countries_expression(&countries),
            "ip.geoip.country in {\"US\" \"FR\"}"
        );
    }
}
