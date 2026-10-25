// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Who reads each member of a delivery, and what a complete one contains.
//!
//! One code-side table, shared by the wizard's review screen and the
//! platform's package application, so the two cannot disagree about which
//! file configures what. It describes the members [`archive::delivery`] writes
//! and the manifest members a signed package adds; it is not a second
//! manifest. A member nobody here recognises is reported as
//! [`Disposition::Unknown`] rather than refused: delivery readers ignore root
//! members they do not know, and a newer producer must stay readable.
//!
//! Only members with [`Disposition::Applied`] are configured by applying a
//! package. Everything else is shown with its disposition, so a sidecar in
//! the zip is never mistaken for configuration that happened.
//!
//! [`archive::delivery`]: super::archive::delivery

use super::archive::{ADMIN_PORTAL_MEMBER, IMPORTABLE_MEMBER, PLAN_MEMBER, WORKBOOK_MEMBER};
use super::manifest::{FileEntry, CHAIN_MEMBER, MANIFEST_MEMBER, SIGNATURE_MEMBER};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Tenant settings: a Keycloak admin-realm representation of which the
/// tenant-configuration importer applies the display name and localization
/// only. Named after the tenant like the permissions file.
pub const TENANT_SETTINGS_PREFIX: &str = "keycloak_admin-";

/// Native trustees' public material: names with signing and share-encryption
/// public keys. Never private keys; see [`super::trustee_public`].
pub const TRUSTEES_MEMBER: &str = "trustees_public.json";

/// Administrator accounts, without credentials.
pub const ADMIN_USERS_MEMBER: &str = "admin_users.csv";

/// The record of a historical (SQL + workbook) conversion: source digests,
/// counts and decisions. Kept for review; nothing applies it.
pub const HISTORICAL_SUMMARY_MEMBER: &str = "historical_conversion.json";

const EVENT_DOCUMENT_PREFIX: &str = "export_election_event-";
const PERMISSIONS_PREFIX: &str = "export_permissions-";
const TEMPLATES_PREFIX: &str = "export_templates-";
const TEMPLATES_DIR: &str = "templates/";
const ADMIN_REALM_PATCH: &str = "keycloak_admin_realm_patch.json";
const EVENT_REALM_PATCH: &str = "keycloak_event_realm_patch.json";

/// What a package can configure, in the order applying one does it.
///
/// Tenant-wide configuration goes first because the event refers to it: its
/// roles gate the administrators, and the key ceremony resolves trustees by
/// name.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "kebab-case")]
pub enum Section {
    TenantSettings,
    RolePermissions,
    AdminAccounts,
    Trustees,
    ElectionEvent,
}

impl Section {
    pub const APPLY_ORDER: [Section; 5] = [
        Section::TenantSettings,
        Section::RolePermissions,
        Section::AdminAccounts,
        Section::Trustees,
        Section::ElectionEvent,
    ];
}

/// What happens to one member when a package is applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind", content = "section")]
pub enum Disposition {
    /// Applied by the platform as part of this section.
    Applied(Section),
    /// Applied by a person with the right access; the platform does not.
    Manual,
    /// Kept for people and for reopening the authoring source.
    RecordOnly,
    /// Proves the package's origin; checked before anything is read.
    Verification,
    /// Not a member this version knows. Shown, not applied.
    Unknown,
}

/// One member and its disposition. `container` is the nested zip it sits
/// in, or `None` at the package root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemberDisposition {
    pub container: Option<String>,
    pub path: String,
    pub sha256: String,
    pub size: u64,
    pub disposition: Disposition,
}

/// Whether a section has anything to apply.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "state")]
pub enum SectionState {
    /// The members that configure it.
    Present { members: Vec<String> },
    /// Required by the caller and absent: an explicit gap, never a default.
    Missing,
    /// Neither present nor required.
    NotIncluded,
}

/// Every member's disposition and every section's state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Coverage {
    pub members: Vec<MemberDisposition>,
    pub sections: BTreeMap<Section, SectionState>,
}

impl Coverage {
    /// The required sections the package does not carry.
    pub fn missing(&self) -> Vec<Section> {
        self.sections
            .iter()
            .filter(|(_, state)| **state == SectionState::Missing)
            .map(|(section, _)| *section)
            .collect()
    }
}

/// The disposition of a root member.
pub fn root_disposition(path: &str) -> Disposition {
    match path {
        MANIFEST_MEMBER | SIGNATURE_MEMBER | CHAIN_MEMBER => Disposition::Verification,
        ADMIN_REALM_PATCH | EVENT_REALM_PATCH => Disposition::Manual,
        PLAN_MEMBER
        | WORKBOOK_MEMBER
        | HISTORICAL_SUMMARY_MEMBER
        | "ceremony_schedule.json"
        | "points_of_contact.json"
        | "trustees_list.json"
        | "voter_messaging.json"
        | "admin_portal/communication_templates.json" => Disposition::RecordOnly,
        _ => Disposition::Unknown,
    }
}

/// The disposition of a member inside one of the two nested zips.
pub fn nested_disposition(container: &str, path: &str) -> Disposition {
    match container {
        // The event importer reads every member of its zip; which ones it
        // understands is its own contract, versioned with the event export.
        IMPORTABLE_MEMBER => Disposition::Applied(Section::ElectionEvent),
        ADMIN_PORTAL_MEMBER => portal_disposition(path),
        _ => Disposition::Unknown,
    }
}

fn portal_disposition(path: &str) -> Disposition {
    if path == ADMIN_USERS_MEMBER {
        Disposition::Applied(Section::AdminAccounts)
    } else if path == TRUSTEES_MEMBER {
        Disposition::Applied(Section::Trustees)
    } else if path.starts_with(PERMISSIONS_PREFIX) && path.ends_with(".csv") {
        Disposition::Applied(Section::RolePermissions)
    } else if path.starts_with(TENANT_SETTINGS_PREFIX) && path.ends_with(".json") {
        Disposition::Applied(Section::TenantSettings)
    } else if path.starts_with(TEMPLATES_PREFIX) || path.starts_with(TEMPLATES_DIR) {
        // A templates importer exists in the Admin Portal; applying a package
        // does not drive it.
        Disposition::Manual
    } else {
        Disposition::Unknown
    }
}

/// Every member's disposition and each section's state. `required` names the
/// sections the caller (a client profile) expects; an absent one is
/// [`SectionState::Missing`].
pub fn coverage(entries: &[FileEntry], required: &[Section]) -> Coverage {
    let mut members = Vec::new();
    for entry in entries {
        let nested = entry.path == IMPORTABLE_MEMBER || entry.path == ADMIN_PORTAL_MEMBER;
        if nested {
            for inner in &entry.members {
                members.push(MemberDisposition {
                    container: Some(entry.path.clone()),
                    path: inner.path.clone(),
                    sha256: inner.sha256.clone(),
                    size: inner.size,
                    disposition: nested_disposition(&entry.path, &inner.path),
                });
            }
        } else {
            members.push(MemberDisposition {
                container: None,
                path: entry.path.clone(),
                sha256: entry.sha256.clone(),
                size: entry.size,
                disposition: root_disposition(&entry.path),
            });
        }
    }
    let mut sections = BTreeMap::new();
    for section in Section::APPLY_ORDER {
        let present: Vec<String> = members
            .iter()
            .filter(|m| m.disposition == Disposition::Applied(section))
            .map(|m| m.path.clone())
            .collect();
        // The event importer refuses a zip without its event document, so
        // other members alone do not make the section present.
        let usable = match section {
            Section::ElectionEvent => present
                .iter()
                .any(|p| p.starts_with(EVENT_DOCUMENT_PREFIX) && p.ends_with(".json")),
            _ => !present.is_empty(),
        };
        let state = if usable {
            SectionState::Present { members: present }
        } else if required.contains(&section) {
            SectionState::Missing
        } else {
            SectionState::NotIncluded
        };
        sections.insert(section, state);
    }
    Coverage { members, sections }
}

#[cfg(test)]
#[path = "package_contract_tests.rs"]
mod tests;
