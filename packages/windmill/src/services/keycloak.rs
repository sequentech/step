// SPDX-FileCopyrightText: 2024 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::types::error::Result;
use anyhow::{anyhow, Context};
use keycloak::types::{GroupRepresentation, RoleRepresentation};
use sequent_core::services::keycloak::KeycloakAdminClient;
use std::collections::HashSet;
use std::io::{Cursor, Read, Seek};
use tempfile::NamedTempFile;
use tracing::{event, info, instrument, Level};
use uuid::Uuid;

#[instrument(err, skip_all)]
pub async fn read_roles_config_file(
    temp_file: NamedTempFile,
    container_id: String,
    tenant_id: &str,
) -> Result<()> {
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
                        container_id: Some(container_id.clone()),
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

    // TODO: make call to delete all (or the ones that are not in the file) realm_groups and realm_roles

    let if_resource_exists = "OVERWRITE";
    let keycloak_client = KeycloakAdminClient::new().await?;
    let pub_keycloak_client = KeycloakAdminClient::pub_new().await?;

    keycloak_client
        .partial_import_realm_with_cleanup(
            &pub_keycloak_client,
            tenant_id,
            &container_id,
            realm_groups,
            realm_roles,
            if_resource_exists,
        )
        .await
        .map_err(|e| anyhow!("Error importing realm: {e}"))?;

    Ok(())
}
