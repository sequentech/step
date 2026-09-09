// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::keycloak::{get_event_realm, KeycloakAdminClient};
use anyhow::{anyhow, bail, Result};
use rand::rngs::OsRng;
use rand::seq::SliceRandom;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PasswordPolicyRule {
    MinimumLength,
    MaximumLength,
    Uppercase,
    Lowercase,
    Digits,
    SpecialCharacters,
}

impl PasswordPolicyRule {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MinimumLength => "minimumLength",
            Self::MaximumLength => "maximumLength",
            Self::Uppercase => "uppercase",
            Self::Lowercase => "lowercase",
            Self::Digits => "digits",
            Self::SpecialCharacters => "specialCharacters",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PasswordPolicyViolation {
    pub rule: PasswordPolicyRule,
    pub required_count: i32,
}

impl Display for PasswordPolicyViolation {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let message = match self.rule {
            PasswordPolicyRule::MinimumLength => {
                "Password is shorter than the realm policy minimum"
            }
            PasswordPolicyRule::MaximumLength => {
                "Password is longer than the realm policy maximum"
            }
            PasswordPolicyRule::Uppercase => {
                "Password does not contain enough uppercase characters"
            }
            PasswordPolicyRule::Lowercase => {
                "Password does not contain enough lowercase characters"
            }
            PasswordPolicyRule::Digits => {
                "Password does not contain enough digits"
            }
            PasswordPolicyRule::SpecialCharacters => {
                "Password does not contain enough special characters"
            }
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for PasswordPolicyViolation {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PasswordPolicyGenerationError {
    NotConfigured,
    MinimumLengthMissing,
    MinimumLengthOutOfRange,
    MaximumLengthOutOfRange,
    MinimumExceedsMaximum,
    CharacterClassMissing,
    MaximumTooSmallForRequiredCharacters,
}

impl Display for PasswordPolicyGenerationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotConfigured => formatter.write_str("Password policy is not configured"),
            Self::MinimumLengthMissing => formatter.write_str(
                "Password policy must include a minimum length for generation",
            ),
            Self::MinimumLengthOutOfRange => write!(
                formatter,
                "Minimum password length must be between {MIN_PASSWORD_LENGTH} and {MAX_PASSWORD_LENGTH}"
            ),
            Self::MaximumLengthOutOfRange => write!(
                formatter,
                "Maximum password length must be between {MIN_PASSWORD_LENGTH} and {MAX_PASSWORD_LENGTH}"
            ),
            Self::MinimumExceedsMaximum => formatter.write_str(
                "Minimum password length cannot exceed maximum password length",
            ),
            Self::CharacterClassMissing => formatter.write_str(
                "Password policy must include at least one character class",
            ),
            Self::MaximumTooSmallForRequiredCharacters => formatter.write_str(
                "Maximum password length is too small for the required character classes",
            ),
        }
    }
}

impl std::error::Error for PasswordPolicyGenerationError {}

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

fn split_policy_rules(password_policy: &str) -> Vec<&str> {
    let bytes = password_policy.as_bytes();
    let separator = POLICY_SEPARATOR.as_bytes();
    let mut rules = Vec::new();
    let mut start = 0;
    let mut index = 0;
    let mut parenthesis_depth = 0_u32;

    while index < bytes.len() {
        match bytes[index] {
            b'\\' => {
                index = (index + 2).min(bytes.len());
                continue;
            }
            b'(' => parenthesis_depth += 1,
            b')' => parenthesis_depth = parenthesis_depth.saturating_sub(1),
            _ => {}
        }

        if parenthesis_depth == 0 && bytes[index..].starts_with(separator) {
            rules.push(password_policy[start..index].trim());
            index += separator.len();
            start = index;
            continue;
        }
        index += 1;
    }

    rules.push(password_policy[start..].trim());
    rules.into_iter().filter(|rule| !rule.is_empty()).collect()
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

        for rule in split_policy_rules(password_policy) {
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

        let current_policy = ParsedRealmPasswordPolicy::from_keycloak_policy(
            current_password_policy,
        );

        let mut rules =
            split_policy_rules(current_password_policy.unwrap_or_default())
                .into_iter()
                .filter(|rule| !is_managed_policy_rule(rule))
                .map(str::to_string)
                .collect::<Vec<_>>();

        rules.push(format!("{LENGTH_POLICY}({})", self.minimum_length));
        rules.push(format!("{MAX_LENGTH_POLICY}({})", self.maximum_length));
        if self.include_uppercase {
            rules.push(format!(
                "{UPPERCASE_POLICY}({})",
                current_policy
                    .required_uppercase
                    .filter(|value| *value > 0)
                    .unwrap_or(1)
            ));
        }
        if self.include_lowercase {
            rules.push(format!(
                "{LOWERCASE_POLICY}({})",
                current_policy
                    .required_lowercase
                    .filter(|value| *value > 0)
                    .unwrap_or(1)
            ));
        }
        if self.include_digits {
            rules.push(format!(
                "{DIGITS_POLICY}({})",
                current_policy
                    .required_digits
                    .filter(|value| *value > 0)
                    .unwrap_or(1)
            ));
        }
        if self.include_special_characters {
            rules.push(format!(
                "{SPECIAL_CHARACTERS_POLICY}({})",
                current_policy
                    .required_special_characters
                    .filter(|value| *value > 0)
                    .unwrap_or(1)
            ));
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
        if !self.include_uppercase
            && !self.include_lowercase
            && !self.include_digits
            && !self.include_special_characters
        {
            bail!("Password policy must include at least one character class");
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

    pub fn validate_for_generation(
        &self,
    ) -> std::result::Result<(), PasswordPolicyGenerationError> {
        if !self.managed_rules_present {
            return Err(PasswordPolicyGenerationError::NotConfigured);
        }

        let minimum_length = self
            .minimum_length
            .ok_or(PasswordPolicyGenerationError::MinimumLengthMissing)?;
        if !(MIN_PASSWORD_LENGTH..=MAX_PASSWORD_LENGTH)
            .contains(&minimum_length)
        {
            return Err(PasswordPolicyGenerationError::MinimumLengthOutOfRange);
        }
        if let Some(maximum_length) = self.maximum_length {
            if !(MIN_PASSWORD_LENGTH..=MAX_PASSWORD_LENGTH)
                .contains(&maximum_length)
            {
                return Err(
                    PasswordPolicyGenerationError::MaximumLengthOutOfRange,
                );
            }
            if minimum_length > maximum_length {
                return Err(
                    PasswordPolicyGenerationError::MinimumExceedsMaximum,
                );
            }
        }

        let required_characters = self
            .required_character_sets()
            .iter()
            .map(|(required, _)| *required)
            .sum::<usize>();
        if required_characters == 0 {
            return Err(PasswordPolicyGenerationError::CharacterClassMissing);
        }
        if self.maximum_length.is_some_and(|maximum_length| {
            minimum_length.max(required_characters as i32) > maximum_length
        }) {
            return Err(PasswordPolicyGenerationError::MaximumTooSmallForRequiredCharacters);
        }

        Ok(())
    }

    pub fn validate_password(
        &self,
        password: &str,
    ) -> std::result::Result<(), PasswordPolicyViolation> {
        let length = password.chars().count() as i32;
        if let Some(minimum_length) =
            self.minimum_length.filter(|minimum_length| {
                *minimum_length > 0 && length < *minimum_length
            })
        {
            return Err(PasswordPolicyViolation {
                rule: PasswordPolicyRule::MinimumLength,
                required_count: minimum_length,
            });
        }
        if let Some(maximum_length) =
            self.maximum_length.filter(|maximum_length| {
                *maximum_length > 0 && length > *maximum_length
            })
        {
            return Err(PasswordPolicyViolation {
                rule: PasswordPolicyRule::MaximumLength,
                required_count: maximum_length,
            });
        }

        let uppercase = password
            .chars()
            .filter(|value| value.is_ascii_uppercase())
            .count() as i32;
        if let Some(required) = self
            .required_uppercase
            .filter(|required| *required > 0 && uppercase < *required)
        {
            return Err(PasswordPolicyViolation {
                rule: PasswordPolicyRule::Uppercase,
                required_count: required,
            });
        }
        let lowercase = password
            .chars()
            .filter(|value| value.is_ascii_lowercase())
            .count() as i32;
        if let Some(required) = self
            .required_lowercase
            .filter(|required| *required > 0 && lowercase < *required)
        {
            return Err(PasswordPolicyViolation {
                rule: PasswordPolicyRule::Lowercase,
                required_count: required,
            });
        }
        let digits = password
            .chars()
            .filter(|value| value.is_ascii_digit())
            .count() as i32;
        if let Some(required) = self
            .required_digits
            .filter(|required| *required > 0 && digits < *required)
        {
            return Err(PasswordPolicyViolation {
                rule: PasswordPolicyRule::Digits,
                required_count: required,
            });
        }
        let special_characters = password
            .chars()
            .filter(|value| !value.is_ascii_alphanumeric())
            .count() as i32;
        if let Some(required) = self
            .required_special_characters
            .filter(|required| *required > 0 && special_characters < *required)
        {
            return Err(PasswordPolicyViolation {
                rule: PasswordPolicyRule::SpecialCharacters,
                required_count: required,
            });
        }

        Ok(())
    }

    pub fn generate_password(&self) -> Result<String> {
        self.validate_for_generation()
            .map_err(anyhow::Error::from)?;
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
        ParsedRealmPasswordPolicy, PasswordPolicyGenerationError,
        PasswordPolicyRule, RealmPasswordPolicy,
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
    fn round_trip_preserves_existing_character_class_counts() {
        let current = "digits(3) and upperCase(2)";
        let admin_configuration =
            ParsedRealmPasswordPolicy::from_keycloak_policy(Some(current))
                .to_admin_configuration();

        let merged = admin_configuration
            .merge_into_keycloak_policy(Some(current))
            .unwrap();
        let round_trip =
            ParsedRealmPasswordPolicy::from_keycloak_policy(Some(&merged));

        assert_eq!(Some(3), round_trip.required_digits);
        assert_eq!(Some(2), round_trip.required_uppercase);
    }

    #[test]
    fn preserves_unmanaged_rules_containing_the_policy_separator() {
        let policy = RealmPasswordPolicy {
            configured: true,
            minimum_length: 12,
            maximum_length: 72,
            include_uppercase: false,
            include_lowercase: false,
            include_digits: true,
            include_special_characters: false,
        };

        let merged = policy
            .merge_into_keycloak_policy(Some(
                "regexPattern(^foo and bar$) and hashIterations(27500) and length(8) and digits(2)",
            ))
            .unwrap();

        assert_eq!(
            "regexPattern(^foo and bar$) and hashIterations(27500) and length(12) and maxLength(72) and digits(2)",
            merged
        );
    }

    #[test]
    fn rejects_invalid_length_ranges() {
        let mut policy = RealmPasswordPolicy {
            minimum_length: 0,
            ..Default::default()
        };
        assert!(policy.validate().is_err());

        policy.minimum_length = 100;
        policy.maximum_length = 50;
        assert!(policy.validate().is_err());

        policy.minimum_length = 12;
        policy.maximum_length = 257;
        assert!(policy.validate().is_err());
    }

    #[test]
    fn rejects_admin_configuration_without_a_character_class() {
        let policy = RealmPasswordPolicy {
            include_uppercase: false,
            include_lowercase: false,
            include_digits: false,
            include_special_characters: false,
            ..RealmPasswordPolicy::default()
        };

        assert_eq!(
            "Password policy must include at least one character class",
            policy.validate().unwrap_err().to_string()
        );
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
        let violation = policy.validate_password("ABc1").unwrap_err();
        assert_eq!(PasswordPolicyRule::Digits, violation.rule);
        assert_eq!(3, violation.required_count);
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
        assert_eq!(
            PasswordPolicyGenerationError::NotConfigured,
            missing.validate_for_generation().unwrap_err()
        );

        let unmanaged = ParsedRealmPasswordPolicy::from_keycloak_policy(Some(
            "hashIterations(27500)",
        ));
        assert_eq!(
            PasswordPolicyGenerationError::NotConfigured,
            unmanaged.validate_for_generation().unwrap_err()
        );

        let length_only =
            ParsedRealmPasswordPolicy::from_keycloak_policy(Some("length(12)"));
        assert_eq!(
            PasswordPolicyGenerationError::CharacterClassMissing,
            length_only.validate_for_generation().unwrap_err()
        );

        let character_class_only =
            ParsedRealmPasswordPolicy::from_keycloak_policy(Some("digits(1)"));
        assert_eq!(
            PasswordPolicyGenerationError::MinimumLengthMissing,
            character_class_only.validate_for_generation().unwrap_err()
        );
    }
}
