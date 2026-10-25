// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! What each section of a delivery would change on the target, computed from
//! the target's current state. Pure: the platform target reads the state and
//! calls these, so preview and convergence checks are the same code.

use super::configuration_package_apply::SectionPreview;
use anyhow::{anyhow, Context, Result};
use sequent_core::election_config::tenant_package::{TenantConfiguration, TrusteePublic};
use sequent_core::types::hasura::core::{Tenant, Trustee};
use sequent_core::types::permissions::RealmRolePolicy;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

/// One row of `export_permissions-<tenant>.csv`: a group and exactly its roles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupRoles {
    pub group: String,
    pub roles: BTreeSet<String>,
}

/// Read the roles file the same way `read_roles_config_file` does: a header,
/// then `group,role|role`; an empty cell is a group with no roles.
pub fn read_group_roles(csv_bytes: &[u8]) -> Result<Vec<GroupRoles>> {
    let mut reader = csv::Reader::from_reader(csv_bytes);
    let mut rows = Vec::new();
    for record in reader.records() {
        let record = record.context("Error reading the roles file")?;
        let group = record
            .get(0)
            .ok_or_else(|| anyhow!("Role not found"))?
            .to_string();
        let roles = record
            .get(1)
            .ok_or_else(|| anyhow!("Permissions not found"))?
            .split('|')
            .filter(|role| !role.is_empty())
            .map(str::to_string)
            .collect();
        rows.push(GroupRoles { group, roles });
    }
    Ok(rows)
}

/// The role section's changes. `current` maps each existing group to the
/// realm roles assigned to it; `realm_roles` are the roles the realm defines.
/// Like the importer, a listed group gets exactly its roles (additions and
/// removals) and groups not listed are left alone.
pub fn role_changes(
    wanted: &[GroupRoles],
    current: &BTreeMap<String, BTreeSet<String>>,
    realm_roles: &BTreeSet<String>,
) -> SectionPreview {
    let mut blockers = Vec::new();
    let mut changes = Vec::new();
    let mut needs_apply = false;
    for row in wanted {
        let unknown: Vec<&String> = row
            .roles
            .iter()
            .filter(|role| !realm_roles.contains(*role))
            .collect();
        if !unknown.is_empty() {
            blockers.push(format!(
                "group '{}' requests roles the realm does not define: {}",
                row.group,
                unknown
                    .iter()
                    .map(|r| r.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        for role in &row.roles {
            if let Err(reserved) = RealmRolePolicy::require_ordinary(role) {
                blockers.push(format!("group '{}': {reserved}", row.group));
            }
        }
        match current.get(&row.group) {
            None => {
                needs_apply = true;
                changes.push(json!({
                    "group": row.group, "action": "create",
                    "add": row.roles, "remove": [],
                }));
            }
            Some(assigned) => {
                let add: Vec<&String> = row.roles.difference(assigned).collect();
                let remove: Vec<&String> = assigned.difference(&row.roles).collect();
                let action = if add.is_empty() && remove.is_empty() {
                    "unchanged"
                } else {
                    needs_apply = true;
                    "update"
                };
                changes.push(json!({
                    "group": row.group, "action": action,
                    "add": add, "remove": remove,
                }));
            }
        }
    }
    SectionPreview {
        needs_apply,
        changes: Value::Array(changes),
        blockers,
    }
}

/// One object field of the tenant record with the package's keys over the
/// current ones: keys the package does not name are kept.
fn merged_object(
    current: Option<&Value>,
    wanted: Option<&serde_json::Map<String, Value>>,
) -> Option<Value> {
    let Some(wanted) = wanted else {
        return current.cloned();
    };
    let mut merged = match current {
        Some(Value::Object(map)) => map.clone(),
        _ => serde_json::Map::new(),
    };
    for (key, value) in wanted {
        merged.insert(key.clone(), value.clone());
    }
    Some(Value::Object(merged))
}

/// The tenant record after applying the package: its id, slug, active state
/// and test flag are the target's own.
pub fn merged_tenant(current: &Tenant, wanted: &TenantConfiguration) -> Tenant {
    Tenant {
        labels: wanted.labels.clone().or_else(|| current.labels.clone()),
        annotations: merged_object(current.annotations.as_ref(), wanted.annotations.as_ref()),
        voting_channels: merged_object(
            current.voting_channels.as_ref(),
            wanted.voting_channels.as_ref(),
        ),
        settings: merged_object(current.settings.as_ref(), wanted.settings.as_ref()),
        ..current.clone()
    }
}

/// The tenant record's changes: each field whose merged value differs, with
/// the keys that change.
pub fn tenant_configuration_changes(
    current: &Tenant,
    wanted: &TenantConfiguration,
) -> SectionPreview {
    let merged = merged_tenant(current, wanted);
    let mut changes = Vec::new();
    for (field, before, after) in [
        ("labels", &current.labels, &merged.labels),
        ("annotations", &current.annotations, &merged.annotations),
        (
            "voting_channels",
            &current.voting_channels,
            &merged.voting_channels,
        ),
        ("settings", &current.settings, &merged.settings),
    ] {
        if before == after {
            continue;
        }
        let keys: Vec<&String> = match (before, after) {
            (Some(Value::Object(b)), Some(Value::Object(a))) => a
                .iter()
                .filter(|(key, value)| b.get(*key) != Some(*value))
                .map(|(key, _)| key)
                .collect(),
            (_, Some(Value::Object(a))) => a.keys().collect(),
            _ => vec![],
        };
        changes.push(json!({"field": field, "action": "update", "keys": keys}));
    }
    SectionPreview {
        needs_apply: !changes.is_empty(),
        changes: Value::Array(changes),
        blockers: vec![],
    }
}

/// Tenant settings: the display name and localization texts the importer
/// applies, compared with the realm's.
pub fn tenant_settings_changes(
    current_display_name: Option<&str>,
    current_localization: Option<&Value>,
    wanted: &Value,
) -> SectionPreview {
    let mut changes = Vec::new();
    if let Some(name) = wanted.get("displayName").and_then(Value::as_str) {
        let action = if current_display_name == Some(name) {
            "unchanged"
        } else {
            "update"
        };
        changes.push(json!({
            "field": "displayName", "action": action,
            "from": current_display_name, "to": name,
        }));
    }
    if let Some(texts) = wanted.get("localizationTexts") {
        let action = if current_localization == Some(texts) {
            "unchanged"
        } else {
            "update"
        };
        changes.push(json!({"field": "localizationTexts", "action": action}));
    }
    let needs_apply = changes.iter().any(|c| c["action"] == "update");
    SectionPreview {
        needs_apply,
        changes: Value::Array(changes),
        blockers: vec![],
    }
}

/// One administrator row of `admin_users.csv`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdminRow {
    pub username: String,
    pub group: String,
}

pub fn read_admin_rows(csv_bytes: &[u8]) -> Result<Vec<AdminRow>> {
    let mut reader = csv::Reader::from_reader(csv_bytes);
    let headers = reader
        .headers()
        .context("Error reading the administrators file")?
        .clone();
    let column = |name: &str| {
        headers
            .iter()
            .position(|h| h == name)
            .ok_or_else(|| anyhow!("the administrators file has no `{name}` column"))
    };
    let (username, group) = (column("username")?, column("group_name")?);
    let mut rows = Vec::new();
    for record in reader.records() {
        let record = record.context("Error reading the administrators file")?;
        rows.push(AdminRow {
            username: record.get(username).unwrap_or_default().to_string(),
            group: record.get(group).unwrap_or_default().to_string(),
        });
    }
    Ok(rows)
}

/// Administrators: an account that already exists is left as it is (never
/// re-created, never given credentials); only absent ones are created.
/// `groups` are the groups the realm has or the package's roles section
/// creates.
pub fn admin_changes(
    wanted: &[AdminRow],
    existing_usernames: &BTreeSet<String>,
    groups: &BTreeSet<String>,
) -> SectionPreview {
    let mut blockers = Vec::new();
    let mut changes = Vec::new();
    let mut needs_apply = false;
    for row in wanted {
        if !groups.contains(&row.group) {
            blockers.push(format!(
                "administrator '{}' names group '{}', which neither the realm nor the package defines",
                row.username, row.group
            ));
        }
        let exists = existing_usernames.contains(&row.username.to_lowercase());
        needs_apply |= !exists;
        changes.push(json!({
            "username": row.username,
            "group": row.group,
            "action": if exists { "exists" } else { "create" },
        }));
    }
    SectionPreview {
        needs_apply,
        changes: Value::Array(changes),
        blockers,
    }
}

/// The rows of `admin_users.csv` whose accounts do not exist yet, with its
/// header, so applying never touches an existing account.
pub fn absent_admin_rows(
    csv_bytes: &[u8],
    existing_usernames: &BTreeSet<String>,
) -> Result<Vec<u8>> {
    let mut reader = csv::Reader::from_reader(csv_bytes);
    let headers = reader.headers()?.clone();
    let username = headers
        .iter()
        .position(|h| h == "username")
        .ok_or_else(|| anyhow!("the administrators file has no `username` column"))?;
    let mut writer = csv::Writer::from_writer(Vec::new());
    writer.write_record(&headers)?;
    for record in reader.records() {
        let record = record?;
        let name = record.get(username).unwrap_or_default().to_lowercase();
        if !existing_usernames.contains(&name) {
            writer.write_record(&record)?;
        }
    }
    writer
        .into_inner()
        .map_err(|e| anyhow!("Error writing the administrators file: {e}"))
}

/// Trustees: registered by name with their two public keys. A trustee whose
/// registered keys differ is a blocker, never an overwrite: key material is
/// not replaced by a package. Keys are checked natively before anything is
/// registered.
pub fn trustee_changes(wanted: &[TrusteePublic], existing: &[Trustee]) -> SectionPreview {
    let mut blockers = Vec::new();
    let records: Vec<protocol_board::RawTrusteeRecord> = wanted
        .iter()
        .map(|t| protocol_board::RawTrusteeRecord {
            name: t.name.clone(),
            signing_public_key: Some(t.signing_public_key.clone()),
            share_encryption_public_key: Some(t.share_encryption_public_key.clone()),
        })
        .collect();
    if let Err(error) = protocol_board::Committee::new(&records) {
        blockers.push(format!(
            "trustee public keys are not valid native keys: {error:#}"
        ));
    }
    let mut changes = Vec::new();
    let mut needs_apply = false;
    for trustee in wanted {
        let registered = existing
            .iter()
            .find(|t| t.name.as_deref() == Some(trustee.name.as_str()));
        let action = match registered {
            None => {
                needs_apply = true;
                "register"
            }
            Some(row)
                if row.public_key.as_deref() == Some(trustee.signing_public_key.as_str())
                    && row.share_encryption_public_key.as_deref()
                        == Some(trustee.share_encryption_public_key.as_str()) =>
            {
                "unchanged"
            }
            Some(_) => {
                blockers.push(format!(
                    "trustee '{}' is registered with different public keys; a package does not replace key material",
                    trustee.name
                ));
                "conflict"
            }
        };
        changes.push(json!({"name": trustee.name, "action": action}));
    }
    SectionPreview {
        needs_apply,
        changes: Value::Array(changes),
        blockers,
    }
}

/// The event: imported once. An existing event with the package's id is the
/// package's event already imported.
pub fn event_changes(event_id: &str, exists: bool, problems: Vec<String>) -> SectionPreview {
    SectionPreview {
        needs_apply: !exists,
        changes: json!([{
            "electionEventId": event_id,
            "action": if exists { "exists" } else { "import" },
        }]),
        blockers: problems,
    }
}

#[cfg(test)]
#[path = "configuration_package_sections_tests.rs"]
mod tests;
