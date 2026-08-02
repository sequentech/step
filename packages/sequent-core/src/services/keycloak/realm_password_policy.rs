// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::keycloak::{get_event_realm, KeycloakAdminClient};
use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};
use tracing::instrument;

pub const MIN_PASSWORD_LENGTH: u16 = 1;
pub const MAX_PASSWORD_LENGTH: u16 = 256;
pub const DEFAULT_MINIMUM_PASSWORD_LENGTH: u16 = 12;
pub const DEFAULT_MAXIMUM_PASSWORD_LENGTH: u16 = 72;

const POLICY_SEPARATOR: &str = " and ";
const LENGTH_POLICY: &str = "length";
const MAX_LENGTH_POLICY: &str = "maxLength";
const UPPERCASE_POLICY: &str = "upperCase";
const LOWERCASE_POLICY: &str = "lowerCase";
const DIGITS_POLICY: &str = "digits";
const SPECIAL_CHARACTERS_POLICY: &str = "specialChars";
const MANAGED_POLICIES: [&str; 6] = [
    LENGTH_POLICY,
    MAX_LENGTH_POLICY,
    UPPERCASE_POLICY,
    LOWERCASE_POLICY,
    DIGITS_POLICY,
    SPECIAL_CHARACTERS_POLICY,
];

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct RealmPasswordPolicy {
    pub configured: bool,
    pub minimum_length: u16,
    pub maximum_length: u16,
    pub include_uppercase: bool,
    pub include_lowercase: bool,
    pub include_digits: bool,
    pub include_special_characters: bool,
}

impl Default for RealmPasswordPolicy {
    fn default() -> Self {
        Self {
            configured: false,
            minimum_length: DEFAULT_MINIMUM_PASSWORD_LENGTH,
            maximum_length: DEFAULT_MAXIMUM_PASSWORD_LENGTH,
            include_uppercase: true,
            include_lowercase: true,
            include_digits: true,
            include_special_characters: true,
        }
    }
}

fn policy_rule_name(rule: &str) -> &str {
    rule.split_once('(')
        .map(|(name, _)| name.trim())
        .unwrap_or_else(|| rule.trim())
}

fn policy_rule_value(rule: &str) -> Option<&str> {
    let (_, value) = rule.split_once('(')?;
    value.strip_suffix(')').map(str::trim)
}

fn positive_policy_value(rule: &str) -> bool {
    policy_rule_value(rule)
        .and_then(|value| value.parse::<u16>().ok())
        .is_some_and(|value| value > 0)
}

fn is_managed_policy_rule(rule: &str) -> bool {
    MANAGED_POLICIES.contains(&policy_rule_name(rule))
}

impl RealmPasswordPolicy {
    pub fn from_keycloak_policy(password_policy: Option<&str>) -> Self {
        let Some(password_policy) = password_policy
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            return Self::default();
        };

        let mut parsed = Self {
            configured: true,
            include_uppercase: false,
            include_lowercase: false,
            include_digits: false,
            include_special_characters: false,
            ..Self::default()
        };

        for rule in password_policy
            .split(POLICY_SEPARATOR)
            .map(str::trim)
            .filter(|rule| !rule.is_empty())
        {
            match policy_rule_name(rule) {
                LENGTH_POLICY => {
                    if let Some(value) = policy_rule_value(rule)
                        .and_then(|value| value.parse().ok())
                    {
                        parsed.minimum_length = value;
                    }
                }
                MAX_LENGTH_POLICY => {
                    if let Some(value) = policy_rule_value(rule)
                        .and_then(|value| value.parse().ok())
                    {
                        parsed.maximum_length = value;
                    }
                }
                UPPERCASE_POLICY => {
                    parsed.include_uppercase = positive_policy_value(rule);
                }
                LOWERCASE_POLICY => {
                    parsed.include_lowercase = positive_policy_value(rule);
                }
                DIGITS_POLICY => {
                    parsed.include_digits = positive_policy_value(rule);
                }
                SPECIAL_CHARACTERS_POLICY => {
                    parsed.include_special_characters =
                        positive_policy_value(rule);
                }
                _ => {}
            }
        }

        parsed
    }

    pub fn merge_into_keycloak_policy(
        &self,
        current_password_policy: Option<&str>,
    ) -> Result<String> {
        self.validate()?;

        let mut rules = current_password_policy
            .unwrap_or_default()
            .split(POLICY_SEPARATOR)
            .map(str::trim)
            .filter(|rule| !rule.is_empty() && !is_managed_policy_rule(rule))
            .map(str::to_string)
            .collect::<Vec<_>>();

        rules.push(format!("{LENGTH_POLICY}({})", self.minimum_length));
        rules.push(format!("{MAX_LENGTH_POLICY}({})", self.maximum_length));
        if self.include_uppercase {
            rules.push(format!("{UPPERCASE_POLICY}(1)"));
        }
        if self.include_lowercase {
            rules.push(format!("{LOWERCASE_POLICY}(1)"));
        }
        if self.include_digits {
            rules.push(format!("{DIGITS_POLICY}(1)"));
        }
        if self.include_special_characters {
            rules.push(format!("{SPECIAL_CHARACTERS_POLICY}(1)"));
        }

        Ok(rules.join(POLICY_SEPARATOR))
    }

    pub fn validate(&self) -> Result<()> {
        if !(MIN_PASSWORD_LENGTH..=MAX_PASSWORD_LENGTH)
            .contains(&self.minimum_length)
        {
            bail!(
                "Minimum password length must be between {MIN_PASSWORD_LENGTH} and {MAX_PASSWORD_LENGTH}"
            );
        }
        if !(MIN_PASSWORD_LENGTH..=MAX_PASSWORD_LENGTH)
            .contains(&self.maximum_length)
        {
            bail!(
                "Maximum password length must be between {MIN_PASSWORD_LENGTH} and {MAX_PASSWORD_LENGTH}"
            );
        }
        if self.minimum_length > self.maximum_length {
            bail!(
                "Minimum password length cannot exceed maximum password length"
            );
        }

        Ok(())
    }
}

pub async fn get_realm_password_policy(
    tenant_id: &str,
    election_event_id: &str,
) -> Result<RealmPasswordPolicy> {
    KeycloakAdminClient::new()
        .await
        .map_err(|error| {
            anyhow!("Error creating Keycloak admin client: {error:?}")
        })?
        .get_realm_password_policy(&get_event_realm(
            tenant_id,
            election_event_id,
        ))
        .await
        .map_err(|error| {
            anyhow!("Error getting Keycloak realm password policy: {error:?}")
        })
}

pub async fn update_realm_password_policy(
    tenant_id: &str,
    election_event_id: &str,
    password_policy: RealmPasswordPolicy,
) -> Result<()> {
    KeycloakAdminClient::new()
        .await
        .map_err(|error| {
            anyhow!("Error creating Keycloak admin client: {error:?}")
        })?
        .update_realm_password_policy(
            &get_event_realm(tenant_id, election_event_id),
            password_policy,
        )
        .await
        .map_err(|error| {
            anyhow!("Error updating Keycloak realm password policy: {error:?}")
        })
}

impl KeycloakAdminClient {
    #[instrument(skip(self), err)]
    pub async fn get_realm_password_policy(
        self,
        realm: &str,
    ) -> Result<RealmPasswordPolicy> {
        let current_realm = self
            .client
            .realm_get(realm)
            .await
            .map_err(|error| anyhow!("{error:?}"))?;

        Ok(RealmPasswordPolicy::from_keycloak_policy(
            current_realm.password_policy.as_deref(),
        ))
    }

    #[instrument(skip(self, password_policy), err)]
    pub async fn update_realm_password_policy(
        self,
        realm: &str,
        password_policy: RealmPasswordPolicy,
    ) -> Result<()> {
        password_policy.validate()?;

        let mut current_realm = self
            .client
            .realm_get(realm)
            .await
            .map_err(|error| anyhow!("{error:?}"))?;
        let merged_policy = password_policy.merge_into_keycloak_policy(
            current_realm.password_policy.as_deref(),
        )?;
        current_realm.password_policy = Some(merged_policy);

        self.client
            .realm_put(realm, current_realm)
            .await
            .map_err(|error| anyhow!("{error:?}"))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        RealmPasswordPolicy, DEFAULT_MAXIMUM_PASSWORD_LENGTH,
        DEFAULT_MINIMUM_PASSWORD_LENGTH,
    };

    #[test]
    fn missing_policy_uses_safe_form_defaults_without_marking_it_configured() {
        let policy = RealmPasswordPolicy::from_keycloak_policy(None);

        assert!(!policy.configured);
        assert_eq!(DEFAULT_MINIMUM_PASSWORD_LENGTH, policy.minimum_length);
        assert_eq!(DEFAULT_MAXIMUM_PASSWORD_LENGTH, policy.maximum_length);
        assert!(policy.include_uppercase);
        assert!(policy.include_lowercase);
        assert!(policy.include_digits);
        assert!(policy.include_special_characters);
    }

    #[test]
    fn parses_the_rules_managed_by_the_admin_portal() {
        let policy = RealmPasswordPolicy::from_keycloak_policy(Some(
            "hashIterations(27500) and length(16) and digits(1) and maxLength(96) and specialChars(1)",
        ));

        assert!(policy.configured);
        assert_eq!(16, policy.minimum_length);
        assert_eq!(96, policy.maximum_length);
        assert!(!policy.include_uppercase);
        assert!(!policy.include_lowercase);
        assert!(policy.include_digits);
        assert!(policy.include_special_characters);
    }

    #[test]
    fn merges_managed_rules_without_removing_unrelated_keycloak_rules() {
        let policy = RealmPasswordPolicy {
            configured: true,
            minimum_length: 14,
            maximum_length: 80,
            include_uppercase: true,
            include_lowercase: false,
            include_digits: true,
            include_special_characters: false,
        };

        let merged = policy
            .merge_into_keycloak_policy(Some(
                "length(8) and lowerCase(1) and hashIterations(27500) and notUsername(undefined)",
            ))
            .unwrap();

        assert_eq!(
            "hashIterations(27500) and notUsername(undefined) and length(14) and maxLength(80) and upperCase(1) and digits(1)",
            merged
        );
    }

    #[test]
    fn rejects_invalid_length_ranges() {
        let mut policy = RealmPasswordPolicy::default();
        policy.minimum_length = 0;
        assert!(policy.validate().is_err());

        policy.minimum_length = 100;
        policy.maximum_length = 50;
        assert!(policy.validate().is_err());

        policy.minimum_length = 12;
        policy.maximum_length = 257;
        assert!(policy.validate().is_err());
    }
}
