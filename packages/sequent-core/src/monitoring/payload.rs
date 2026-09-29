// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! What a producer stores for one data source at one scope.
//!
//! The snapshot job writes one payload per source, scope and set of visible
//! elections; Harvest only ever reads them, so the number of viewers never
//! changes how often a source is scanned. Every count in a payload was
//! counted at its own scope — a region's voters are counted over the region,
//! never added up from its Posts — which is what keeps distinct voters
//! distinct.
//!
//! Payloads are content-addressed, so they must serialize the same way every
//! time: maps are ordered, and nothing here carries a clock.

use super::sources::{Measure, PostState};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use strum_macros::{Display, EnumIter, EnumString};

/// Counts by measure. A measure the payload leaves out was not counted,
/// which is not the same as zero; compute refuses to read one.
pub type Counts = BTreeMap<Measure, u64>;

/// The key a missing dimension value is stored and returned under.
pub const UNKNOWN_KEY: &str = "__unknown__";

/// How a missing dimension value is shown.
pub const UNKNOWN_LABEL: &str = "Unknown";

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScopePayload {
    /// The scope's totals.
    pub totals: Counts,

    /// Rows of a dimension that a voter can have more than one value of
    /// within the scope (region, Post), or that the source counts in groups
    /// of its own (disapproval reason). Keyed by dimension; each row counted
    /// on its own, so the rows need not add up to the totals.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub groups: BTreeMap<String, Vec<GroupRow>>,

    /// The scope's voters partitioned by every configured voter dimension.
    /// Each voter has exactly one value of each (Unknown included), so cells
    /// add up to exact distinct counts for any grouping or filter.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cube: Option<Cube>,

    /// One row per Post in scope.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub posts: Vec<PostRow>,

    /// Hourly buckets in the event's time zone, oldest first, with no gaps
    /// between the first and the last. Days are the sum of their hours.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub series: Vec<Bucket>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notices: Vec<Notice>,
}

/// One value of a dimension and its counts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupRow {
    /// The stored value; [`UNKNOWN_KEY`] when the fact is missing.
    pub key: String,
    /// How to show it, when that differs from the key: a Post's name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub counts: Counts,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cube {
    /// The configured voter dimensions, in the order of each cell's values.
    pub dimensions: Vec<String>,
    pub cells: Vec<CubeCell>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CubeCell {
    /// One value per dimension; [`UNKNOWN_KEY`] when the voter has none.
    pub values: Vec<String>,
    pub counts: Counts,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PostRow {
    /// The election's id.
    pub post_id: String,
    pub post: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    /// Where the Post stands, for sources that count Posts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<PostState>,
    /// For a Post-counting source each measure is 1 or 0; for a voter source
    /// the Post's own distinct counts.
    pub counts: Counts,
}

/// One hour, `[start, start + 1 h)` in the event's time zone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bucket {
    /// Local wall-clock start, `YYYY-MM-DDTHH:MM:SS`, without an offset so a
    /// chart shows it as the event's local time.
    pub start: String,
    /// The local day the hour belongs to, `YYYY-MM-DD`.
    pub day: String,
    pub counts: Counts,
}

/// Something a viewer must be told to read a figure correctly.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    Display,
    EnumString,
    EnumIter,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum Notice {
    /// Attempts by unknown usernames belong to no Post, so they are counted
    /// for the whole event only.
    UnregisteredAttemptsAtEventScopeOnly,
    /// This scope's figures exclude them.
    UnregisteredAttemptsExcluded,
}
