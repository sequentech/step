// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::ballot::VoterCertificatePolicy;
use crate::services::keycloak::{get_event_realm, KeycloakAdminClient};
use crate::types::keycloak::{
    CredentialInputPolicy, MAX_CREDENTIAL_PATTERN_GROUPS,
    MAX_CREDENTIAL_PATTERN_GROUP_SIZE, MAX_CREDENTIAL_PATTERN_TOTAL_SIZE,
    REALM_ATTR_CREDENTIAL_INPUT_PATTERN,
    REALM_ATTR_CREDENTIAL_INPUT_PLACEHOLDER,
    REALM_ATTR_CREDENTIAL_INPUT_POLICY, REALM_ATTR_SMARTLINK_CLOCK_SKEW_SECS,
    REALM_ATTR_SMARTLINK_ENABLED, REALM_ATTR_SMARTLINK_REQUIRED_ATTRIBUTES,
    REALM_ATTR_SMARTLINK_SHARED_SECRET, REALM_ATTR_SMARTLINK_TIMEOUT_SECS,
    REALM_ATTR_VOTER_CERTIFICATE_POLICY, SMARTLINK_REQUIRED_ATTRIBUTES_MAX_LEN,
    SMARTLINK_SHARED_SECRET_MAX_LEN,
};
use anyhow::{anyhow, bail, Result};
use std::collections::HashMap;
use std::str::FromStr;
use tracing::{info, instrument};

/// Placeholder returned instead of sensitive attribute values (secrets,
/// passwords, tokens) when reading realm attributes. Updates that send this
/// placeholder back for a sensitive key leave the stored value untouched, so
/// read-modify-write clients never see nor overwrite the real secret.
pub const REDACTED_ATTRIBUTE_VALUE: &str = "<redacted>";

pub async fn get_realm_attributes(
    tenant_id: &str,
    election_event_id: &str,
) -> Result<HashMap<String, String>> {
    KeycloakAdminClient::new()
        .await
        .map_err(|e| anyhow!("Error creating Keycloak admin client: {e:?}"))?
        .get_realm_attributes(&get_event_realm(tenant_id, election_event_id))
        .await
        .map_err(|e| anyhow!("Error getting Keycloak realm attributes: {e:?}"))
}

pub async fn update_realm_attributes(
    tenant_id: &str,
    election_event_id: &str,
    attributes: HashMap<String, String>,
) -> Result<()> {
    KeycloakAdminClient::new()
        .await
        .map_err(|e| anyhow!("Error creating Keycloak admin client: {e:?}"))?
        .update_realm_attributes(
            &get_event_realm(tenant_id, election_event_id),
            attributes,
        )
        .await
        .map_err(|e| anyhow!("Error updating Keycloak realm attributes: {e:?}"))
}

impl KeycloakAdminClient {
    #[instrument(skip(self), err)]
    pub async fn get_realm_attributes(
        self,
        realm: &str,
    ) -> Result<HashMap<String, String>> {
        let current_realm = self
            .client
            .realm_get(realm)
            .await
            .map_err(|err| anyhow!("{:?}", err))?;

        Ok(current_realm.attributes.unwrap_or_default())
    }

    /// Merges `updates` into the realm's current attributes: provided keys are
    /// set, an empty value removes the key, and attributes not present in
    /// `updates` are left untouched.
    #[instrument(skip(self, updates), err)]
    pub async fn update_realm_attributes(
        self,
        realm: &str,
        updates: HashMap<String, String>,
    ) -> Result<()> {
        validate_realm_attributes(&updates)?;

        let mut current_realm = self
            .client
            .realm_get(realm)
            .await
            .map_err(|err| anyhow!("{:?}", err))?;

        info!(
            "Updating realm {realm} with attributes: {:?}",
            redacted_attributes(&updates)
        );
        let mut attributes = current_realm.attributes.unwrap_or_default();
        apply_realm_attribute_updates(&mut attributes, updates);
        current_realm.attributes = Some(attributes);

        self.client
            .realm_put(realm, current_realm)
            .await
            .map_err(|err| anyhow!("{:?}", err))?;

        Ok(())
    }
}

fn apply_realm_attribute_updates(
    attributes: &mut HashMap<String, String>,
    updates: HashMap<String, String>,
) {
    for (key, value) in updates {
        if value == REDACTED_ATTRIBUTE_VALUE && is_sensitive_attribute_key(&key)
        {
            continue;
        }
        if value.is_empty() {
            attributes.remove(&key);
        } else {
            attributes.insert(key, value);
        }
    }
}

pub fn validate_realm_attributes(
    attributes: &HashMap<String, String>,
) -> Result<()> {
    for (key, value) in attributes {
        if key.trim().is_empty() {
            bail!("Realm attribute names cannot be blank");
        }
        if key.chars().any(char::is_control) {
            bail!("Realm attribute names cannot contain control characters");
        }
        // Empty values remove the attribute and redacted placeholders keep the
        // stored value, so neither carries a value to validate.
        if value.is_empty()
            || (value == REDACTED_ATTRIBUTE_VALUE
                && is_sensitive_attribute_key(key))
        {
            continue;
        }
        validate_realm_attribute_value(key, value)?;
    }
    Ok(())
}

fn validate_realm_attribute_value(key: &str, value: &str) -> Result<()> {
    match key {
        REALM_ATTR_CREDENTIAL_INPUT_POLICY => {
            if CredentialInputPolicy::from_str(value).is_err() {
                bail!("Invalid value {value:?} for realm attribute {key}");
            }
        }
        REALM_ATTR_CREDENTIAL_INPUT_PATTERN => {
            if !is_valid_credential_input_pattern(value) {
                bail!(
                    "Realm attribute {key} must contain 1 to {MAX_CREDENTIAL_PATTERN_GROUPS} dash-separated groups of 1 to {MAX_CREDENTIAL_PATTERN_GROUP_SIZE} 'd' digit tokens, with no more than {MAX_CREDENTIAL_PATTERN_TOTAL_SIZE} digits in total"
                );
            }
        }
        REALM_ATTR_CREDENTIAL_INPUT_PLACEHOLDER => {
            if !is_valid_credential_input_placeholder(value) {
                bail!(
                    "Realm attribute {key} must be one visible non-digit character other than '-' or '*'"
                );
            }
        }
        REALM_ATTR_SMARTLINK_ENABLED => {
            if bool::from_str(value).is_err() {
                bail!("Realm attribute {key} must be 'true' or 'false'");
            }
        }
        REALM_ATTR_SMARTLINK_TIMEOUT_SECS
        | REALM_ATTR_SMARTLINK_CLOCK_SKEW_SECS => {
            if !value.parse::<i64>().is_ok_and(|seconds| seconds > 0) {
                bail!("Realm attribute {key} must be a positive integer number of seconds");
            }
        }
        REALM_ATTR_SMARTLINK_SHARED_SECRET => {
            if value.len() > SMARTLINK_SHARED_SECRET_MAX_LEN {
                bail!(
                    "Realm attribute {key} cannot exceed {SMARTLINK_SHARED_SECRET_MAX_LEN} bytes"
                );
            }
        }
        REALM_ATTR_SMARTLINK_REQUIRED_ATTRIBUTES => {
            if value.len() > SMARTLINK_REQUIRED_ATTRIBUTES_MAX_LEN {
                bail!(
                    "Realm attribute {key} cannot exceed {SMARTLINK_REQUIRED_ATTRIBUTES_MAX_LEN} bytes"
                );
            }
        }
        REALM_ATTR_VOTER_CERTIFICATE_POLICY => {
            if VoterCertificatePolicy::from_str(value).is_err() {
                bail!("Invalid value {value:?} for realm attribute {key}");
            }
        }
        _ => {}
    }
    Ok(())
}

fn is_valid_credential_input_pattern(value: &str) -> bool {
    let groups = value.split('-').collect::<Vec<_>>();
    if groups.len() > MAX_CREDENTIAL_PATTERN_GROUPS {
        return false;
    }

    let mut total = 0;
    for group in groups {
        if group.is_empty() || !group.bytes().all(|byte| byte == b'd') {
            return false;
        }

        let size = group.len();
        if size > MAX_CREDENTIAL_PATTERN_GROUP_SIZE {
            return false;
        }

        total += size;
        if total > MAX_CREDENTIAL_PATTERN_TOTAL_SIZE {
            return false;
        }
    }

    true
}

fn is_valid_credential_input_placeholder(value: &str) -> bool {
    let mut characters = value.chars();
    let Some(character) = characters.next() else {
        return false;
    };

    characters.next().is_none()
        && !character.is_control()
        && !character.is_whitespace()
        && !character.is_ascii_digit()
        && !matches!(character, '-' | '*')
}

pub fn redacted_attributes(
    attributes: &HashMap<String, String>,
) -> HashMap<String, String> {
    attributes
        .iter()
        .map(|(key, value)| {
            let value = if is_sensitive_attribute_key(key) {
                REDACTED_ATTRIBUTE_VALUE.to_string()
            } else {
                value.clone()
            };
            (key.clone(), value)
        })
        .collect()
}

fn is_sensitive_attribute_key(key: &str) -> bool {
    let lower_key = key.to_ascii_lowercase();
    lower_key.contains("secret")
        || lower_key.contains("password")
        || lower_key.contains("token")
}

#[cfg(test)]
mod tests {
    use super::{
        apply_realm_attribute_updates, redacted_attributes,
        validate_realm_attributes, REDACTED_ATTRIBUTE_VALUE,
    };
    use std::collections::HashMap;

    fn attributes(entries: &[(&str, &str)]) -> HashMap<String, String> {
        entries
            .iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect()
    }

    #[test]
    fn validate_realm_attributes_accepts_generic_names() {
        let attributes = attributes(&[
            ("acr.loa.map", "{}"),
            ("credential-input-policy", "structured"),
            ("credential-input-pattern", "dddd-dddd-dddd-dddd"),
            ("credential-input-placeholder", "#"),
            ("smart-link-enabled", "true"),
            ("smart-link-timeout-secs", "90"),
            ("smart-link-clock-skew-secs", "5"),
            ("voter-certificate-policy", "enabled"),
        ]);

        assert!(validate_realm_attributes(&attributes).is_ok());
    }

    #[test]
    fn validate_realm_attributes_accepts_credential_input_configuration() {
        for (policy, pattern) in [
            ("standard", "dddd-dddd-dddd-dddd"),
            ("structured", "ddd-ddd"),
            ("structured", "dd-dddd-dd"),
            ("structured", "dddddddd-dddddddd-dddddddd-dddddddd-dddddddd-dddddddd-dddddddd-dddddddd"),
        ] {
            assert!(
                validate_realm_attributes(&attributes(&[
                    ("credential-input-policy", policy),
                    ("credential-input-pattern", pattern),
                    ("credential-input-placeholder", "#"),
                ]))
                .is_ok(),
                "expected policy={policy:?}, pattern={pattern:?} to be accepted"
            );
        }

        for placeholder in ["d", "#", "_", "•"] {
            assert!(
                validate_realm_attributes(&attributes(&[(
                    "credential-input-placeholder",
                    placeholder,
                )]))
                .is_ok(),
                "expected credential-input-placeholder={placeholder:?} to be accepted"
            );
        }
    }

    #[test]
    fn validate_realm_attributes_rejects_invalid_credential_input_policy() {
        for value in ["true", "segmented-numeric", "numeric", "STANDARD"] {
            assert!(
                validate_realm_attributes(&attributes(&[(
                    "credential-input-policy",
                    value,
                )]))
                .is_err(),
                "expected credential-input-policy={value:?} to be rejected"
            );
        }
    }

    #[test]
    fn validate_realm_attributes_rejects_invalid_credential_input_pattern() {
        for value in [
            "4-4",
            "dddd--dddd",
            "dddd-",
            "-dddd",
            "dddd_dddd",
            "dddd?",
            "dddd*",
            "DDDD",
            "ddddddddddddd",
            "dddddddd-dddddddd-dddddddd-dddddddd-dddddddd-dddddddd-dddddddd-dddddddd-dd",
            "dddddddd-dddddddd-dddddddd-dddddddd-dddddddd-dddddddd-dddddddd-dddddddd-dddddddd",
            "ｄｄｄｄ-ｄｄｄｄ",
        ] {
            assert!(
                validate_realm_attributes(&attributes(&[(
                    "credential-input-pattern",
                    value,
                )]))
                .is_err(),
                "expected credential-input-pattern={value:?} to be rejected"
            );
        }
    }

    #[test]
    fn validate_realm_attributes_rejects_invalid_credential_input_placeholder()
    {
        for value in ["##", "1", "-", "*", " ", "\n"] {
            assert!(
                validate_realm_attributes(&attributes(&[(
                    "credential-input-placeholder",
                    value,
                )]))
                .is_err(),
                "expected credential-input-placeholder={value:?} to be rejected"
            );
        }
    }

    #[test]
    fn validate_realm_attributes_rejects_blank_names() {
        let attributes = attributes(&[(" ", "value")]);

        assert!(validate_realm_attributes(&attributes).is_err());
    }

    #[test]
    fn validate_realm_attributes_rejects_control_characters_in_names() {
        let attributes = attributes(&[("bad\nkey", "value")]);

        assert!(validate_realm_attributes(&attributes).is_err());
    }

    #[test]
    fn validate_realm_attributes_rejects_invalid_smart_link_values() {
        for (key, value) in [
            ("smart-link-enabled", "yes"),
            ("smart-link-timeout-secs", "banana"),
            ("smart-link-timeout-secs", "0"),
            ("smart-link-timeout-secs", "-90"),
            ("smart-link-clock-skew-secs", "1.5"),
            ("voter-certificate-policy", "sometimes"),
        ] {
            assert!(
                validate_realm_attributes(&attributes(&[(key, value)]))
                    .is_err(),
                "expected {key}={value:?} to be rejected"
            );
        }
    }

    #[test]
    fn validate_realm_attributes_rejects_oversized_values() {
        let oversized = "x".repeat(1001);
        for key in
            ["smart-link-shared-secret", "smart-link-required-attributes"]
        {
            assert!(
                validate_realm_attributes(&attributes(&[(
                    key,
                    oversized.as_str()
                )]))
                .is_err(),
                "expected oversized {key} to be rejected"
            );
        }
    }

    #[test]
    fn validate_realm_attributes_accepts_removals_and_redacted_secrets() {
        let attributes = attributes(&[
            ("smart-link-timeout-secs", ""),
            ("smart-link-shared-secret", REDACTED_ATTRIBUTE_VALUE),
        ]);

        assert!(validate_realm_attributes(&attributes).is_ok());
    }

    #[test]
    fn apply_realm_attribute_updates_merges_into_current_attributes() {
        let mut current = attributes(&[
            ("acr.loa.map", "{}"),
            ("smart-link-enabled", "false"),
            ("smart-link-client-id", "voting-portal"),
        ]);

        apply_realm_attribute_updates(
            &mut current,
            attributes(&[
                ("smart-link-enabled", "true"),
                ("smart-link-client-id", ""),
            ]),
        );

        assert_eq!(
            current,
            attributes(&[
                ("acr.loa.map", "{}"),
                ("smart-link-enabled", "true")
            ])
        );
    }

    #[test]
    fn apply_realm_attribute_updates_keeps_secret_on_redacted_placeholder() {
        let mut current =
            attributes(&[("smart-link-shared-secret", "the real secret")]);

        apply_realm_attribute_updates(
            &mut current,
            attributes(&[(
                "smart-link-shared-secret",
                REDACTED_ATTRIBUTE_VALUE,
            )]),
        );

        assert_eq!(
            current.get("smart-link-shared-secret").map(String::as_str),
            Some("the real secret")
        );
    }

    #[test]
    fn redacted_attributes_hides_sensitive_values() {
        let attributes = attributes(&[
            ("smart-link-shared-secret", "the cake is in the oven"),
            ("api-token", "hello"),
            ("smart-link-enabled", "true"),
        ]);

        let redacted = redacted_attributes(&attributes);
        assert_eq!(
            redacted.get("smart-link-shared-secret").map(String::as_str),
            Some(REDACTED_ATTRIBUTE_VALUE)
        );
        assert_eq!(
            redacted.get("api-token").map(String::as_str),
            Some(REDACTED_ATTRIBUTE_VALUE)
        );
        assert_eq!(
            redacted.get("smart-link-enabled").map(String::as_str),
            Some("true")
        );
    }
}
