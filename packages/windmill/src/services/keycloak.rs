// SPDX-FileCopyrightText: 2024 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::types::error::Result;
use anyhow::{anyhow, Context};
use keycloak::types::{GroupRepresentation, RealmRepresentation, RoleRepresentation};
use keycloak::{KeycloakAdmin, KeycloakAdminToken};
use rocket::http::Status;
use sequent_core::services::keycloak::get_tenant_realm;
use sequent_core::services::keycloak::RoleAction;
use sequent_core::{services::keycloak::KeycloakAdminClient, types::keycloak::Role};
use std::collections::{HashMap, HashSet};
use tempfile::NamedTempFile;
use tracing::{event, info, instrument, Level};
use uuid::Uuid;

pub fn map_realm_data(
    realm: &RealmRepresentation,
) -> (
    Option<String>,
    Vec<GroupRepresentation>,
    Vec<RoleRepresentation>,
) {
    let container_id = realm.id.clone();
    let existing_roles = realm
        .roles
        .clone()
        .unwrap_or_default()
        .realm
        .unwrap_or(vec![]);
    let existing_groups = realm.groups.clone().unwrap_or(vec![]);
    return (container_id, existing_groups, existing_roles);
}

#[instrument(err)]
pub async fn delete_realm_groups_and_roles(
    existing_groups: &Vec<GroupRepresentation>,
    existing_roles: &Vec<RoleRepresentation>,
    new_realm_groups: &mut Vec<GroupRepresentation>,
    new_realm_roles: &mut Vec<RoleRepresentation>,
    tenant_id: &str,
) -> Result<()> {
    let keycloak_client = KeycloakAdminClient::new().await?;
    let pub_keycloak_client = KeycloakAdminClient::pub_new().await?;

    let imported_role_names: HashSet<String> = new_realm_roles
        .iter()
        .filter_map(|r| r.name.clone())
        .collect();

    let imported_group_names: HashSet<String> = new_realm_groups
        .iter()
        .filter_map(|g| g.name.clone())
        .collect();

    for role in existing_roles.iter() {
        if let Some(name) = &role.name {
            if !imported_role_names.contains(name) {
                keycloak_client
                    .realm_delete(
                        &pub_keycloak_client,
                        tenant_id,
                        "roles-by-id",
                        role.id.as_ref().ok_or(anyhow!("Empty role id"))?,
                    )
                    .await
                    .map_err(|e| anyhow!("Failed to send request: {:?}", e))?;
                println!("Deleted role: {}", name);
            }
        }
    }

    for group in existing_groups.iter() {
        if let Some(name) = &group.name {
            if !imported_group_names.contains(name) {
                keycloak_client
                    .realm_delete(
                        &pub_keycloak_client,
                        tenant_id,
                        "groups",
                        group.id.as_ref().ok_or(anyhow!("Empty role id"))?,
                    )
                    .await?;
                println!("Deleted group: {}", name);
            } else {
                // Update the group id in new_realm_groups
                new_realm_groups
                    .iter_mut()
                    .find(|g| g.name == group.name)
                    .ok_or(anyhow!("Can't find realm group"))?
                    .id = group.id.clone();
            }
        }
    }

    Ok(())
}

pub fn find_group_by_name(
    groups: &[GroupRepresentation],
    group_name: &str,
) -> Option<GroupRepresentation> {
    groups
        .iter()
        .cloned()
        .find(|group| group.name.as_deref() == Some(group_name))
}

#[instrument(err, skip_all)]
pub async fn read_roles_config_file(
    temp_file: NamedTempFile,
    realm: &RealmRepresentation,
    tenant_id: &str,
) -> Result<()> {
    let keycloak_pub_client = KeycloakAdminClient::pub_new().await?;
    let keycloak_client = KeycloakAdminClient::new()
        .await
        .map_err(|e| anyhow!("Failed to create Keycloak client: {:?}", e))?;
    let (container_id, existing_realm_groups, existing_realm_roles) = map_realm_data(realm);
    let mut reader = csv::Reader::from_path(temp_file.path())
        .map_err(|e| anyhow!("Error reading roles and permissions config file: {e}"))?;

    info!("existing_realm_groups: {:?}", existing_realm_groups);
    let mut realm_roles: Vec<RoleRepresentation> = vec![];
    let mut existing_permissions: HashSet<String> = HashSet::new();
    for result in reader.records() {
        let record = result.map_err(|e| anyhow!("Error reading CSV record: {e:?}"))?;
        let role: String = record
            .get(0)
            .ok_or_else(|| anyhow!("Role not found"))?
            .to_string();
        let permissions_str: String = record
            .get(1)
            .ok_or_else(|| anyhow!("Permissions not found"))?
            .to_string();
        let permissions: Vec<String> = permissions_str
            .split("|")
            .map(|permission| {
                // Ensure adding unique permissions using the HashSet
                if existing_permissions.insert(permission.to_string()) {
                    realm_roles.push(RoleRepresentation {
                        id: Some(Uuid::new_v4().to_string()),
                        name: Some(permission.to_string()),
                        container_id: container_id.clone(),
                        description: None,
                        composite: Some(false),
                        composites: None,
                        client_role: Some(false),
                        ..Default::default()
                    });
                }

                permission.to_string()
            })
            .collect();

        let existing_roles_map: HashMap<String, RoleRepresentation> = existing_realm_roles
            .clone()
            .into_iter()
            .filter_map(|r| r.name.clone().map(|name| (name, r)))
            .collect();

        if let Some(group) = find_group_by_name(&existing_realm_groups, &role) {
            let current_group_roles = keycloak_client
                .get_group_assigned_roles(
                    &tenant_id,
                    group.id.as_deref().unwrap_or_default(),
                    &keycloak_pub_client,
                )
                .await
                .map_err(|e| anyhow!("Failed to get group assigned roles: {:?}", e))?;

            let current_role_names: HashSet<String> = current_group_roles
                .iter()
                .filter_map(|r| r.name.clone())
                .collect();

            let target_role_names: HashSet<String> = permissions.iter().cloned().collect();

            // Determine names to add vs remove
            let to_add_names: Vec<String> = target_role_names
                .difference(&current_role_names)
                .cloned()
                .collect();

            let to_remove_names: Vec<String> = current_role_names
                .difference(&target_role_names)
                .cloned()
                .collect();

            // Convert role names → RoleRepresentation
            let to_add: Vec<RoleRepresentation> = to_add_names
                .iter()
                .filter_map(|role_name| existing_roles_map.get(role_name))
                .cloned()
                .collect();

            let to_remove: Vec<RoleRepresentation> = to_remove_names
                .iter()
                .filter_map(|role_name| existing_roles_map.get(role_name))
                .cloned()
                .collect();

            // Add missing roles
            keycloak_client
                .add_roles_to_group(
                    &tenant_id,
                    &keycloak_pub_client,
                    group.id.as_deref().unwrap_or_default(),
                    &to_add,
                    RoleAction::Add,
                )
                .await
                .with_context(|| {
                    format!(
                        "Error adding missing roles to group '{}'",
                        group.name.as_deref().unwrap_or_default()
                    )
                })?;

            // Remove unnecessary roles
            keycloak_client
                .add_roles_to_group(
                    &tenant_id,
                    &keycloak_pub_client,
                    group.id.as_deref().unwrap_or_default(),
                    &to_remove,
                    RoleAction::Remove,
                )
                .await
                .with_context(|| {
                    format!(
                        "Error removing unnecessary roles from group '{}'",
                        group.name.as_deref().unwrap_or_default()
                    )
                })?;
        } else {
            // Create new group and assign permissions
            let new_group_id = keycloak_client
                .create_new_group(&tenant_id, &role, &keycloak_pub_client)
                .await
                .with_context(|| {
                    format!("Error creating group '{}' and assigning permissions", role)
                })?;

            match new_group_id {
                Some(group_id) => {
                    keycloak_client
                        .add_roles_to_group(
                            &tenant_id,
                            &keycloak_pub_client,
                            &group_id,
                            &realm_roles,
                            RoleAction::Add,
                        )
                        .await
                        .with_context(|| format!("Error adding roles to new group '{}'", role))?;
                }
                None => {}
            }
        }
    }

    Ok(())
}

pub async fn update_keycloak_admin_golden_authentication(
    tenant_id: Option<String>,
    golden_authentication: bool,
) -> Result<()> {
    let Some(ref tenant_id) = tenant_id else {
        return Ok(());
    };

    let realm_name = get_tenant_realm(tenant_id);

    // Define authentication flows to update
    let authentication_flows = vec!["sequent browser flow"];

    // Loop through each flow to update its execution
    for flow_name in authentication_flows {
        let keycloak_client = KeycloakAdminClient::new().await?;
        let pub_client = KeycloakAdminClient::pub_new().await?;

        let flow_executions = keycloak_client
            .get_flow_executions(&pub_client, &realm_name, flow_name)
            .await
            .with_context(|| format!("Error fetching flow executions for '{}'", flow_name))?;

        for mut execution in flow_executions {
            if execution.provider_id.as_deref() == Some("auth-password-form") {
                println!("auth-password-form entered");
                let new_golden_authentication_state = if golden_authentication {
                    "REQUIRED".to_string()
                } else {
                    "DISABLED".to_string()
                };

                println!("-- auth-password-form entered ga: {golden_authentication}, ngas: {new_golden_authentication_state}");

                execution.requirement = Some(new_golden_authentication_state.clone());

                keycloak_client
                    .upsert_flow_execution(
                        &pub_client,
                        &realm_name,
                        flow_name,
                        &serde_json::to_string(&execution)?,
                    )
                    .await
                    .with_context(|| {
                        format!("Error updating flow execution for '{}'", flow_name)
                    })?;
            }
            if execution.provider_id.as_deref() == Some("allow-access-authenticator") {
                let new_golden_authentication_state = if golden_authentication {
                    "DISABLED".to_string()
                } else {
                    "REQUIRED".to_string()
                };

                println!("-- allow-access-authenticator ga: {golden_authentication}, ngas: {new_golden_authentication_state}");

                execution.requirement = Some(new_golden_authentication_state.clone());

                keycloak_client
                    .upsert_flow_execution(
                        &pub_client,
                        &realm_name,
                        flow_name,
                        &serde_json::to_string(&execution)?,
                    )
                    .await
                    .with_context(|| {
                        format!("Error updating flow execution for '{}'", flow_name)
                    })?;
            }
        }
    }

    Ok(())
}
