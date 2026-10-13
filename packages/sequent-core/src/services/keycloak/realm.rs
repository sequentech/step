// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::serialization::deserialize_with_path::deserialize_str;
use crate::services::jwks::{expected_audiences, AUDIENCES_ENV};
use crate::services::{
    keycloak::KeycloakAdminClient, replace_uuids::replace_uuids,
};
use crate::types::keycloak::{
    Role, HARVEST_AUDIENCE_MAPPER_NAME, TENANT_ID_ATTR_NAME,
};
use anyhow::{anyhow, Context, Result};
use keycloak::types::{
    AuthenticationExecutionInfoRepresentation, ClientRepresentation,
    GroupRepresentation, ProtocolMapperRepresentation, RealmRepresentation,
    RoleRepresentation,
};
use keycloak::{
    KeycloakAdmin, KeycloakAdminToken, KeycloakError, KeycloakTokenSupplier,
};
use rand::distributions::Alphanumeric;
use rand::Rng;
use reqwest::Client;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::env;
use std::hash::RandomState;
use tracing::{error, info, instrument};
use uuid::Uuid;

use super::PubKeycloakAdmin;

const VOTING_PORTAL_CLIENT_ID: &str = "voting-portal";
const KIOSK_VOTING_PORTAL_CLIENT_ID: &str = "voting-portal-kiosk";
const ONSITE_VOTING_PORTAL_CLIENT_ID: &str = "onsite-voting-portal";
const RESULTS_PORTAL_CLIENT_ID: &str = "results-portal";
const CONDITIONAL_CLIENT_ID: &str = "conditional-client";
const AUDIENCE_MAPPER: &str = "oidc-audience-mapper";
/// Settings of an audience mapper that name the audience it adds.
const AUDIENCE_MAPPER_SETTINGS: [&str; 2] =
    ["included.custom.audience", "included.client.audience"];

fn kiosk_voting_portal_url(
    voting_portal_url: &str,
    configured_kiosk_url: Option<String>,
) -> String {
    configured_kiosk_url
        .as_deref()
        .map(str::trim)
        .filter(|url| !url.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| voting_portal_url.to_string())
}

fn voting_portal_redirect_uris(ballot_verifier_url: &str) -> Vec<String> {
    vec![
        "/*".to_string(),
        format!("{}/*", ballot_verifier_url.trim_end_matches('/')),
    ]
}

fn results_portal_redirect_uris(results_portal_url: &str) -> Vec<String> {
    vec![format!("{}/*", results_portal_url.trim_end_matches('/'))]
}

/// Whether a protocol mapper adds Harvest's audience: the mapper Step manages,
/// or any audience mapper that adds one of `harvest_audiences`, whatever its
/// name.
fn adds_harvest_audience(
    protocol_mapper: &ProtocolMapperRepresentation,
    harvest_audiences: &[String],
) -> bool {
    if protocol_mapper.name.as_deref() == Some(HARVEST_AUDIENCE_MAPPER_NAME) {
        return true;
    }
    if protocol_mapper.protocol_mapper.as_deref() != Some(AUDIENCE_MAPPER) {
        return false;
    }
    let Some(config) = protocol_mapper.config.as_ref() else {
        return false;
    };
    AUDIENCE_MAPPER_SETTINGS
        .iter()
        .filter_map(|setting| config.get(*setting))
        .any(|audience| {
            harvest_audiences
                .iter()
                .any(|harvest| harvest.as_str() == audience.trim())
        })
}

/// The audiences the results portal client must not get, read from
/// `HARVEST_JWT_AUDIENCES`. Without them, a renamed Harvest audience mapper
/// could be copied, so the client is not created.
fn results_portal_harvest_audiences(
    setting: Option<&str>,
) -> Result<Vec<String>> {
    expected_audiences(setting).with_context(|| {
        format!(
            "Windmill needs {AUDIENCES_ENV} to create the results portal client"
        )
    })
}

/// The results portal client, made from the voting portal's. The results
/// portal does not call Harvest, so it does not get Harvest's audience.
fn results_portal_client(
    voting_portal: ClientRepresentation,
    results_portal_url: &str,
    harvest_audiences: &[String],
) -> ClientRepresentation {
    let mut client = voting_portal;
    client.id = None;
    if let Some(protocol_mappers) = client.protocol_mappers.as_mut() {
        protocol_mappers.retain(|protocol_mapper| {
            !adds_harvest_audience(protocol_mapper, harvest_audiences)
        });
        for protocol_mapper in protocol_mappers {
            protocol_mapper.id = None;
        }
    }
    client.client_id = Some(RESULTS_PORTAL_CLIENT_ID.to_string());
    client.name = Some("Results Portal".to_string());
    client.root_url = Some(results_portal_url.to_string());
    client.base_url = Some(results_portal_url.to_string());
    client.redirect_uris =
        Some(results_portal_redirect_uris(results_portal_url));
    client
}

fn include_results_portal_in_client_condition(value: &str) -> String {
    let mut client_ids = value
        .split(',')
        .map(str::trim)
        .filter(|client_id| !client_id.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    if client_ids
        .iter()
        .any(|client_id| client_id == VOTING_PORTAL_CLIENT_ID)
        && !client_ids
            .iter()
            .any(|client_id| client_id == RESULTS_PORTAL_CLIENT_ID)
    {
        client_ids.push(RESULTS_PORTAL_CLIENT_ID.to_string());
    }
    client_ids.join(",")
}

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
/// election_event_id, and compares them with the new values to determine if
/// replacements are needed.
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
    // Get the realm name
    let Some(realm_name) = realm_config.realm.as_ref() else {
        return (None, None);
    };

    // Parse the realm name to extract tenant_id and election_event_id
    let Some((old_tenant_id, old_election_event_id)) = parse_realm(realm_name)
    else {
        return (None, None);
    };

    // Determine tenant_id replacement if different
    let tenant_id_replacement =
        Some((old_tenant_id.clone(), new_tenant_id.to_string()));

    // Determine election_event_id replacement if applicable
    let election_event_id_replacement =
        match (old_election_event_id, new_election_event_id) {
            (Some(old_event_id), Some(new_event_id))
                if old_event_id != *new_event_id =>
            {
                Some((old_event_id, new_event_id.clone()))
            }
            _ => None,
        };

    (tenant_id_replacement, election_event_id_replacement)
}

/// Replaces UUIDs in a Keycloak realm JSON configuration while preserving
/// specific IDs.
///
/// This function processes the realm configuration and replaces most UUIDs with
/// new ones, while keeping certain IDs unchanged based on the provided `keep`
/// list. It also automatically preserves UUIDs referenced in Keycloak
/// authenticator configurations to maintain consistency of internal references.
///
/// # Arguments
/// * `json_realm_config` - The original JSON string representation of the realm
///   configuration
/// * `keep` - A list of UUID strings that should NOT be replaced with new ones
/// * `tenant_id_replacement` - Optional tuple of (old_tenant_id, new_tenant_id)
///   for explicit replacement
/// * `election_event_id_replacement` - Optional tuple of (old_event_id,
///   new_event_id) for explicit replacement
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

    // Find and preserve UUIDs referenced in Keycloak authenticator
    // configurations These UUIDs are references that must remain consistent
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
    let (mut new_data, replacement_map) =
        replace_uuids(json_realm_config, keep);

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

/// Generates a Keycloak-style client secret: a 32-character random alphanumeric
/// string matching the output of Keycloak's `SecretGenerator.randomSecret(32)`.
pub fn generate_client_secret() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect()
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
        let realm_get_result = self.client.realm_get(board_name).await;
        let replaced_ids_config = if replace_ids {
            let realm_config: RealmRepresentation =
                deserialize_str(&json_realm_config)?;
            let (tenant_id_replacement, election_event_id_replacement) =
                extract_realm_replacements(
                    &realm_config,
                    tenant_id,
                    &election_event_id,
                );

            let (result, _) = replace_realm_ids(
                json_realm_config,
                vec![],
                tenant_id_replacement,
                election_event_id_replacement,
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
        let kiosk_voting_portal_url_env = kiosk_voting_portal_url(
            &voting_portal_url_env,
            env::var("KIOSK_VOTING_PORTAL_URL").ok(),
        );
        let login_url = if let Some(election_event_id) =
            election_event_id.as_ref()
        {
            Some(format!("{voting_portal_url_env}/tenant/{tenant_id}/event/{election_event_id}/login"))
        } else {
            None
        };
        let kiosk_login_url = election_event_id.as_ref().map(|event_id| {
            format!(
                "{kiosk_voting_portal_url_env}/tenant/{tenant_id}/event/{event_id}/login?kiosk"
            )
        });
        let ballot_verifier_url = env::var("BALLOT_VERIFIER_URL")
            .with_context(|| "Error fetching BALLOT_VERIFIER_URL env var")?;
        let results_portal_url = env::var("RESULTS_PORTAL_URL").ok();

        let mut clients = realm.clients.take().unwrap_or_default();
        let voting_portal_template = clients
            .iter()
            .find(|client| {
                client.client_id.as_deref() == Some(VOTING_PORTAL_CLIENT_ID)
            })
            .cloned();
        for client in &mut clients {
            match client.client_id.as_deref() {
                Some(VOTING_PORTAL_CLIENT_ID)
                | Some(ONSITE_VOTING_PORTAL_CLIENT_ID) => {
                    client.root_url = Some(voting_portal_url_env.clone());
                    client.base_url = login_url.clone();
                    client.redirect_uris =
                        Some(voting_portal_redirect_uris(&ballot_verifier_url));
                }
                Some(KIOSK_VOTING_PORTAL_CLIENT_ID) => {
                    client.root_url = Some(kiosk_voting_portal_url_env.clone());
                    client.base_url = kiosk_login_url.clone();
                    client.redirect_uris =
                        Some(voting_portal_redirect_uris(&ballot_verifier_url));
                }
                Some(RESULTS_PORTAL_CLIENT_ID) => {
                    if let Some(results_portal_url) =
                        results_portal_url.as_ref()
                    {
                        client.root_url = Some(results_portal_url.clone());
                        client.base_url = Some(results_portal_url.clone());
                        client.redirect_uris = Some(
                            results_portal_redirect_uris(results_portal_url),
                        );
                    }
                }
                // When an Action Token expires, for example a Manual
                // Verification QR Code, the `Back to Application` link in
                // the resulting error page will redirect to the `base_url`.
                // Related: https://github.com/sequentech/meta/issues/5063
                Some("account") if login_url.is_some() => {
                    client.base_url = login_url.clone();
                }
                _ => {}
            }
        }

        if let Some(results_portal_url) = results_portal_url {
            if !clients.iter().any(|client| {
                client.client_id.as_deref() == Some(RESULTS_PORTAL_CLIENT_ID)
            }) {
                let voting_portal = voting_portal_template.ok_or_else(|| {
                    anyhow!(
                        "Event realm does not contain a voting portal client template"
                    )
                })?;
                let harvest_audiences = results_portal_harvest_audiences(
                    env::var(AUDIENCES_ENV).ok().as_deref(),
                )?;
                clients.push(results_portal_client(
                    voting_portal,
                    &results_portal_url,
                    &harvest_audiences,
                ));
            }

            if let Some(authenticator_configs) =
                realm.authenticator_config.as_mut()
            {
                for authenticator_config in authenticator_configs {
                    if let Some(config) = authenticator_config.config.as_mut() {
                        if let Some(value) =
                            config.get_mut(CONDITIONAL_CLIENT_ID)
                        {
                            *value = include_results_portal_in_client_condition(
                                value,
                            );
                        }
                    }
                }
            }
        }
        realm.clients = Some(clients);

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

        match realm_get_result {
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

#[cfg(test)]
mod tests {
    use super::{
        include_results_portal_in_client_condition, kiosk_voting_portal_url,
        results_portal_client, results_portal_harvest_audiences,
        results_portal_redirect_uris, voting_portal_redirect_uris,
    };
    use crate::types::keycloak::HARVEST_AUDIENCE_MAPPER_NAME;
    use keycloak::types::ClientRepresentation;
    use serde_json::json;

    fn audience_mapper(
        name: &str,
        setting: &str,
        audience: &str,
    ) -> serde_json::Value {
        json!({
            "id": format!("{name}-id"),
            "name": name,
            "protocol": "openid-connect",
            "protocolMapper": "oidc-audience-mapper",
            "config": {setting: audience, "access.token.claim": "true"}
        })
    }

    fn results_portal_mapper_names(
        mappers: Vec<serde_json::Value>,
    ) -> Vec<String> {
        let voting_portal: ClientRepresentation =
            serde_json::from_value(json!({
                "id": "client-id",
                "clientId": "voting-portal",
                "protocolMappers": mappers
            }))
            .unwrap();
        let results_portal = results_portal_client(
            voting_portal,
            "https://results.example.test",
            &["harvest".to_string(), "harvest-kiosk".to_string()],
        );
        assert_eq!(results_portal.client_id.as_deref(), Some("results-portal"));
        assert_eq!(results_portal.id, None);
        let mappers = results_portal.protocol_mappers.unwrap();
        assert!(mappers.iter().all(|m| m.id.is_none()));
        mappers.iter().filter_map(|m| m.name.clone()).collect()
    }

    #[test]
    fn the_results_portal_does_not_get_harvests_audience() {
        let names = results_portal_mapper_names(vec![
            json!({
                "id": "tenant-id",
                "name": "x-hasura-tenant-id",
                "protocol": "openid-connect",
                "protocolMapper": "oidc-usermodel-attribute-mapper"
            }),
            audience_mapper(
                HARVEST_AUDIENCE_MAPPER_NAME,
                "included.custom.audience",
                "harvest",
            ),
            // Added by an operator under other names.
            audience_mapper("Harvest", "included.custom.audience", "harvest"),
            audience_mapper(
                "kiosk",
                "included.custom.audience",
                " harvest-kiosk ",
            ),
            audience_mapper(
                "harvest-client",
                "included.client.audience",
                "harvest",
            ),
            // Audience mappers for other services stay.
            audience_mapper(
                "ballot-verifier",
                "included.custom.audience",
                "verifier",
            ),
        ]);
        assert_eq!(names, ["x-hasura-tenant-id", "ballot-verifier"]);
    }

    #[test]
    fn copying_the_results_portal_needs_harvests_audience_setting() {
        for setting in [None, Some(""), Some(" , "), Some("account")] {
            let error = results_portal_harvest_audiences(setting)
                .expect_err("an invalid setting is refused")
                .to_string();
            assert!(error.contains("HARVEST_JWT_AUDIENCES"), "{error}");
            assert!(error.contains("results portal"), "{error}");
        }
        assert_eq!(
            results_portal_harvest_audiences(Some("harvest, harvest-kiosk"))
                .unwrap(),
            ["harvest", "harvest-kiosk"]
        );
    }

    #[test]
    fn mappers_of_other_kinds_naming_harvest_are_kept() {
        let names = results_portal_mapper_names(vec![json!({
            "id": "claim-id",
            "name": "service",
            "protocol": "openid-connect",
            "protocolMapper": "oidc-hardcoded-claim-mapper",
            "config": {
                "included.custom.audience": "harvest",
                "claim.value": "harvest"
            }
        })]);
        assert_eq!(names, ["service"]);
    }

    #[test]
    fn voting_portal_redirect_uris_exclude_the_results_portal() {
        assert_eq!(
            voting_portal_redirect_uris("https://verifier.example.test/"),
            vec!["/*", "https://verifier.example.test/*"]
        );
    }

    #[test]
    fn results_portal_redirect_uris_are_scoped_to_results() {
        assert_eq!(
            results_portal_redirect_uris("https://results.example.test/"),
            vec!["https://results.example.test/*"]
        );
    }

    #[test]
    fn kiosk_voting_portal_url_uses_configured_url() {
        assert_eq!(
            kiosk_voting_portal_url(
                "https://voting.example.test",
                Some("https://voting-kiosk.example.test".to_string()),
            ),
            "https://voting-kiosk.example.test"
        );
    }

    #[test]
    fn kiosk_voting_portal_url_falls_back_to_voting_portal_url() {
        assert_eq!(
            kiosk_voting_portal_url(
                "https://voting.example.test",
                Some("  ".to_string()),
            ),
            "https://voting.example.test"
        );
    }

    #[test]
    fn results_portal_is_added_once_to_client_condition() {
        assert_eq!(
            include_results_portal_in_client_condition("voting-portal"),
            "voting-portal,results-portal"
        );
        assert_eq!(
            include_results_portal_in_client_condition(
                "voting-portal,results-portal"
            ),
            "voting-portal,results-portal"
        );
    }
}
