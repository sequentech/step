use crate::serialization::deserialize_with_path::deserialize_str;
// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::{
    keycloak::KeycloakAdminClient, replace_uuids::replace_uuids,
};
use crate::types::keycloak::TENANT_ID_ATTR_NAME;
use anyhow::{anyhow, Result};
use keycloak::types::RealmRepresentation;
use keycloak::{
    KeycloakAdmin, KeycloakAdminToken, KeycloakError, KeycloakTokenSupplier,
};
use serde_json::{json, Value};
use std::collections::HashMap;
use tracing::instrument;

use super::PubKeycloakAdmin;

pub fn get_event_realm(tenant_id: &str, election_event_id: &str) -> String {
    format!("tenant-{}-event-{}", tenant_id, election_event_id)
}

pub fn get_tenant_realm(tenant_id: &str) -> String {
    format!("tenant-{}", tenant_id)
}

async fn error_check(
    response: reqwest::Response,
) -> Result<reqwest::Response, KeycloakError> {
    if !response.status().is_success() {
        let status = response.status().into();
        let text = response.text().await?;
        return Err(KeycloakError::HttpFailure {
            status,
            body: serde_json::from_str(&text).ok(),
            text,
        });
    }

    Ok(response)
}

impl KeycloakAdminClient {
    pub async fn get_realm(
        self,
        client: &PubKeycloakAdmin,
        board_name: &str,
    ) -> Result<RealmRepresentation, KeycloakError> {
        // see https://docs.rs/keycloak/latest/src/keycloak/rest/generated_rest.rs.html#6315-6334
        let mut builder = client
            .client
            .post(&format!(
                "{}/admin/realms/{board_name}/partial-export",
                client.url
            ))
            .bearer_auth(client.token_supplier.get(&client.url).await?);
        builder = builder.query(&[("exportClients", true)]);
        builder = builder.query(&[("exportGroupsAndRoles", true)]);
        let response = builder.send().await?;
        Ok(error_check(response).await?.json().await?)
    }

    #[instrument(skip(self, json_realm_config), err)]
    pub async fn upsert_realm(
        self,
        board_name: &str,
        json_realm_config: &str,
        tenant_id: &str,
        replace_ids: bool,
        display_name: Option<String>,
    ) -> Result<()> {
        let real_get_result = self.client.realm_get(board_name).await;
        let replaced_ids_config = if replace_ids {
            let (result, _) = replace_uuids(json_realm_config, vec![]);
            result
        } else {
            json_realm_config.to_string()
        };
        let mut realm: RealmRepresentation =
            deserialize_str(&replaced_ids_config)?;

        // set realm name
        realm.realm = Some(board_name.into());

        if let Some(name) = display_name {
            realm.display_name = Some(name);
        }

        // set tenant id attribute on all users
        realm.users = Some(
            realm
                .users
                .unwrap_or(vec![])
                .into_iter()
                .map(|user| {
                    let mut mod_user = user.clone();
                    let mut attributes =
                        mod_user.attributes.clone().unwrap_or(HashMap::new());
                    let tenant_attribute_js: Vec<String> =
                        vec![tenant_id.to_string()];
                    attributes.insert(
                        TENANT_ID_ATTR_NAME.into(),
                        tenant_attribute_js,
                    );
                    mod_user.attributes = Some(attributes);
                    mod_user
                })
                .collect(),
        );

        match real_get_result {
            Ok(_) => self
                .client
                .realm_put(&board_name, realm)
                .await
                .map_err(|err| anyhow!("Keycloak error: {:?}", err)),
            Err(_) => self
                .client
                .post(realm)
                .await
                .map_err(|err| anyhow!("Keycloak error: {:?}", err)),
        }
    }
}
