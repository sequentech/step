// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! What an importer says when a file fails before, or beside, validation.
//!
//! [`crate::election_config::validate`] speaks about a bundle it has in hand.
//! An import fails earlier than that too: the archive does not open, the JSON
//! does not parse, the file was written by another version of the platform, the
//! checksum does not match, a voters file has a weight nobody can count. Those
//! used to reach an operator as whatever `anyhow` chain happened to hold them —
//! `Error checking import: Failed to parse import data as JSON: expected value
//! at line 1 column 1` — which is English, and mostly the platform talking to
//! itself.
//!
//! So each is a [`Problem`] here, with a stable `id` a front end translates and
//! the specifics it interpolates, built in one place so windmill, harvest and
//! any other importer say the same thing for the same failure. Pure, like the
//! rest of the module: a browser can produce them too.

use super::problem::{Code, Problem};

/// Where a problem about the file as a whole points.
pub const FILE: &str = "file";

/// The document could not be parsed as JSON.
pub fn not_json(reason: impl std::fmt::Display) -> Problem {
    Problem::error(
        Code::Unreadable,
        FILE,
        format!("the file is not a readable election event: {reason}"),
    )
    .id("file.not-json")
    .detail("reason", reason)
}

/// The JSON parsed and is not an election event bundle.
pub fn not_a_bundle(reason: impl std::fmt::Display) -> Problem {
    Problem::error(
        Code::Unreadable,
        FILE,
        format!("the file is not an election event export: {reason}"),
    )
    .id("file.not-a-bundle")
    .detail("reason", reason)
}

/// The archive does not open, or holds no election event document.
pub fn unreadable_archive(reason: impl std::fmt::Display) -> Problem {
    Problem::error(
        Code::Unreadable,
        FILE,
        format!("the archive could not be read: {reason}"),
    )
    .id("file.unreadable-archive")
    .detail("reason", reason)
}

/// An encrypted file that did not decrypt, which is nearly always the password.
pub fn cannot_decrypt() -> Problem {
    Problem::error(
        Code::Unreadable,
        FILE,
        "the file could not be decrypted; check the password",
    )
    .id("file.cannot-decrypt")
}

/// Exported by a version of the platform this one cannot import.
pub fn incompatible_version(found: &str, current: &str) -> Problem {
    Problem::error(
        Code::IncompatibleVersion,
        "version",
        format!(
            "the file was exported by version {found}, which version {current} \
             cannot import"
        ),
    )
    .id("file.version-incompatible")
    .detail("found", found)
    .detail("current", current)
}

/// The file's checksum is not the one the operator gave.
pub fn checksum_mismatch(expected: &str, actual: &str) -> Problem {
    Problem::error(
        Code::IntegrityMismatch,
        "sha256",
        format!(
            "the file's SHA-256 is {actual}, not the {expected} that was given, \
             so it is not the file that was meant"
        ),
    )
    .id("file.checksum-mismatch")
    .detail("expected", expected)
    .detail("actual", actual)
}

/// Where in a voters file: the row as a spreadsheet numbers it, header included.
fn voter_cell(row: usize, column: &str) -> String {
    format!("row {row} column '{column}'")
}

/// A vote weight that is not a whole number.
pub fn vote_weight_not_a_number(
    row: usize,
    column: &str,
    value: &str,
    max: u64,
) -> Problem {
    Problem::error(
        Code::InvalidValue,
        voter_cell(row, column),
        format!(
            "'{value}' on row {row} is not a vote weight; it must be a whole \
             number between 1 and {max}"
        ),
    )
    .id("voters.vote-weight-not-a-number")
    .detail("row", row)
    .detail("value", value)
    .detail("max", max)
}

/// A vote weight outside the range the tally accepts.
pub fn vote_weight_out_of_range(
    row: usize,
    column: &str,
    value: &str,
    min: u64,
    max: u64,
) -> Problem {
    Problem::error(
        Code::InvalidValue,
        voter_cell(row, column),
        format!(
            "the vote weight {value} on row {row} must be between {min} and {max}"
        ),
    )
    .id("voters.vote-weight-out-of-range")
    .detail("row", row)
    .detail("value", value)
    .detail("min", min)
    .detail("max", max)
}

/// A column that is almost the vote weight, and would be silently ignored.
pub fn vote_weight_misspelled(column: &str, expected: &str) -> Problem {
    Problem::error(
        Code::InvalidValue,
        format!("column '{column}'"),
        format!(
            "column '{column}' is not recognised; the vote weight column is \
             spelled exactly '{expected}'"
        ),
    )
    .id("voters.vote-weight-misspelled")
    .detail("column", column)
    .detail("expected", expected)
}

/// Two headers that name the same field.
pub fn duplicate_column(column: &str) -> Problem {
    Problem::error(
        Code::ConflictingColumns,
        format!("column '{column}'"),
        format!("two columns both set '{column}'; keep only one of them"),
    )
    .id("voters.duplicate-column")
    .detail("column", column)
}

/// A row the CSV reader could not read.
pub fn unreadable_row(row: usize, reason: impl std::fmt::Display) -> Problem {
    Problem::error(
        Code::Unreadable,
        format!("row {row}"),
        format!("row {row} could not be read: {reason}"),
    )
    .id("voters.unreadable-row")
    .detail("row", row)
    .detail("reason", reason)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::election_config::problem::Severity;

    #[test]
    fn every_import_problem_is_named_and_refuses_the_file() {
        let all = [
            not_json("expected value"),
            not_a_bundle("missing field `elections`"),
            unreadable_archive("no JSON file found"),
            cannot_decrypt(),
            incompatible_version("8.1.0", "9.2.0"),
            checksum_mismatch("aa", "bb"),
            vote_weight_not_a_number(4, "vote-weight", "x", 100),
            vote_weight_out_of_range(4, "vote-weight", "0", 1, 100),
            vote_weight_misspelled("vote_weight", "vote-weight"),
            duplicate_column("email"),
            unreadable_row(7, "wrong number of fields"),
        ];
        for problem in &all {
            assert_eq!(problem.severity, Severity::Error, "{problem}");
            assert!(problem.id.is_some(), "{problem} has no id");
        }
        let mut ids: Vec<_> = all.iter().filter_map(|p| p.id.clone()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), all.len(), "two import problems share an id");
    }

    #[test]
    fn the_specifics_travel_beside_the_sentence() {
        let problem = vote_weight_out_of_range(12, "vote-weight", "0", 1, 100);
        assert_eq!(problem.path, "row 12 column 'vote-weight'");
        assert_eq!(problem.details["row"], "12");
        assert_eq!(problem.details["value"], "0");
        assert_eq!(problem.details["min"], "1");
        assert_eq!(problem.details["max"], "100");

        let version = incompatible_version("8.1.0", "9.2.0");
        assert_eq!(version.code, Code::IncompatibleVersion);
        assert_eq!(version.details["found"], "8.1.0");
    }
}
