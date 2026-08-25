// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Bundles with known verdicts, shared by every caller of [`super::validate`].
//!
//! The point of one validator is that two callers reach the same answer. Tests
//! written separately in Rust and in TypeScript would not prove that — they would
//! prove each side agrees with itself. So the cases are **data**, compiled in here
//! and handed to the browser through the WASM surface, and both sides run the same
//! list.
//!
//! A case is a patch, not a whole bundle. `sound.json` is the one bundle anybody
//! has to keep valid; each case says what it changes and what that should be worth.
//! Reading a case means reading the difference, which is what a case is about.
//!
//! These are not a substitute for the tests in `validate_tests.rs`, which pin exact
//! messages and paths. These pin the *verdict*, which is the part two languages
//! have to agree on.

use crate::election_config::problem::{Code, Severity};
use crate::election_config::schema::ImportElectionEventSchema;
use crate::election_config::validate;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// The bundle every case starts from.
pub const SOUND: &str = include_str!("fixtures/sound.json");

/// What each case changes, and what it should be worth.
pub const CASES: &str = include_str!("fixtures/cases.json");

/// What a bundle should be found to be.
///
/// Distinct codes rather than counts. A case is about *what kind* of thing is
/// wrong; how many times validation says so is a detail that should be free to
/// change without a fixture needing an edit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Expect {
    pub errors: Vec<Code>,
    pub warnings: Vec<Code>,
}

/// One bundle, and the verdict every caller must reach on it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Case {
    pub name: String,

    /// Why this case is worth having. Read this before changing what it expects.
    pub why: String,

    /// The bundle, with the case's patch already applied.
    pub bundle: Value,

    pub expect: Expect,
}

#[derive(Deserialize)]
struct RawCase {
    name: String,
    why: String,
    patch: Value,
    expect: Expect,
}

/// Every case, with its patch applied to the sound bundle.
///
/// Panics if the fixtures do not parse: they are compiled in, so a broken one is a
/// build-time mistake rather than anything a caller can handle.
pub fn cases() -> Vec<Case> {
    let sound: Value =
        serde_json::from_str(SOUND).expect("fixtures/sound.json must parse");
    let raw: Vec<RawCase> =
        serde_json::from_str(CASES).expect("fixtures/cases.json must parse");

    raw.into_iter()
        .map(|case| Case {
            name: case.name,
            why: case.why,
            bundle: merge_patch(sound.clone(), case.patch),
            expect: case.expect,
        })
        .collect()
}

/// The verdict [`super::validate`] reaches on a case, in the same shape as its
/// expectation.
///
/// Sorted and deduplicated so the comparison is about which codes appeared, not
/// the order validation happened to find them in.
pub fn verdict(bundle: &ImportElectionEventSchema) -> Expect {
    let report = validate(bundle);
    let collect = |severity: Severity| {
        let mut codes: Vec<Code> = report
            .problems
            .iter()
            .filter(|problem| problem.severity == severity)
            .map(|problem| problem.code)
            .collect();
        codes.sort_by_key(|code| format!("{code:?}"));
        codes.dedup();
        codes
    };
    Expect {
        errors: collect(Severity::Error),
        warnings: collect(Severity::Warning),
    }
}

/// RFC 7386 JSON Merge Patch: objects recurse, `null` removes, everything else
/// replaces.
///
/// Arrays replace wholesale, which is what a case wants — "these three contests"
/// rather than "these appended to whatever was there".
fn merge_patch(target: Value, patch: Value) -> Value {
    let Value::Object(patch) = patch else {
        return patch;
    };

    let mut target = match target {
        Value::Object(target) => target,
        _ => Map::new(),
    };
    for (key, value) in patch {
        if value.is_null() {
            target.remove(&key);
        } else {
            let existing = target.remove(&key).unwrap_or(Value::Null);
            target.insert(key, merge_patch(existing, value));
        }
    }
    Value::Object(target)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_case_reaches_the_verdict_it_claims() {
        // The suite's whole job. When this fails, either validation changed or a
        // case's `why` no longer describes what it tests — read the `why` before
        // editing the expectation.
        let all = cases();
        // Guard against the way a loop like this fails open: an empty list, a
        // fixture file that stopped being found, and every assertion below is
        // skipped while the test still passes.
        assert!(all.len() >= 8, "only {} cases loaded", all.len());

        for case in all {
            let bundle: ImportElectionEventSchema = serde_json::from_value(
                case.bundle.clone(),
            )
            .unwrap_or_else(|error| {
                panic!("{}: does not deserialize: {error}", case.name)
            });
            let got = verdict(&bundle);
            assert_eq!(
                got, case.expect,
                "{}: {}\n  expected {:?}\n  got      {:?}",
                case.name, case.why, case.expect, got
            );
        }
    }

    #[test]
    fn the_base_bundle_is_the_one_with_nothing_wrong_with_it() {
        // Every other case is a patch on it, so a problem here would be attributed
        // to whichever case happened to be read first.
        let sound: ImportElectionEventSchema =
            serde_json::from_str(SOUND).expect("sound.json deserializes");
        let report = validate(&sound);
        assert!(report.is_empty(), "the base bundle is not sound:\n{report}");
    }

    #[test]
    fn the_suite_covers_every_code_validation_can_produce() {
        // A code no case exercises is a verdict the two callers have never been
        // checked to agree on. Two are absent on purpose and named here, so
        // adding a code without a case fails rather than passing quietly.
        let covered: Vec<Code> = cases()
            .iter()
            .flat_map(|case| {
                case.expect
                    .errors
                    .iter()
                    .chain(case.expect.warnings.iter())
                    .copied()
            })
            .collect();

        // An exhaustive `match` rather than a list, so adding a `Code` does not
        // compile until somebody decides which side it belongs on. A list left the
        // new variant neither covered nor reported, which is the opposite of what
        // the comment claimed.
        // Every variant, and the `match` below is what makes this hold: adding a
        // `Code` does not compile until somebody says which side it belongs on. The
        // old hard-coded list left a new variant neither covered nor reported, which
        // is the opposite of what its comment claimed.
        const EVERY_CODE: [Code; 11] = [
            Code::MissingField,
            Code::InvalidValue,
            Code::DanglingReference,
            Code::DuplicateId,
            Code::AreaCycle,
            Code::ContestArithmetic,
            Code::TallyMismatch,
            Code::BallotCoverage,
            Code::PermissionLabel,
            Code::MissingSchedule,
            Code::ConflictingColumns,
        ];

        for code in EVERY_CODE {
            let wanted = match code {
                Code::DanglingReference
                | Code::DuplicateId
                | Code::AreaCycle
                | Code::ContestArithmetic
                | Code::TallyMismatch
                | Code::BallotCoverage
                | Code::PermissionLabel => true,
                // From a bundle that does not deserialize at all, or from reading a
                // source document — neither of which is a case here.
                Code::MissingField | Code::InvalidValue => false,
                // Needs scheduled events, which arrive with the builder rather than
                // with a bundle on its own.
                Code::MissingSchedule => false,
                // Reported while reading a workbook's columns, not from a bundle.
                Code::ConflictingColumns => false,
            };
            assert_eq!(
                covered.contains(&code),
                wanted,
                "{code:?}: the fixture cases and this list disagree"
            );
        }
    }

    #[test]
    fn a_patch_replaces_a_list_rather_than_appending_to_it() {
        let merged = merge_patch(
            serde_json::json!({"a": [1, 2, 3], "keep": true}),
            serde_json::json!({"a": [9]}),
        );
        assert_eq!(merged, serde_json::json!({"a": [9], "keep": true}));
    }

    #[test]
    fn a_patch_recurses_through_objects_and_null_removes() {
        let merged = merge_patch(
            serde_json::json!({"o": {"x": 1, "y": 2}}),
            serde_json::json!({"o": {"y": null, "z": 3}}),
        );
        assert_eq!(merged, serde_json::json!({"o": {"x": 1, "z": 3}}));
    }

    #[test]
    fn every_case_says_why_it_exists() {
        // A case with no reason is a case nobody can safely change.
        for case in cases() {
            assert!(!case.name.is_empty());
            assert!(
                case.why.len() > 40,
                "{}: `why` should say what breaks, not just name it",
                case.name
            );
        }
    }
}
