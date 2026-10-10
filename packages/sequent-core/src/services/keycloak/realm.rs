// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::serialization::deserialize_with_path::deserialize_str;
use crate::services::uuid_validation::parse_uuid_v4;
use crate::services::{
    keycloak::KeycloakAdminClient, replace_uuids::replace_uuids,
};
use crate::types::keycloak::{Role, TENANT_ID_ATTR_NAME};
use anyhow::{anyhow, Context, Result};
use keycloak::types::{
    AuthenticationExecutionInfoRepresentation, ClientRepresentation,
    GroupRepresentation, RealmRepresentation, RoleRepresentation,
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

use super::PubKeycloakAdmin;

const ADMIN_PORTAL_CLIENT_ID: &str = "admin-portal";
const CLI_ACCOUNT_ADMIN_CLIENT_ID: &str = "cli-account-admin";
const ADMIN_PORTAL_URL_ENV_VAR: &str = "ADMIN_PORTAL_URL";
const PKCE_CODE_CHALLENGE_METHOD_ATTR: &str = "pkce.code.challenge.method";
const PKCE_S256_METHOD: &str = "S256";
const POST_LOGOUT_REDIRECT_URIS_ATTR: &str = "post.logout.redirect.uris";
const REQUEST_URIS_ATTR: &str = "request.uris";
const SAME_AS_REDIRECT_URIS: &str = "+";

fn admin_portal_redirect_uris(admin_portal_url: &str) -> Vec<String> {
    vec![format!("{}/*", admin_portal_url.trim_end_matches('/'))]
}

/// Returns the configured admin portal URL for a tenant realm, where it is
/// required, and `None` for any other realm.
fn admin_portal_url_for_realm(
    realm_name: &str,
    configured_url: Option<String>,
) -> Result<Option<String>> {
    let Some((_, None)) = parse_realm(realm_name) else {
        return Ok(None);
    };
    configured_url
        .as_deref()
        .map(str::trim)
        .filter(|url| !url.is_empty())
        .map(|url| Some(url.to_string()))
        .ok_or_else(|| {
            anyhow!("Error fetching {ADMIN_PORTAL_URL_ENV_VAR} env var")
        })
}

fn set_client_attributes(
    client: &mut ClientRepresentation,
    attributes_to_set: &[(&str, &str)],
    attributes_to_remove: &[&str],
) {
    let attributes = client.attributes.get_or_insert_with(HashMap::new);
    for (name, value) in attributes_to_set {
        attributes.insert(name.to_string(), value.to_string());
    }
    for name in attributes_to_remove {
        attributes.remove(*name);
    }
}

/// Limits the browser redirects of the tenant realm clients that issue tokens
/// carrying the user's realm roles: `admin-portal` to the admin portal, and
/// `cli-account-admin`, which is only used for direct access grants, to none.
fn scope_tenant_realm_clients(
    clients: &mut [ClientRepresentation],
    admin_portal_url: &str,
) {
    for client in clients {
        match client.client_id.as_deref() {
            Some(ADMIN_PORTAL_CLIENT_ID) => {
                client.root_url = Some(admin_portal_url.to_string());
                client.base_url = Some(admin_portal_url.to_string());
                client.redirect_uris =
                    Some(admin_portal_redirect_uris(admin_portal_url));
                client.web_origins = Some(vec![SAME_AS_REDIRECT_URIS.into()]);
                set_client_attributes(
                    client,
                    &[
                        (PKCE_CODE_CHALLENGE_METHOD_ATTR, PKCE_S256_METHOD),
                        (POST_LOGOUT_REDIRECT_URIS_ATTR, SAME_AS_REDIRECT_URIS),
                    ],
                    &[REQUEST_URIS_ATTR],
                );
            }
            Some(CLI_ACCOUNT_ADMIN_CLIENT_ID) => {
                client.redirect_uris = Some(vec![]);
                client.web_origins = Some(vec![]);
                set_client_attributes(
                    client,
                    &[(POST_LOGOUT_REDIRECT_URIS_ATTR, SAME_AS_REDIRECT_URIS)],
                    &[REQUEST_URIS_ATTR],
                );
            }
            _ => {}
        }
    }
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
        let ballot_verifier_url = env::var("BALLOT_VERIFIER_URL")
            .with_context(|| "Error fetching BALLOT_VERIFIER_URL env var")?;
        let admin_portal_url = admin_portal_url_for_realm(
            board_name,
            env::var(ADMIN_PORTAL_URL_ENV_VAR).ok(),
        )?;

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

        if let Some(admin_portal_url) = admin_portal_url.as_deref() {
            let mut clients = realm.clients.take().unwrap_or_default();
            scope_tenant_realm_clients(&mut clients, admin_portal_url);
            realm.clients = Some(clients);
        }

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

#[cfg(test)]
mod tests {
    use super::{
        admin_portal_redirect_uris, admin_portal_url_for_realm,
        scope_tenant_realm_clients, ADMIN_PORTAL_CLIENT_ID,
        CLI_ACCOUNT_ADMIN_CLIENT_ID, PKCE_CODE_CHALLENGE_METHOD_ATTR,
        PKCE_S256_METHOD, POST_LOGOUT_REDIRECT_URIS_ATTR, REQUEST_URIS_ATTR,
        SAME_AS_REDIRECT_URIS,
    };
    use crate::serialization::deserialize_with_path::deserialize_str;
    use keycloak::types::{ClientRepresentation, RealmRepresentation};
    use std::collections::HashMap;

    const TENANT_REALM_TEMPLATE: &str = include_str!(
        "../../../../../.devcontainer/keycloak/import/tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5.json"
    );
    const TENANT_REALM: &str = "tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5";
    const EVENT_REALM: &str = "tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5-event-33f18502-a67c-4853-8333-a58630663559";
    const ADMIN_PORTAL_URL: &str = "https://admin.example.test";
    const ANY_URI: &str = "*";

    fn find_client<'a>(
        clients: &'a [ClientRepresentation],
        client_id: &str,
    ) -> &'a ClientRepresentation {
        clients
            .iter()
            .find(|client| client.client_id.as_deref() == Some(client_id))
            .unwrap_or_else(|| panic!("missing client {client_id}"))
    }

    fn client_accepting_any_uri(client_id: &str) -> ClientRepresentation {
        ClientRepresentation {
            client_id: Some(client_id.to_string()),
            root_url: Some("http://127.0.0.1:3002/".to_string()),
            base_url: Some("http://127.0.0.1:3002/".to_string()),
            redirect_uris: Some(vec![ANY_URI.to_string()]),
            web_origins: Some(vec![ANY_URI.to_string()]),
            attributes: Some(HashMap::from([
                (
                    POST_LOGOUT_REDIRECT_URIS_ATTR.to_string(),
                    ANY_URI.to_string(),
                ),
                (REQUEST_URIS_ATTR.to_string(), ANY_URI.to_string()),
            ])),
            ..Default::default()
        }
    }

    fn attribute<'a>(
        client: &'a ClientRepresentation,
        name: &str,
    ) -> Option<&'a str> {
        client
            .attributes
            .as_ref()
            .and_then(|attributes| attributes.get(name))
            .map(String::as_str)
    }

    #[test]
    fn admin_portal_redirect_uris_are_scoped_to_admin_portal() {
        assert_eq!(
            admin_portal_redirect_uris("https://admin.example.test/"),
            vec!["https://admin.example.test/*"]
        );
    }

    #[test]
    fn admin_portal_url_is_required_for_tenant_realms() {
        assert!(admin_portal_url_for_realm(TENANT_REALM, None).is_err());
        assert!(admin_portal_url_for_realm(
            TENANT_REALM,
            Some("  ".to_string())
        )
        .is_err());
        assert_eq!(
            admin_portal_url_for_realm(
                TENANT_REALM,
                Some(format!(" {ADMIN_PORTAL_URL} "))
            )
            .ok(),
            Some(Some(ADMIN_PORTAL_URL.to_string()))
        );
    }

    #[test]
    fn admin_portal_url_is_not_used_for_event_realms() {
        assert_eq!(
            admin_portal_url_for_realm(EVENT_REALM, None).ok(),
            Some(None)
        );
    }

    #[test]
    fn tenant_realm_admin_portal_client_is_scoped_to_admin_portal_url() {
        let mut clients =
            vec![client_accepting_any_uri(ADMIN_PORTAL_CLIENT_ID)];
        scope_tenant_realm_clients(&mut clients, ADMIN_PORTAL_URL);
        let admin_portal = find_client(&clients, ADMIN_PORTAL_CLIENT_ID);

        assert_eq!(admin_portal.root_url.as_deref(), Some(ADMIN_PORTAL_URL));
        assert_eq!(admin_portal.base_url.as_deref(), Some(ADMIN_PORTAL_URL));
        assert_eq!(
            admin_portal.redirect_uris,
            Some(vec![format!("{ADMIN_PORTAL_URL}/*")])
        );
        assert_eq!(
            admin_portal.web_origins,
            Some(vec![SAME_AS_REDIRECT_URIS.to_string()])
        );
        assert_eq!(
            attribute(admin_portal, PKCE_CODE_CHALLENGE_METHOD_ATTR),
            Some(PKCE_S256_METHOD)
        );
        assert_eq!(
            attribute(admin_portal, POST_LOGOUT_REDIRECT_URIS_ATTR),
            Some(SAME_AS_REDIRECT_URIS)
        );
        assert_eq!(attribute(admin_portal, REQUEST_URIS_ATTR), None);
    }

    #[test]
    fn tenant_realm_cli_client_accepts_no_browser_redirects() {
        let mut clients =
            vec![client_accepting_any_uri(CLI_ACCOUNT_ADMIN_CLIENT_ID)];
        scope_tenant_realm_clients(&mut clients, ADMIN_PORTAL_URL);
        let cli_client = find_client(&clients, CLI_ACCOUNT_ADMIN_CLIENT_ID);

        assert_eq!(cli_client.redirect_uris, Some(vec![]));
        assert_eq!(cli_client.web_origins, Some(vec![]));
        assert_eq!(
            attribute(cli_client, POST_LOGOUT_REDIRECT_URIS_ATTR),
            Some(SAME_AS_REDIRECT_URIS)
        );
        assert_eq!(attribute(cli_client, REQUEST_URIS_ATTR), None);
    }

    #[test]
    fn tenant_realm_scoping_leaves_other_clients_unchanged() {
        let other_client = client_accepting_any_uri("api-key-client");
        let mut clients = vec![other_client.clone()];
        scope_tenant_realm_clients(&mut clients, ADMIN_PORTAL_URL);

        assert_eq!(clients, vec![other_client]);
    }

    #[test]
    fn tenant_realm_template_admin_clients_list_explicit_redirect_uris() {
        let realm: RealmRepresentation =
            deserialize_str(TENANT_REALM_TEMPLATE).expect("tenant template");
        let clients = realm.clients.unwrap_or_default();

        for client_id in [ADMIN_PORTAL_CLIENT_ID, CLI_ACCOUNT_ADMIN_CLIENT_ID] {
            let client = find_client(&clients, client_id);
            for uris in [&client.redirect_uris, &client.web_origins] {
                assert!(
                    !uris.iter().flatten().any(|uri| uri == ANY_URI),
                    "{client_id} lists {uris:?}"
                );
            }
            assert_ne!(
                attribute(client, POST_LOGOUT_REDIRECT_URIS_ATTR),
                Some(ANY_URI),
                "{client_id} post logout redirect URIs"
            );
            assert_eq!(
                attribute(client, REQUEST_URIS_ATTR),
                None,
                "{client_id} request URIs"
            );
        }
        assert_eq!(
            attribute(
                find_client(&clients, ADMIN_PORTAL_CLIENT_ID),
                PKCE_CODE_CHALLENGE_METHOD_ATTR
            ),
            Some(PKCE_S256_METHOD)
        );
    }
}
