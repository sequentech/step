// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! What an election's slates must satisfy before a ballot carrying them is
//! published.
//!
//! The parser and the reference checks say whether a configuration is well
//! formed and points at things that exist. These are the rules that need the
//! election itself: its default language, each contest's limit and counting
//! method, and what kind of option each candidate is. The Admin Portal, the
//! importer, ballot style generation and publication all ask here, so they
//! cannot disagree.

use super::{check_references, parse, Scope, SlatesConfig, SLATES_ANNOTATION};
use crate::ballot;
use crate::ballot_style::create_contest;
use crate::election_config::problem::{Code, Problem};
use crate::services::translations::DEFAULT_LANG;
use crate::types::hasura::core::{Candidate, Contest, Election};
use std::collections::HashMap;
use std::fmt;

/// A slate configuration refused, with every reason why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidSlates {
    pub problems: Vec<Problem>,
}

impl InvalidSlates {
    /// One line per problem, for an error list an administrator reads.
    pub fn reasons(&self) -> Vec<String> {
        self.problems
            .iter()
            .map(|problem| format!("{}: {}", problem.path, problem.message))
            .collect()
    }
}

impl fmt::Display for InvalidSlates {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "Invalid slate configuration:\n- {}",
            self.reasons().join("\n- ")
        )
    }
}

impl std::error::Error for InvalidSlates {}

/// The configuration in the one form that is stored and published.
///
/// Two administrators who mean the same slates must publish the same ballot,
/// whatever whitespace or key order they typed.
pub fn canonicalize(text: &str, path: &str) -> Result<String, Vec<Problem>> {
    let config = parse(text, path)?;
    serde_json::to_string(&config).map_err(|error| {
        vec![Problem::error(
            Code::InvalidValue,
            path,
            format!("the slate configuration cannot be written: {error}"),
        )]
    })
}

/// The annotation as stored on an election or bundle row, if there is one.
pub fn annotation_text<'a>(
    annotations: Option<&'a serde_json::Value>,
    path: &str,
) -> Result<Option<&'a str>, Problem> {
    match annotations.and_then(|annotations| annotations.get(SLATES_ANNOTATION))
    {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::String(text)) => Ok(Some(text)),
        Some(_) => Err(Problem::error(
            Code::InvalidValue,
            path,
            "the slate configuration must be stored as JSON text",
        )),
    }
}

/// Every slate needs a name a voter can read when theirs is missing.
pub fn check_default_language_names(
    config: &SlatesConfig,
    default_language: &str,
    path: &str,
) -> Vec<Problem> {
    config
        .slates
        .iter()
        .enumerate()
        .filter(|(_, slate)| {
            slate
                .name
                .get(default_language)
                .is_none_or(|name| name.trim().is_empty())
        })
        .map(|(index, slate)| {
            Problem::error(
                Code::MissingField,
                format!("{path}.slates[{index}].name"),
                format!(
                    "slate '{}' has no name in the election's default \
                     language '{default_language}'",
                    slate.id
                ),
            )
        })
        .collect()
}

fn unsuitable_member(candidate: &ballot::Candidate) -> Option<&'static str> {
    if candidate.is_disabled() {
        Some("is disabled")
    } else if candidate.is_write_in() {
        Some("is a write-in placeholder")
    } else if candidate.is_explicit_invalid() {
        Some("is the explicit invalid option")
    } else if candidate.is_explicit_blank() {
        Some("is the explicit blank option")
    } else if candidate.is_category_list() {
        Some("is a category list")
    } else {
        None
    }
}

/// The rules a contest's own policy imposes on a slate's members.
///
/// A contest that is not in `contests` is left to [`check_references`], which
/// knows whether that is an error in this scope. Listing fewer candidates than
/// a contest allows, or covering only some contests, is a partial slate and is
/// valid.
pub fn check_membership_policies(
    config: &SlatesConfig,
    contests: &[ballot::Contest],
    path: &str,
) -> Vec<Problem> {
    let contests_by_id: HashMap<&str, &ballot::Contest> = contests
        .iter()
        .map(|contest| (contest.id.as_str(), contest))
        .collect();
    let mut problems = Vec::new();

    for (index, slate) in config.slates.iter().enumerate() {
        for (contest_id, members) in &slate.members {
            let Some(contest) = contests_by_id.get(contest_id.as_str()) else {
                continue;
            };
            let members_path =
                format!("{path}.slates[{index}].members[\"{contest_id}\"]");

            let counting_algorithm =
                contest.counting_algorithm.unwrap_or_default();
            if counting_algorithm.is_preferential()
                || counting_algorithm.is_cumulative()
            {
                problems.push(Problem::error(
                    Code::InvalidValue,
                    members_path.clone(),
                    format!(
                        "slate '{}' lists candidates in a contest counted \
                         with '{counting_algorithm}'; slates support ordinary \
                         candidate voting only",
                        slate.id
                    ),
                ));
            }

            let capacity = usize::try_from(contest.max_votes).unwrap_or(0);
            if members.len() > capacity {
                problems.push(Problem::error(
                    Code::ContestArithmetic,
                    members_path.clone(),
                    format!(
                        "slate '{}' lists {} candidates in a contest that \
                         allows at most {}",
                        slate.id,
                        members.len(),
                        contest.max_votes
                    ),
                ));
            }

            for candidate_id in members {
                let reason = contest
                    .candidates
                    .iter()
                    .find(|candidate| &candidate.id == candidate_id)
                    .and_then(unsuitable_member);
                if let Some(reason) = reason {
                    problems.push(Problem::error(
                        Code::InvalidValue,
                        members_path.clone(),
                        format!(
                            "candidate {candidate_id} {reason} and cannot \
                             be a member of slate '{}'",
                            slate.id
                        ),
                    ));
                }
            }
        }
    }

    problems
}

/// An election's contests as a ballot carries them, each with its candidates.
pub fn election_contests(
    contests: &[Contest],
    candidates: &[Candidate],
    default_language: &str,
    path: &str,
) -> Result<Vec<ballot::Contest>, Vec<Problem>> {
    let mut built = Vec::new();
    let mut problems = Vec::new();
    for contest in contests {
        let contest_candidates: Vec<Candidate> = candidates
            .iter()
            .filter(|candidate| {
                candidate.contest_id.as_deref() == Some(contest.id.as_str())
            })
            .cloned()
            .collect();
        match create_contest(
            contest.clone(),
            contest_candidates,
            default_language.to_string(),
        ) {
            Ok(contest) => built.push(contest),
            Err(error) => problems.push(Problem::error(
                Code::InvalidValue,
                path,
                format!(
                    "contest {} cannot be read to check its slates: {error}",
                    contest.id
                ),
            )),
        }
    }
    if problems.is_empty() {
        Ok(built)
    } else {
        Err(problems)
    }
}

fn check_config(
    config: &SlatesConfig,
    default_language: &str,
    contests: &[ballot::Contest],
    scope: Scope,
    path: &str,
) -> Vec<Problem> {
    let mut problems = check_references(config, contests, scope, path);
    problems.extend(check_default_language_names(
        config,
        default_language,
        path,
    ));
    problems.extend(check_membership_policies(config, contests, path));
    problems
}

/// Check a slate configuration against the contests and candidates of the
/// election it belongs to. `contests` must be that election's contests only.
pub fn check_annotation(
    text: &str,
    default_language: &str,
    contests: &[Contest],
    candidates: &[Candidate],
    path: &str,
) -> Vec<Problem> {
    let config = match parse(text, path) {
        Ok(config) => config,
        Err(problems) => return problems,
    };
    match election_contests(contests, candidates, default_language, path) {
        Ok(contests) => check_config(
            &config,
            default_language,
            &contests,
            Scope::Election,
            path,
        ),
        Err(problems) => problems,
    }
}

/// Check the slates an election row configures, if it configures any.
///
/// `contests` and `candidates` may span the whole election event; only this
/// election's are considered, so a member from another election is a dangling
/// reference.
pub fn check_election(
    election: &Election,
    contests: &[Contest],
    candidates: &[Candidate],
    path: &str,
) -> Vec<Problem> {
    let text = match annotation_text(election.annotations.as_ref(), path) {
        Ok(Some(text)) => text,
        Ok(None) => return Vec::new(),
        Err(problem) => return vec![problem],
    };
    let election_contests: Vec<Contest> = contests
        .iter()
        .filter(|contest| contest.election_id == election.id)
        .cloned()
        .collect();
    check_annotation(
        text,
        &election.get_default_language(),
        &election_contests,
        candidates,
        path,
    )
}

/// Check the slates one voter's ballot style carries, if it carries any.
///
/// A ballot style holds only the contests its area votes on, so a contest the
/// style does not have is out of style rather than unknown. A member missing
/// from a contest that is present is still an error.
pub fn check_ballot_style(
    ballot_style: &ballot::BallotStyle,
    path: &str,
) -> Vec<Problem> {
    let Some(text) = ballot_style
        .election_annotations
        .as_ref()
        .and_then(|annotations| annotations.get(SLATES_ANNOTATION))
    else {
        return Vec::new();
    };
    match parse(text, path) {
        Ok(config) => check_ballot_style_config(&config, ballot_style, path),
        Err(problems) => problems,
    }
}

/// The checks of [`check_ballot_style`] for a configuration already read.
pub fn check_ballot_style_config(
    config: &SlatesConfig,
    ballot_style: &ballot::BallotStyle,
    path: &str,
) -> Vec<Problem> {
    let default_language = ballot_style
        .election_presentation
        .as_ref()
        .and_then(|presentation| presentation.language_conf.as_ref())
        .and_then(|conf| conf.default_language_code.clone())
        .unwrap_or_else(|| DEFAULT_LANG.to_string());
    check_config(
        config,
        &default_language,
        &ballot_style.contests,
        Scope::BallotStyle,
        path,
    )
}

#[cfg(test)]
#[path = "election_tests.rs"]
mod election_tests;
