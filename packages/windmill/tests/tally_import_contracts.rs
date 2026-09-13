// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Import review compares a canonical digest and displays a CSV. Both must
//! preserve the election data, including names with punctuation and newlines.

use sequent_core::types::tally_sheet_import::TallySheetImportChangeType;
use sequent_core::types::tally_sheets::{AreaContestResults, CandidateResults, InvalidVotes};
use std::collections::HashMap;
use windmill::services::tally_sheet_import::{
    diff::{classify_change, render_ballot_box_csv},
    hash::hash_area_contest_results,
};

const AREA_ID: &str = "area-a";
const CONTEST_ID: &str = "council";
const CANDIDATE_ID: &str = "candidate-a";

fn sheet() -> AreaContestResults {
    AreaContestResults {
        area_id: AREA_ID.into(),
        contest_id: CONTEST_ID.into(),
        total_votes: Some(5),
        total_valid_votes: Some(4),
        total_blank_votes: Some(1),
        blank_ballots: Some(1),
        census: Some(20),
        invalid_votes: Some(InvalidVotes {
            total_invalid: Some(1),
            implicit_invalid: Some(1),
            explicit_invalid: Some(0),
        }),
        candidate_results: HashMap::from([(
            CANDIDATE_ID.into(),
            CandidateResults {
                candidate_id: CANDIDATE_ID.into(),
                total_votes: Some(3),
            },
        )]),
        annotations: None,
    }
}

#[test]
fn review_csv_round_trips_candidate_names_and_external_identifiers() {
    let content = sheet();
    let name = "Lee, \"Ada\"\nIndependent";
    let external_id = "district,7\noption-1";
    let csv = render_ballot_box_csv(
        &content,
        &HashMap::from([(CANDIDATE_ID.into(), name.into())]),
        &HashMap::from([(CANDIDATE_ID.into(), external_id.into())]),
    );
    let mut reader = csv::Reader::from_reader(csv.as_bytes());
    assert_eq!(
        reader.headers().unwrap().iter().collect::<Vec<_>>(),
        ["field", "candidate_external_id", "candidate_name", "value"]
    );
    let records = reader.records().collect::<Result<Vec<_>, _>>().unwrap();
    assert_eq!(
        records.len(),
        8,
        "seven totals and one candidate, even with embedded newlines"
    );
    assert_eq!(
        records[7].iter().collect::<Vec<_>>(),
        ["candidate_votes", external_id, name, "3"]
    );
}

#[test]
fn absent_candidate_metadata_keeps_the_count_and_uses_empty_labels() {
    let csv = render_ballot_box_csv(&sheet(), &HashMap::new(), &HashMap::new());
    let records = csv::Reader::from_reader(csv.as_bytes())
        .records()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(
        records[7].iter().collect::<Vec<_>>(),
        ["candidate_votes", "", "", "3"]
    );
}

#[test]
fn change_classification_distinguishes_new_unchanged_and_each_material_edit() {
    let original = sheet();
    assert_eq!(
        classify_change(None, &original).unwrap(),
        TallySheetImportChangeType::NEW
    );
    assert_eq!(
        classify_change(Some(&original), &original).unwrap(),
        TallySheetImportChangeType::UNCHANGED
    );

    // Every field participates in a reviewed import. A dropped field would
    // incorrectly label one of these changes as already approved.
    let mutations: &[fn(&mut AreaContestResults)] = &[
        |s| s.area_id = "other-area".into(),
        |s| s.contest_id = "other-contest".into(),
        |s| s.total_votes = Some(6),
        |s| s.total_valid_votes = Some(5),
        |s| s.total_blank_votes = Some(2),
        |s| s.blank_ballots = Some(2),
        |s| s.census = Some(21),
        |s| s.invalid_votes.as_mut().unwrap().total_invalid = Some(2),
        |s| s.invalid_votes.as_mut().unwrap().implicit_invalid = Some(2),
        |s| s.invalid_votes.as_mut().unwrap().explicit_invalid = Some(1),
        |s| {
            s.candidate_results
                .get_mut(CANDIDATE_ID)
                .unwrap()
                .total_votes = Some(4)
        },
        |s| {
            s.candidate_results.insert(
                "other-candidate".into(),
                CandidateResults {
                    candidate_id: "other-candidate".into(),
                    total_votes: Some(0),
                },
            );
        },
    ];
    for (index, mutate) in mutations.iter().enumerate() {
        let mut changed = original.clone();
        mutate(&mut changed);
        assert_eq!(
            classify_change(Some(&original), &changed).unwrap(),
            TallySheetImportChangeType::CHANGED,
            "mutation {index}"
        );
        assert_ne!(
            hash_area_contest_results(&original).unwrap(),
            hash_area_contest_results(&changed).unwrap()
        );
    }
}
