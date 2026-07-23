// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::datafix_types::DatafixReconciliationField;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use strum_macros::{Display, EnumString};

/// The mandatory `#META,Sequence=N,GeneratedAt=T` line every reconciliation
/// and patch file starts with. `Sequence` is the ordering authority for
/// stale-file protection; `GeneratedAt` is informational only.
#[derive(Debug, Clone, Copy)]
pub struct ReconciliationFileMeta {
    pub sequence: i64,
    pub generated_at: i64,
}

/// Source-of-truth category a reconciliation change falls under. Wire
/// values match the admin portal's `ESyncChangeCategory` exactly.
#[allow(non_camel_case_types)]
#[derive(Display, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, EnumString)]
pub enum ReconciliationChangeCategory {
    /// A: Sequent holds a valid Internet ballot, Datafix reported `NONE`.
    VOTED_INTERNET,
    /// B: Datafix reports a non-`INTERNET` channel Sequent doesn't have.
    VOTED_OTHER_CHANNEL,
    /// C: `Deleted=true` in the file for a voter who has not voted. Named
    /// for the Keycloak `disable-comment` reason this drives
    /// (`DISABLE_REASON_DELETE_CALL`), set generically via `KeycloakUA` —
    /// see `SequentReconciliationField`.
    DISABLED_DELETE_CALL,
    /// C exception: `Deleted=true` in the file for a voter who *has* voted —
    /// the deletion is not applied, the Datafix patch reverts it.
    DELETION_REVERTED,
    /// C: `Ward`/`Poll`/`SchoolSupportCode`/`DoB` changed on the Datafix side.
    PROFILE_UPDATE,
    /// D: a voter present on one side, missing on the other.
    VOTER_ADDED,
    /// A voter Sequent disabled solely because of a Datafix `/delete-voter`
    /// call (`disable-comment = DISABLE_REASON_DELETE_CALL`) is re-enabled
    /// because the file no longer reports them `Deleted` — see the match
    /// block in `reconciliation::diff::classify_disabled_voter`.
    REENABLED,
    /// Excluded from both diffs/patch: a `CountyMun` processing error, or the
    /// "voted via other channel but holds a valid Internet ballot" guard.
    ROW_FAILURE,
}

/// Sequent-side field a reconciliation change applies to, when `target =
/// Sequent`, carrying its own `(old, new)` pair directly instead of leaving
/// it to a separate `old_value`/`new_value` on `DiffItem` — a field and its
/// old/new values are never meaningful apart from each other. Distinct from
/// `DatafixReconciliationField`:
/// - `AreaName` replaces `Ward`/`Poll`/`SchoolSupportCode` — Sequent only
///   stores the composed area name (see `Area::name` /
///   `reconciliation::diff::composed_area_name`), resolved to an `area-id`
///   attribute by looking the name up, not by decomposing it back into parts.
/// - `Enabled` drives the Keycloak `enabled` flag directly, `(old, new)` in
///   Keycloak's own enabled terms (not `Deleted`'s inverted CSV sense) — a
///   file row with `Deleted=true` for a voter currently enabled in Sequent
///   means `Enabled(true, false)`.
/// - Everything else Sequent stores as a plain Keycloak user attribute
///   (date of birth, voted channel, the disable-comment reason, ...) is
///   carried verbatim in `KeycloakUA`, keyed exactly as Keycloak expects.
///   The origin (today, only Datafix — see `reconciliation::diff`) decides
///   these key/value pairs; `reconciliation::apply` only ever merges and
///   writes the new set, with no knowledge of *why* an attribute has a given
///   value. This is what lets `apply` stay a plain, source-agnostic Keycloak
///   edit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SequentReconciliationField {
    AreaName(String, String),
    Enabled(bool, bool),
    KeycloakUA(HashMap<String, String>, HashMap<String, String>),
}

impl SequentReconciliationField {
    /// The new area name, if this is an `AreaName` change.
    pub fn new_area_name(&self) -> Option<&str> {
        match self {
            Self::AreaName(_, new) => Some(new.as_str()),
            Self::Enabled(..) | Self::KeycloakUA(..) => None,
        }
    }

    /// The target `enabled` state, if this is an `Enabled` change.
    pub fn new_enabled(&self) -> Option<bool> {
        match self {
            Self::Enabled(_, new) => Some(*new),
            Self::AreaName(..) | Self::KeycloakUA(..) => None,
        }
    }

    /// The Keycloak attributes to write, if this is a `KeycloakUA` change.
    pub fn new_keycloak_attributes(&self) -> Option<&HashMap<String, String>> {
        match self {
            Self::KeycloakUA(_, new) => Some(new),
            Self::AreaName(..) | Self::Enabled(..) => None,
        }
    }
}

/// Which side of the reconciliation a change is applied to, carrying the
/// field it applies to in the shape that side actually understands: `Datafix`
/// changes are written into the downloadable patch for Datafix to apply on
/// their end, described in terms of `DatafixReconciliationField`; `Sequent`
/// changes are applied directly to this system, described in terms of
/// `SequentReconciliationField` — `None` when the change (e.g. a `CountyMun`
/// mismatch `ROW_FAILURE`) doesn't correspond to any field Sequent actually
/// stores. Kept generic on the Sequent side (an area name and freeform
/// Keycloak attributes rather than Datafix's raw Ward/Poll/SchoolSupportCode
/// columns) so a future voter interface provider other than Datafix can
/// populate the same shape.
///
/// Wire shape: adjacently tagged (`{"target": "datafix"|"sequent", "field":
/// ...}`), matching the admin portal's separate `target`/`field` columns.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "target", content = "field", rename_all = "lowercase")]
pub enum ReconciliationPatchTarget {
    Datafix(DatafixReconciliationField),
    Sequent(Option<SequentReconciliationField>),
}

impl ReconciliationPatchTarget {
    pub fn is_datafix(&self) -> bool {
        matches!(self, Self::Datafix(_))
    }

    pub fn is_sequent(&self) -> bool {
        matches!(self, Self::Sequent(_))
    }

    pub fn datafix_field(&self) -> Option<&DatafixReconciliationField> {
        match self {
            Self::Datafix(field) => Some(field),
            Self::Sequent(_) => None,
        }
    }

    pub fn sequent_field(&self) -> Option<&SequentReconciliationField> {
        match self {
            Self::Sequent(field) => field.as_ref(),
            Self::Datafix(_) => None,
        }
    }
}

/// Which external voter registry a reconciliation round came from (generate)
/// or is being applied against (apply) — same shape as
/// `ReconciliationPatchTarget`: today only `Datafix` exists, carrying
/// whatever config that source's own classification (`CountyMun`, see
/// `reconciliation::diff::classify_file_row`) and apply-time bookkeeping
/// (its own `Sequence` tracking, see `reconciliation::apply`) needs — so a
/// future non-Datafix voter registry source can add its own variant without
/// any of the generic reconciliation code needing to change.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "source", rename_all = "lowercase")]
pub enum ReconciliationPatchSource {
    Datafix { county_mun: String },
}
