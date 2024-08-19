// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use sequent_core::ballot::{Candidate, CandidatePresentation};
use uuid::Uuid;

#[allow(unused)]
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
        name: Some("José Rabano Pimiento".into()),
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
        name: Some("Miguel Pimentel Inventado".into()),
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
        name: Some("Juan Iglesias Torquemada".into()),
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
        name: Some("Mari Pili Hernández Ordoñez".into()),
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
        name: Some("Juan Y Medio".into()),
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
        name: Some("Spiderman".into()),
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
