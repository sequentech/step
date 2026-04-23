// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::ballot::VoterCertificatePolicy;
use crate::services::keycloak::{get_event_realm, KeycloakAdminClient};
use crate::types::keycloak::REALM_ATTR_VOTER_CERTIFICATE_POLICY;
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::hash::BuildHasher;
use std::str::FromStr;
use tracing::{info, instrument, warn};

/// Updates custom attributes on the Keycloak realm for the given election event.
//
///
/// # Errors
///
/// Returns an error if the Keycloak admin client cannot be created, if fetching or
/// updating the realm fails, or if the underlying HTTP client reports an error.
pub async fn update_realm_attributes<S: BuildHasher + Send>(
    tenant_id: &str,
    election_event_id: &str,
    attributes: HashMap<String, String, S>,
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
    /// Update `attributes` into the realm's existing Keycloak realm attributes.
    ///
    /// # Errors
    ///
    /// Returns an error if the realm cannot be read or updated via the Keycloak Admin API.
    #[instrument(skip(self), err)]
    pub async fn update_realm_attributes<S: BuildHasher + Send>(
        self,
        realm: &str,
        attributes: HashMap<String, String, S>,
    ) -> Result<()> {
        let mut current_realm = self
            .client
            .realm_get(realm)
            .await
            .map_err(|err| anyhow!("{err:?}"))?;

        let mut current_attributes =
            current_realm.attributes.unwrap_or_default();

        for (key, value) in attributes {
            if key == REALM_ATTR_VOTER_CERTIFICATE_POLICY {
                match VoterCertificatePolicy::from_str(&value) {
                    Ok(policy) => {
                        current_attributes.insert(key, policy.to_string());
                    }
                    Err(_) => {
                        warn!(
                            "Ignoring invalid value {:?} for realm attribute {:?}",
                            value, key
                        );
                    }
                }
            } else {
                warn!("Ignoring unknown realm attribute {:?}", key);
            }
        }

        info!("Updating realm {realm} with attributes: {current_attributes:?}");
        current_realm.attributes = Some(current_attributes);

        self.client
            .realm_put(realm, current_realm)
            .await
            .map_err(|err| anyhow!("{err:?}"))?;

        Ok(())
    }
}
