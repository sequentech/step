// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::datafix_types::DatafixReconciliationField;
use serde::{Deserialize, Serialize};
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
    /// C: `Deleted=true` in the file for a voter who has not voted.
    DISABLED,
    /// C exception: `Deleted=true` in the file for a voter who *has* voted —
    /// the deletion is not applied, the Datafix patch reverts it.
    DELETION_REVERTED,
    /// C: `Ward`/`Poll`/`SchoolSupportCode`/`DoB` changed on the Datafix side.
    PROFILE_UPDATE,
    /// D: a voter present on one side, missing on the other.
    VOTER_ADDED,
    /// A voter Sequent disabled solely because of a Datafix `/delete-voter`
    /// call (`disable-comment = DISABLE_REASON_DELETE_CALL`) is re-enabled
    /// because the file no longer reports them `Deleted` (D12 — see the
    /// match block in `reconciliation::diff::classify_disabled_voter`).
    REENABLED,
    /// Excluded from both diffs/patch: a `CountyMun` processing error, or the
    /// "voted via other channel but holds a valid Internet ballot" guard.
    ROW_FAILURE,
}

/// Sequent-side field a reconciliation change applies to, when `target =
/// Sequent`. Distinct from `DatafixReconciliationField`: Sequent doesn't
/// store `Ward`/`Poll`/`SchoolSupportCode` separately, only the composed area
/// name (see `Area::name` / `reconciliation::diff::composed_area_name`), so
/// those three external columns collapse into a single `AreaName` here.
#[derive(Display, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, EnumString)]
pub enum SequentReconciliationField {
    AreaName,
    DoB,
    Channel,
    Deleted,
}

/// Which side of the reconciliation a change is applied to, carrying the
/// field it applies to in the shape that side actually understands: `Datafix`
/// changes are written into the downloadable patch for Datafix to apply on
/// their end, described in terms of `DatafixReconciliationField`; `Sequent`
/// changes are applied directly to this system, described in terms of
/// `SequentReconciliationField` — `None` when the change (e.g. a `CountyMun`
/// mismatch `ROW_FAILURE`) doesn't correspond to any field Sequent actually
/// stores. Kept generic on the Sequent side (an area name rather than
/// Datafix's raw Ward/Poll/SchoolSupportCode columns) so a future voter
/// interface provider other than Datafix can populate the same shape.
///
/// Wire shape: adjacently tagged (`{"target": "datafix"|"sequent", "field":
/// ...}`), matching the admin portal's separate `target`/`field` columns
/// (D4).
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
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

    pub fn datafix_field(&self) -> Option<DatafixReconciliationField> {
        match self {
            Self::Datafix(field) => Some(*field),
            Self::Sequent(_) => None,
        }
    }

    pub fn sequent_field(&self) -> Option<SequentReconciliationField> {
        match self {
            Self::Sequent(field) => *field,
            Self::Datafix(_) => None,
        }
    }
}
