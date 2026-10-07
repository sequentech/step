// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! What choosing a slate does to a ballot.
//!
//! Choosing a slate marks its candidates in their contests and nothing else.
//! The result is the same per-contest selection a voter builds by marking
//! those candidates one by one, so the ballot is reviewed, encoded, cast and
//! counted as any other. Nothing in it says a slate was chosen.

use super::Slate;
use crate::ballot::Contest;
use crate::election_config::problem::{Code, Problem};
use crate::plaintext::{DecodedVoteChoice, DecodedVoteContest};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

const SELECTED: i64 = 0;
const UNSELECTED: i64 = -1;

/// The marks a slate adds to and removes from one contest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlateContestChange {
    pub contest_id: String,
    pub added: Vec<String>,
    pub removed: Vec<String>,
}

/// The ballot that choosing a slate produces, and what it changes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlateChoices {
    pub selection: Vec<DecodedVoteContest>,
    /// The contests whose marks change, in ballot order.
    pub changes: Vec<SlateContestChange>,
}

impl SlateChoices {
    /// Whether choosing the slate drops a mark the voter had made.
    pub fn removes_choices(&self) -> bool {
        self.changes.iter().any(|change| !change.removed.is_empty())
    }
}

/// The selection that choosing `slate` produces from `current`.
///
/// `contests` are the contests of the voter's ballot, and `current` holds one
/// entry for each. In every contest the slate covers, its candidates replace
/// the current marks, so no contest ends above its maximum. Contests the slate
/// does not cover, and contests outside this ballot, keep their choices.
/// Everything is checked first: a slate that cannot be applied in full
/// returns its problems and no selection.
pub fn apply_slate(
    slate: &Slate,
    contests: &[Contest],
    current: &[DecodedVoteContest],
) -> Result<SlateChoices, Vec<Problem>> {
    let mut problems = Vec::new();
    let mut selection = current.to_vec();
    let mut changes = Vec::new();
    let mut covered = 0_usize;

    for contest in contests {
        let Some(members) = slate.members.get(&contest.id) else {
            continue;
        };
        covered += 1;
        let path = format!("members[\"{}\"]", contest.id);

        let mut entries = selection
            .iter_mut()
            .filter(|entry| entry.contest_id == contest.id);
        let (Some(entry), None) = (entries.next(), entries.next()) else {
            problems.push(Problem::error(
                Code::DanglingReference,
                path,
                format!(
                    "contest '{}' of slate '{}' is missing from the ballot selection or repeated in it",
                    contest.id, slate.id
                ),
            ));
            continue;
        };

        match apply_to_contest(slate, contest, members, entry, &path) {
            Ok(Some(change)) => changes.push(change),
            Ok(None) => {}
            Err(mut found) => problems.append(&mut found),
        }
    }

    if covered == 0 {
        problems.push(Problem::error(
            Code::MissingField,
            "members",
            format!("slate '{}' has no candidates on this ballot", slate.id),
        ));
    }
    if !problems.is_empty() {
        return Err(problems);
    }

    for entry in &mut selection {
        entry.is_blank_ballot = false;
        entry.is_decline_to_vote = false;
    }
    Ok(SlateChoices { selection, changes })
}

fn apply_to_contest(
    slate: &Slate,
    contest: &Contest,
    members: &[String],
    entry: &mut DecodedVoteContest,
    path: &str,
) -> Result<Option<SlateContestChange>, Vec<Problem>> {
    let mut problems = Vec::new();
    let member_ids: BTreeSet<&str> =
        members.iter().map(String::as_str).collect();

    if members.is_empty() {
        problems.push(Problem::error(
            Code::MissingField,
            path,
            format!(
                "slate '{}' lists contest '{}' without candidates",
                slate.id, contest.id
            ),
        ));
    }
    if member_ids.len() != members.len() {
        problems.push(Problem::error(
            Code::DuplicateId,
            path,
            format!(
                "slate '{}' lists a candidate more than once in contest '{}'",
                slate.id, contest.id
            ),
        ));
    }
    if i64::try_from(member_ids.len())
        .map_or(true, |count| count > contest.max_votes)
    {
        problems.push(Problem::error(
            Code::ContestArithmetic,
            path,
            format!(
                "slate '{}' has {} candidates in contest '{}', which allows {}",
                slate.id,
                member_ids.len(),
                contest.id,
                contest.max_votes
            ),
        ));
    }
    for member in members {
        let is_candidate = contest
            .candidates
            .iter()
            .any(|candidate| &candidate.id == member);
        let is_choice = entry.choices.iter().any(|choice| &choice.id == member);
        if !is_candidate || !is_choice {
            problems.push(Problem::error(
                Code::DanglingReference,
                path,
                format!(
                    "candidate '{member}' of slate '{}' is not a choice of contest '{}'",
                    slate.id, contest.id
                ),
            ));
        }
    }
    if !problems.is_empty() {
        return Err(problems);
    }

    let was_selected: BTreeSet<&str> = entry
        .choices
        .iter()
        .filter(|choice| choice.is_selected())
        .map(|choice| choice.id.as_str())
        .collect();
    let added: Vec<String> = members
        .iter()
        .filter(|member| !was_selected.contains(member.as_str()))
        .cloned()
        .collect();
    let mut removed: Vec<String> = entry
        .choices
        .iter()
        .filter(|choice| {
            choice.is_selected() && !member_ids.contains(choice.id.as_str())
        })
        .map(|choice| choice.id.clone())
        .collect();
    if entry.is_explicit_invalid {
        let invalid_candidate = contest
            .candidates
            .iter()
            .find(|candidate| candidate.is_explicit_invalid());
        if let Some(candidate) = invalid_candidate {
            if !removed.contains(&candidate.id) {
                removed.push(candidate.id.clone());
            }
        }
    }

    entry.is_explicit_invalid = false;
    entry.invalid_errors.clear();
    entry.invalid_alerts.clear();
    entry.choices = entry
        .choices
        .iter()
        .map(|choice| DecodedVoteChoice {
            id: choice.id.clone(),
            selected: if member_ids.contains(choice.id.as_str()) {
                SELECTED
            } else {
                UNSELECTED
            },
            write_in_text: None,
        })
        .collect();

    Ok(
        (!added.is_empty() || !removed.is_empty()).then(|| {
            SlateContestChange {
                contest_id: contest.id.clone(),
                added,
                removed,
            }
        }),
    )
}

#[cfg(test)]
#[path = "selection_tests.rs"]
mod tests;
