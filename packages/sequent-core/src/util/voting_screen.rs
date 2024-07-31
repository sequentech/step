// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::ballot::*;
use crate::plaintext::*;
use std::collections::HashMap;

extern crate console_error_panic_hook;


pub fn check_voting_not_allowed_next_util(
    contests: Vec<Contest>,
    decoded_contests: HashMap<String, DecodedVoteContest>,
) -> bool {
    let voting_not_allowed = contests.iter().any(|contest| {
        let default_vote_policy = InvalidVotePolicy::default();
        let vote_policy = contest
            .presentation
            .as_ref()
            .and_then(|p| p.invalid_vote_policy.as_ref())
            .unwrap_or(&default_vote_policy);

        let default_blank_policy = EBlankVotePolicy::default();
        let blank_policy = contest
            .presentation
            .as_ref()
            .and_then(|p| p.blank_vote_policy.as_ref())
            .unwrap_or(&default_blank_policy);

        if let Some(decoded_contest) = decoded_contests.get(&contest.id) {
            let choices_selected = decoded_contest
                .choices
                .iter()
                .any(|choice| choice.selected == 0);
            let invalid_errors: Vec<InvalidPlaintextError> =
                decoded_contest.invalid_errors.clone();
            invalid_errors.iter().any(|error| {
                matches!(
                    error.error_type,
                    InvalidPlaintextErrorType::Explicit
                        | InvalidPlaintextErrorType::EncodingError
                )
            }) || (invalid_errors.len() > 0
                && *vote_policy == InvalidVotePolicy::NOT_ALLOWED)
                || (!choices_selected
                    && *blank_policy == EBlankVotePolicy::NOT_ALLOWED)
        } else {
            false
        }
    });

    voting_not_allowed
}

pub fn check_voting_error_dialog_util(
    contests: Vec<Contest>,
    decoded_contests: HashMap<String, DecodedVoteContest>,
) -> bool {
    let show_voting_alert = contests.iter().any(|contest| {
        let default_vote_policy = InvalidVotePolicy::default();
        let vote_policy = contest
            .presentation
            .as_ref()
            .and_then(|p| p.invalid_vote_policy.as_ref())
            .unwrap_or(&default_vote_policy);

        let default_blank_policy = EBlankVotePolicy::default();
        let blank_policy = contest
            .presentation
            .as_ref()
            .and_then(|p| p.blank_vote_policy.as_ref())
            .unwrap_or(&default_blank_policy);

        if let Some(decoded_contest) = decoded_contests.get(&contest.id) {
            let choices_selected = decoded_contest
                .choices
                .iter()
                .any(|choice| choice.selected == 0);
            let invalid_errors: Vec<InvalidPlaintextError> =
                decoded_contest.invalid_errors.clone();
            let explicit_invalid = decoded_contest.is_explicit_invalid;
            (invalid_errors.len() > 0
                && *vote_policy != InvalidVotePolicy::ALLOWED)
                || (*vote_policy
                    == InvalidVotePolicy::WARN_INVALID_IMPLICIT_AND_EXPLICIT
                    && explicit_invalid)
                || (*blank_policy == EBlankVotePolicy::WARN
                    && !choices_selected)
        } else {
            false
        }
    });

    show_voting_alert
}



// // TESTS: WIP
// #[cfg(test)]
// mod tests {
//     use uuid::Uuid;
//     use crate::ballot::{Candidate, Contest, ContestPresentation, EBlankVotePolicy, InvalidVotePolicy};
//     use crate::voting_screen::{ContestPresentation, HashableBallot, Contest};

//     #[test]
//     fn test_check_voting_not_allowed_next() {
//         let tenant_id = Uuid::new_v4().to_string();
//         let election_event_id = Uuid::new_v4().to_string();
//         let election_id = Uuid::new_v4().to_string();
//         // Create mock data for candidates
//         let mut candidate = Candidate {
//             id: Uuid::new_v4(),
//             tenant_id: tenant_id.clone(),
//             election_event_id: election_event_id.clone(),
//             election_id: election_id.clone(),
//         };
//         // Create mock data for contests
//         let contest1_id = Uuid::new_v4().to_string();
//         let contest1 = Contest {
//             id: contest1_id.clone(),
//             tenant_id: tenant_id.clone(),
//             election_event_id: election_event_id.clone(),
//             election_id: election_id.clone(),
//             max_votes: 4,
//             min_votes: 2,
//             is_encrypted: true,
//             candidates: vec![...candidate, contest_id: contest1_id],
//             presentation: Some(ContestPresentation {
//                 invalid_vote_policy: Some(InvalidVotePolicy::NOT_ALLOWED),
//                 blank_vote_policy: Some(EBlankVotePolicy::ALLOWED),
//             }),
//         };
//         let contest2_id = Uuid::new_v4();
//         let contest2 = Contest {
//             id: contest2_id.clone(),
//             tenant_id: tenant_id.clone(),
//             election_event_id: election_event_id.clone(),
//             election_id: election_id.clone(),
//             max_votes: 4,
//             min_votes: 2,
//             is_encrypted: true,
//             candidates: vec![...candidate, contest_id: contest2_id],
//             presentation: Some(ContestPresentation {
//                 invalid_vote_policy: Some(InvalidVotePolicy::ALLOWED),
//                 blank_vote_policy: Some(EBlankVotePolicy::NOT_ALLOWED),
//             }),
//         };
//         let contest3_id = Uuid::new_v4();
//         let contest3 = Contest {
//             id: contest3_id.clone(),
//             tenant_id: tenant_id.clone(),
//             election_event_id: election_event_id.clone(),
//             election_id: election_id.clone(),
//             max_votes: 4,
//             min_votes: 2,
//             is_encrypted: true,
//             candidates: vec![...candidate, contest_id: contest3_id],
//             presentation: Some(ContestPresentation {
//                 invalid_vote_policy: Some(InvalidVotePolicy::ALLOWED),
//                 blank_vote_policy: Some(EBlankVotePolicy::ALLOWED),
//             }),
//         };
//         let contests = vec![contest1, contest2];
//         candidate.contest_id = contest1.id.clone();
//         // Create mock data for decoded contests
//         let mut decoded_contests = HashMap::new<String, DecodedVoteContest>();
//         let decoded_contest = DecodedVoteContest {
//             choices: vec![Choice { selected: 0 }],
//             invalid_errors: vec![InvalidPlaintextError {
//                 error_type: InvalidPlaintextErrorType::Explicit,
//             }],
//             is_explicit_invalid: true,
//         };
//         decoded_contests.insert("contest1".to_string(), decoded_contest);

//         // Test the function
//         let result = check_voting_not_allowed_next(contests, decoded_contests);
//         assert_eq!(result.unwrap(), true); //TODO: unwrap or
//     }




//     // #[test]
//     // fn test_check_voting_error_dialog() {
//     //     // Create mock data for contests
//     //     let contests = get_contest_plurality();

//     //     // Create mock data for decoded contests
//     //     let mut decoded_contests = HashMap::new();
//     //     let decoded_contest = DecodedVoteContest {
//     //         choices: vec![Choice { selected: 0 }],
//     //         invalid_errors: vec![InvalidPlaintextError {
//     //             error_type: InvalidPlaintextErrorType::Explicit,
//     //         }],
//     //         is_explicit_invalid: true,
//     //     };
//     //     decoded_contests.insert("contest2".to_string(), decoded_contest);

//     //     // Test the function
//     //     let result = check_voting_error_dialog(contests, decoded_contests);
//     //     assert_eq!(result.unwrap(), true);
//     // }
// }
