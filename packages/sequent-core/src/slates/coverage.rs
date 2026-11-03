// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! How much of a ballot a slate covers.
//!
//! A slate may have candidates in only some contests, or fewer candidates than
//! a contest has seats. Coverage is derived from the contests a voter can vote
//! in and is never configured, so a slate cannot claim offices it has no
//! candidate for.

use super::{Slate, SlatesConfig};
use crate::ballot::Contest;
use serde::{Deserialize, Serialize};

#[cfg(test)]
#[path = "coverage_tests.rs"]
mod tests;

/// Whether a slate fills every choice a voter has.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CoverageKind {
    /// The slate has a candidate for every seat of every contest.
    Complete,
    /// The slate leaves at least one seat without a candidate.
    Partial,
}

/// The candidates of a slate in one contest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContestCoverage {
    pub contest_id: String,
    /// In the order the slate lists them.
    pub candidate_ids: Vec<String>,
    /// The choices a voter has in this contest.
    pub seats: usize,
}

/// What one slate covers of the contests a voter can vote in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlateCoverage {
    pub slate_id: String,
    pub kind: CoverageKind,
    /// The contests the slate has candidates in, in ballot order.
    pub covered: Vec<ContestCoverage>,
    /// The contests the slate has no candidate in, in ballot order.
    pub uncovered_contest_ids: Vec<String>,
    /// The candidates of the slate across the covered contests.
    pub members: usize,
    /// The choices a voter has across every contest.
    pub seats: usize,
}

/// The coverage of `slate` over `contests`, or `None` when the slate has no
/// candidate a voter of these contests can choose.
///
/// Acclaimed contests are displayed but not voted, so they are neither covered
/// nor missing. Candidates that cannot be chosen are not counted.
pub fn slate_coverage(
    slate: &Slate,
    contests: &[Contest],
) -> Option<SlateCoverage> {
    let mut covered = Vec::new();
    let mut uncovered_contest_ids = Vec::new();
    let mut seats = 0usize;

    for contest in contests.iter().filter(|contest| !contest.is_acclaimed()) {
        let contest_seats = usize::try_from(contest.max_votes).unwrap_or(0);
        seats = seats.saturating_add(contest_seats);

        let candidate_ids: Vec<String> = slate
            .members
            .get(&contest.id)
            .map(|candidate_ids| {
                candidate_ids
                    .iter()
                    .filter(|candidate_id| {
                        contest.candidates.iter().any(|candidate| {
                            &candidate.id == *candidate_id
                                && candidate.is_acclamation_eligible()
                                && !candidate.is_category_list()
                        })
                    })
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();

        if candidate_ids.is_empty() {
            uncovered_contest_ids.push(contest.id.clone());
        } else {
            covered.push(ContestCoverage {
                contest_id: contest.id.clone(),
                candidate_ids,
                seats: contest_seats,
            });
        }
    }

    if covered.is_empty() {
        return None;
    }

    let members = covered
        .iter()
        .map(|contest| contest.candidate_ids.len())
        .sum();
    let fills_every_seat = uncovered_contest_ids.is_empty()
        && covered
            .iter()
            .all(|contest| contest.candidate_ids.len() == contest.seats);

    Some(SlateCoverage {
        slate_id: slate.id.clone(),
        kind: if fills_every_seat {
            CoverageKind::Complete
        } else {
            CoverageKind::Partial
        },
        covered,
        uncovered_contest_ids,
        members,
        seats,
    })
}

/// The coverage of every slate that has a candidate in `contests`, in the
/// configured order. A slate with none is left out.
pub fn slates_coverage(
    config: &SlatesConfig,
    contests: &[Contest],
) -> Vec<SlateCoverage> {
    config
        .slates
        .iter()
        .filter_map(|slate| slate_coverage(slate, contests))
        .collect()
}
