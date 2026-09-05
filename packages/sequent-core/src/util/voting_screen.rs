// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::ballot::*;
use crate::plaintext::*;
use crate::types::ceremonies::CountingAlgType;
use crate::validation::BallotValidator;

use std::collections::HashMap;

// Function used to decide if the voter needs to change his/her ballot before
// continuing
pub fn check_voting_not_allowed_next_util(
    contests: Vec<Contest>,
    decoded_contests: HashMap<String, DecodedVoteContest>,
) -> bool {
    BallotValidator::for_ballot(&contests, &decoded_contests).hard_gate()
}

/// if returns true, when the user click next, there will be a dialog that
/// prompts the user to confirm before going to the next screen
pub fn check_voting_error_dialog_util(
    contests: Vec<Contest>,
    decoded_contests: HashMap<String, DecodedVoteContest>,
) -> bool {
    BallotValidator::for_ballot(&contests, &decoded_contests).soft_gate()
}

pub fn get_contest_plurality(
    over_vote_policy: EOverVotePolicy,
    blank_vote_policy: EBlankVotePolicy,
    invalid_vote_policy: InvalidVotePolicy,
    min_votes: Option<i64>,
) -> Contest {
    let min_votes = min_votes.unwrap_or(1);

    Contest {
        created_at: None,
        id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into(),
        tenant_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into(),
        election_event_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into(),
        election_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into(),
        name: Some("Secretario General".into()),
        name_i18n: None,
        alias: None,
        alias_i18n: None,
        winning_candidates_num: 1,
        description: Some(
            "Elige quien quieres que sea tu Secretario General en tu municipio"
                .into(),
        ),
        description_i18n: None,
        max_votes: 3,
        min_votes,
        voting_type: Some("first-past-the-post".into()),
        counting_algorithm: Some(CountingAlgType::PluralityAtLarge),
        is_encrypted: true,
        is_acclaimed: None,
        annotations: None,
        candidates: vec![
            Candidate {
                id: "0".into(),
                tenant_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into(),
                election_event_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46"
                    .into(),
                election_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into(),
                contest_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into(),
                name: Some("José Rabano Pimiento".into()),
                name_i18n: None,
                alias: None,
                alias_i18n: None,
                description: None,
                description_i18n: None,
                candidate_type: None,
                annotations: None,
                presentation: Some(CandidatePresentation {
                    i18n: None,
                    is_explicit_invalid: Some(false),
                    is_explicit_blank: Some(false),
                    is_disabled: Some(false),
                    is_write_in: Some(false),
                    sort_order: Some(0),
                    urls: None,
                    invalid_vote_position: None,
                    is_category_list: Some(false),
                    subtype: None,
                }),
            },
            Candidate {
                id: "1".into(),
                tenant_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into(),
                election_event_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46"
                    .into(),
                election_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into(),
                contest_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into(),
                name: Some("Miguel Pimentel Inventado".into()),
                name_i18n: None,
                alias: None,
                alias_i18n: None,
                description: None,
                description_i18n: None,
                candidate_type: None,
                annotations: None,
                presentation: Some(CandidatePresentation {
                    i18n: None,
                    is_explicit_invalid: Some(false),
                    is_explicit_blank: Some(false),
                    is_disabled: Some(false),
                    is_write_in: Some(false),
                    sort_order: Some(1),
                    urls: None,
                    invalid_vote_position: None,
                    is_category_list: Some(false),
                    subtype: None,
                }),
            },
            Candidate {
                id: "2".into(),
                tenant_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into(),
                election_event_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46"
                    .into(),
                election_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into(),
                contest_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into(),
                name: Some("Juan Iglesias Torquemada".into()),
                name_i18n: None,
                alias: None,
                alias_i18n: None,
                description: None,
                description_i18n: None,
                candidate_type: None,
                annotations: None,
                presentation: Some(CandidatePresentation {
                    i18n: None,
                    is_explicit_invalid: Some(false),
                    is_explicit_blank: Some(false),
                    is_disabled: Some(false),
                    is_write_in: Some(false),
                    sort_order: Some(2),
                    urls: None,
                    invalid_vote_position: None,
                    is_category_list: Some(false),
                    subtype: None,
                }),
            },
            Candidate {
                id: "3".into(),
                tenant_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into(),
                election_event_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46"
                    .into(),
                election_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into(),
                contest_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into(),
                name: Some("Mari Pili Hernández Ordoñez".into()),
                name_i18n: None,
                alias: None,
                alias_i18n: None,
                description: None,
                description_i18n: None,
                candidate_type: None,
                annotations: None,
                presentation: Some(CandidatePresentation {
                    i18n: None,
                    is_explicit_invalid: Some(false),
                    is_explicit_blank: Some(false),
                    is_disabled: Some(false),
                    is_write_in: Some(false),
                    sort_order: Some(3),
                    urls: None,
                    invalid_vote_position: None,
                    is_category_list: Some(false),
                    subtype: None,
                }),
            },
            Candidate {
                id: "4".into(),
                tenant_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into(),
                election_event_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46"
                    .into(),
                election_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into(),
                contest_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into(),
                name: Some("Juan Y Medio".into()),
                name_i18n: None,
                alias: None,
                alias_i18n: None,
                description: None,
                description_i18n: None,
                candidate_type: None,
                annotations: None,
                presentation: Some(CandidatePresentation {
                    i18n: None,
                    is_explicit_invalid: Some(false),
                    is_explicit_blank: Some(false),
                    is_disabled: Some(false),
                    is_write_in: Some(false),
                    sort_order: Some(4),
                    urls: None,
                    invalid_vote_position: None,
                    is_category_list: Some(false),
                    subtype: None,
                }),
            },
        ],
        presentation: Some(ContestPresentation {
            i18n: None,
            allow_writeins: Some(false),
            base32_writeins: Some(true),
            cumulative_number_of_checkboxes: None,
            shuffle_categories: Some(true),
            shuffle_category_list: None,
            show_points: Some(false),
            enable_checkable_lists: None,
            collapsible_lists: None,
            candidates_order: None,
            candidates_selection_policy: None,
            candidates_icon_checkbox_policy: None,
            max_selections_per_type: None,
            types_presentation: None,
            sort_order: None,
            under_vote_policy: Some(EUnderVotePolicy::ALLOWED),
            invalid_vote_policy: Some(invalid_vote_policy),
            blank_vote_policy: Some(blank_vote_policy),
            over_vote_policy: Some(over_vote_policy),
            duplicated_rank_policy: None,
            preference_gaps_policy: None,
            pagination_policy: None,
            columns: None,
        }),
        tie_breaking_policy: None,
    }
}

pub fn get_decoded_contest_plurality(contest: &Contest) -> DecodedVoteContest {
    let message_map = [
        ("max".to_string(), "1".to_string()),
        ("min".to_string(), "0".to_string()),
        ("numSelected".to_string(), "0".to_string()),
        ("type".to_string(), "alert".to_string()),
    ]
    .iter()
    .cloned()
    .collect();

    DecodedVoteContest {
        contest_id: contest.id.clone(),
        is_explicit_invalid: true,
        is_decline_to_vote: false,
        is_blank_ballot: false,
        invalid_alerts: vec![InvalidPlaintextError {
            error_type: InvalidPlaintextErrorType::Explicit,
            candidate_id: None,
            message: Some("errors.implicit.underVote".to_string()),
            message_map,
        }],
        invalid_errors: vec![],
        choices: vec![DecodedVoteChoice {
            id: "b11b19c6-7157-4f26-b2e9-b5e353f252c2".into(),
            selected: -1,
            write_in_text: None,
        }],
    }
}
