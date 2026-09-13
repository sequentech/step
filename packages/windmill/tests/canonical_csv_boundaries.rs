// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Malformed imports should produce actionable validation errors, not panic or
//! wrap vote counts. A complete small ballot box is the positive control.

use sequent_core::types::hasura::core::Contest;
use sequent_core::types::tally_sheets::VotingChannel;
use serde_json::json;
use windmill::services::tally_sheet_import::csv::parse_canonical_csv;
use windmill::services::tally_sheet_import::validation::validate_import_content;

const HEADER: &str = "channel,area_name,contest_external_id,field,candidate_external_id,value\n";
const AREA: &str = "Precinct 1";
const CONTEST: &str = "council";

fn csv_with_invalid_counts(implicit: u64, explicit: u64) -> String {
    let mut csv = HEADER.to_string();
    for (field, value) in [
        ("total_votes", u64::MAX),
        ("total_valid_votes", 0),
        ("total_blank_votes", 0),
        ("implicit_invalid", implicit),
        ("explicit_invalid", explicit),
        ("census", u64::MAX),
    ] {
        csv.push_str(&format!("PAPER,{AREA},{CONTEST},{field},,{value}\n"));
    }
    csv
}

#[test]
fn maximum_representable_invalid_total_is_preserved() {
    for (implicit, explicit) in [(u64::MAX, 0), (0, u64::MAX), (u64::MAX - 1, 1)] {
        let (imports, errors) =
            parse_canonical_csv(csv_with_invalid_counts(implicit, explicit).as_bytes());
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(imports.len(), 1);
        assert_eq!(
            imports[0]
                .content
                .invalid_votes
                .as_ref()
                .unwrap()
                .total_invalid,
            Some(u64::MAX)
        );
    }
}

#[test]
fn overflowing_invalid_totals_return_a_scoped_error_and_no_importable_sheet() {
    for (implicit, explicit) in [(u64::MAX, 1), (1, u64::MAX), (u64::MAX, u64::MAX)] {
        let (imports, errors) =
            parse_canonical_csv(csv_with_invalid_counts(implicit, explicit).as_bytes());
        assert!(
            imports.is_empty(),
            "an overflowing sheet cannot be offered for approval"
        );
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code, "invalid_votes_overflow");
        assert_eq!(errors[0].area_name.as_deref(), Some(AREA));
        assert_eq!(errors[0].contest_external_id.as_deref(), Some(CONTEST));
    }
}

#[test]
fn invalid_numeric_cells_and_unknown_channels_are_explained_without_creating_groups() {
    for (channel, value, expected) in [
        ("PAPER", "-1", "invalid_numeric_value"),
        ("PAPER", "18446744073709551616", "invalid_numeric_value"),
        ("PAPER", "1.5", "invalid_numeric_value"),
        ("UNAPPROVED", "1", "invalid_channel"),
    ] {
        let csv = format!("{HEADER}{channel},{AREA},{CONTEST},total_votes,,{value}\n");
        let (imports, errors) = parse_canonical_csv(csv.as_bytes());
        assert!(imports.is_empty());
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code, expected);
    }
}

#[test]
fn duplicate_scalar_and_candidate_rows_are_rejected_instead_of_silently_overwriting() {
    let complete = csv_with_invalid_counts(1, 0);
    for (extra, expected) in [
        (format!("PAPER,{AREA},{CONTEST},total_votes,,4\n"), "duplicate_scalar_field"),
        (format!("PAPER,{AREA},{CONTEST},candidate_votes,candidate-a,3\nPAPER,{AREA},{CONTEST},candidate_votes,candidate-a,4\n"), "duplicate_candidate_votes"),
    ] {
        let (_, errors) = parse_canonical_csv(format!("{complete}{extra}").as_bytes());
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code, expected);
        assert_eq!(errors[0].area_name.as_deref(), Some(AREA));
    }
}

#[test]
fn parsed_vote_counts_reach_shared_validation_without_overflowing() {
    let contest: Contest = serde_json::from_value(json!({
        "id": CONTEST,
        "tenant_id": "tenant-1",
        "election_event_id": "event-1",
        "election_id": "election-1",
        "max_votes": 2,
        "counting_algorithm": "plurality-at-large"
    }))
    .unwrap();

    // Both cells fit u64. Their sum does not, but two marks per ballot make
    // this a valid tally. Exercise the same parse-to-validation handoff used
    // by import preview, including its translation of shared error details.
    let csv = format!(
        "{HEADER}\
         PAPER,{AREA},{CONTEST},total_votes,,{maximum}\n\
         PAPER,{AREA},{CONTEST},total_valid_votes,,{maximum}\n\
         PAPER,{AREA},{CONTEST},total_blank_votes,,0\n\
         PAPER,{AREA},{CONTEST},census,,{maximum}\n\
         PAPER,{AREA},{CONTEST},implicit_invalid,,0\n\
         PAPER,{AREA},{CONTEST},explicit_invalid,,0\n\
         PAPER,{AREA},{CONTEST},candidate_votes,candidate-a,{maximum}\n\
         PAPER,{AREA},{CONTEST},candidate_votes,candidate-b,1\n",
        maximum = u64::MAX
    );
    let (imports, parse_errors) = parse_canonical_csv(csv.as_bytes());
    assert!(parse_errors.is_empty(), "{parse_errors:?}");
    assert_eq!(imports.len(), 1);
    let content = &imports[0].content;
    assert!(
        validate_import_content(&VotingChannel::PAPER, AREA, CONTEST, content, &contest).is_empty()
    );

    let single_mark_contest = Contest {
        max_votes: Some(1),
        ..contest
    };
    let errors = validate_import_content(
        &VotingChannel::PAPER,
        AREA,
        CONTEST,
        content,
        &single_mark_contest,
    );
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, "invalid_total_valid_votes");
    assert_eq!(errors[0].area_name.as_deref(), Some(AREA));
    assert_eq!(errors[0].contest_external_id.as_deref(), Some(CONTEST));
    assert_eq!(
        errors[0].params["candidateVotesSum"],
        "18446744073709551616"
    );
}
