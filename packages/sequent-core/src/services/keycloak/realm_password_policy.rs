// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::keycloak::{get_event_realm, KeycloakAdminClient};
use anyhow::{anyhow, bail, Result};
use rand::rngs::OsRng;
use rand::seq::SliceRandom;
use rand::Rng;
use serde::{Deserialize, Serialize};
use tracing::instrument;

pub const MIN_PASSWORD_LENGTH: i32 = 1;
pub const MAX_PASSWORD_LENGTH: i32 = 256;
pub const DEFAULT_MINIMUM_PASSWORD_LENGTH: i32 = 12;
pub const DEFAULT_MAXIMUM_PASSWORD_LENGTH: i32 = 72;

const POLICY_SEPARATOR: &str = " and ";
const LENGTH_POLICY: &str = "length";
const MAX_LENGTH_POLICY: &str = "maxLength";
const UPPERCASE_POLICY: &str = "upperCase";
const LOWERCASE_POLICY: &str = "lowerCase";
const DIGITS_POLICY: &str = "digits";
const SPECIAL_CHARACTERS_POLICY: &str = "specialChars";
const UPPERCASE_CHARACTERS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const LOWERCASE_CHARACTERS: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
const DIGIT_CHARACTERS: &[u8] = b"0123456789";
const SPECIAL_CHARACTERS: &[u8] = b"!@#$%^&*()-_=+[]{}:,.?";
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
    pub minimum_length: i32,
    pub maximum_length: i32,
    pub include_uppercase: bool,
    pub include_lowercase: bool,
    pub include_digits: bool,
    pub include_special_characters: bool,
}

/// Faithful, in-memory representation of the password rules that are actually
/// present in Keycloak. This type is intentionally separate from
/// `RealmPasswordPolicy`, which is the transient Admin Portal form model and
/// supplies display defaults for rules that are absent.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ParsedRealmPasswordPolicy {
    pub managed_rules_present: bool,
    pub minimum_length: Option<i32>,
    pub maximum_length: Option<i32>,
    pub required_uppercase: Option<i32>,
    pub required_lowercase: Option<i32>,
    pub required_digits: Option<i32>,
    pub required_special_characters: Option<i32>,
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

fn integer_policy_value(rule: &str) -> Option<i32> {
    policy_rule_value(rule).and_then(|value| value.parse::<i32>().ok())
}

fn is_managed_policy_rule(rule: &str) -> bool {
    MANAGED_POLICIES.contains(&policy_rule_name(rule))
}

impl ParsedRealmPasswordPolicy {
    pub fn from_keycloak_policy(password_policy: Option<&str>) -> Self {
        let Some(password_policy) = password_policy
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            return Self::default();
        };

        let mut parsed = Self::default();

        for rule in password_policy
            .split(POLICY_SEPARATOR)
            .map(str::trim)
            .filter(|rule| !rule.is_empty())
        {
            match policy_rule_name(rule) {
                LENGTH_POLICY => {
                    parsed.managed_rules_present = true;
                    parsed.minimum_length = integer_policy_value(rule);
                }
                MAX_LENGTH_POLICY => {
                    parsed.managed_rules_present = true;
                    parsed.maximum_length = integer_policy_value(rule);
                }
                UPPERCASE_POLICY => {
                    parsed.managed_rules_present = true;
                    parsed.required_uppercase = integer_policy_value(rule);
                }
                LOWERCASE_POLICY => {
                    parsed.managed_rules_present = true;
                    parsed.required_lowercase = integer_policy_value(rule);
                }
                DIGITS_POLICY => {
                    parsed.managed_rules_present = true;
                    parsed.required_digits = integer_policy_value(rule);
                }
                SPECIAL_CHARACTERS_POLICY => {
                    parsed.managed_rules_present = true;
                    parsed.required_special_characters =
                        integer_policy_value(rule);
                }
                _ => {}
            }
        }

        parsed
    }

    /// Convert the exact Keycloak constraints into the Admin Portal form DTO.
    /// Defaults here are presentation values only and are never used by manual
    /// password validation or VIL generation.
    pub fn to_admin_configuration(&self) -> RealmPasswordPolicy {
        if !self.managed_rules_present {
            return RealmPasswordPolicy::default();
        }

        RealmPasswordPolicy {
            configured: true,
            minimum_length: self
                .minimum_length
                .unwrap_or(DEFAULT_MINIMUM_PASSWORD_LENGTH),
            maximum_length: self
                .maximum_length
                .unwrap_or(DEFAULT_MAXIMUM_PASSWORD_LENGTH),
            include_uppercase: self
                .required_uppercase
                .is_some_and(|value| value > 0),
            include_lowercase: self
                .required_lowercase
                .is_some_and(|value| value > 0),
            include_digits: self.required_digits.is_some_and(|value| value > 0),
            include_special_characters: self
                .required_special_characters
                .is_some_and(|value| value > 0),
        }
    }
}

impl RealmPasswordPolicy {
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

impl ParsedRealmPasswordPolicy {
    fn required_character_sets(&self) -> Vec<(usize, &'static [u8])> {
        [
            (self.required_uppercase, UPPERCASE_CHARACTERS),
            (self.required_lowercase, LOWERCASE_CHARACTERS),
            (self.required_digits, DIGIT_CHARACTERS),
            (self.required_special_characters, SPECIAL_CHARACTERS),
        ]
        .into_iter()
        .filter_map(|(required, characters)| {
            required
                .filter(|value| *value > 0)
                .map(|value| (value as usize, characters))
        })
        .collect()
    }

    pub fn validate_for_generation(&self) -> Result<()> {
        if !self.managed_rules_present {
            bail!("Password policy is not configured");
        }

        let minimum_length = self.minimum_length.ok_or_else(|| {
            anyhow!(
                "Password policy must include a minimum length for generation"
            )
        })?;
        if !(MIN_PASSWORD_LENGTH..=MAX_PASSWORD_LENGTH)
            .contains(&minimum_length)
        {
            bail!(
                "Minimum password length must be between {MIN_PASSWORD_LENGTH} and {MAX_PASSWORD_LENGTH}"
            );
        }
        if let Some(maximum_length) = self.maximum_length {
            if !(MIN_PASSWORD_LENGTH..=MAX_PASSWORD_LENGTH)
                .contains(&maximum_length)
            {
                bail!(
                    "Maximum password length must be between {MIN_PASSWORD_LENGTH} and {MAX_PASSWORD_LENGTH}"
                );
            }
            if minimum_length > maximum_length {
                bail!(
                    "Minimum password length cannot exceed maximum password length"
                );
            }
        }

        let required_characters = self
            .required_character_sets()
            .iter()
            .map(|(required, _)| *required)
            .sum::<usize>();
        if required_characters == 0 {
            bail!("Password policy must include at least one character class");
        }
        if self.maximum_length.is_some_and(|maximum_length| {
            minimum_length.max(required_characters as i32) > maximum_length
        }) {
            bail!(
                "Maximum password length is too small for the required character classes"
            );
        }

        Ok(())
    }

    pub fn validate_password(&self, password: &str) -> Result<()> {
        let length = password.chars().count() as i32;
        if self.minimum_length.is_some_and(|minimum_length| {
            minimum_length > 0 && length < minimum_length
        }) {
            bail!("Password is shorter than the realm policy minimum");
        }
        if self.maximum_length.is_some_and(|maximum_length| {
            maximum_length > 0 && length > maximum_length
        }) {
            bail!("Password is longer than the realm policy maximum");
        }

        let uppercase = password
            .chars()
            .filter(|value| value.is_ascii_uppercase())
            .count() as i32;
        if self
            .required_uppercase
            .is_some_and(|required| required > 0 && uppercase < required)
        {
            bail!("Password does not contain enough uppercase characters");
        }
        let lowercase = password
            .chars()
            .filter(|value| value.is_ascii_lowercase())
            .count() as i32;
        if self
            .required_lowercase
            .is_some_and(|required| required > 0 && lowercase < required)
        {
            bail!("Password does not contain enough lowercase characters");
        }
        let digits = password
            .chars()
            .filter(|value| value.is_ascii_digit())
            .count() as i32;
        if self
            .required_digits
            .is_some_and(|required| required > 0 && digits < required)
        {
            bail!("Password does not contain enough digits");
        }
        let special_characters = password
            .chars()
            .filter(|value| !value.is_ascii_alphanumeric())
            .count() as i32;
        if self.required_special_characters.is_some_and(|required| {
            required > 0 && special_characters < required
        }) {
            bail!("Password does not contain enough special characters");
        }

        Ok(())
    }

    pub fn generate_password(&self) -> Result<String> {
        self.validate_for_generation()?;
        let mut rng = OsRng;
        let required_sets = self.required_character_sets();

        let required_characters = required_sets
            .iter()
            .map(|(required, _)| *required)
            .sum::<usize>();
        let target_length =
            self.minimum_length
                .expect("validated minimum length")
                .max(required_characters as i32) as usize;
        let all_characters = required_sets
            .iter()
            .flat_map(|(_, characters)| characters.iter().copied())
            .collect::<Vec<_>>();
        let mut password = Vec::with_capacity(target_length);
        for (required, characters) in required_sets {
            for _ in 0..required {
                password.push(characters[rng.gen_range(0..characters.len())]);
            }
        }

        while password.len() < target_length {
            password
                .push(all_characters[rng.gen_range(0..all_characters.len())]);
        }
        password.shuffle(&mut rng);

        String::from_utf8(password).map_err(|error| anyhow!(error))
    }
}

pub async fn get_realm_password_policy(
    tenant_id: &str,
    election_event_id: &str,
) -> Result<ParsedRealmPasswordPolicy> {
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
    ) -> Result<ParsedRealmPasswordPolicy> {
        let current_realm = self
            .client
            .realm_get(realm)
            .await
            .map_err(|error| anyhow!("{error:?}"))?;

        Ok(ParsedRealmPasswordPolicy::from_keycloak_policy(
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
        ParsedRealmPasswordPolicy, RealmPasswordPolicy,
        DEFAULT_MAXIMUM_PASSWORD_LENGTH, DEFAULT_MINIMUM_PASSWORD_LENGTH,
    };

    #[test]
    fn missing_policy_uses_safe_form_defaults_without_marking_it_configured() {
        let parsed = ParsedRealmPasswordPolicy::from_keycloak_policy(None);
        let policy = parsed.to_admin_configuration();

        assert!(!parsed.managed_rules_present);
        assert_eq!(None, parsed.minimum_length);
        assert_eq!(None, parsed.maximum_length);
        assert!(!policy.configured);
        assert_eq!(DEFAULT_MINIMUM_PASSWORD_LENGTH, policy.minimum_length);
        assert_eq!(DEFAULT_MAXIMUM_PASSWORD_LENGTH, policy.maximum_length);
        assert!(policy.include_uppercase);
        assert!(policy.include_lowercase);
        assert!(policy.include_digits);
        assert!(policy.include_special_characters);
    }

    #[test]
    fn parses_exact_values_for_rules_managed_by_the_admin_portal() {
        let parsed = ParsedRealmPasswordPolicy::from_keycloak_policy(Some(
            "hashIterations(27500) and length(16) and digits(3) and maxLength(96) and specialChars(2)",
        ));
        let policy = parsed.to_admin_configuration();

        assert!(parsed.managed_rules_present);
        assert_eq!(Some(16), parsed.minimum_length);
        assert_eq!(Some(96), parsed.maximum_length);
        assert_eq!(Some(3), parsed.required_digits);
        assert_eq!(Some(2), parsed.required_special_characters);
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

    #[test]
    fn unconfigured_policy_does_not_reject_a_manual_password() {
        let policy = ParsedRealmPasswordPolicy::from_keycloak_policy(None);

        policy.validate_password("Abc1").unwrap();
    }

    #[test]
    fn unmanaged_only_policy_does_not_reject_a_manual_password() {
        let policy = ParsedRealmPasswordPolicy::from_keycloak_policy(Some(
            "hashIterations(27500) and notUsername(undefined)",
        ));

        assert!(!policy.managed_rules_present);
        policy.validate_password("Abc1").unwrap();
    }

    #[test]
    fn absent_length_rules_are_not_replaced_with_form_defaults() {
        let policy = ParsedRealmPasswordPolicy::from_keycloak_policy(Some(
            "upperCase(1) and lowerCase(1) and digits(1)",
        ));

        assert_eq!(None, policy.minimum_length);
        assert_eq!(None, policy.maximum_length);
        policy.validate_password("Abc1").unwrap();
        assert!(policy.validate_password("abc1").is_err());
    }

    #[test]
    fn manual_validation_enforces_only_the_explicit_length_rules() {
        let policy =
            ParsedRealmPasswordPolicy::from_keycloak_policy(Some("length(8)"));

        policy.validate_password("12345678").unwrap();
        assert!(policy.validate_password("1234567").is_err());
    }

    #[test]
    fn manual_validation_preserves_exact_character_counts() {
        let policy = ParsedRealmPasswordPolicy::from_keycloak_policy(Some(
            "upperCase(2) and digits(3)",
        ));

        policy.validate_password("AB123").unwrap();
        assert!(policy.validate_password("Ab123").is_err());
        assert!(policy.validate_password("ABc1").is_err());
    }

    #[test]
    fn validates_and_generates_passwords_for_the_exact_managed_policy() {
        let policy = ParsedRealmPasswordPolicy::from_keycloak_policy(Some(
            "length(24) and maxLength(32) and upperCase(2) and lowerCase(2) and digits(3) and specialChars(2)",
        ));

        for _ in 0..64 {
            let password = policy.generate_password().unwrap();
            assert_eq!(24, password.chars().count());
            policy.validate_password(&password).unwrap();
        }
    }

    #[test]
    fn validates_manually_entered_passwords_against_every_managed_rule() {
        let policy = ParsedRealmPasswordPolicy::from_keycloak_policy(Some(
            "length(8) and maxLength(16) and upperCase(1) and lowerCase(1) and digits(1) and specialChars(1)",
        ));

        policy.validate_password("Abcdef1!").unwrap();
        assert!(policy.validate_password("Ab1!").is_err());
        assert!(policy.validate_password("abcdef1!").is_err());
        assert!(policy.validate_password("ABCDEF1!").is_err());
        assert!(policy.validate_password("Abcdefg!").is_err());
        assert!(policy.validate_password("Abcdef12").is_err());
        assert!(policy.validate_password("Abcdefghijklmnop1!").is_err());
    }

    #[test]
    fn generation_requires_an_explicit_minimum_and_usable_character_set() {
        let missing = ParsedRealmPasswordPolicy::from_keycloak_policy(None);
        assert!(missing.generate_password().is_err());

        let unmanaged = ParsedRealmPasswordPolicy::from_keycloak_policy(Some(
            "hashIterations(27500)",
        ));
        assert!(unmanaged.generate_password().is_err());

        let length_only =
            ParsedRealmPasswordPolicy::from_keycloak_policy(Some("length(12)"));
        assert!(length_only.generate_password().is_err());

        let character_class_only =
            ParsedRealmPasswordPolicy::from_keycloak_policy(Some("digits(1)"));
        assert!(character_class_only.generate_password().is_err());
    }
}
