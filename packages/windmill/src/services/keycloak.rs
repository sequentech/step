// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::types::error::Result;
use anyhow::{anyhow, Context};
use keycloak::types::{GroupRepresentation, RealmRepresentation, RoleRepresentation};
use sequent_core::serialization::deserialize_with_path::deserialize_str;
use sequent_core::services::keycloak::KeycloakAdminClient;
use sequent_core::services::keycloak::RoleAction;
use sequent_core::services::s3::{get_file_from_s3, get_private_bucket};
use std::collections::{HashMap, HashSet};
use std::env;
use tempfile::NamedTempFile;
use tracing::instrument;

fn normalize_realm_config_s3_key(s3_key_env_var: &str, s3_key: &str) -> anyhow::Result<String> {
    let s3_key = s3_key.trim();
    if s3_key.is_empty() {
        return Err(anyhow!("{s3_key_env_var} must not be empty"));
    }
    if s3_key.starts_with('/') || s3_key.ends_with('/') {
        return Err(anyhow!("{s3_key_env_var} must not start or end with `/`"));
    }

    Ok(s3_key.to_string())
}

#[instrument(err, skip_all)]
pub async fn read_realm_config_from_s3(
    s3_key_env_var: &str,
) -> anyhow::Result<RealmRepresentation> {
    let configured_s3_key =
        env::var(s3_key_env_var).with_context(|| format!("{s3_key_env_var} must be set"))?;
    let s3_key = normalize_realm_config_s3_key(s3_key_env_var, &configured_s3_key)?;

    let s3_bucket = get_private_bucket()?;
    if s3_bucket.trim().is_empty() {
        return Err(anyhow!("AWS_S3_BUCKET must not be empty"));
    }

    let realm_config = get_file_from_s3(s3_bucket.clone(), s3_key.clone())
        .await
        .with_context(|| {
            format!(
                "Failed to read default Keycloak realm config configured by {s3_key_env_var} from S3 bucket `{s3_bucket}` at key `{s3_key}`"
            )
        })?;

    parse_realm_config(&realm_config, &s3_bucket, &s3_key)
}

fn parse_realm_config(
    realm_config: &[u8],
    s3_bucket: &str,
    s3_key: &str,
) -> anyhow::Result<RealmRepresentation> {
    let realm_config = std::str::from_utf8(realm_config).with_context(|| {
        format!(
            "Default Keycloak realm config in S3 bucket `{s3_bucket}` at key `{s3_key}` is not valid UTF-8"
        )
    })?;

    deserialize_str(realm_config).with_context(|| {
        format!(
            "Error parsing default Keycloak realm config from S3 bucket `{s3_bucket}` at key `{s3_key}` into RealmRepresentation"
        )
    })
}

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

fn parse_roles_config(reader: impl std::io::Read) -> Result<Vec<(String, Vec<String>)>> {
    let mut reader = csv::Reader::from_reader(reader);
    if reader.headers()?.iter().collect::<Vec<_>>() != ["role", "permissions"] {
        return Err(anyhow!("Expected CSV headers: role,permissions").into());
    }
    let mut roles = Vec::new();
    let mut names = HashSet::new();
    for record in reader.records() {
        let record = record?;
        let role = record.get(0).ok_or_else(|| anyhow!("Role not found"))?;
        if role.trim().is_empty() || !names.insert(role.to_string()) {
            return Err(anyhow!("Empty or duplicate role in roles config: '{role}'").into());
        }
        let mut seen = HashSet::new();
        let permissions = record
            .get(1)
            .ok_or_else(|| anyhow!("Permissions not found"))?
            .split('|')
            .filter(|name| !name.is_empty() && seen.insert(*name))
            .map(str::to_string)
            .collect();
        roles.push((role.to_string(), permissions));
    }
    Ok(roles)
}

#[instrument(err, skip_all)]
pub async fn read_roles_config_file(
    temp_file: NamedTempFile,
    realm: &RealmRepresentation,
    tenant_id: &str,
) -> Result<()> {
    // Validate every row before changing Keycloak.
    let roles = parse_roles_config(temp_file.reopen()?)?;
    let keycloak_pub_client = KeycloakAdminClient::pub_new().await?;
    let keycloak_client = KeycloakAdminClient::new().await?;
    let (_, existing_groups, existing_roles) = map_realm_data(realm);
    let realm_name = format!("tenant-{tenant_id}");
    let mut permissions_by_name: HashMap<String, RoleRepresentation> = existing_roles
        .into_iter()
        .filter_map(|role| role.name.clone().map(|name| (name, role)))
        .collect();

    // Role-mapping requests require IDs issued by the destination Keycloak.
    // Resolve all permissions before replacing any group's mappings.
    for (_, permissions) in &roles {
        for name in permissions {
            if !permissions_by_name.contains_key(name) {
                keycloak_client
                    .client
                    .realm_roles_post(
                        &realm_name,
                        RoleRepresentation {
                            name: Some(name.clone()),
                            ..Default::default()
                        },
                    )
                    .await
                    .with_context(|| format!("Error creating permission '{name}'"))?;
                let permission = keycloak_client
                    .client
                    .realm_roles_with_role_name_get(&realm_name, name)
                    .await
                    .with_context(|| format!("Error resolving permission '{name}'"))?;
                permissions_by_name.insert(name.clone(), permission);
            }
            if permissions_by_name
                .get(name)
                .and_then(|role| role.id.as_deref())
                .filter(|id| !id.is_empty())
                .is_none()
            {
                return Err(anyhow!("Missing destination ID for permission '{name}'").into());
            }
        }
    }

    for (name, permissions) in roles {
        let (group_id, current_roles) = if let Some(group) =
            find_group_by_name(&existing_groups, &name)
        {
            let group_id = group
                .id
                .filter(|id| !id.is_empty())
                .ok_or_else(|| anyhow!("Missing ID for group '{name}'"))?;
            let current_roles = keycloak_client
                .get_group_assigned_roles(tenant_id, &group_id, &keycloak_pub_client)
                .await
                .map_err(|err| anyhow!("Error reading permissions for group '{name}': {err}"))?;
            (group_id, current_roles)
        } else {
            let group_id = keycloak_client
                .create_new_group(tenant_id, &name, &keycloak_pub_client)
                .await
                .with_context(|| format!("Error creating group '{name}'"))?
                .filter(|id| !id.is_empty())
                .ok_or_else(|| anyhow!("Keycloak returned no ID for new group '{name}'"))?;
            (group_id, Vec::new())
        };
        let current_names: HashSet<_> = current_roles
            .iter()
            .filter_map(|role| role.name.as_ref())
            .collect();
        let target_names: HashSet<_> = permissions.iter().collect();
        let to_add = permissions
            .iter()
            .filter(|name| !current_names.contains(name))
            .map(|name| {
                permissions_by_name
                    .get(name)
                    .cloned()
                    .ok_or_else(|| anyhow!("Unresolved permission '{name}'"))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        let to_remove = current_roles
            .iter()
            .filter(|role| {
                role.name
                    .as_ref()
                    .is_some_and(|name| !target_names.contains(name))
            })
            .cloned()
            .collect();

        for (mappings, action) in [(to_add, RoleAction::Add), (to_remove, RoleAction::Remove)] {
            if !mappings.is_empty() {
                keycloak_client
                    .add_roles_to_group(
                        tenant_id,
                        &keycloak_pub_client,
                        &group_id,
                        &mappings,
                        action,
                    )
                    .await
                    .with_context(|| format!("Error updating permissions for group '{name}'"))?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{normalize_realm_config_s3_key, parse_realm_config};

    const S3_BUCKET: &str = "election-event-documents";
    const S3_KEY: &str = "defaults/keycloak/tenant.json";

    #[test]
    fn normalizes_realm_config_s3_key_whitespace() {
        let key = normalize_realm_config_s3_key("TEST_S3_KEY", &format!("  {S3_KEY}\t"))
            .expect("surrounding whitespace should be trimmed");

        assert_eq!(key, S3_KEY);
    }

    #[test]
    fn rejects_empty_realm_config_s3_key() {
        for key in ["", " \t "] {
            let error = normalize_realm_config_s3_key("TEST_S3_KEY", key)
                .expect_err("empty key should fail");

            assert!(format!("{error:#}").contains("TEST_S3_KEY must not be empty"));
        }
    }

    #[test]
    fn rejects_realm_config_s3_key_starting_or_ending_with_slash() {
        for key in ["/public-assets/tenant.json", "public-assets/tenant.json/"] {
            let error = normalize_realm_config_s3_key("TEST_S3_KEY", key)
                .expect_err("slash-delimited key should fail");

            assert!(format!("{error:#}").contains("TEST_S3_KEY must not start or end with `/`"));
        }
    }

    #[test]
    fn parses_realm_config() {
        let realm = parse_realm_config(br#"{"realm":"tenant-test"}"#, S3_BUCKET, S3_KEY)
            .expect("realm config should parse");

        assert_eq!(realm.realm.as_deref(), Some("tenant-test"));
    }

    #[test]
    fn invalid_utf8_error_names_bucket_and_key() {
        let error =
            parse_realm_config(&[0xff], S3_BUCKET, S3_KEY).expect_err("invalid UTF-8 should fail");
        let error = format!("{error:#}");

        assert!(error.contains(S3_BUCKET));
        assert!(error.contains(S3_KEY));
    }

    #[test]
    fn invalid_json_error_names_bucket_and_key() {
        let error =
            parse_realm_config(b"{", S3_BUCKET, S3_KEY).expect_err("invalid JSON should fail");
        let error = format!("{error:#}");

        assert!(error.contains(S3_BUCKET));
        assert!(error.contains(S3_KEY));
    }

    #[test]
    fn validates_roles_config_before_import() {
        use super::parse_roles_config;

        let roles =
            parse_roles_config(b"role,permissions\nclerk,read|read\nempty,\n".as_slice()).unwrap();
        assert_eq!(
            roles,
            vec![
                ("clerk".to_string(), vec!["read".to_string()]),
                ("empty".to_string(), vec![]),
            ]
        );
        for csv in [
            "permissions,role\nread,clerk\n",
            "role,permissions\nclerk,read\nbroken\n",
            "role,permissions\n,read\n",
            "role,permissions\nclerk,read\nclerk,write\n",
        ] {
            assert!(parse_roles_config(csv.as_bytes()).is_err());
        }
    }

    #[tokio::test]
    #[ignore = "requires a development Keycloak; creates and deletes a disposable realm"]
    async fn imports_roles_with_destination_permission_ids() -> anyhow::Result<()> {
        use super::*;
        use std::io::Write;
        use uuid::Uuid;

        let tenant_id = format!("roles-import-test-{}", Uuid::new_v4());
        let realm_name = format!("tenant-{tenant_id}");
        let client = KeycloakAdminClient::new().await?;
        let public_client = KeycloakAdminClient::pub_new().await?;
        let fixture: RealmRepresentation = serde_json::from_value(serde_json::json!({
            "realm": realm_name,
            "enabled": true,
            "roles": {"realm": [{"name": "read"}, {"name": "old"}]},
            "groups": [
                {"name": "existing", "realmRoles": ["old"]},
                {"name": "untouched", "realmRoles": ["old"]}
            ]
        }))?;
        client.client.post(fixture).await?;

        // Keep cleanup outside the assertions so failures also delete the realm.
        let result: anyhow::Result<()> = async {
            let initial = KeycloakAdminClient::new()
                .await?
                .get_realm(&public_client, &realm_name)
                .await?;
            let existing_id = find_group_by_name(&initial.groups.unwrap_or_default(), "existing")
                .context("Missing existing fixture group")?
                .id;
            for csv in [
                "role,permissions\nclerk,read\nauditor,custom\nexisting,custom\nempty,\n",
                "role,permissions\nclerk,read\nauditor,custom\nexisting,custom\nempty,\n",
                "role,permissions\nclerk,\n",
            ] {
                let realm = KeycloakAdminClient::new()
                    .await?
                    .get_realm(&public_client, &realm_name)
                    .await?;
                let mut file = NamedTempFile::new()?;
                file.write_all(csv.as_bytes())?;
                read_roles_config_file(file, &realm, &tenant_id)
                    .await
                    .map_err(|err| anyhow!("Import failed: {err:?}"))?;
                let imported = KeycloakAdminClient::new()
                    .await?
                    .get_realm(&public_client, &realm_name)
                    .await?;
                let groups = imported.groups.unwrap_or_default();
                for (name, expected) in [
                    (
                        "clerk",
                        if csv == "role,permissions\nclerk,\n" {
                            vec![]
                        } else {
                            vec!["read"]
                        },
                    ),
                    ("auditor", vec!["custom"]),
                    ("existing", vec!["custom"]),
                    ("untouched", vec!["old"]),
                    ("empty", vec![]),
                ] {
                    let group = find_group_by_name(&groups, name)
                        .ok_or_else(|| anyhow!("Missing group {name}"))?;
                    anyhow::ensure!(
                        group.realm_roles.unwrap_or_default() == expected,
                        "Incorrect permissions for {name}"
                    );
                }
                anyhow::ensure!(
                    find_group_by_name(&groups, "existing")
                        .context("Missing existing group")?
                        .id
                        == existing_id,
                    "Existing group identity changed"
                );
            }
            Ok(())
        }
        .await;

        client.client.realm_delete(&realm_name).await?;
        result
    }
}
