// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use tracing::instrument;
use crate::ballot::VoterCertificatePolicy;
use crate::services::keycloak::{get_event_realm, KeycloakAdminClient};
use crate::types::keycloak::REALM_ATTR_VOTER_CERTIFICATE_POLICY;
use crate::types::hasura::core::ElectionEvent;

pub async fn update_realm_attributes(
    election_event: &ElectionEvent,
) -> Result<()> {

    let cert_policy: VoterCertificatePolicy = election_event
        .get_presentation()
        .map_err(|e| anyhow!("Error deserializing election event presentation: {e:?}"))?
        .and_then(|presentation| presentation.voter_certificate_policy)
        .unwrap_or_default();

    KeycloakAdminClient::new()
        .await
        .map_err(|e| anyhow!("Error creating Keycloak admin client: {e:?}"))?
        .update_realm_attributes(
            &get_event_realm(&election_event.tenant_id, &election_event.id),
            HashMap::from([(
                REALM_ATTR_VOTER_CERTIFICATE_POLICY.to_string(),
                cert_policy.to_string(),
            )]),
        )
        .await
        .map_err(|e| anyhow!("Error updating Keycloak realm attributes: {e:?}"))?;

    Ok(())
}

impl KeycloakAdminClient {
    #[instrument(skip(self), err)]
    pub async fn update_realm_attributes(
        self,
        realm: &str,
        attributes: HashMap<String, String>,
    ) -> Result<()> {
        let mut current_realm = self
            .client
            .realm_get(realm)
            .await
            .map_err(|err| anyhow!("{:?}", err))?;

        let mut current_attributes =
            current_realm.attributes.unwrap_or_default();
        for (key, value) in attributes {
            current_attributes.insert(key, value);
        }
        current_realm.attributes = Some(current_attributes);

        self.client
            .realm_put(realm, current_realm)
            .await
            .map_err(|err| anyhow!("{:?}", err))?;

        Ok(())
    }
}
