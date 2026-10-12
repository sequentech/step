// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Whether slates and candidates are listed in the usual order: full slates,
//! then partial slates, then independent candidates.
//!
//! The order among full slates, and among partial slates, is the configured
//! slate order. Election rules commonly require this listing, but the
//! platform does not impose it, so these are warnings: the configuration is
//! accepted as it is.

use super::coverage::{slate_coverage, CoverageKind};
use super::problem::{Code, Problem};
use super::SlatesConfig;
use crate::ballot::{Candidate, CandidatesOrder, Contest};
use std::collections::HashMap;

/// The warnings about how `config` and the candidates of `contests` are
/// ordered. `contests` are every contest of the election.
pub fn check_listing_order(
    config: &SlatesConfig,
    contests: &[Contest],
    path: &str,
) -> Vec<Problem> {
    let mut problems = check_slate_order(config, contests, path);
    problems.extend(check_candidate_order(config, contests, path));
    problems
}

fn check_slate_order(
    config: &SlatesConfig,
    contests: &[Contest],
    path: &str,
) -> Vec<Problem> {
    let mut first_partial: Option<&str> = None;
    let mut problems = Vec::new();

    for (index, slate) in config.slates.iter().enumerate() {
        let Some(coverage) = slate_coverage(slate, contests) else {
            continue;
        };
        match (coverage.kind, first_partial) {
            (CoverageKind::Partial, None) => {
                first_partial = Some(slate.id.as_str());
            }
            (CoverageKind::Complete, Some(partial)) => {
                problems.push(Problem::warning(
                    Code::ListingOrder,
                    format!("{path}.slates[{index}]"),
                    format!(
                        "full slate '{}' is listed after partial slate \
                         '{partial}'; full slates are usually listed first",
                        slate.id
                    ),
                ));
            }
            _ => {}
        }
    }

    problems
}

/// Where a candidate belongs in a contest: the position of its slate, with
/// independent candidates after every slate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Listing {
    Slate(usize),
    Independent,
}

fn is_listed_candidate(candidate: &Candidate) -> bool {
    candidate.is_acclamation_eligible() && !candidate.is_category_list()
}

fn candidate_label(candidate: &Candidate) -> &str {
    candidate.name.as_deref().unwrap_or(candidate.id.as_str())
}

fn contest_label(contest: &Contest) -> &str {
    contest.name.as_deref().unwrap_or(contest.id.as_str())
}

fn check_candidate_order(
    config: &SlatesConfig,
    contests: &[Contest],
    path: &str,
) -> Vec<Problem> {
    let slate_positions: HashMap<&str, usize> = config
        .slates
        .iter()
        .enumerate()
        .flat_map(|(index, slate)| {
            slate
                .members
                .values()
                .flatten()
                .map(move |candidate_id| (candidate_id.as_str(), index))
        })
        .collect();
    let mut problems = Vec::new();

    for contest in contests.iter().filter(|contest| !contest.is_acclaimed()) {
        let has_members = contest.candidates.iter().any(|candidate| {
            slate_positions.contains_key(candidate.id.as_str())
        });
        if !has_members {
            continue;
        }
        let contest_path = format!("{path}.contests[\"{}\"]", contest.id);

        let order = contest
            .presentation
            .as_ref()
            .and_then(|presentation| presentation.candidates_order.clone())
            .unwrap_or_default();
        if order == CandidatesOrder::Random {
            problems.push(Problem::warning(
                Code::ListingOrder,
                contest_path,
                format!(
                    "contest '{}' shows its candidates in random order, so \
                     slate members are not always listed before independent \
                     candidates",
                    contest_label(contest)
                ),
            ));
            continue;
        }

        let mut candidates: Vec<Candidate> = contest
            .candidates
            .iter()
            .filter(|candidate| is_listed_candidate(candidate))
            .cloned()
            .collect();
        order.sort(&mut candidates);

        if let Some(problem) =
            first_out_of_order(&candidates, &slate_positions, config)
        {
            problems.push(Problem::warning(
                Code::ListingOrder,
                contest_path,
                format!("in contest '{}', {problem}", contest_label(contest)),
            ));
        }
    }

    problems
}

/// The first candidate listed before one that should come ahead of it.
fn first_out_of_order(
    candidates: &[Candidate],
    slate_positions: &HashMap<&str, usize>,
    config: &SlatesConfig,
) -> Option<String> {
    let listing = |candidate: &Candidate| {
        slate_positions
            .get(candidate.id.as_str())
            .map_or(Listing::Independent, |index| Listing::Slate(*index))
    };

    candidates.iter().enumerate().find_map(|(index, earlier)| {
        let earlier_listing = listing(earlier);
        let later = candidates[index + 1..]
            .iter()
            .find(|later| listing(later) < earlier_listing)?;
        let message = match (earlier_listing, listing(later)) {
            (Listing::Independent, _) => format!(
                "independent candidate '{}' is listed before slate member \
                 '{}'",
                candidate_label(earlier),
                candidate_label(later)
            ),
            (Listing::Slate(earlier_slate), Listing::Slate(later_slate)) => {
                format!(
                    "'{}' of slate '{}' is listed before '{}' of slate '{}', \
                     which comes first in the slate order",
                    candidate_label(earlier),
                    config.slates[earlier_slate].id,
                    candidate_label(later),
                    config.slates[later_slate].id
                )
            }
            (Listing::Slate(_), Listing::Independent) => return None,
        };
        Some(message)
    })
}

#[cfg(test)]
#[path = "order_tests.rs"]
mod tests;
