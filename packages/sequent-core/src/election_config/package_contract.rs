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

use super::archive::{
    ADMIN_PORTAL_MEMBER, IMPORTABLE_MEMBER, PLAN_MEMBER, WORKBOOK_MEMBER,
};
pub use super::archive::{
    TENANT_CONFIGURATION_PREFIX, TENANT_SETTINGS_PREFIX, TRUSTEES_MEMBER,
};
use super::manifest::{
    FileEntry, CHAIN_MEMBER, MANIFEST_MEMBER, SIGNATURE_MEMBER,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Administrator accounts, without credentials.
pub const ADMIN_USERS_MEMBER: &str = "admin_users.csv";

/// The record of a historical (SQL + workbook) conversion: source digests,
/// counts and decisions. Kept for review; nothing applies it.
pub const HISTORICAL_SUMMARY_MEMBER: &str = "historical_conversion.json";

const EVENT_DOCUMENT_PREFIX: &str = "export_election_event-";
const VOTERS_FAMILY: &str = "export_voters";
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
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
)]
#[serde(rename_all = "kebab-case")]
pub enum Section {
    TenantConfiguration,
    TenantSettings,
    RolePermissions,
    AdminAccounts,
    Trustees,
    ElectionEvent,
}

impl Section {
    pub const APPLY_ORDER: [Section; 6] = [
        Section::TenantConfiguration,
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

/// Whether the event must carry its census. A profile that delivers voters
/// requires it; the event section alone does not say whether voters came.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize,
)]
#[serde(rename_all = "kebab-case")]
pub enum CensusRequirement {
    #[default]
    Optional,
    Required,
}

/// What the caller's profile expects a delivery to contain.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct Requirements {
    pub sections: Vec<Section>,
    pub census: CensusRequirement,
}

/// Every member's disposition and every section's state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Coverage {
    pub members: Vec<MemberDisposition>,
    pub sections: BTreeMap<Section, SectionState>,
    /// The event's voters file, the same three states as a section.
    pub census: SectionState,
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

    /// Whether everything required is present.
    pub fn is_complete(&self) -> bool {
        self.missing().is_empty() && self.census != SectionState::Missing
    }
}

/// The disposition of a root member.
pub fn root_disposition(path: &str) -> Disposition {
    match path {
        MANIFEST_MEMBER | SIGNATURE_MEMBER | CHAIN_MEMBER => {
            Disposition::Verification
        }
        ADMIN_REALM_PATCH | EVENT_REALM_PATCH => Disposition::Manual,
        PLAN_MEMBER
        | WORKBOOK_MEMBER
        | HISTORICAL_SUMMARY_MEMBER
        | "ceremony_schedule.json"
        | "points_of_contact.json"
        | "trustees_list.json"
        | "voter_messaging.json"
        | "admin_portal/communication_templates.json" => {
            Disposition::RecordOnly
        }
        _ => Disposition::Unknown,
    }
}

/// The disposition of a member inside one of the two nested zips.
pub fn nested_disposition(container: &str, path: &str) -> Disposition {
    match container {
        IMPORTABLE_MEMBER => event_disposition(path),
        ADMIN_PORTAL_MEMBER => portal_disposition(path),
        _ => Disposition::Unknown,
    }
}

/// The member families the election-event importer dispatches on, matched
/// the way it matches them (`process_document`: a name containing the
/// family). Anything else in the event zip is ignored by the importer, so it
/// is unknown here rather than applied.
const EVENT_FAMILIES: [&str; 13] = [
    "export_election_event",
    "export_voters",
    "export_scheduled_events",
    "export_reports",
    "export_bulletin_boards",
    "export_publications",
    "election_event_config",
    "export_protocol_manager_keys",
    "export_activity_logs",
    "electoral_log_manifest",
    "electoral_log_records",
    "export_certificates",
    "export_staff_issuers",
];
/// Directory families, matched with their trailing slash as the importer does.
const EVENT_DIRECTORIES: [&str; 3] =
    ["export_S3_files/", "images/", "export_tally_data/"];

fn event_disposition(path: &str) -> Disposition {
    if EVENT_FAMILIES.iter().any(|family| path.contains(family))
        || EVENT_DIRECTORIES.iter().any(|dir| path.contains(dir))
    {
        Disposition::Applied(Section::ElectionEvent)
    } else {
        Disposition::Unknown
    }
}

fn portal_disposition(path: &str) -> Disposition {
    if path == ADMIN_USERS_MEMBER {
        Disposition::Applied(Section::AdminAccounts)
    } else if path == TRUSTEES_MEMBER {
        Disposition::Applied(Section::Trustees)
    } else if path.starts_with(PERMISSIONS_PREFIX) && path.ends_with(".csv") {
        Disposition::Applied(Section::RolePermissions)
    } else if path.starts_with(TENANT_CONFIGURATION_PREFIX)
        && path.ends_with(".json")
    {
        Disposition::Applied(Section::TenantConfiguration)
    } else if path.starts_with(TENANT_SETTINGS_PREFIX)
        && path.ends_with(".json")
    {
        Disposition::Applied(Section::TenantSettings)
    } else if path.starts_with(TEMPLATES_PREFIX)
        || path.starts_with(TEMPLATES_DIR)
    {
        // A templates importer exists in the Admin Portal; applying a package
        // does not drive it.
        Disposition::Manual
    } else {
        Disposition::Unknown
    }
}

/// Whether the event importer reads this member of the event zip as the
/// census: any name containing the voters family, as `process_document`
/// matches it (including files under `export_S3_files/`). The one rule the
/// contract, the application's preview and the importer share.
pub fn is_census_member(path: &str) -> bool {
    path.contains(VOTERS_FAMILY)
}

/// Every member's disposition and each section's state. `required` names
/// what the caller (a client profile) expects; an absent one is
/// [`SectionState::Missing`].
pub fn coverage(entries: &[FileEntry], required: &Requirements) -> Coverage {
    let mut members = Vec::new();
    for entry in entries {
        let nested = entry.path == IMPORTABLE_MEMBER
            || entry.path == ADMIN_PORTAL_MEMBER;
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
            Section::ElectionEvent => present.iter().any(|p| {
                p.starts_with(EVENT_DOCUMENT_PREFIX) && p.ends_with(".json")
            }),
            _ => !present.is_empty(),
        };
        let state = if usable {
            SectionState::Present { members: present }
        } else if required.sections.contains(&section) {
            SectionState::Missing
        } else {
            SectionState::NotIncluded
        };
        sections.insert(section, state);
    }
    let voters: Vec<String> = members
        .iter()
        .filter(|m| {
            m.container.as_deref() == Some(IMPORTABLE_MEMBER)
                && is_census_member(&m.path)
        })
        .map(|m| m.path.clone())
        .collect();
    let census = if !voters.is_empty() {
        SectionState::Present { members: voters }
    } else if required.census == CensusRequirement::Required {
        SectionState::Missing
    } else {
        SectionState::NotIncluded
    };
    Coverage {
        members,
        sections,
        census,
    }
}

#[cfg(test)]
#[path = "package_contract_tests.rs"]
mod tests;
