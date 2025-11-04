use crate::serialization::deserialize_with_path::deserialize_str;
// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::{
    keycloak::KeycloakAdminClient, replace_uuids::replace_uuids,
};
use crate::types::keycloak::{Role, TENANT_ID_ATTR_NAME};
use anyhow::{anyhow, Context, Result};
use keycloak::types::{
    AuthenticationExecutionInfoRepresentation, GroupRepresentation,
    RealmRepresentation, RoleRepresentation,
};
use keycloak::{
    KeycloakAdmin, KeycloakAdminToken, KeycloakError, KeycloakTokenSupplier,
};
use reqwest::Client;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::env;
use std::hash::RandomState;
use tracing::{error, info, instrument};
use uuid::Uuid;

use super::PubKeycloakAdmin;

#[derive(Debug, Clone, Copy)]
pub enum RoleAction {
    Add,
    Remove,
}

impl RoleAction {
    fn is_delete(&self) -> bool {
        matches!(self, RoleAction::Remove)
    }
}

pub fn get_event_realm(tenant_id: &str, election_event_id: &str) -> String {
    format!("tenant-{}-event-{}", tenant_id, election_event_id)
}
pub fn parse_realm(realm: &str) -> Option<(String, Option<String>)> {
    let parts: Vec<&str> = realm.split('-').collect();
    
    // Expected formats:
    // - Tenant realm: "tenant-{tenant_id}"
    // - Event realm: "tenant-{tenant_id}-event-{election_event_id}"
    
    if parts.len() >= 2 && parts[0] == "tenant" {
        // Check if this is an event realm
        if let Some(event_idx) = parts.iter().position(|&p| p == "event") {
            if event_idx > 1 && event_idx < parts.len() - 1 {
                let tenant_id = parts[1..event_idx].join("-");
                let election_event_id = parts[event_idx + 1..].join("-");
                return Some((tenant_id, Some(election_event_id)));
            }
        } else {
            // This is a tenant realm (no "event" found)
            let tenant_id = parts[1..].join("-");
            return Some((tenant_id, None));
        }
    }
    
    None
}

pub fn get_tenant_realm(tenant_id: &str) -> String {
    format!("tenant-{}", tenant_id)
}

/// Extracts tenant_id and election_event_id replacements from a realm config.
///
/// This function parses the realm name to extract the old tenant_id and 
/// election_event_id, and compares them with the new values to determine if replacements
/// are needed.
///
/// # Arguments
/// * `realm_config` -  Realm config
/// * `new_tenant_id` - The new tenant_id to use
/// * `new_election_event_id` - Optional new election_event_id to use
///
/// # Returns
/// A tuple of:
/// * Optional (old_tenant_id, new_tenant_id) for replacement
/// * Optional (old_event_id, new_event_id) for replacement
pub fn extract_realm_replacements(
    realm_config: &RealmRepresentation,
    new_tenant_id: &str,
    new_election_event_id: &Option<String>,
) -> (Option<(String, String)>, Option<(String, String)>) {
    info!("edulix6");

    // Get the realm name
    let Some(realm_name) = realm_config.realm.as_ref() else {
        return (None, None);
    };
    info!(?realm_name, "edulix7");

    // Parse the realm name to extract tenant_id and election_event_id
    let Some((old_tenant_id, old_election_event_id)) = parse_realm(realm_name) else {
        info!("edulix8");
        return (None, None);
    };
    info!(?old_tenant_id, ?old_election_event_id, "edulix9");

    // Determine tenant_id replacement if different
    let tenant_id_replacement = Some((old_tenant_id.clone(), new_tenant_id.to_string()));

    // Determine election_event_id replacement if applicable
    let election_event_id_replacement = match (old_election_event_id, new_election_event_id) {
        (Some(old_event_id), Some(new_event_id)) if old_event_id != *new_event_id => {
            Some((old_event_id, new_event_id.clone()))
        }
        _ => None,
    };

    (tenant_id_replacement, election_event_id_replacement)
}

/// Replaces UUIDs in a Keycloak realm JSON configuration while preserving specific IDs.
///
/// This function processes the realm configuration and replaces most UUIDs with new ones,
/// while keeping certain IDs unchanged based on the provided `keep` list.
/// It also automatically preserves UUIDs referenced in Keycloak authenticator configurations
/// to maintain consistency of internal references.
///
/// # Arguments
/// * `json_realm_config` - The original JSON string representation of the realm configuration
/// * `keep` - A list of UUID strings that should NOT be replaced with new ones
/// * `tenant_id_replacement` - Optional tuple of (old_tenant_id, new_tenant_id) for explicit replacement
/// * `election_event_id_replacement` - Optional tuple of (old_event_id, new_event_id) for explicit replacement
///
/// # Returns
/// A tuple containing:
/// * The modified JSON string with replaced UUIDs
/// * A HashMap mapping old UUIDs to their new replacements
#[instrument(err, skip(json_realm_config))]
pub fn replace_realm_ids(
    json_realm_config: &str,
    mut keep: Vec<String>,
    tenant_id_replacement: Option<(String, String)>,
    election_event_id_replacement: Option<(String, String)>,
) -> Result<(String, HashMap<String, String>)> {
    // Parse the realm to find UUIDs that should be preserved
    let realm: RealmRepresentation = deserialize_str(json_realm_config)?;
    
    // Add tenant_id to keep list if we're doing an explicit replacement
    if let Some((old_tenant_id, _)) = &tenant_id_replacement {
        keep.push(old_tenant_id.clone());
    }
    
    // Add election_event_id to keep list if we're doing an explicit replacement
    if let Some((old_event_id, _)) = &election_event_id_replacement {
        keep.push(old_event_id.clone());
    }
    
    // Find and preserve UUIDs referenced in Keycloak authenticator configurations
    // These UUIDs are references that must remain consistent
    if let Some(authenticator_configs) = realm.authenticator_config.clone() {
        for authenticator_config in authenticator_configs {
            let Some(config) = authenticator_config.config.clone() else {
                continue;
            };
            // Check each config value to see if it's a valid UUID
            for (_key, value) in config {
                if Uuid::parse_str(&value).is_ok() {
                    keep.push(value.clone());
                }
            }
        }
    }

    // Replace all UUIDs in the JSON string except those in the 'keep' list
    // Returns the modified JSON string and a map of old UUID -> new UUID
    let (mut new_data, replacement_map) = replace_uuids(json_realm_config, keep);

    // Apply explicit tenant_id replacement if provided
    if let Some((old_tenant_id, new_tenant_id)) = tenant_id_replacement {
        if old_tenant_id != new_tenant_id {
            new_data = new_data.replace(&old_tenant_id, &new_tenant_id);
        }
    }

    // Apply explicit election_event_id replacement if provided
    if let Some((old_event_id, new_event_id)) = election_event_id_replacement {
        new_data = new_data.replace(&old_event_id, &new_event_id);
    }

    Ok((new_data, replacement_map))
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
        info!("get_realm: board_name={board_name:?}");
        // see https://docs.rs/keycloak/latest/src/keycloak/rest/generated_rest.rs.html#6315-6334
        let mut builder = client
            .client
            .post(&format!(
                "{}/admin/realms/{board_name}/partial-export",
                client.url
            ))
            .bearer_auth(
                client.token_supplier.get(&client.url).await.map_err(
                    |error| {
                        error!("error obtaining token: {error:?}");
                        return error;
                    },
                )?,
            );
        builder = builder.query(&[("exportClients", true)]);
        builder = builder.query(&[("exportGroupsAndRoles", true)]);
        let response = builder.send().await.map_err(|error| {
            error!("error sending built query: {error:?}");
            return error;
        })?;
        Ok(
            error_check(response)
            .await
            .map_err(|error| {
                error!("error checking response for realm name {board_name:?}: {error:?}");
                return error;
            })?
            .json()
            .await
            .map_err(|error| {
                error!("error mapping to json: {error:?}");
                return error;
            })?
        )
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

    pub async fn partial_import_realm_with_cleanup(
        &self,
        client: &PubKeycloakAdmin,
        tenant_id: &str,
        container_id: &str,
        realm_groups: Vec<GroupRepresentation>,
        realm_roles: Vec<RoleRepresentation>,
        if_resource_exists: &str,
    ) -> Result<()> {
        let realm = format!("tenant-{}", tenant_id);

        // Proceed with partial import
        let req_url =
            format!("{}/admin/realms/{}/partialImport", client.url, realm);
        let payload = json!({
            "groups": realm_groups,
            "roles": {
                "realm": realm_roles,
            },
            "id": container_id,
            "ifResourceExists": if_resource_exists,
            "realm": realm,
        });

        let response = client
            .client
            .post(&req_url)
            .bearer_auth(client.token_supplier.get(&client.url).await?)
            .json(&payload)
            .send()
            .await?;

        error_check(response).await?;

        Ok(())
    }

    pub async fn realm_delete(
        &self,
        client: &PubKeycloakAdmin,
        tenant_id: &str,
        delete_by: &str,
        id: &str,
    ) -> Result<(), KeycloakError> {
        let realm = format!("tenant-{}", tenant_id);
        let req_url = format!(
            "{}/admin/realms/{}/{}/{}",
            client.url, realm, delete_by, id
        );

        let response = client
            .client
            .delete(&req_url)
            .bearer_auth(client.token_supplier.get(&client.url).await?)
            .send()
            .await?;

        error_check(response).await?;

        Ok(())
    }

    pub async fn create_new_group(
        &self,
        tenant_id: &str,
        group_name: &str,
        keycloak_client: &PubKeycloakAdmin,
    ) -> Result<Option<String>, KeycloakError> {
        let realm = format!("tenant-{}", tenant_id);
        let url =
            format!("{}/admin/realms/{}/groups", keycloak_client.url, realm);

        let body = serde_json::json!({ "name": group_name });

        let response = keycloak_client
            .client
            .post(&url)
            .bearer_auth(
                keycloak_client
                    .token_supplier
                    .get(&keycloak_client.url)
                    .await?,
            )
            .json(&body)
            .send()
            .await?;

        if let Some(location_header) =
            response.headers().get(reqwest::header::LOCATION)
        {
            let location_str = location_header.to_str().map_err(|e| {
                KeycloakError::HttpFailure {
                    status: response.status().into(),
                    body: None,
                    text: e.to_string(),
                }
            })?;
            // The ID is the trailing part of the URL
            if let Some(id) = location_str.split('/').last() {
                return Ok(Some(id.to_string()));
            }
        }

        Ok(None)
    }

    pub async fn add_roles_to_group(
        &self,
        tenant_id: &str,
        keycloak_client: &PubKeycloakAdmin,
        group_id: &str,
        roles: &Vec<RoleRepresentation>,
        action: RoleAction,
    ) -> Result<(), KeycloakError> {
        let realm = format!("tenant-{}", tenant_id);
        let url = format!(
            "{}/admin/realms/{}/groups/{}/role-mappings/realm",
            keycloak_client.url, realm, group_id
        );

        // The body expects an array of role representations (id + name are
        // enough).
        let payload: Vec<_> = roles
            .iter()
            .map(|r| json!({ "id": r.id, "name": r.name }))
            .collect();
        let resp = if action.is_delete() {
            keycloak_client
                .client
                .delete(&url)
                .bearer_auth(
                    keycloak_client
                        .token_supplier
                        .get(&keycloak_client.url)
                        .await?,
                )
                .json(&payload)
                .send()
                .await?
        } else {
            keycloak_client
                .client
                .post(&url)
                .bearer_auth(
                    keycloak_client
                        .token_supplier
                        .get(&keycloak_client.url)
                        .await?,
                )
                .json(&payload)
                .send()
                .await?
        };

        error_check(resp).await?;

        Ok(())
    }

    pub async fn get_group_assigned_roles(
        &self,
        tenant_id: &str,
        group_id: &str,
        keycloak_client: &PubKeycloakAdmin,
    ) -> Result<Vec<RoleRepresentation>, Box<dyn std::error::Error>> {
        let realm = format!("tenant-{}", tenant_id);
        let url = format!(
            "{}/admin/realms/{}/groups/{}/role-mappings/realm",
            keycloak_client.url, realm, group_id
        );
        let resp = keycloak_client
            .client
            .get(&url)
            .bearer_auth(
                keycloak_client
                    .token_supplier
                    .get(&keycloak_client.url)
                    .await?,
            )
            .send()
            .await
            .context("Failed to get groups roles")?;

        let roles: Vec<RoleRepresentation> = resp.json().await?;
        Ok(roles)
    }

    pub async fn update_group(
        &self,
        tenant_id: &str,
        group: &GroupRepresentation,
    ) -> Result<()> {
        let client = &KeycloakAdminClient::pub_new().await?;
        let realm = format!("tenant-{}", tenant_id);

        let req_url = format!(
            "{}/admin/realms/{}/groups/{}",
            client.url,
            realm,
            group.id.as_ref().unwrap()
        );
        let response = client
            .client
            .put(&req_url)
            .bearer_auth(client.token_supplier.get(&client.url).await?)
            .json(group)
            .send()
            .await
            .context("Failed to update group")?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(anyhow!("Failed to update group"))
        }
    }

    pub async fn update_localization_texts_from_import(
        &self,
        imported_localization_texts: Option<
            HashMap<String, HashMap<String, String>>,
        >,
        keycloak_client: &PubKeycloakAdmin,
        tenant_id: &str,
    ) -> Result<()> {
        let realm = format!("tenant-{}", tenant_id);

        if let Some(localization_texts) = imported_localization_texts {
            for (locale, locale_texts) in localization_texts {
                println!("Processing locale: {}", locale);

                let url = format!(
                    "{}/admin/realms/{}/localization/{}",
                    keycloak_client.url, realm, locale
                );

                let response = keycloak_client
                .client
                .post(&url)
                .bearer_auth(keycloak_client.token_supplier.get(&keycloak_client.url).await?) // Use the access token for authorization
                .json(&locale_texts)
                .send()
                .await
                .context(format!("Failed to send request to update localization texts for locale '{}'", locale))?;
            }
        }

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
            let realm_config: RealmRepresentation =
                deserialize_str(&json_realm_config)?;
            let (tenant_id_replacement, election_event_id_replacement) = 
                extract_realm_replacements(&realm_config, tenant_id, &election_event_id);
            info!(?tenant_id_replacement, ?election_event_id_replacement, ?tenant_id, ?election_event_id, "edulix5");

            let (result, _) = replace_realm_ids(
                json_realm_config,
                vec![],
                tenant_id_replacement,
                election_event_id_replacement
            )?;
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
        let ballot_verifier_url = env::var("BALLOT_VERIFIER_URL")
            .with_context(|| "Error fetching BALLOT_VERIFIER_URL env var")?;

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
                        client.redirect_uris = Some(vec![
                            "/*".to_string(),
                            format!("{}/*", ballot_verifier_url),
                        ]);
                    }

                    // When an Action Token expires, for example a Manual
                    // Verification QR Code, the `Back to Application` link in
                    // the resulting error page will redirect to the `base_url`.
                    // For this reason, we ensure that base_url is linking to
                    // the login_url if we have any.
                    //
                    // Related: https://github.com/sequentech/meta/issues/5063
                    if client.client_id == Some(String::from("account"))
                        && login_url.is_some()
                    {
                        client.base_url = login_url.clone();
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
