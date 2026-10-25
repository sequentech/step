// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The tenant-wide part of a delivery: settings, role mappings, administrator
//! accounts and native trustees' public keys.
//!
//! One versioned input document describes them; [`artifacts`] writes each in
//! the format its existing importer reads, for the Admin Portal settings zip.
//! The document is public configuration only. It cannot carry a credential or
//! a private key: every record refuses unknown fields, and a field that names
//! one is refused with that reason rather than as a typo.
//!
//! Nothing here invents content. An absent section stays absent, and a profile
//! that requires it reports it missing (see [`super::package_contract`]).

use super::archive::Artifact;
use super::package_contract::{
    ADMIN_USERS_MEMBER, TENANT_SETTINGS_PREFIX, TRUSTEES_MEMBER,
};
use super::problem::{Code, Problem, Report};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};

/// Field names that would carry a secret. Refused anywhere in the document.
const SECRET_FIELDS: [&str; 9] = [
    "password",
    "passwordSalt",
    "password_salt",
    "hashedPassword",
    "hashed_password",
    "privateKey",
    "private_key",
    "signingPrivateKey",
    "shareEncryptionPrivateKey",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TenantPackageFormat {
    #[serde(rename = "sequent.tenant-package/1")]
    V1,
}

/// What the tenant's administration realm shows: the two fields the
/// tenant-configuration importer applies. The tenant's id and slug are the
/// target's own and are never part of a package.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TenantSettings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub localization_texts: Option<BTreeMap<String, BTreeMap<String, String>>>,
}

/// One administration group and exactly the realm roles it is granted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RoleMapping {
    pub group: String,
    pub roles: Vec<String>,
}

/// An administrator, without credentials: they are issued on the installed
/// system, never carried in a package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct AdminAccount {
    pub username: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_name: Option<String>,
    /// One of the package's role-mapping groups, or one the target defines.
    pub group: String,
}

/// A native trustee's public keys, as `trustee public-keys` prints them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TrusteePublic {
    pub name: String,
    pub signing_public_key: String,
    pub share_encryption_public_key: String,
}

/// Where each section came from, for review. Free-form labels and digests.
pub type Provenance = BTreeMap<String, String>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TenantPackage {
    pub format: TenantPackageFormat,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenant_settings: Option<TenantSettings>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role_mappings: Option<Vec<RoleMapping>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub admin_accounts: Option<Vec<AdminAccount>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trustees: Option<Vec<TrusteePublic>>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub provenance: Provenance,
}

/// Read and check a tenant package document.
pub fn read(json: &Value) -> Result<TenantPackage, Report> {
    let mut report = Report::default();
    find_secret_fields(json, "tenant package", &mut report);
    if report.has_errors() {
        return Err(report);
    }
    let package: TenantPackage =
        serde_json::from_value(json.clone()).map_err(|e| {
            Report::from_problem(Problem::error(
                Code::InvalidValue,
                "tenant package",
                format!("not a tenant package: {e}"),
            ))
        })?;
    check(&package)?;
    Ok(package)
}

fn find_secret_fields(value: &Value, at: &str, report: &mut Report) {
    match value {
        Value::Object(map) => {
            for (key, inner) in map {
                let path = format!("{at}.{key}");
                if SECRET_FIELDS.contains(&key.as_str()) {
                    report.push(Problem::error(
                        Code::NotApproved,
                        path.clone(),
                        "credentials and private keys never travel in a package; issue them on the installed system",
                    ));
                }
                find_secret_fields(inner, &path, report);
            }
        }
        Value::Array(items) => {
            for (i, inner) in items.iter().enumerate() {
                find_secret_fields(inner, &format!("{at}[{i}]"), report);
            }
        }
        _ => {}
    }
}

fn blank(value: &str) -> bool {
    value.trim().is_empty()
}

/// Every problem in the document's records, not only the first.
pub fn check(package: &TenantPackage) -> Result<(), Report> {
    let mut report = Report::default();
    let mut groups = BTreeSet::new();
    for (i, mapping) in package.role_mappings.iter().flatten().enumerate() {
        let at = format!("roleMappings[{i}]");
        if blank(&mapping.group) {
            report.push(Problem::error(
                Code::MissingField,
                format!("{at}.group"),
                "a role mapping names its group",
            ));
        } else if !groups.insert(mapping.group.as_str()) {
            report.push(Problem::error(
                Code::DuplicateId,
                format!("{at}.group"),
                "a group is mapped once",
            ));
        }
        let mut roles = BTreeSet::new();
        for (j, role) in mapping.roles.iter().enumerate() {
            if blank(role) || role.contains('|') {
                report.push(Problem::error(
                    Code::InvalidValue,
                    format!("{at}.roles[{j}]"),
                    "a role name is non-empty and has no '|'",
                ));
            } else if !roles.insert(role.as_str()) {
                report.push(Problem::error(
                    Code::DuplicateId,
                    format!("{at}.roles[{j}]"),
                    "a role is listed once per group",
                ));
            }
        }
    }
    let mut usernames = BTreeSet::new();
    for (i, account) in package.admin_accounts.iter().flatten().enumerate() {
        let at = format!("adminAccounts[{i}]");
        if blank(&account.username) {
            report.push(Problem::error(
                Code::MissingField,
                format!("{at}.username"),
                "an administrator has a username",
            ));
        } else if !usernames.insert(account.username.to_lowercase()) {
            report.push(Problem::error(
                Code::DuplicateId,
                format!("{at}.username"),
                "a username is listed once",
            ));
        }
        if blank(&account.group) {
            report.push(Problem::error(
                Code::MissingField,
                format!("{at}.group"),
                "an administrator belongs to a group",
            ));
        } else if package.role_mappings.is_some()
            && !groups.contains(account.group.as_str())
        {
            report.push(Problem::error(
                Code::DanglingReference,
                format!("{at}.group"),
                "the group is not one of the package's role mappings",
            ));
        }
    }
    let mut names = BTreeSet::new();
    for (i, trustee) in package.trustees.iter().flatten().enumerate() {
        let at = format!("trustees[{i}]");
        if blank(&trustee.name) {
            report.push(Problem::error(
                Code::MissingField,
                format!("{at}.name"),
                "a trustee has a name",
            ));
        } else if !names.insert(trustee.name.as_str()) {
            report.push(Problem::error(
                Code::DuplicateId,
                format!("{at}.name"),
                "a trustee is listed once",
            ));
        }
        for (field, value) in [
            ("signingPublicKey", &trustee.signing_public_key),
            ("shareEncryptionPublicKey", &trustee.share_encryption_public_key),
        ] {
            if !is_base64(value) {
                report.push(Problem::error(
                    Code::InvalidValue,
                    format!("{at}.{field}"),
                    "a public key is the canonical base64 `trustee public-keys` prints; the platform checks the key itself when the package is applied",
                ));
            }
        }
    }
    if report.has_errors() {
        Err(report)
    } else {
        Ok(())
    }
}

fn is_base64(value: &str) -> bool {
    use base64::Engine;
    !value.is_empty()
        && base64::engine::general_purpose::STANDARD
            .decode(value)
            .is_ok_and(|bytes| !bytes.is_empty())
}

/// Each present section in the format its importer reads, for the Admin
/// Portal settings zip. `tenant_id` names the files the importers key by
/// tenant.
pub fn artifacts(
    package: &TenantPackage,
    tenant_id: &str,
) -> Result<Vec<Artifact>, Problem> {
    let mut written = Vec::new();
    if let Some(settings) = &package.tenant_settings {
        // A RealmRepresentation of which the importer reads displayName and
        // localizationTexts.
        written.push(json_artifact(
            format!("{TENANT_SETTINGS_PREFIX}{tenant_id}.json"),
            &serde_json::to_value(settings).map_err(encode)?,
        ));
    }
    if let Some(mappings) = &package.role_mappings {
        let rows: Vec<Vec<String>> = mappings
            .iter()
            .map(|m| vec![m.group.clone(), m.roles.join("|")])
            .collect();
        written.push(Artifact {
            name: format!("export_permissions-{tenant_id}.csv"),
            bytes: csv(&["role", "permissions"], &rows)?,
        });
    }
    if let Some(accounts) = &package.admin_accounts {
        let rows: Vec<Vec<String>> = accounts
            .iter()
            .map(|a| {
                vec![
                    a.username.clone(),
                    a.email.clone().unwrap_or_default(),
                    a.first_name.clone().unwrap_or_default(),
                    a.last_name.clone().unwrap_or_default(),
                    a.group.clone(),
                ]
            })
            .collect();
        written.push(Artifact {
            name: ADMIN_USERS_MEMBER.to_string(),
            bytes: csv(
                &["username", "email", "first_name", "last_name", "group_name"],
                &rows,
            )?,
        });
    }
    if let Some(trustees) = &package.trustees {
        written.push(json_artifact(
            TRUSTEES_MEMBER.to_string(),
            &Value::Array(
                trustees
                    .iter()
                    .map(serde_json::to_value)
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(encode)?,
            ),
        ));
    }
    Ok(written)
}

fn encode(_: serde_json::Error) -> Problem {
    Problem::error(Code::InvalidValue, "tenant package", "cannot encode")
}

fn json_artifact(name: String, value: &Value) -> Artifact {
    let mut bytes = serde_json::to_vec_pretty(value).unwrap_or_default();
    bytes.push(b'\n');
    Artifact { name, bytes }
}

fn csv(header: &[&str], rows: &[Vec<String>]) -> Result<Vec<u8>, Problem> {
    let mut writer = csv::Writer::from_writer(Vec::new());
    let fail = |_| Problem::error(Code::InvalidValue, "tenant package", "cannot write CSV");
    writer.write_record(header).map_err(fail)?;
    for row in rows {
        writer.write_record(row).map_err(fail)?;
    }
    writer.into_inner().map_err(|_| {
        Problem::error(Code::InvalidValue, "tenant package", "cannot write CSV")
    })
}

/// Add the package's files to a layout's auxiliary members, refusing one a
/// workbook sheet already produced: two sources for one file would make the
/// delivery depend on which was written last.
pub fn add_to_layout(
    layout: &mut super::archive::Layout,
    package: &TenantPackage,
    tenant_id: &str,
) -> Result<(), Problem> {
    for artifact in artifacts(package, tenant_id)? {
        if layout.auxiliary.iter().any(|a| a.name == artifact.name) {
            return Err(Problem::error(
                Code::DuplicateId,
                artifact.name.clone(),
                "the tenant package and the workbook both describe this file; keep one source",
            ));
        }
        layout.auxiliary.push(artifact);
    }
    Ok(())
}

/// The tenant package as a JSON object with only its present sections; used
/// to record what a delivery was built from.
pub fn to_value(package: &TenantPackage) -> Map<String, Value> {
    match serde_json::to_value(package) {
        Ok(Value::Object(map)) => map,
        _ => Map::new(),
    }
}

#[cfg(test)]
#[path = "tenant_package_tests.rs"]
mod tests;
