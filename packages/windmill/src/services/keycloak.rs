// SPDX-FileCopyrightText: 2024 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::types::error::Result;
use anyhow::{anyhow, Context};
use keycloak::types::{GroupRepresentation, RoleRepresentation};
use std::collections::HashMap;
use std::io::{Cursor, Read, Seek};
use tempfile::NamedTempFile;
use tracing::{event, info, instrument, Level};
use uuid::Uuid;

#[instrument(level = "info", skip(temp_file))]
pub async fn read_roles_config_file(temp_file: NamedTempFile, container_id: String) -> Result<()> {
    let mut reader = csv::Reader::from_path(temp_file.path())
        .map_err(|e| anyhow!("Error reading roles and permissions config file: {e}"))?;

    let headers = reader
        .headers()
        .map(|headers| headers.clone())
        .map_err(|err| anyhow!("Error reading CSV headers: {err:?}"))?;

    let mut realm_groups = HashMap::new();
    let mut realm_roles: Vec<RoleRepresentation> = vec![];
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
                // Add RoleRepresentation object to realm_roles
                // TODO: make sure im not adding duplicates
                realm_roles.push(RoleRepresentation {
                    id: Some(Uuid::new_v4().to_string()),
                    name: Some(permission.clone().to_string()),
                    container_id: Some(container_id.clone()),
                    ..Default::default()
                });
                permission.to_string()
            })
            .collect();

        // Add GroupRepresentation object to realm_groups
        let group = GroupRepresentation {
            id: Some(Uuid::new_v4().to_string()),
            name: Some(role.clone().to_string()),
            path: Some(format!("{} {}", '/', role.clone().to_string())),
            realm_roles: Some(permissions),
            ..Default::default()
        };
        realm_groups.insert(role, group);
    }
    println!("**** {:?}", realm_groups);
    println!("**** {:?}", realm_roles);

    Ok(())
}
