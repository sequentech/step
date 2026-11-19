// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Counts decrypted single-contest votes by `plurality-at-large`, for the
//! result statement and the election verifier. Deliberately separate from the
//! results tally's counting, so that the verifier's check of the stored result
//! does not reuse the code that produced it.

use crate::ballot::Contest;
use crate::ballot_codec::PlaintextCodec;
use crate::types::result_statement::ResultCounts;
use std::collections::HashSet;

/// One vote per payload. A payload that does not decode with the contest's
/// bases is an implicit invalid vote; a declined vote is not counted.
pub fn count_contest_votes(
    contest: &Contest,
    payloads: &[[u8; 30]],
) -> ResultCounts {
    let explicit_blank: HashSet<&str> = contest
        .candidates
        .iter()
        .filter(|candidate| candidate.is_explicit_blank())
        .map(|candidate| candidate.id.as_str())
        .collect();
    let mut counts = ResultCounts::default();
    for payload in payloads {
        let Ok(vote) = contest.decode_plaintext_contest(payload) else {
            counts.implicit_invalid_votes += 1;
            continue;
        };
        let selected: Vec<&str> = vote
            .choices
            .iter()
            .filter(|choice| choice.selected >= 0)
            .map(|choice| choice.id.as_str())
            .collect();
        let blank_selected =
            selected.iter().any(|id| explicit_blank.contains(id));
        let other_selected =
            selected.iter().any(|id| !explicit_blank.contains(id));
        if vote.is_decline_to_vote() {
            if !vote.is_blank() {
                counts.implicit_invalid_votes += 1;
            }
        } else if vote.is_invalid() {
            if vote.is_explicit_invalid {
                counts.explicit_invalid_votes += 1;
            } else {
                counts.implicit_invalid_votes += 1;
            }
        } else if blank_selected && other_selected {
            counts.implicit_invalid_votes += 1;
        } else if blank_selected || vote.is_blank() {
            counts.blank_votes += 1;
            counts.total_valid_votes += 1;
        } else {
            for id in selected {
                *counts.candidates.entry(id.to_string()).or_insert(0) += 1;
            }
            counts.total_valid_votes += 1;
        }
    }
    counts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::ballot_codec::get_test_contest;
    use crate::plaintext::DecodedVoteContest;

    fn vote(contest: &Contest, selected: &[usize]) -> [u8; 30] {
        let decoded = DecodedVoteContest {
            contest_id: contest.id.clone(),
            is_explicit_invalid: false,
            is_decline_to_vote: false,
            is_blank_ballot: false,
            invalid_errors: vec![],
            invalid_alerts: vec![],
            choices: contest
                .candidates
                .iter()
                .enumerate()
                .map(|(index, candidate)| {
                    crate::plaintext::DecodedVoteChoice {
                        id: candidate.id.clone(),
                        selected: if selected.contains(&index) {
                            0
                        } else {
                            -1
                        },
                        write_in_text: None,
                    }
                })
                .collect(),
        };
        contest.encode_plaintext_contest(&decoded).unwrap()
    }

    #[test]
    fn counts_candidates_blank_and_undecodable_votes() {
        // A blank vote is valid only where no selection is required.
        let mut contest = get_test_contest();
        contest.min_votes = 0;
        // Too many selections, flagged by the codec as explicitly invalid.
        let mut too_large = [0xff_u8; 30];
        too_large[0] = 29;
        // A length byte beyond the payload: the codec cannot read it.
        let unreadable = [0xff_u8; 30];
        let payloads = [
            vote(&contest, &[0]),
            vote(&contest, &[]),
            too_large,
            unreadable,
        ];
        let counts = count_contest_votes(&contest, &payloads);
        assert_eq!(counts.total_valid_votes, 2);
        assert_eq!(counts.blank_votes, 1);
        assert_eq!(counts.explicit_invalid_votes, 1);
        assert_eq!(counts.implicit_invalid_votes, 1);
        assert_eq!(
            counts.candidates.get(&contest.candidates[0].id).copied(),
            Some(1)
        );
    }
}
