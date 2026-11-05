// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;

/// A check that runs once per row says the same thing for every row; the
/// report says it once.
#[test]
fn a_report_says_each_problem_once() {
    let mut report = Report::default();
    let missing =
        || Problem::error(Code::NotCounted, "", "The snapshot has no count.");
    report.push(missing());
    report.push(missing());
    let mut other = Report::default();
    other.push(missing());
    other.push(Problem::warning(Code::NotCounted, "", "Another."));
    report.extend(other);
    assert_eq!(report.problems.len(), 2, "{report}");
}
