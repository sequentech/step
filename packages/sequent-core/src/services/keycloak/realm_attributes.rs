// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::keycloak::{get_event_realm, KeycloakAdminClient};
use anyhow::{anyhow, bail, Result};
use std::collections::HashMap;
use tracing::{info, instrument};

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

    #[instrument(skip(self), err)]
    pub async fn update_realm_attributes(
        self,
        realm: &str,
        attributes: HashMap<String, String>,
    ) -> Result<()> {
        validate_realm_attributes(&attributes)?;

        let mut current_realm = self
            .client
            .realm_get(realm)
            .await
            .map_err(|err| anyhow!("{:?}", err))?;

        info!(
            "Updating realm {realm} with attributes: {:?}",
            redacted_attributes(&attributes)
        );
        current_realm.attributes = Some(attributes);

        self.client
            .realm_put(realm, current_realm)
            .await
            .map_err(|err| anyhow!("{:?}", err))?;

        Ok(())
    }
}

fn validate_realm_attributes(
    attributes: &HashMap<String, String>,
) -> Result<()> {
    for key in attributes.keys() {
        if key.trim().is_empty() {
            bail!("Realm attribute names cannot be blank");
        }
        if key.chars().any(char::is_control) {
            bail!("Realm attribute names cannot contain control characters");
        }
    }
    Ok(())
}

fn redacted_attributes(
    attributes: &HashMap<String, String>,
) -> HashMap<String, String> {
    let mut redacted = attributes.clone();
    let sensitive_keys: Vec<String> = redacted
        .keys()
        .filter(|key| is_sensitive_attribute_key(key))
        .cloned()
        .collect();
    for key in sensitive_keys {
        redacted.insert(key, "<redacted>".to_string());
    }
    redacted
}

fn is_sensitive_attribute_key(key: &str) -> bool {
    let lower_key = key.to_ascii_lowercase();
    lower_key.contains("secret")
        || lower_key.contains("password")
        || lower_key.contains("token")
}

#[cfg(test)]
mod tests {
    use super::{redacted_attributes, validate_realm_attributes};
    use std::collections::HashMap;

    #[test]
    fn validate_realm_attributes_accepts_generic_names() {
        let mut attributes = HashMap::new();
        attributes.insert("acr.loa.map".to_string(), "{}".to_string());
        attributes.insert("smart-link-enabled".to_string(), "true".to_string());

        assert!(validate_realm_attributes(&attributes).is_ok());
    }

    #[test]
    fn validate_realm_attributes_rejects_blank_names() {
        let mut attributes = HashMap::new();
        attributes.insert(" ".to_string(), "value".to_string());

        assert!(validate_realm_attributes(&attributes).is_err());
    }

    #[test]
    fn validate_realm_attributes_rejects_control_characters_in_names() {
        let mut attributes = HashMap::new();
        attributes.insert("bad\nkey".to_string(), "value".to_string());

        assert!(validate_realm_attributes(&attributes).is_err());
    }

    #[test]
    fn redacted_attributes_hides_sensitive_values() {
        let mut attributes = HashMap::new();
        attributes.insert(
            "smart-link-shared-secret".to_string(),
            "the cake is in the oven".to_string(),
        );
        attributes.insert("api-token".to_string(), "hello".to_string());

        assert_eq!(
            redacted_attributes(&attributes)
                .get("smart-link-shared-secret")
                .map(String::as_str),
            Some("<redacted>")
        );
        assert_eq!(
            redacted_attributes(&attributes)
                .get("api-token")
                .map(String::as_str),
            Some("<redacted>")
        );
    }
}
