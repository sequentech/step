use crate::serialization::deserialize_with_path::deserialize_str;
// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::{
    keycloak::KeycloakAdminClient, replace_uuids::replace_uuids,
};
use crate::types::keycloak::TENANT_ID_ATTR_NAME;
use anyhow::{anyhow, Context, Result};
use keycloak::types::{
    AuthenticationExecutionInfoRepresentation, RealmRepresentation,
};
use keycloak::{
    KeycloakAdmin, KeycloakAdminToken, KeycloakError, KeycloakTokenSupplier,
};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::env;
use tracing::{info, instrument};

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

    pub async fn get_flow_executions(
        &self,
        client: &PubKeycloakAdmin,
        board_name: &str,
        execution_name: &str,
    ) -> Result<Vec<AuthenticationExecutionInfoRepresentation>, KeycloakError>
    {
        let req_url = format!(
            "{}/admin/realms/{}/authentication/flows/{}/executions",
            client.url, board_name, execution_name
        );

        // Send GET request to fetch flow executions
        let response = client
            .client
            .get(&req_url)
            .bearer_auth(client.token_supplier.get(&client.url).await?)
            .send()
            .await?;

        Ok(error_check(response).await?.json().await?)
    }

    pub async fn upsert_flow_execution(
        &self,
        client: &PubKeycloakAdmin,
        board_name: &str,
        execution_name: &str,
        json_execution_config: &str,
    ) -> Result<()> {
        // Deserialize execution config
        let execution: AuthenticationExecutionInfoRepresentation =
            serde_json::from_str(json_execution_config).with_context(|| {
                "Failed to deserialize execution configuration"
            })?;

        let req_url = format!(
            "{}/admin/realms/{}/authentication/flows/{}/executions",
            client.url, board_name, execution_name
        );

        // Send PUT request to update flow execution
        let response = client
            .client
            .put(&req_url)
            .json(&execution) // Serialize execution to JSON
            .bearer_auth(client.token_supplier.get(&client.url).await?)
            .send()
            .await
            .with_context(|| {
                format!("Error sending update request to '{}'", req_url)
            })?;

        error_check(response).await?;

        Ok(())
    }

    #[instrument(skip(self, json_realm_config), err)]
    pub async fn upsert_realm(
        self,
        board_name: &str,
        json_realm_config: &str,
        tenant_id: &str,
        replace_ids: bool,
        display_name: Option<String>,
        election_event_id: Option<String>,
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

        let voting_portal_url_env = env::var("VOTING_PORTAL_URL")
            .with_context(|| "Error fetching VOTING_PORTAL_URL env var")?;
        let login_url = if let Some(election_event_id) = election_event_id {
            Some(format!("{voting_portal_url_env}/tenant/{tenant_id}/event/{election_event_id}/login"))
        } else {
            None
        };

        // set the voting portal and voting portal kiosk urls
        realm.clients = Some(
            realm
                .clients
                .unwrap_or_default()
                .into_iter()
                .map(|mut client| {
                    if client.client_id == Some(String::from("voting-portal"))
                        || client.client_id
                            == Some(String::from("onsite-voting-portal"))
                    {
                        client.root_url = Some(voting_portal_url_env.clone());
                        client.base_url = login_url.clone();
                        client.redirect_uris = Some(vec!["/*".to_string()]);
                    }
                    Ok(client) // Return the modified client
                })
                .collect::<Result<Vec<_>>>()
                .map_err(|err| {
                    anyhow!("Error setting the voting portal urls: {:?}", err)
                })?,
        );

        // set tenant id attribute on all users
        realm.users = Some(
            realm
                .users
                .unwrap_or_default()
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
