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

    // Every count and identifier participates in a reviewed import. A dropped
    // field would incorrectly label one of these changes as already approved.
    // Annotations and the redundant inner candidate ID are not hashed, which
    // keeps the digests persisted for approved sheets stable.
    let mut annotated = original.clone();
    annotated.annotations = Some(serde_json::json!({"note": "synthetic"}));
    assert_eq!(
        classify_change(Some(&original), &annotated).unwrap(),
        TallySheetImportChangeType::UNCHANGED
    );
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

#[test]
fn review_csv_distinguishes_unavailable_counts_from_reported_zeroes() {
    let mut content = sheet();
    content.total_votes = Some(0);
    content.total_valid_votes = Some(0);
    content.total_blank_votes = Some(0);
    content.blank_ballots = None;
    content.census = None;
    content.invalid_votes = None;
    content
        .candidate_results
        .get_mut(CANDIDATE_ID)
        .unwrap()
        .total_votes = None;

    let csv = render_ballot_box_csv(&content, &HashMap::new(), &HashMap::new());
    // An absent count is unknown, not a certified count of zero. Pin the
    // complete public layout so a renderer/parser round trip cannot hide it.
    assert_eq!(
        csv,
        concat!(
            "field,candidate_external_id,candidate_name,value\n",
            "total_votes,,,0\n",
            "total_valid_votes,,,0\n",
            "implicit_invalid,,,\n",
            "explicit_invalid,,,\n",
            "total_blank_votes,,,0\n",
            "blank_ballots,,,\n",
            "census,,,\n",
            "candidate_votes,,,\n",
        )
    );

    content.blank_ballots = Some(0);
    content.census = Some(0);
    content.invalid_votes = Some(InvalidVotes {
        total_invalid: Some(0),
        implicit_invalid: Some(0),
        explicit_invalid: Some(0),
    });
    content
        .candidate_results
        .get_mut(CANDIDATE_ID)
        .unwrap()
        .total_votes = Some(0);
    let csv = render_ballot_box_csv(&content, &HashMap::new(), &HashMap::new());
    let rows = csv::Reader::from_reader(csv.as_bytes())
        .records()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(rows.len(), 8);
    assert!(rows.iter().all(|row| &row[3] == "0"));
}

#[test]
fn review_requires_approval_when_a_known_zero_becomes_unavailable() {
    let mut known = sheet();
    known.blank_ballots = Some(0);
    let mut unknown = known.clone();
    unknown.blank_ballots = None;
    assert_eq!(
        classify_change(Some(&known), &unknown).unwrap(),
        TallySheetImportChangeType::CHANGED
    );
    assert_eq!(
        classify_change(Some(&unknown), &unknown).unwrap(),
        TallySheetImportChangeType::UNCHANGED
    );
}

#[test]
fn review_candidate_rows_have_stable_identifier_order_independent_of_labels() {
    // Names and external IDs sort in the reverse order of the candidate IDs,
    // so ordering rows by either label is detected. Six map entries make an
    // unsorted iteration match the identifier order only once in 720 runs.
    let candidates = [
        (CANDIDATE_ID, "ext-6", "Zulu", 3),
        ("candidate-b", "ext-5", "Yankee", 9),
        ("candidate-c", "ext-4", "X-ray", 1),
        ("candidate-d", "ext-3", "Whiskey", 7),
        ("candidate-e", "ext-2", "Victor", 0),
        ("candidate-f", "ext-1", "Uniform", 4),
    ];
    let mut content = sheet();
    let mut names = HashMap::new();
    let mut external_ids = HashMap::new();
    for (id, external_id, name, votes) in candidates {
        content.candidate_results.insert(
            id.into(),
            CandidateResults {
                candidate_id: id.into(),
                total_votes: Some(votes),
            },
        );
        names.insert(id.to_string(), name.to_string());
        external_ids.insert(id.to_string(), external_id.to_string());
    }
    let csv = render_ballot_box_csv(&content, &names, &external_ids);
    let rows = csv::Reader::from_reader(csv.as_bytes())
        .records()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let candidate_rows: Vec<Vec<String>> = rows[7..]
        .iter()
        .map(|row| row.iter().map(str::to_owned).collect())
        .collect();
    let expected: Vec<Vec<String>> = candidates
        .iter()
        .map(|(_, external_id, name, votes)| {
            vec![
                "candidate_votes".to_owned(),
                external_id.to_string(),
                name.to_string(),
                votes.to_string(),
            ]
        })
        .collect();
    assert_eq!(candidate_rows, expected);
}
