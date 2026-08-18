// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::BTreeMap;

use anyhow::Result;
use sequent_core::types::tally_sheets::AreaContestResults;
use serde::Serialize;
use sha2::{Digest, Sha256};
use tracing::instrument;

#[instrument(skip_all, err)]
pub fn hash_area_contest_results(content: &AreaContestResults) -> Result<String> {
    let bytes = serde_json::to_vec(&CanonicalAreaContestResults::from(content))?;
    let digest = Sha256::digest(bytes);
    Ok(hex::encode(digest))
}

#[instrument(skip_all)]
pub fn hash_bytes(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    hex::encode(digest)
}

#[derive(Serialize)]
struct CanonicalAreaContestResults<'a> {
    area_id: &'a str,
    contest_id: &'a str,
    total_votes: Option<u64>,
    total_valid_votes: Option<u64>,
    invalid_votes: &'a Option<sequent_core::types::tally_sheets::InvalidVotes>,
    total_blank_votes: Option<u64>,
    // Unlike every other field here, `blank_ballots` is a new addition to a
    // struct whose hashes are already persisted in production
    // (`tally_sheet.baseline_content_hash`) for real approved sheets, none
    // of which have a value for it. Omitting the key entirely when it's
    // `None` keeps those existing hashes stable -- serializing it as an
    // explicit `null` would change every one of them and make the next
    // import diff report every ballot box as CHANGED.
    #[serde(skip_serializing_if = "Option::is_none")]
    blank_ballots: Option<u64>,
    census: Option<u64>,
    candidate_results: BTreeMap<&'a str, CanonicalCandidateResults>,
}

#[derive(Serialize)]
struct CanonicalCandidateResults {
    total_votes: Option<u64>,
}

impl<'a> From<&'a AreaContestResults> for CanonicalAreaContestResults<'a> {
    fn from(content: &'a AreaContestResults) -> Self {
        let candidate_results = content
            .candidate_results
            .iter()
            .map(|(candidate_id, result)| {
                (
                    candidate_id.as_str(),
                    CanonicalCandidateResults {
                        total_votes: result.total_votes,
                    },
                )
            })
            .collect();

        Self {
            area_id: &content.area_id,
            contest_id: &content.contest_id,
            total_votes: content.total_votes,
            total_valid_votes: content.total_valid_votes,
            invalid_votes: &content.invalid_votes,
            total_blank_votes: content.total_blank_votes,
            blank_ballots: content.blank_ballots,
            census: content.census,
            candidate_results,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use sequent_core::types::tally_sheets::{CandidateResults, InvalidVotes};

    use super::*;

    #[test]
    fn hashes_candidate_results_independently_of_hash_map_iteration_order() {
        let mut first_candidates = HashMap::new();
        first_candidates.insert(
            "candidate-1".to_string(),
            CandidateResults {
                candidate_id: "candidate-1".to_string(),
                total_votes: Some(10),
            },
        );
        first_candidates.insert(
            "candidate-2".to_string(),
            CandidateResults {
                candidate_id: "candidate-2".to_string(),
                total_votes: Some(7),
            },
        );

        let mut second_candidates = HashMap::new();
        second_candidates.insert(
            "candidate-2".to_string(),
            CandidateResults {
                candidate_id: "candidate-2".to_string(),
                total_votes: Some(7),
            },
        );
        second_candidates.insert(
            "candidate-1".to_string(),
            CandidateResults {
                candidate_id: "candidate-1".to_string(),
                total_votes: Some(10),
            },
        );

        let first = area_contest_results(first_candidates);
        let second = area_contest_results(second_candidates);

        assert_eq!(
            hash_area_contest_results(&first).unwrap(),
            hash_area_contest_results(&second).unwrap()
        );
    }

    #[test]
    fn blank_ballots_none_hashes_identically_to_before_the_field_existed() {
        // Pins backward compatibility for every sheet approved before this
        // field existed: their content deserializes with blank_ballots:
        // None, and must still hash exactly as it did when the field
        // didn't exist at all, or the next import diff would report every
        // one of them as CHANGED.
        #[derive(Serialize)]
        struct PreExistingFieldShape<'a> {
            area_id: &'a str,
            contest_id: &'a str,
            total_votes: Option<u64>,
            total_valid_votes: Option<u64>,
            invalid_votes: &'a Option<InvalidVotes>,
            total_blank_votes: Option<u64>,
            census: Option<u64>,
            candidate_results: BTreeMap<&'a str, CanonicalCandidateResults>,
        }

        let content = area_contest_results(HashMap::new());
        assert_eq!(content.blank_ballots, None);

        let old_shape = PreExistingFieldShape {
            area_id: &content.area_id,
            contest_id: &content.contest_id,
            total_votes: content.total_votes,
            total_valid_votes: content.total_valid_votes,
            invalid_votes: &content.invalid_votes,
            total_blank_votes: content.total_blank_votes,
            census: content.census,
            candidate_results: BTreeMap::new(),
        };
        let expected_hash = hash_bytes(&serde_json::to_vec(&old_shape).unwrap());

        assert_eq!(hash_area_contest_results(&content).unwrap(), expected_hash);
    }

    #[test]
    fn hash_changes_when_blank_ballots_differs() {
        let mut with_three = area_contest_results(HashMap::new());
        with_three.blank_ballots = Some(3);
        let mut with_five = area_contest_results(HashMap::new());
        with_five.blank_ballots = Some(5);

        assert_ne!(
            hash_area_contest_results(&with_three).unwrap(),
            hash_area_contest_results(&with_five).unwrap()
        );
        assert_ne!(
            hash_area_contest_results(&with_three).unwrap(),
            hash_area_contest_results(&area_contest_results(HashMap::new())).unwrap()
        );
    }

    fn area_contest_results(
        candidate_results: HashMap<String, CandidateResults>,
    ) -> AreaContestResults {
        AreaContestResults {
            area_id: "area-1".to_string(),
            contest_id: "contest-1".to_string(),
            total_votes: Some(20),
            total_valid_votes: Some(18),
            invalid_votes: Some(InvalidVotes {
                total_invalid: Some(2),
                implicit_invalid: Some(2),
                explicit_invalid: Some(0),
            }),
            total_blank_votes: Some(1),
            blank_ballots: None,
            census: Some(25),
            candidate_results,
        }
    }
}
