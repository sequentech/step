// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Named slates: groups of candidates across the contests of one election.
//!
//! A slate is configuration, stored as a versioned JSON string in the
//! election's annotations under [`SLATES_ANNOTATION`]. The ballot style already
//! carries those annotations to the Voting Portal, so the serialized ballot
//! structure does not change and an election without slates keeps its hash.
//!
//! A slate names candidates that are on the ballot anyway. It is never a vote
//! of its own.

use crate::ballot::{BallotStyle, Contest};
use crate::election_config::problem::{Code, Problem};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub mod coverage;
mod election;
pub mod selection;

#[cfg(test)]
mod tests;

pub use election::{
    annotation_text, canonicalize, check_annotation, check_ballot_style,
    check_ballot_style_config, check_default_language_names, check_election,
    check_membership_policies, election_contests, InvalidSlates,
};

/// The election annotation holding the slate configuration.
pub const SLATES_ANNOTATION: &str = "sequent.slates";

/// The only configuration version this platform reads.
pub const SLATES_VERSION: u32 = 1;

/// The largest configuration accepted, in bytes.
pub const MAX_CONFIG_BYTES: usize = 65_536;

/// The longest slate identifier, in characters.
pub const MAX_ID_CHARS: usize = 64;

/// The longest slate name in one language, in characters.
pub const MAX_NAME_CHARS: usize = 120;

const VERSION_FIELD: &str = "version";

/// Whether a phone shows each slate's candidates before the voter asks.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum MobileCandidateLists {
    #[default]
    Collapsed,
    Expanded,
}

/// The slates of one election, in the order they are shown.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SlatesConfig {
    pub version: u32,
    #[serde(default)]
    pub mobile_candidate_lists: MobileCandidateLists,
    pub slates: Vec<Slate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Slate {
    pub id: String,
    /// The display name by language code.
    pub name: BTreeMap<String, String>,
    /// The candidate ids of this slate by contest id.
    pub members: BTreeMap<String, Vec<String>>,
}

/// What a list of contests is, when checking a configuration against it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// Every contest of the election: an unknown contest is a mistake.
    Election,
    /// The contests one voter can vote in: a contest that is not there is
    /// outside this ballot style, and is skipped.
    BallotStyle,
}

/// Read a slate configuration and check everything that needs no election.
///
/// `path` prefixes the path of every problem reported.
pub fn parse(text: &str, path: &str) -> Result<SlatesConfig, Vec<Problem>> {
    if text.len() > MAX_CONFIG_BYTES {
        return Err(vec![Problem::error(
            Code::InvalidValue,
            path,
            format!(
                "the slate configuration is larger than {MAX_CONFIG_BYTES} bytes"
            ),
        )]);
    }

    let document: serde_json::Value =
        serde_json::from_str(text).map_err(|error| {
            vec![Problem::error(
                Code::Unreadable,
                path,
                format!("the slate configuration is not valid JSON: {error}"),
            )]
        })?;

    let version = document.get(VERSION_FIELD);
    if version.and_then(serde_json::Value::as_u64)
        != Some(u64::from(SLATES_VERSION))
    {
        let found = version
            .map(serde_json::Value::to_string)
            .unwrap_or_else(|| "none".to_string());
        return Err(vec![Problem::error(
            Code::IncompatibleVersion,
            format!("{path}.{VERSION_FIELD}"),
            format!(
                "the slate configuration version must be {SLATES_VERSION}, found {found}"
            ),
        )]);
    }

    let config: SlatesConfig =
        serde_json::from_value(document).map_err(|error| {
            vec![Problem::error(
                Code::InvalidValue,
                path,
                format!("the slate configuration is not valid: {error}"),
            )]
        })?;

    let problems = check_structure(&config, path);
    if problems.is_empty() {
        Ok(config)
    } else {
        Err(problems)
    }
}

/// Check the contests and candidates a configuration names against `contests`.
pub fn check_references(
    config: &SlatesConfig,
    contests: &[Contest],
    scope: Scope,
    path: &str,
) -> Vec<Problem> {
    let mut problems = Vec::new();

    for (index, slate) in config.slates.iter().enumerate() {
        for (contest_id, candidate_ids) in &slate.members {
            let members_path =
                format!("{path}.slates[{index}].members[\"{contest_id}\"]");
            let Some(contest) =
                contests.iter().find(|contest| &contest.id == contest_id)
            else {
                if scope == Scope::Election {
                    problems.push(Problem::error(
                        Code::DanglingReference,
                        members_path,
                        format!(
                            "slate '{}' names contest '{contest_id}', which is not in the election",
                            slate.id
                        ),
                    ));
                }
                continue;
            };

            for candidate_id in candidate_ids {
                let is_candidate = contest
                    .candidates
                    .iter()
                    .any(|candidate| &candidate.id == candidate_id);
                if !is_candidate {
                    problems.push(Problem::error(
                        Code::DanglingReference,
                        members_path.clone(),
                        format!(
                            "slate '{}' names candidate '{candidate_id}', which is not in contest '{contest_id}'",
                            slate.id
                        ),
                    ));
                }
            }
        }
    }

    problems
}

/// The slates a ballot style carries, or `None` when its election has none.
pub fn ballot_style_slates(
    ballot_style: &BallotStyle,
) -> Result<Option<SlatesConfig>, Vec<Problem>> {
    let Some(text) = ballot_style
        .election_annotations
        .as_ref()
        .and_then(|annotations| annotations.get(SLATES_ANNOTATION))
    else {
        return Ok(None);
    };

    let path = format!("election_annotations[\"{SLATES_ANNOTATION}\"]");
    let config = parse(text, &path)?;
    let problems = check_ballot_style_config(&config, ballot_style, &path);
    if problems.is_empty() {
        Ok(Some(config))
    } else {
        Err(problems)
    }
}

fn check_structure(config: &SlatesConfig, path: &str) -> Vec<Problem> {
    let mut problems = Vec::new();
    let mut slate_ids = BTreeSet::new();
    let mut names = BTreeSet::new();
    let mut candidate_slates: BTreeMap<&str, &str> = BTreeMap::new();

    for (index, slate) in config.slates.iter().enumerate() {
        let slate_path = format!("{path}.slates[{index}]");

        if !is_valid_id(&slate.id) {
            problems.push(Problem::error(
                Code::InvalidValue,
                format!("{slate_path}.id"),
                format!(
                    "a slate id starts with a letter or digit and has up to {MAX_ID_CHARS} letters, digits, hyphens or underscores; found '{}'",
                    slate.id
                ),
            ));
        }
        if !slate_ids.insert(slate.id.as_str()) {
            problems.push(Problem::error(
                Code::DuplicateId,
                format!("{slate_path}.id"),
                format!("two slates have the id '{}'", slate.id),
            ));
        }

        check_name(slate, &slate_path, &mut names, &mut problems);
        check_members(slate, &slate_path, &mut candidate_slates, &mut problems);
    }

    problems
}

fn check_name(
    slate: &Slate,
    slate_path: &str,
    names: &mut BTreeSet<(String, String)>,
    problems: &mut Vec<Problem>,
) {
    if slate.name.is_empty() {
        problems.push(Problem::error(
            Code::MissingField,
            format!("{slate_path}.name"),
            format!("slate '{}' needs a name", slate.id),
        ));
    }

    for (language, name) in &slate.name {
        let name_path = format!("{slate_path}.name.{language}");
        let trimmed = name.trim();

        if language.trim().is_empty() {
            problems.push(Problem::error(
                Code::InvalidValue,
                format!("{slate_path}.name"),
                format!(
                    "slate '{}' has a name without a language code",
                    slate.id
                ),
            ));
        }
        if trimmed.is_empty() {
            problems.push(Problem::error(
                Code::MissingField,
                name_path,
                format!(
                    "slate '{}' has an empty name in '{language}'",
                    slate.id
                ),
            ));
            continue;
        }
        if name.chars().count() > MAX_NAME_CHARS {
            problems.push(Problem::error(
                Code::InvalidValue,
                name_path.clone(),
                format!(
                    "the name of slate '{}' in '{language}' is longer than {MAX_NAME_CHARS} characters",
                    slate.id
                ),
            ));
        }
        if name.chars().any(char::is_control) {
            problems.push(Problem::error(
                Code::InvalidValue,
                name_path.clone(),
                format!(
                    "the name of slate '{}' in '{language}' must be plain text on one line",
                    slate.id
                ),
            ));
        }
        if !names.insert((language.clone(), trimmed.to_lowercase())) {
            problems.push(Problem::error(
                Code::InvalidValue,
                name_path,
                format!("two slates are named '{trimmed}' in '{language}'"),
            ));
        }
    }
}

fn check_members<'a>(
    slate: &'a Slate,
    slate_path: &str,
    candidate_slates: &mut BTreeMap<&'a str, &'a str>,
    problems: &mut Vec<Problem>,
) {
    if slate.members.is_empty() {
        problems.push(Problem::error(
            Code::MissingField,
            format!("{slate_path}.members"),
            format!("slate '{}' has no candidates", slate.id),
        ));
    }

    for (contest_id, candidate_ids) in &slate.members {
        let members_path = format!("{slate_path}.members[\"{contest_id}\"]");

        if candidate_ids.is_empty() {
            problems.push(Problem::error(
                Code::MissingField,
                members_path.clone(),
                format!(
                    "slate '{}' lists contest '{contest_id}' without candidates",
                    slate.id
                ),
            ));
        }

        for candidate_id in candidate_ids {
            let Some(other) =
                candidate_slates.insert(candidate_id, slate.id.as_str())
            else {
                continue;
            };
            let message = if other == slate.id {
                format!(
                    "slate '{}' lists candidate '{candidate_id}' more than once",
                    slate.id
                )
            } else {
                format!(
                    "candidate '{candidate_id}' is in slates '{other}' and '{}'; a candidate belongs to one slate at most",
                    slate.id
                )
            };
            problems.push(Problem::error(
                Code::DuplicateId,
                members_path.clone(),
                message,
            ));
        }
    }
}

fn is_valid_id(id: &str) -> bool {
    let mut characters = id.chars();
    let starts_well = characters
        .next()
        .is_some_and(|first| first.is_ascii_alphanumeric());

    starts_well
        && id.chars().count() <= MAX_ID_CHARS
        && characters.all(|character| {
            character.is_ascii_alphanumeric()
                || character == '-'
                || character == '_'
        })
}
