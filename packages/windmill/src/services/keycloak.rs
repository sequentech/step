// SPDX-FileCopyrightText: 2024 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::types::error::Result;
use anyhow::{anyhow, Context};
use keycloak::types::{GroupRepresentation, RealmRepresentation, RoleRepresentation};
use sequent_core::services::keycloak::KeycloakAdminClient;
use std::collections::HashSet;
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
                        role.id.as_ref().unwrap(),
                    )
                    .await?;
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
                        group.id.as_ref().unwrap(),
                    )
                    .await?;
                println!("Deleted group: {}", name);
            } else {
                // Update the group id in new_realm_groups
                new_realm_groups
                    .iter_mut()
                    .find(|g| g.name == group.name)
                    .unwrap()
                    .id = group.id.clone();
            }
        }
    }

    Ok(())
}

#[instrument(err, skip_all)]
pub async fn read_roles_config_file(
    temp_file: NamedTempFile,
    realm: &RealmRepresentation,
    tenant_id: &str,
) -> Result<()> {
    let (container_id, existing_realm_groups, existing_realm_roles) = map_realm_data(realm);
    let mut reader = csv::Reader::from_path(temp_file.path())
        .map_err(|e| anyhow!("Error reading roles and permissions config file: {e}"))?;

    let headers = reader
        .headers()
        .map(|headers| headers.clone())
        .map_err(|err| anyhow!("Error reading CSV headers: {err:?}"))?;

    let mut realm_groups: Vec<GroupRepresentation> = vec![];
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

        // Add GroupRepresentation object to realm_groups
        let group = GroupRepresentation {
            id: Some(Uuid::new_v4().to_string()),
            name: Some(role.clone().to_string()),
            path: Some(format!("/{}", role.clone())),
            realm_roles: Some(permissions),
            ..Default::default()
        };
        realm_groups.push(group);
    }

    // Delete existing roles and groups
    delete_realm_groups_and_roles(
        &existing_realm_groups,
        &existing_realm_roles,
        &mut realm_groups,
        &mut realm_roles,
        tenant_id,
    )
    .await?;

    // Import realm groups and roles
    let if_resource_exists = "OVERWRITE";
    let keycloak_client = KeycloakAdminClient::new().await?;
    let pub_keycloak_client = KeycloakAdminClient::pub_new().await?;

    keycloak_client
        .partial_import_realm_with_cleanup(
            &pub_keycloak_client,
            tenant_id,
            &container_id.unwrap_or_default(),
            realm_groups,
            realm_roles,
            if_resource_exists,
        )
        .await
        .map_err(|e| anyhow!("Error importing realm: {e}"))?;

    Ok(())
}
