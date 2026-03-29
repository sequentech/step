// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use sequent_core::ballot::{Candidate, CandidatePresentation};
use uuid::Uuid;

#[allow(unused)]
/// Returns a test candidate with ID "0".
pub fn get_candidate_0(
    tenant_id: &Uuid,
    election_event_id: &Uuid,
    election_id: &Uuid,
    contest_id: &Uuid,
) -> Candidate {
    Candidate {
        id: "0".to_string(),
        tenant_id: (tenant_id.to_string()),
        election_event_id: (election_event_id.to_string()),
        election_id: (election_id.to_string()),
        contest_id: (contest_id.to_string()),
        name: Some(String::from("José Rabano Pimiento")),
        name_i18n: None,
        description: None,
        description_i18n: None,
        alias: None,
        alias_i18n: None,
        candidate_type: None,
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
        annotations: None,
    }
}

#[allow(unused)]
/// Return candidate with ID "1".
pub fn get_candidate_1(
    tenant_id: &Uuid,
    election_event_id: &Uuid,
    election_id: &Uuid,
    contest_id: &Uuid,
) -> Candidate {
    Candidate {
        id: "1".to_string(),
        tenant_id: (tenant_id.to_string()),
        election_event_id: (election_event_id.to_string()),
        election_id: (election_id.to_string()),
        contest_id: (contest_id.to_string()),
        name: Some(String::from("Miguel Pimentel Inventado")),
        name_i18n: None,
        description: None,
        description_i18n: None,
        alias: None,
        alias_i18n: None,
        candidate_type: None,
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
        annotations: None,
    }
}

#[allow(unused)]
/// Return candidate with ID "2".
pub fn get_candidate_2(
    tenant_id: &Uuid,
    election_event_id: &Uuid,
    election_id: &Uuid,
    contest_id: &Uuid,
) -> Candidate {
    Candidate {
        id: "2".to_string(),
        tenant_id: (tenant_id.to_string()),
        election_event_id: (election_event_id.to_string()),
        election_id: (election_id.to_string()),
        contest_id: (contest_id.to_string()),
        name: Some(String::from("Juan Iglesias Torquemada")),
        name_i18n: None,
        description: None,
        description_i18n: None,
        alias: None,
        alias_i18n: None,
        candidate_type: None,
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
        annotations: None,
    }
}

#[allow(unused)]
/// Return candidate with ID "3".
pub fn get_candidate_3(
    tenant_id: &Uuid,
    election_event_id: &Uuid,
    election_id: &Uuid,
    contest_id: &Uuid,
) -> Candidate {
    Candidate {
        id: "3".to_string(),
        tenant_id: (tenant_id.to_string()),
        election_event_id: (election_event_id.to_string()),
        election_id: (election_id.to_string()),
        contest_id: (contest_id.to_string()),
        name: Some(String::from("Mari Pili Hernández Ordoñez")),
        name_i18n: None,
        description: None,
        description_i18n: None,
        alias: None,
        alias_i18n: None,
        candidate_type: None,
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
        annotations: None,
    }
}

#[allow(unused)]
/// Return candidate with ID "4".
pub fn get_candidate_4(
    tenant_id: &Uuid,
    election_event_id: &Uuid,
    election_id: &Uuid,
    contest_id: &Uuid,
) -> Candidate {
    Candidate {
        id: "4".to_string(),
        tenant_id: (tenant_id.to_string()),
        election_event_id: (election_event_id.to_string()),
        election_id: (election_id.to_string()),
        contest_id: (contest_id.to_string()),
        name: Some(String::from("Juan Y Medio")),
        name_i18n: None,
        description: None,
        description_i18n: None,
        alias: None,
        alias_i18n: None,
        candidate_type: None,
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
        annotations: None,
    }
}

#[allow(unused)]
/// Return candidate with ID "5".
pub fn get_candidate_5(
    tenant_id: &Uuid,
    election_event_id: &Uuid,
    election_id: &Uuid,
    contest_id: &Uuid,
) -> Candidate {
    Candidate {
        id: "5".to_string(),
        tenant_id: tenant_id.to_string(),
        election_event_id: election_event_id.to_string(),
        election_id: election_id.to_string(),
        contest_id: contest_id.to_string(),
        name: Some(String::from("Spiderman")),
        name_i18n: None,
        description: None,
        description_i18n: None,
        alias: None,
        alias_i18n: None,
        candidate_type: None,
        presentation: Some(CandidatePresentation {
            i18n: None,
            is_explicit_invalid: Some(false),
            is_explicit_blank: Some(false),
            is_disabled: Some(false),
            is_write_in: Some(false),
            sort_order: Some(5),
            urls: None,
            invalid_vote_position: None,
            is_category_list: Some(false),
            subtype: None,
        }),
        annotations: None,
    }
}
