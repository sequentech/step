// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use sequent_core::ballot::{
    Contest, ContestPresentation, EOverVotePolicy, EUnderVotePolicy, InvalidVotePolicy,
};
use uuid::Uuid;

use super::candidates;

#[allow(unused)]
pub fn get_contest_1(tenant_id: &Uuid, election_event_id: &Uuid, election_id: &Uuid) -> Contest {
    let contest_id = Uuid::new_v4();
    Contest {
        created_at: None,
        id: contest_id.to_string(),
        tenant_id: (tenant_id.to_string()),
        election_event_id: (election_event_id.to_string()),
        election_id: (election_id.to_string()),
        name: Some("Secretario <strong><em>General</em></strong>".into()),
        description: Some(
            "<strong>Elige</strong> quien quieres que sea tu Secretario General en tu municipio.<br/>Hello,<br>World!"
                .into(),
        ),
        name_i18n: None,
        description_i18n: None,
        alias: None,
        alias_i18n: None,
        max_votes: (1),
        min_votes: (0),
        winning_candidates_num: (1),
        voting_type: Some("first-past-the-post".into()),
        counting_algorithm: Some("plurality-at-large".into()), /* plurality-at-large|borda-nauru|borda|borda-mas-madrid|desborda3|desborda2|desborda|cumulative */
        is_encrypted: (true),
        candidates: vec![
            candidates::get_candidate_0(tenant_id, election_event_id, election_id, &contest_id),
            candidates::get_candidate_1(tenant_id, election_event_id, election_id, &contest_id),
            candidates::get_candidate_2(tenant_id, election_event_id, election_id, &contest_id),
            candidates::get_candidate_3(tenant_id, election_event_id, election_id, &contest_id),
            candidates::get_candidate_4(tenant_id, election_event_id, election_id, &contest_id),
        ],
        presentation: Some(ContestPresentation {
            i18n: None,
            allow_writeins: Some(false),
            base32_writeins: Some(true),
            invalid_vote_policy: Some(InvalidVotePolicy::ALLOWED),
            blank_vote_policy: None,
            over_vote_policy: Some(EOverVotePolicy::ALLOWED_WITH_MSG_AND_ALERT),
            pagination_policy: None,
            cumulative_number_of_checkboxes: None,
            shuffle_categories: Some(true),
            shuffle_category_list: None,
            show_points: Some(false),
            enable_checkable_lists: None,
            candidates_order: None,
            candidates_selection_policy: None,
            candidates_icon_checkbox_policy: None,
            max_selections_per_type: None,
            types_presentation: None,
            sort_order: None,
            under_vote_policy: Some(EUnderVotePolicy::ALLOWED),
            columns: None,
        }),
        annotations: None,
    }
}

#[allow(unused)]
pub fn get_contest_min_max_votes(
    tenant_id: &Uuid,
    election_event_id: &Uuid,
    election_id: &Uuid,
    min_votes: u64,
    max_votes: u64,
) -> Contest {
    let contest_id = Uuid::new_v4();
    Contest {
        created_at: None,
        id: contest_id.to_string(),
        tenant_id: (tenant_id.to_string()),
        election_event_id: (election_event_id.to_string()),
        election_id: (election_id.to_string()),
        name: Some("Secretario General".into()),
        description: Some(
            "Elige quien quieres que sea tu Secretario General en tu municipio".into(),
        ),
        name_i18n: None,
        description_i18n: None,
        alias: None,
        alias_i18n: None,
        max_votes: (max_votes as i64),
        min_votes: (min_votes as i64),
        winning_candidates_num: (1),
        voting_type: Some("first-past-the-post".into()),
        counting_algorithm: Some("plurality-at-large".into()), /* plurality-at-large|borda-nauru|borda|borda-mas-madrid|desborda3|desborda2|desborda|cumulative */
        is_encrypted: (true),
        candidates: vec![
            candidates::get_candidate_0(tenant_id, election_event_id, election_id, &contest_id),
            candidates::get_candidate_1(tenant_id, election_event_id, election_id, &contest_id),
            candidates::get_candidate_2(tenant_id, election_event_id, election_id, &contest_id),
            candidates::get_candidate_3(tenant_id, election_event_id, election_id, &contest_id),
            candidates::get_candidate_4(tenant_id, election_event_id, election_id, &contest_id),
        ],
        presentation: Some(ContestPresentation {
            i18n: None,
            allow_writeins: Some(false),
            base32_writeins: Some(true),
            invalid_vote_policy: Some(InvalidVotePolicy::ALLOWED),
            blank_vote_policy: None,
            over_vote_policy: Some(EOverVotePolicy::ALLOWED_WITH_MSG_AND_ALERT),
            pagination_policy: None,
            cumulative_number_of_checkboxes: None,
            shuffle_categories: Some(true),
            shuffle_category_list: None,
            show_points: Some(false),
            enable_checkable_lists: None,
            candidates_order: None,
            candidates_selection_policy: None,
            candidates_icon_checkbox_policy: None,
            max_selections_per_type: None,
            types_presentation: None,
            sort_order: None,
            under_vote_policy: Some(EUnderVotePolicy::ALLOWED),
            columns: None,
        }),
        annotations: None,
    }
}
