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
        settings: merged_settings(current.settings.as_ref(), wanted.settings.as_ref()),
        ..current.clone()
    }
}

/// The settings field merged like the others, except that a reserved setting
/// is always the installed one. The preview blocks a package that would change
/// it; this keeps it even when the installed value changed after that preview.
fn merged_settings(
    current: Option<&Value>,
    wanted: Option<&serde_json::Map<String, Value>>,
) -> Option<Value> {
    let mut merged = merged_object(current, wanted)?;
    if let Value::Object(map) = &mut merged {
        for key in RESERVED_SETTINGS {
            match current.and_then(|settings| settings.get(key)) {
                Some(installed) => {
                    map.insert(key.to_string(), installed.clone());
                }
                None => {
                    map.remove(key);
                }
            }
        }
    }
    Some(merged)
}

/// Tenant settings a package may not change: they decide what the tenant
/// trusts, so they are set on the installed system only.
pub const RESERVED_SETTINGS: [&str; 1] =
    [sequent_core::election_config::package_verify::CONFIGURATION_SIGNING_SETTING];

/// The tenant record's changes: each field whose merged value differs, with
/// the keys that change. Changing a reserved setting blocks the section.
pub fn tenant_configuration_changes(
    current: &Tenant,
    wanted: &TenantConfiguration,
) -> SectionPreview {
    let reserved: Vec<String> = RESERVED_SETTINGS
        .iter()
        .filter(|key| {
            wanted
                .settings
                .as_ref()
                .and_then(|s| s.get(**key))
                .is_some_and(|value| {
                    current.settings.as_ref().and_then(|s| s.get(**key)) != Some(value)
                })
        })
        .map(|key| {
            format!(
                "the package would change the tenant's {key}, which only the installed system sets"
            )
        })
        .collect();
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
        blockers: reserved,
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

/// The columns the producer writes for administrators. Anything else, a
/// credential column above all, is refused: the user import would accept a
/// `password` or a hash.
pub const ADMIN_COLUMNS: [&str; 5] = ["username", "email", "first_name", "last_name", "group_name"];

/// Columns of `admin_users.csv` the package may not carry.
pub fn admin_column_problems(csv_bytes: &[u8]) -> Result<Vec<String>> {
    let mut reader = csv::Reader::from_reader(csv_bytes);
    let headers = reader
        .headers()
        .context("Error reading the administrators file")?;
    Ok(headers
        .iter()
        .filter(|h| !ADMIN_COLUMNS.contains(h))
        .map(|h| {
            format!("the administrators file carries column '{h}', which a package may not set")
        })
        .collect())
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

/// Administrators: an account that already exists is never re-created or
/// given credentials. It counts as in place only when it is in its package
/// group; an existing account outside that group (an earlier import that
/// created it but did not assign it, or someone else's account) blocks the
/// section rather than being changed or passed over. `existing` maps each
/// existing username (lower-cased) to its groups; `groups` are the groups the
/// realm has or the package's roles section creates.
pub fn admin_changes(
    wanted: &[AdminRow],
    existing: &BTreeMap<String, BTreeSet<String>>,
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
        let action = match existing.get(&row.username.to_lowercase()) {
            None => {
                needs_apply = true;
                "create"
            }
            Some(member_of) if member_of.contains(&row.group) => "exists",
            Some(_) => {
                blockers.push(format!(
                    "administrator '{}' exists but is not in group '{}'; assign it by hand or remove the account, then apply again",
                    row.username, row.group
                ));
                "conflict"
            }
        };
        changes.push(json!({
            "username": row.username,
            "group": row.group,
            "action": action,
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
    existing_usernames: &BTreeMap<String, BTreeSet<String>>,
) -> Result<Vec<u8>> {
    let mut reader = csv::Reader::from_reader(csv_bytes);
    let headers = reader.headers()?.clone();
    let positions: Vec<Option<usize>> = ADMIN_COLUMNS
        .iter()
        .map(|column| headers.iter().position(|h| h == *column))
        .collect();
    let username =
        positions[0].ok_or_else(|| anyhow!("the administrators file has no `username` column"))?;
    let mut writer = csv::Writer::from_writer(Vec::new());
    writer.write_record(ADMIN_COLUMNS)?;
    for record in reader.records() {
        let record = record?;
        let name = record.get(username).unwrap_or_default().to_lowercase();
        if !existing_usernames.contains_key(&name) {
            writer.write_record(
                positions
                    .iter()
                    .map(|p| p.and_then(|p| record.get(p)).unwrap_or_default()),
            )?;
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
    // The key ceremony and the importer resolve trustees by name, so a name
    // must be present and appear once.
    let mut names = BTreeSet::new();
    for trustee in wanted {
        if trustee.name.trim().is_empty() {
            blockers.push("a trustee in the package has no name".to_string());
        } else if !names.insert(trustee.name.as_str()) {
            blockers.push(format!(
                "trustee '{}' appears more than once in the package",
                trustee.name
            ));
        }
    }
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

/// What the target holds of the package's event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventState {
    /// Neither the event nor its realm.
    Absent,
    /// The event's realm exists without the event: an earlier import stopped
    /// after creating it. Importing again would meet that realm.
    RealmOnly,
    /// The event exists. The import commits the event last, after its
    /// census, so its existence means the import finished; `voters` are the
    /// realm's people, compared with the package's census.
    Present { voters: BTreeSet<String> },
}

/// The event: imported once, and confirmed with its census by username.
/// `problems` are the event's own preflight blockers (validation, trustee
/// references), reported before anything is applied.
pub fn event_changes(
    event_id: &str,
    state: &EventState,
    census: &BTreeSet<String>,
    problems: Vec<String>,
) -> SectionPreview {
    let mut blockers = problems;
    let action = match state {
        EventState::Absent => "import",
        EventState::RealmOnly => {
            blockers.push(format!(
                "the realm of event {event_id} exists without the event: an earlier import stopped part-way; remove that realm before applying again"
            ));
            "partial"
        }
        EventState::Present { voters } if voters == census => "exists",
        EventState::Present { voters } => {
            let missing = census.difference(voters).count();
            let extra = voters.difference(census).count();
            blockers.push(format!(
                "event {event_id}'s voters differ from the package's census: {missing} of the census are not in the event and {extra} people in the event are not in the census; reconcile them by hand"
            ));
            "census-differs"
        }
    };
    SectionPreview {
        needs_apply: *state == EventState::Absent,
        changes: json!([{"electionEventId": event_id, "action": action, "censusSize": census.len()}]),
        blockers,
    }
}

/// The usernames of the event zip's census, read the way the importer would
/// find it. More than one census file, or one that is not CSV, is ambiguous
/// and refused rather than guessed.
pub fn census_usernames(event_zip: &[u8]) -> Result<std::result::Result<BTreeSet<String>, String>> {
    let members = sequent_core::election_config::manifest::read_zip(event_zip, "election event")
        .map_err(|p| anyhow!("{p}"))?;
    let census: Vec<_> = members
        .iter()
        .filter(|m| sequent_core::election_config::package_contract::is_census_member(&m.name))
        .collect();
    let voters = match census.as_slice() {
        [] => return Ok(Ok(BTreeSet::new())),
        [one] if one.name.ends_with(".csv") => one,
        _ => {
            return Ok(Err(format!(
                "the event carries {} census files ({}); exactly one CSV is applied",
                census.len(),
                census
                    .iter()
                    .map(|m| m.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )))
        }
    };
    let mut reader = csv::Reader::from_reader(voters.bytes.as_slice());
    let column = reader
        .headers()
        .context("Error reading the census")?
        .iter()
        .position(|h| h == "username");
    let Some(column) = column else {
        return Ok(Err("the census has no username column".to_string()));
    };
    // The user insert is all-or-nothing and fails on a repeated username, and
    // the event is applied last: refused here, before any section changes.
    // Rows are named by number, never by the voter's username.
    let mut usernames = BTreeSet::new();
    for (index, record) in reader.records().enumerate() {
        let record = record.context("Error reading the census")?;
        let row = index + 2;
        let username = record.get(column).unwrap_or_default();
        if username.trim().is_empty() {
            return Ok(Err(format!("census row {row} has no username")));
        }
        if !usernames.insert(username.to_lowercase()) {
            return Ok(Err(format!(
                "census row {row} repeats an earlier username (compared ignoring case)"
            )));
        }
    }
    Ok(Ok(usernames))
}

/// The event document of an event zip.
pub fn event_document(event_zip: &[u8]) -> Result<Vec<u8>> {
    let members = sequent_core::election_config::manifest::read_zip(event_zip, "election event")
        .map_err(|p| anyhow!("{p}"))?;
    members
        .into_iter()
        .find(|m| m.name.contains("export_election_event") && m.name.ends_with(".json"))
        .map(|m| m.bytes)
        .ok_or_else(|| anyhow!("the event zip has no event document"))
}

/// The key ceremonies' trustee references, resolved the way the importer
/// resolves them (by name) against the trustees the tenant has and those
/// the package registers. The importer maps an unknown name to an empty id,
/// so an unresolved or ambiguous name blocks before anything is applied.
pub fn trustee_reference_problems(
    referenced: &[String],
    registered: &[String],
    planned: &BTreeSet<String>,
) -> Vec<String> {
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for name in registered {
        *counts.entry(name.as_str()).or_default() += 1;
    }
    let mut problems = Vec::new();
    let mut seen = BTreeSet::new();
    for name in referenced {
        if !seen.insert(name) {
            continue;
        }
        match counts.get(name.as_str()) {
            Some(n) if *n > 1 => problems.push(format!(
                "the key ceremony names trustee '{name}', which {n} registered trustees share"
            )),
            Some(_) => {}
            None if planned.contains(name) => {}
            None => problems.push(format!(
                "the key ceremony names trustee '{name}', which neither the tenant nor the package registers"
            )),
        }
    }
    problems
}

#[cfg(test)]
#[path = "configuration_package_sections_tests.rs"]
mod tests;

/// Why a signed package may not be applied, given how it compares with the
/// tenant's last import of its configuration (`None` when never imported)
/// and the event this application imports it into.
pub fn admission_problem(
    last: Option<(
        std::result::Result<
            sequent_core::election_config::package_verify::Freshness,
            sequent_core::election_config::problem::Problem,
        >,
        &str,
    )>,
    derived_event_id: &str,
) -> Option<String> {
    use sequent_core::election_config::package_verify::Freshness;
    match last? {
        (Ok(Freshness::New), _) => None,
        (Ok(Freshness::AlreadyImported), imported_into) if imported_into == derived_event_id => {
            None
        }
        (Ok(Freshness::AlreadyImported), imported_into) => Some(format!(
            "this package was already imported into event {imported_into}, not by this application; it cannot be applied again"
        )),
        (Err(problem), _) => Some(format!("{}: {}", problem.path, problem.message)),
    }
}

/// The importer skips a messaging configuration its caller may not write and
/// still reports the event imported, so that is a blocker beforehand.
pub fn messaging_problem(
    carries_messaging_config: bool,
    permission: &crate::services::messaging::config::MessagingConfigPermission,
) -> Option<String> {
    use crate::services::messaging::config::MessagingConfigPermission;
    (carries_messaging_config && *permission != MessagingConfigPermission::PERMITTED).then(|| {
        "the event carries a messaging configuration, which the import would skip without messaging-config-write".to_string()
    })
}
