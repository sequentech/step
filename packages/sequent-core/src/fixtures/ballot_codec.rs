// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::ballot::BallotStyle;
use crate::ballot::*;
use crate::ballot_codec::{vec_to_30_array, RawBallotContest};
use crate::plaintext::{
    DecodedVoteChoice, DecodedVoteContest, InvalidPlaintextError,
    InvalidPlaintextErrorType,
};
use crate::types::ceremonies::CountingAlgType;
use std::collections::HashMap;

pub struct BallotCodecFixture {
    pub title: String,
    pub contest: Contest,
    pub raw_ballot: RawBallotContest,
    pub plaintext: DecodedVoteContest,
    pub encoded_ballot_bigint: String,
    pub encoded_ballot: [u8; 30],
    pub expected_errors: Option<HashMap<String, String>>,
}
pub struct BasesFixture {
    pub contest: Contest,
    pub bases: Vec<u64>,
}

fn get_contest_plurality() -> Contest {
    Contest {
        created_at: None,
        id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into(),
        tenant_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
        election_event_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
        election_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
        name: Some("Secretario General".into()),
        name_i18n: None,
        alias: None,
        alias_i18n: None,
        winning_candidates_num: (1),
        description: Some(
            "Elige quien quieres que sea tu Secretario General en tu municipio"
                .into(),
        ),
        description_i18n: None,
        max_votes: (1),
        min_votes: (0),
        voting_type: Some("first-past-the-post".into()),
        counting_algorithm: Some(CountingAlgType::PluralityAtLarge), /* plurality-at-large|borda-nauru|borda|borda-mas-madrid|desborda3|desborda2|desborda|cumulative */
        is_encrypted: (true),
        annotations: None,
        candidates: vec![
            Candidate {
                id: "0".into(),
                tenant_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                election_event_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46"
                    .into()),
                election_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                contest_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                name: Some("José Rabano Pimiento".into()),
                name_i18n: None,
                alias: None,
                alias_i18n: None,
                description: None,
                description_i18n: None,
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
            },
            Candidate {
                id: "1".into(),
                tenant_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                election_event_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46"
                    .into()),
                election_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                contest_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                name: Some("Miguel Pimentel Inventado".into()),
                name_i18n: None,
                alias: None,
                alias_i18n: None,
                description: None,
                description_i18n: None,
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
            },
            Candidate {
                id: "2".into(),
                tenant_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                election_event_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46"
                    .into()),
                election_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                contest_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                name: Some("Juan Iglesias Torquemada".into()),
                name_i18n: None,
                alias: None,
                alias_i18n: None,
                description: None,
                description_i18n: None,
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
            },
            Candidate {
                id: "3".into(),
                tenant_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                election_event_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46"
                    .into()),
                election_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                contest_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                name: Some("Mari Pili Hernández Ordoñez".into()),
                name_i18n: None,
                alias: None,
                alias_i18n: None,
                description: None,
                description_i18n: None,

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
            },
            Candidate {
                id: "4".into(),
                tenant_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                election_event_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46"
                    .into()),
                election_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                contest_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                name: Some("Juan Y Medio".into()),
                name_i18n: None,
                alias: None,
                alias_i18n: None,
                description: None,
                description_i18n: None,
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
            },
        ],
        presentation: Some(ContestPresentation {
            i18n: None,
            allow_writeins: Some(false),
            base32_writeins: Some(true),
            invalid_vote_policy: Some(InvalidVotePolicy::ALLOWED),
            blank_vote_policy: None,
            over_vote_policy: None,
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
    }
}

fn get_contest_borda() -> Contest {
    let mut contest = get_contest_plurality();
    contest.counting_algorithm = Some(CountingAlgType::Borda);
    contest.max_votes = 4;
    contest
}

fn get_contest_irv() -> Contest {
    let mut contest = get_contest_plurality();
    contest.counting_algorithm = Some(CountingAlgType::InstantRunoff);
    contest.max_votes = 3;
    contest
}

pub fn get_irv_fixture() -> BallotCodecFixture {
    BallotCodecFixture {
        title: "irv_fixture".to_string(),
        contest: get_contest_irv(),
        raw_ballot: RawBallotContest {
            bases: vec![2u64, 4u64, 4u64, 4u64, 4u64, 4u64],
            choices: vec![0u64, 1u64, 2u64, 0u64, 3u64, 0u64],
        },
        plaintext: DecodedVoteContest {
            contest_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".to_string(),
            is_explicit_invalid: false,
            choices: vec![
                DecodedVoteChoice {
                    id: 0.to_string(),
                    selected: 0,
                    write_in_text: None,
                },
                DecodedVoteChoice {
                    id: 1.to_string(),
                    selected: 1,
                    write_in_text: None,
                },
                DecodedVoteChoice {
                    id: 2.to_string(),
                    selected: -1,
                    write_in_text: None,
                },
                DecodedVoteChoice {
                    id: 3.to_string(),
                    selected: 2,
                    write_in_text: None,
                },
                DecodedVoteChoice {
                    id: 4.to_string(),
                    selected: -1,
                    write_in_text: None,
                },
            ],
            invalid_errors: vec![],
            invalid_alerts: vec![],
        },
        encoded_ballot_bigint: "402".to_string(),
        encoded_ballot: vec_to_30_array(&vec![2, 146, 1]).unwrap(),
        expected_errors: None,
    }
}

pub fn get_test_decoded_vote_contest() -> DecodedVoteContest {
    DecodedVoteContest {
        contest_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".to_string(),
        is_explicit_invalid: false,
        invalid_errors: vec![],
        invalid_alerts: vec![],
        choices: vec![
            DecodedVoteChoice {
                id: "38df9caf-2dc8-472c-87f2-f003241e9510".to_string(),
                selected: 0,
                write_in_text: None,
            },
            DecodedVoteChoice {
                id: "97ac7d0a-e0f5-4e51-a1ee-6614c0836fec".to_string(),
                selected: -1,
                write_in_text: None,
            },
            DecodedVoteChoice {
                id: "94c9eafa-ebc6-4594-a176-24788f761ced".to_string(),
                selected: 0,
                write_in_text: None,
            },
        ],
    }
}

pub fn get_writein_ballot_style() -> BallotStyle {
    BallotStyle {
        id: "9570d82a-d92a-44d7-b483-d5a6c8c398a8".into(),
        tenant_id: "9570d82a-d92a-44d7-b483-d5a6c8c398a8".into(),
        election_event_id: "9570d82a-d92a-44d7-b483-d5a6c8c398a8".into(),
        election_id: "9570d82a-d92a-44d7-b483-d5a6c8c398a8".into(),
        num_allowed_revotes: Some(1),
        description: Some("Write-ins simple".into()),
        public_key: Some(PublicKeyConfig {
            public_key: "ajR/I9RqyOwbpsVRucSNOgXVLCvLpfQxCgPoXGQ2RF4".into(),
            is_demo: false,
        }),
        area_id: "9570d82a-d92a-44d7-b483-d5a6c8c398a8".into(),
        area_presentation: None,
        election_event_presentation: None,
        election_presentation: None,
        election_event_annotations: Default::default(),
        election_annotations: Default::default(),
        election_dates: None,
        contests: vec![Contest {
            created_at: None,
            id: "1c1500ac-173e-4e78-a59d-91bfa3678c5a".into(),
            tenant_id: ("9570d82a-d92a-44d7-b483-d5a6c8c398a8".into()),
            election_event_id: ("9570d82a-d92a-44d7-b483-d5a6c8c398a8".into()),
            election_id: ("9570d82a-d92a-44d7-b483-d5a6c8c398a8".into()),
            name: Some("Test contest title".into()),
            name_i18n: None,
            alias: None,
            alias_i18n: None,
            description: None,
            description_i18n: None,
            winning_candidates_num: (1),
            max_votes: (2),
            min_votes: (1),
            voting_type: Some("first-past-the-post".into()),
            counting_algorithm: Some(CountingAlgType::PluralityAtLarge),
            is_encrypted: (true),
            annotations: None,
            candidates: vec![
                Candidate {
                    id: "f257cd3a-d1cf-4b97-91f8-2dfe156b015c".into(),
                    tenant_id: ("9570d82a-d92a-44d7-b483-d5a6c8c398a8".into()),
                    election_event_id: ("9570d82a-d92a-44d7-b483-d5a6c8c398a8"
                        .into()),
                    election_id: ("9570d82a-d92a-44d7-b483-d5a6c8c398a8"
                        .into()),
                    contest_id: ("1c1500ac-173e-4e78-a59d-91bfa3678c5a".into()),
                    name: Some("Example option 1".into()),
                    name_i18n: None,
                    alias: None,
                    alias_i18n: None,
                    description: Some(
                        "This is an option with an simple example description."
                            .into(),
                    ),
                    description_i18n: None,
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
                },
                Candidate {
                    id: "17325099-f5ab-4c48-a142-6d7ed721e9bb".into(),
                    tenant_id: ("9570d82a-d92a-44d7-b483-d5a6c8c398a8".into()),
                    election_event_id: ("9570d82a-d92a-44d7-b483-d5a6c8c398a8"
                        .into()),
                    election_id: ("9570d82a-d92a-44d7-b483-d5a6c8c398a8"
                        .into()),
                    contest_id: ("1c1500ac-173e-4e78-a59d-91bfa3678c5a".into()),
                    name: Some("Example option 1".into()),
                    name_i18n: None,
                    alias: None,
                    alias_i18n: None,
                    description: Some(
                        "This is an option with an simple example description."
                            .into(),
                    ),
                    description_i18n: None,
                    candidate_type: None,
                    presentation: Some(CandidatePresentation {
                        i18n: None,
                        is_explicit_invalid: Some(false),
                        is_explicit_blank: Some(false),
                        is_disabled: Some(false),
                        is_write_in: Some(false),
                        sort_order: Some(1),
                        invalid_vote_position: None,
                        is_category_list: Some(false),
                        urls: Some(vec![
                            CandidateUrl {
                                url: "https://sequentech.io".into(),
                                kind: None,
                                title: None,
                                is_image: false,
                            },
                            CandidateUrl {
                                url: "/XFQwVFL.jpg".into(),
                                kind: None,
                                title: None,
                                is_image: true,
                            },
                        ]),
                        subtype: None,
                    }),
                    annotations: None,
                },
                Candidate {
                    id: "61320aac-0d78-4001-845e-a2f2bd8e800b".into(),
                    tenant_id: ("9570d82a-d92a-44d7-b483-d5a6c8c398a8".into()),
                    election_event_id: ("9570d82a-d92a-44d7-b483-d5a6c8c398a8"
                        .into()),
                    election_id: ("9570d82a-d92a-44d7-b483-d5a6c8c398a8"
                        .into()),
                    contest_id: ("1c1500ac-173e-4e78-a59d-91bfa3678c5a".into()),
                    name: None,
                    name_i18n: None,
                    alias: None,
                    alias_i18n: None,
                    description: None,
                    description_i18n: None,
                    candidate_type: None,
                    presentation: Some(CandidatePresentation {
                        i18n: None,
                        is_explicit_invalid: Some(false),
                        is_explicit_blank: Some(false),
                        is_disabled: Some(false),
                        is_write_in: Some(true),
                        sort_order: Some(2),
                        urls: None,
                        invalid_vote_position: None,
                        is_category_list: Some(false),
                        subtype: None,
                    }),
                    annotations: None,
                },
                Candidate {
                    id: "e9ad3ed1-4fd5-4498-a0e7-3a3c22ef57d5".into(),
                    tenant_id: ("9570d82a-d92a-44d7-b483-d5a6c8c398a8".into()),
                    election_event_id: ("9570d82a-d92a-44d7-b483-d5a6c8c398a8"
                        .into()),
                    election_id: ("9570d82a-d92a-44d7-b483-d5a6c8c398a8"
                        .into()),
                    contest_id: ("1c1500ac-173e-4e78-a59d-91bfa3678c5a".into()),
                    name: None,
                    name_i18n: None,
                    alias: None,
                    alias_i18n: None,
                    description: None,
                    description_i18n: None,
                    candidate_type: None,
                    presentation: Some(CandidatePresentation {
                        i18n: None,
                        is_explicit_invalid: Some(false),
                        is_explicit_blank: Some(false),
                        is_disabled: Some(false),
                        is_write_in: Some(true),
                        sort_order: Some(3),
                        urls: None,
                        invalid_vote_position: None,
                        is_category_list: Some(false),
                        subtype: None,
                    }),
                    annotations: None,
                },
            ],
            presentation: Some(ContestPresentation {
                i18n: None,
                allow_writeins: Some(true),
                base32_writeins: Some(true),
                invalid_vote_policy: Some(InvalidVotePolicy::ALLOWED),
                blank_vote_policy: None,
                over_vote_policy: None,
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
        }],
        area_annotations: None,
    }
}

pub fn get_too_long_writein_plaintext(increase: i64) -> DecodedVoteContest {
    let write_in = "THERE IS SOME VERY LARGE STRING BEING WRITTEN".to_string();

    let mod_write_in = if 0 == increase {
        write_in
    } else if increase > 0 {
        write_in + &"Z".repeat(increase as usize)
    } else {
        let trunc_len: i64 = write_in.len() as i64 + increase;
        let mut res = write_in.clone();
        res.truncate(trunc_len as usize);
        res
    };

    DecodedVoteContest {
        contest_id: "1c1500ac-173e-4e78-a59d-91bfa3678c5a".to_string(),
        is_explicit_invalid: false,
        choices: vec![
            DecodedVoteChoice {
                id: "17325099-f5ab-4c48-a142-6d7ed721e9bb".to_string(),
                selected: 0,
                write_in_text: None,
            },
            DecodedVoteChoice {
                id: "61320aac-0d78-4001-845e-a2f2bd8e800b".to_string(),
                selected: 0,
                write_in_text: Some(mod_write_in),
            },
            DecodedVoteChoice {
                id: "e9ad3ed1-4fd5-4498-a0e7-3a3c22ef57d5".to_string(),
                selected: -1,
                write_in_text: None,
            },
            DecodedVoteChoice {
                id: "f257cd3a-d1cf-4b97-91f8-2dfe156b015c".to_string(),
                selected: -1,
                write_in_text: None,
            },
        ],
        invalid_errors: vec![],
        invalid_alerts: vec![],
    }
}

pub fn get_writein_plaintext() -> DecodedVoteContest {
    DecodedVoteContest {
        contest_id: "1c1500ac-173e-4e78-a59d-91bfa3678c5a".to_string(),
        is_explicit_invalid: false,
        choices: vec![
            DecodedVoteChoice {
                id: "f257cd3a-d1cf-4b97-91f8-2dfe156b015c".to_string(),
                selected: -1,
                write_in_text: None,
            },
            DecodedVoteChoice {
                id: "17325099-f5ab-4c48-a142-6d7ed721e9bb".to_string(),
                selected: 0,
                write_in_text: None,
            },
            DecodedVoteChoice {
                id: "61320aac-0d78-4001-845e-a2f2bd8e800b".to_string(),
                selected: 0,
                write_in_text: Some("FELIX".to_string()),
            },
            DecodedVoteChoice {
                id: "e9ad3ed1-4fd5-4498-a0e7-3a3c22ef57d5".to_string(),
                selected: -1,
                write_in_text: None,
            },
        ],
        invalid_errors: vec![],
        invalid_alerts: vec![],
    }
}

pub fn get_test_contest() -> Contest {
    Contest {
        created_at:None,
        id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into(),
        tenant_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
        election_event_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
        election_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
        name: Some("Test contest title".into()),
        name_i18n:None,
        alias:None,alias_i18n:None,
        winning_candidates_num: (1),
        description: Some("This is the description of this contest. You can have multiple contests. You can add simple html like.".into()),
        description_i18n: None,
        max_votes: (3),
        min_votes: (1),
        voting_type: Some("first-past-the-post".into()),
        counting_algorithm: Some(CountingAlgType::PluralityAtLarge),
        is_encrypted: (true),
        annotations: None,
        candidates: vec![
            Candidate {
                id: "38df9caf-2dc8-472c-87f2-f003241e9510".into(),
                tenant_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                election_event_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                election_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                contest_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                name: Some("Example option 1".into()),
            name_i18n:None,
            alias:None,alias_i18n:None,
                description: Some("This is an option with an simple example description.".into()),
        description_i18n: None,
                candidate_type: None,
                presentation: Some(CandidatePresentation {
                    i18n: None,
                    is_explicit_invalid: Some(false),
                    is_explicit_blank: Some(false),
                    is_disabled: Some(false),
                    is_write_in: Some(false),
                    sort_order: Some(0),
                    invalid_vote_position: None,
                    is_category_list: Some(false),
                    urls: Some(vec![
                        CandidateUrl {
                            url: "https://i.imgur.com/XFQwVFL.jpg".into(),
                            kind: None,
                            title: Some("Image URL".into()),
                            is_image: true,
                        }
                    ]),
                    subtype: None,
                }),
                annotations: None,
            },
            Candidate {
                id: "97ac7d0a-e0f5-4e51-a1ee-6614c0836fec".into(),
                tenant_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                election_event_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                election_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                contest_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                name: Some("Example option 2".into()),
            name_i18n:None,
            alias:None,alias_i18n:None,
                description: Some("An option can contain a description. You can add simple html like ".into()),
        description_i18n: None,
                candidate_type: None,
                presentation: Some(CandidatePresentation {
                    i18n: None,
                    is_explicit_invalid: Some(false),
                    is_explicit_blank: Some(false),
                    is_disabled: Some(false),
                    is_write_in: Some(false),
                    sort_order: Some(1),
                    invalid_vote_position: None,
                    is_category_list: Some(false),
                    urls: Some(vec![
                        CandidateUrl {
                            url: "https://sequentech.io".into(),
                            kind: None,
                            title: Some("URL".into()),
                            is_image: false,
                        },
                        CandidateUrl {
                            url: "/XFQwVFL.jpg".into(),
                            kind: None,
                            title: Some("Image URL".into()),
                            is_image: true,
                        }
                    ]),
                    subtype: None,
                }),
                annotations: None,
            },
            Candidate {
                id: "94c9eafa-ebc6-4594-a176-24788f761ced".into(),
                tenant_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                election_event_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                election_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                contest_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                name: Some("Example option 3".into()),
            name_i18n:None,
            alias:None,alias_i18n:None,
                description: None,
        description_i18n: None,
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
        ],
        presentation: Some(ContestPresentation {
            i18n: None,
            allow_writeins: Some(true),
            base32_writeins: Some(true),
            invalid_vote_policy: Some(InvalidVotePolicy::ALLOWED),
            blank_vote_policy: None,
            over_vote_policy: None,
            pagination_policy: None,
            cumulative_number_of_checkboxes: None,
            shuffle_categories: Some(true),
            shuffle_category_list: None,
            show_points: Some(false),
            enable_checkable_lists: None,
            candidates_order:None,
            candidates_selection_policy: None,
            candidates_icon_checkbox_policy: None,
            max_selections_per_type: None,
            types_presentation: None,
            sort_order: None,
            under_vote_policy: Some(EUnderVotePolicy::ALLOWED),
            columns: None,
        }),
    }
}

pub(crate) fn get_configurable_contest(
    max: i64,
    num_candidates: usize,
    counting_algorithm: CountingAlgType,
    enable_writeins: bool,
    write_in_contests: Option<Vec<usize>>,
    base32_writeins: bool,
) -> Contest {
    let mut contest: Contest = Contest {
        created_at: None,
        id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into(),
        tenant_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
        election_event_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
        election_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
        name: Some("Secretario General".into()),
        name_i18n: None,
        alias: None,
        alias_i18n: None,
        description: Some(
            "Elige quien quieres que sea tu Secretario General en tu municipio"
                .into(),
        ),
        description_i18n: None,
        winning_candidates_num: (1),
        max_votes: (3),
        min_votes: (0),
        voting_type: Some("first-past-the-post".into()),
        counting_algorithm: Some(CountingAlgType::PluralityAtLarge),
        is_encrypted: (true),
        annotations: None,
        candidates: vec![
            Candidate {
                id: "0".into(),
                tenant_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                election_event_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46"
                    .into()),
                election_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                contest_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                name: Some("José Rabano Pimiento".into()),
                name_i18n: None,
                alias: None,
                alias_i18n: None,
                description: None,
                description_i18n: None,
                candidate_type: Some("Candidaturas no agrupadas".into()),
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
            },
            Candidate {
                id: "1".into(),
                tenant_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                election_event_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46"
                    .into()),
                election_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                contest_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                name: Some("Miguel Pimentel Inventado".into()),
                name_i18n: None,
                alias: None,
                alias_i18n: None,
                description: None,
                description_i18n: None,
                candidate_type: Some("Candidaturas no agrupadas".into()),
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
            },
            Candidate {
                id: "2".into(),
                tenant_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                election_event_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46"
                    .into()),
                election_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                contest_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                name: Some("Juan Iglesias Torquemada".into()),
                name_i18n: None,
                alias: None,
                alias_i18n: None,
                description: None,
                description_i18n: None,
                candidate_type: Some("Candidaturas no agrupadas".into()),
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
            },
            Candidate {
                id: "3".into(),
                tenant_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                election_event_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46"
                    .into()),
                election_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                contest_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                name: Some("Mari Pili Hernández Ordoñez".into()),
                name_i18n: None,
                alias: None,
                alias_i18n: None,
                description: None,
                description_i18n: None,
                candidate_type: Some("Candidaturas no agrupadas".into()),
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
            },
            Candidate {
                id: "4".into(),
                tenant_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                election_event_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46"
                    .into()),
                election_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                contest_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                name: Some("Juan Y Medio".into()),
                name_i18n: None,
                alias: None,
                alias_i18n: None,
                description: None,
                description_i18n: None,
                candidate_type: Some("Candidaturas no agrupadas".into()),
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
            },
            Candidate {
                id: "5".into(),
                tenant_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                election_event_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46"
                    .into()),
                election_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                contest_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                name: Some("Juan Y Medio".into()),
                name_i18n: None,
                alias: None,
                alias_i18n: None,
                description: None,
                description_i18n: None,
                candidate_type: Some("Candidaturas no agrupadas".into()),
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
            },
            Candidate {
                id: "6".into(),
                tenant_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                election_event_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46"
                    .into()),
                election_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                contest_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                name: Some("Juan Y Medio".into()),
                name_i18n: None,
                alias: None,
                alias_i18n: None,
                description: None,
                description_i18n: None,
                candidate_type: Some("Candidaturas no agrupadas".into()),
                presentation: Some(CandidatePresentation {
                    i18n: None,
                    is_explicit_invalid: Some(false),
                    is_explicit_blank: Some(false),
                    is_disabled: Some(false),
                    is_write_in: Some(false),
                    sort_order: Some(6),
                    urls: None,
                    invalid_vote_position: None,
                    is_category_list: Some(false),
                    subtype: None,
                }),
                annotations: None,
            },
        ],
        presentation: Some(ContestPresentation {
            i18n: None,
            allow_writeins: Some(true),
            base32_writeins: Some(true),
            invalid_vote_policy: Some(InvalidVotePolicy::NOT_ALLOWED),
            blank_vote_policy: None,
            over_vote_policy: Some(EOverVotePolicy::ALLOWED),
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
    };

    contest.counting_algorithm = Some(counting_algorithm);
    contest.max_votes = max;
    if enable_writeins {
        let mut presentation =
            contest.presentation.unwrap_or(ContestPresentation::new());
        presentation.allow_writeins = Some(true);

        contest.presentation = Some(presentation);
        let write_in_indexes =
            write_in_contests.unwrap_or_else(|| vec![4, 5, 6]);
        for write_in_index in write_in_indexes {
            if write_in_index < contest.candidates.len() {
                contest.candidates[write_in_index].set_is_write_in(true);
            }
        }
    }
    // set base32_writeins
    let mut presentation =
        contest.presentation.unwrap_or(ContestPresentation::new());
    presentation.base32_writeins = Some(base32_writeins);
    contest.presentation = Some(presentation);

    contest.candidates = contest.candidates[0..num_candidates].to_vec();
    contest
}

pub(crate) fn get_contest_candidates_n(num_candidates: usize) -> Contest {
    let candidates: Vec<Candidate> = (0..num_candidates)
        .map(|i| Candidate {
            annotations: None,
            id: i.to_string(),
            tenant_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
            election_event_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
            election_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
            contest_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
            name: Some("José Rabano Pimiento".into()),
            name_i18n: None,
            alias: None,
            alias_i18n: None,
            description: None,
            description_i18n: None,
            candidate_type: Some("Candidaturas no agrupadas".into()),
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
        })
        .collect();

    let mut contest: Contest = Contest {
        annotations: None,
        created_at: None,
        id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into(),
        tenant_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
        election_event_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
        election_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
        name: Some("Secretario General".into()),
        name_i18n: None,
        alias: None,
        alias_i18n: None,
        description: Some(
            "Elige quien quieres que sea tu Secretario General en tu municipio"
                .into(),
        ),
        description_i18n: None,
        winning_candidates_num: (1),
        max_votes: (200),
        min_votes: (0),
        voting_type: Some("first-past-the-post".into()),
        counting_algorithm: Some(CountingAlgType::PluralityAtLarge),
        is_encrypted: (true),
        candidates,
        presentation: Some(ContestPresentation {
            i18n: None,
            allow_writeins: Some(true),
            base32_writeins: Some(true),
            invalid_vote_policy: Some(InvalidVotePolicy::NOT_ALLOWED),
            blank_vote_policy: None,
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
            // FIXME check these two fields:
            over_vote_policy: None,
            under_vote_policy: None,
            columns: None,
        }),
    };

    let mut presentation =
        contest.presentation.unwrap_or(ContestPresentation::new());
    contest.presentation = Some(presentation);

    contest
}

pub fn get_fixtures() -> Vec<BallotCodecFixture> {
    vec![
        BallotCodecFixture {
            title: "plurality_fixture".to_string(),
            contest: {
                let mut contest = get_contest_plurality();
                if let Some(ref mut presentation) = contest.presentation {
                    presentation.invalid_vote_policy = Some(InvalidVotePolicy::WARN);
                }
                contest
            },
            raw_ballot: RawBallotContest {
                bases: vec![2u64, 2u64, 2u64, 2u64, 2u64, 2u64],
                choices: vec![0u64, 1u64, 0u64, 0u64, 1u64, 1u64],
            },
            plaintext: DecodedVoteContest {
                contest_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".to_string(),
                is_explicit_invalid: false,
                choices: vec![
                    DecodedVoteChoice {
                        id: 0.to_string(),
                        selected: 0,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 1.to_string(),
                        selected: -1,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 2.to_string(),
                        selected: -1,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 3.to_string(),
                        selected: 0,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 4.to_string(),
                        selected: 0,
                        write_in_text: None,
                    },
                ],
                invalid_errors: vec![
                    InvalidPlaintextError {
                        error_type: InvalidPlaintextErrorType::Implicit,
                        candidate_id: None,
                        message: Some("errors.implicit.selectedMax".to_string()),
                        message_map: HashMap::from([
                            ("numSelected".to_string(), 3.to_string()),
                            ("max".to_string(), 1.to_string()),
                        ]),
                    }
                ],
                invalid_alerts: vec![
                    InvalidPlaintextError {
                        error_type: InvalidPlaintextErrorType::Implicit,
                        candidate_id: None,
                        message: Some("errors.implicit.selectedMax".to_string()),
                        message_map: HashMap::from([
                            ("numSelected".to_string(), 3.to_string()),
                            ("max".to_string(), 1.to_string()),
                        ]),
                    }
                ],
            },
            encoded_ballot_bigint: "50".to_string(),
            encoded_ballot: vec_to_30_array(&vec![1, 50]).unwrap(),
            expected_errors: None
        },
        BallotCodecFixture {
            title: "borda_fixture".to_string(),
            contest: get_contest_borda(),
            raw_ballot: RawBallotContest {
                bases: vec![2u64, 5u64, 5u64, 5u64, 5u64, 5u64],
                choices: vec![0u64, 3u64, 0u64, 0u64, 1u64, 2u64],
            },
            plaintext: DecodedVoteContest {
                contest_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".to_string(),
                is_explicit_invalid: false,
                choices: vec![
                    DecodedVoteChoice {
                        id: 0.to_string(),
                        selected: 2,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 1.to_string(),
                        selected: -1,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 2.to_string(),
                        selected: -1,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 3.to_string(),
                        selected: 0,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 4.to_string(),
                        selected: 1,
                        write_in_text: None,
                    },
                ],
                invalid_errors: vec![],
                invalid_alerts: vec![],
            },
            encoded_ballot_bigint: "2756".to_string(),
            encoded_ballot: vec_to_30_array(&vec![2, 196, 10]).unwrap(),
            expected_errors: None
        },
        BallotCodecFixture {
            title: "example_3_explicit_and_implicit_invalid".to_string(),
            contest: Contest {
        created_at:None,
                id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into(),
                tenant_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                election_event_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                election_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                name: Some("Poste de maire(sse)".into()),
        name_i18n:None,
        alias:None,alias_i18n:None,
                description: None,
        description_i18n: None,
                max_votes: (1),
                winning_candidates_num: (1),
                min_votes: (0),
                voting_type: Some("first-past-the-post".into()),
                counting_algorithm: Some(CountingAlgType::PluralityAtLarge),
                is_encrypted: (true),
                annotations: None,
                candidates: vec![
                    Candidate {
                        id: "0".into(),
                        tenant_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        election_event_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        election_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        contest_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        name: Some("Chloe HUTCHISON".into()),
        name_i18n:None,
        alias:None,alias_i18n:None,
                        description: Some("Independent".into()),
        description_i18n: None,
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
                    },
                    Candidate {
                        id: "1".into(),
                        tenant_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        election_event_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        election_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        contest_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        name: Some("Helen KURGANSKY".into()),
        name_i18n:None,
        alias:None,alias_i18n:None,
                        description: Some("Political Affiliation 1".into()),
        description_i18n: None,
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
                    },
                    Candidate {
                        id: "2".into(),
                        tenant_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        election_event_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        election_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        contest_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        name: Some("Jamie NICHOLLS".into()),
        name_i18n:None,
        alias:None,alias_i18n:None,
                        description: Some("Political Affiliation 2".into()),
        description_i18n: None,
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
                    },
                    Candidate {
                        id: "3".into(),
                        tenant_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        election_event_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        election_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        contest_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        name: None,
        name_i18n:None,
        alias:None,alias_i18n:None,
                        description: None,
        description_i18n: None,
                        candidate_type: None,
                        presentation: Some(CandidatePresentation {
                            i18n: None,
                            is_explicit_invalid: Some(true),
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
                    },
                ],
                presentation: Some(ContestPresentation {
                    i18n: None,
                    allow_writeins: Some(true),
                    base32_writeins: Some(true),
                    invalid_vote_policy: Some(InvalidVotePolicy::WARN),
                    blank_vote_policy: None,
                    over_vote_policy: None,
                    pagination_policy: None,
                    cumulative_number_of_checkboxes: None,
                    shuffle_categories: Some(true),
                    shuffle_category_list: None,
                    show_points: Some(false),
                    enable_checkable_lists: None,
                    candidates_selection_policy: None,
                    candidates_icon_checkbox_policy: None,
                    candidates_order: None,
                    max_selections_per_type: None,
                    types_presentation: None,
                    sort_order: None,
                    under_vote_policy: Some(EUnderVotePolicy::ALLOWED),
                    columns: None,
                }),
            },
            raw_ballot: RawBallotContest {
                bases: vec![2u64, 2u64, 2u64, 2u64],
                choices: vec![1u64, 1u64, 1u64, 1u64],
            },
            plaintext: DecodedVoteContest {
                contest_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".to_string(),
                is_explicit_invalid: true,
                choices: vec![
                    DecodedVoteChoice {
                        id: 0.to_string(),
                        selected: 0,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 1.to_string(),
                        selected: 0,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 2.to_string(),
                        selected: 0,
                        write_in_text: None,
                    }
                ],
                invalid_errors: vec![
                    InvalidPlaintextError {
                        error_type: InvalidPlaintextErrorType::Implicit,
                        candidate_id: None,
                        message: Some("errors.implicit.selectedMax".to_string()),
                        message_map: HashMap::from([
                            ("numSelected".to_string(), 3.to_string()),
                            ("max".to_string(), 1.to_string()),
                        ]),
                    }
                ],
                invalid_alerts: vec![
                    InvalidPlaintextError {
                        error_type: InvalidPlaintextErrorType::Implicit,
                        candidate_id: None,
                        message: Some("errors.implicit.selectedMax".to_string()),
                        message_map: HashMap::from([
                            ("numSelected".to_string(), 3.to_string()),
                            ("max".to_string(), 1.to_string()),
                        ]),
                    }
                ],
            },
            encoded_ballot_bigint: "15".to_string(),
            encoded_ballot: vec_to_30_array(&vec![1, 15]).unwrap(),
            expected_errors: None
        },
        BallotCodecFixture {
            title: "example_3_explicit_invalid".to_string(),
            contest: Contest {
        created_at:None,
                id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into(),
                tenant_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                election_event_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                election_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                name: Some("Poste de maire(sse)".into()),
        name_i18n:None,
        alias:None,alias_i18n:None,
                description: None,
        description_i18n: None,
                max_votes: (1),
                min_votes: (0),
                winning_candidates_num: (1),
                voting_type: Some("first-past-the-post".into()),
                counting_algorithm: Some(CountingAlgType::PluralityAtLarge),
                is_encrypted: (true),
                annotations: None,
                candidates: vec![
                    Candidate {
                        id: "0".into(),
                        tenant_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        election_event_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        election_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        contest_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        name: Some("Chloe HUTCHISON".into()),
        name_i18n:None,
        alias:None,alias_i18n:None,
                        description: Some("Independent".into()),
        description_i18n: None,
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
                    },
                    Candidate {
                        id: "1".into(),
                        tenant_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        election_event_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        election_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        contest_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        name: Some("Helen KURGANSKY".into()),
        name_i18n:None,
        alias:None,alias_i18n:None,
                        description: Some("Political Affiliation 1".into()),
        description_i18n: None,
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
                    },
                    Candidate {
                        id: "2".into(),
                        tenant_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        election_event_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        election_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        contest_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        name: Some("Jamie NICHOLLS".into()),
        name_i18n:None,
        alias:None,alias_i18n:None,
                        description: Some("Political Affiliation 2".into()),
        description_i18n: None,
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
                    },
                    Candidate {
                        id: "3".into(),
                        tenant_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        election_event_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        election_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        contest_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        name: None,
        name_i18n:None,
        alias:None,alias_i18n:None,
                        description: None,
        description_i18n: None,
                        candidate_type: None,
                        presentation: Some(CandidatePresentation {
                            i18n: None,
                            is_explicit_invalid: Some(true),
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
                    },
                ],
                presentation: Some(ContestPresentation {
                    i18n: None,
                    allow_writeins: Some(true),
                    base32_writeins: Some(true),
                    invalid_vote_policy: Some(InvalidVotePolicy::ALLOWED),
                    blank_vote_policy: None,
                    over_vote_policy: None,
                    pagination_policy: None,
                    cumulative_number_of_checkboxes: None,
                    shuffle_categories: Some(true),
                    shuffle_category_list: None,
                    show_points: Some(false),
                    enable_checkable_lists: None,
                    candidates_selection_policy: None,
                    candidates_icon_checkbox_policy: None,
                    candidates_order: None,
                    max_selections_per_type: None,
                    types_presentation: None,
                    sort_order: None,
                    under_vote_policy: Some(EUnderVotePolicy::ALLOWED),
                    columns: None,
                }),
            },
            raw_ballot: RawBallotContest {
                bases: vec![2u64, 2u64, 2u64, 2u64],
                choices: vec![1u64, 1u64, 0u64, 0u64],
            },
            plaintext: DecodedVoteContest {
                contest_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".to_string(),
                is_explicit_invalid: true,
                choices: vec![
                    DecodedVoteChoice {
                        id: 0.to_string(),
                        selected: 0,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 1.to_string(),
                        selected: -1,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 2.to_string(),
                        selected: -1,
                        write_in_text: None,
                    }
                ],
                invalid_errors: vec![],
                invalid_alerts: vec![],
            },
            encoded_ballot_bigint: "3".to_string(),
            encoded_ballot: vec_to_30_array(&vec![1, 3]).unwrap(),
            expected_errors: None
        },
        BallotCodecFixture {
            title: "example_3_implicit_too_many".to_string(),
            contest: Contest {
        created_at:None,
                id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into(),
                tenant_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                election_event_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                election_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                name: Some("Poste de maire(sse)".into()),
        name_i18n:None,
        alias:None,alias_i18n:None,
                description: None,
        description_i18n: None,
                max_votes: (1),
                min_votes: (0),
                winning_candidates_num: (1),
                voting_type: Some("first-past-the-post".into()),
                counting_algorithm: Some(CountingAlgType::PluralityAtLarge),
                is_encrypted: (true),
                annotations: None,
                candidates: vec![
                    Candidate {
                        id: "0".into(),
                        tenant_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        election_event_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        election_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        contest_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        name: Some("Chloe HUTCHISON".into()),
        name_i18n:None,
        alias:None,alias_i18n:None,
                        description: Some("Independent".into()),
        description_i18n: None,
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
                    },
                    Candidate {
                        id: "1".into(),
                        tenant_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        election_event_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        election_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        contest_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        name: Some("Helen KURGANSKY".into()),
        name_i18n:None,
        alias:None,alias_i18n:None,
                        description: Some("Political Affiliation 1".into()),
        description_i18n: None,
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
                    },
                    Candidate {
                        id: "2".into(),
                        tenant_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        election_event_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        election_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        contest_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        name: Some("Jamie NICHOLLS".into()),
        name_i18n:None,
        alias:None,alias_i18n:None,
                        description: Some("Political Affiliation 2".into()),
        description_i18n: None,
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
                    },
                    Candidate {
                        id: "3".into(),
                        tenant_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        election_event_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        election_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        contest_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        name: None,
        name_i18n:None,
        alias:None,alias_i18n:None,
                        description: None,
        description_i18n: None,
                        candidate_type: None,
                        presentation: Some(CandidatePresentation {
                            i18n: None,
                            is_explicit_invalid: Some(true),
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
                    },
                ],
                presentation: Some(ContestPresentation {
                    i18n: None,
                    allow_writeins: Some(true),
                    base32_writeins: Some(true),
                    invalid_vote_policy: Some(InvalidVotePolicy::WARN),
                    blank_vote_policy: None,
                    over_vote_policy: Some(EOverVotePolicy::ALLOWED),
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
            },
            raw_ballot: RawBallotContest {
                bases: vec![2u64, 2u64, 2u64, 2u64],
                choices: vec![0u64, 1u64, 1u64, 1u64],
            },
            plaintext: DecodedVoteContest {
                contest_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".to_string(),
                is_explicit_invalid: false,
                choices: vec![
                    DecodedVoteChoice {
                        id: 0.to_string(),
                        selected: 0,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 1.to_string(),
                        selected: 0,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 2.to_string(),
                        selected: 0,
                        write_in_text: None,
                    }
                ],
                invalid_errors: vec![
                    InvalidPlaintextError {
                        error_type: InvalidPlaintextErrorType::Implicit,
                        candidate_id: None,
                        message: Some("errors.implicit.selectedMax".to_string()),
                        message_map: HashMap::from([
                            ("numSelected".to_string(), 3.to_string()),
                            ("max".to_string(), 1.to_string()),
                        ]),
                    }
                ],
                invalid_alerts: vec![],
            },
            encoded_ballot_bigint: "14".to_string(),
            encoded_ballot: vec_to_30_array(&vec![1, 14]).unwrap(),
            expected_errors: None
        },
        BallotCodecFixture {
            title: "example_4_implicit_empty".to_string(),
            contest: Contest {
        created_at:None,
                id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into(),
                tenant_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                election_event_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                election_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                name: Some("Test contest title".into()),
        name_i18n:None,
        alias:None,alias_i18n:None,
                description: None,
        description_i18n: None,
                max_votes: (1),
                min_votes: (1),
                winning_candidates_num: (1),
                voting_type: Some("first-past-the-post".into()),
                counting_algorithm: Some(CountingAlgType::PluralityAtLarge),
                is_encrypted: (true),
                annotations: None,
                candidates: vec![
                    Candidate {
                        id: "0".into(),
                        tenant_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        election_event_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        election_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        contest_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        name: Some("Example option 1".into()),
        name_i18n:None,
        alias:None,alias_i18n:None,
                        description: Some("This is an option with an simple example description.".into()),
        description_i18n: None,
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
                    },
                    Candidate {
                        id: "1".into(),
                        tenant_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        election_event_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        election_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        contest_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        name: Some("Example option 2".into()),
        name_i18n:None,
        alias:None,alias_i18n:None,
                        description: Some("An option can contain a description. You can add simple html like <strong>bold</strong> or <a href=\"https://sequentech.io\" rel=\"nofollow\">links to websites</a>. You can also set an image url below, but be sure it&#39;s HTTPS or else it won&#39;t load.\n\n<br /><br />You need to use two br element for new paragraphs.".into()),
        description_i18n: None,
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
                    },
                    Candidate {
                        id: "2".into(),
                        tenant_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        election_event_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        election_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        contest_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        name: Some("Example option 3".into()),
        name_i18n:None,
        alias:None,alias_i18n:None,
                        description: None,
        description_i18n: None,
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
                    },
                ],
                presentation: Some(ContestPresentation {
                    i18n: None,
                    allow_writeins: Some(true),
                    base32_writeins: Some(true),
                    invalid_vote_policy: Some(InvalidVotePolicy::ALLOWED),
                    blank_vote_policy: Some(EBlankVotePolicy::ALLOWED),
                    over_vote_policy: None,
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
                    under_vote_policy: Some(EUnderVotePolicy::WARN),
                    columns: None,
                }),
            },
            raw_ballot: RawBallotContest {
                bases: vec![2u64, 2u64, 2u64, 2u64],
                choices: vec![0u64, 0u64, 0u64, 0u64],
            },
            plaintext: DecodedVoteContest {
                contest_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".to_string(),
                is_explicit_invalid: false,
                choices: vec![
                    DecodedVoteChoice {
                        id: 0.to_string(),
                        selected: -1,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 1.to_string(),
                        selected: -1,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 2.to_string(),
                        selected: -1,
                        write_in_text: None,
                    }
                ],
                invalid_errors: vec![
                    InvalidPlaintextError {
                        error_type: InvalidPlaintextErrorType::Implicit,
                        candidate_id: None,
                        message: Some("errors.implicit.selectedMin".to_string()),
                        message_map: HashMap::from([
                            ("numSelected".to_string(), 0.to_string()),
                            ("min".to_string(), 1.to_string()),
                        ]),
                    },
                ],
                invalid_alerts: vec![],
            },
            encoded_ballot_bigint: "0".to_string(),
            encoded_ballot: vec_to_30_array(&vec![1, 0]).unwrap(),
            expected_errors: None
        },
        BallotCodecFixture {
            title: "example_4_implicit_empty_warn".to_string(),
            contest: Contest {
        created_at:None,
                id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into(),
                tenant_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                election_event_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                election_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                name: Some("Test contest title".into()),
        name_i18n:None,
        alias:None,alias_i18n:None,
                description: None,
        description_i18n: None,
                max_votes: (1),
                min_votes: (1),
                winning_candidates_num: (1),
                voting_type: Some("first-past-the-post".into()),
                counting_algorithm: Some(CountingAlgType::PluralityAtLarge),
                is_encrypted: (true),
                annotations: None,
                candidates: vec![
                    Candidate {
                        id: "0".into(),
                        tenant_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        election_event_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        election_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        contest_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        name: Some("Example option 1".into()),
        name_i18n:None,
        alias:None,alias_i18n:None,
                        description: Some("This is an option with an simple example description.".into()),
        description_i18n: None,
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
                    },
                    Candidate {
                        id: "1".into(),
                        tenant_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        election_event_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        election_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        contest_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        name: Some("Example option 2".into()),
        name_i18n:None,
        alias:None,alias_i18n:None,
                        description: Some("An option can contain a description. You can add simple html like <strong>bold</strong> or <a href=\"https://sequentech.io\" rel=\"nofollow\">links to websites</a>. You can also set an image url below, but be sure it&#39;s HTTPS or else it won&#39;t load.\n\n<br /><br />You need to use two br element for new paragraphs.".into()),
        description_i18n: None,
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
                    },
                    Candidate {
                        id: "2".into(),
                        tenant_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        election_event_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        election_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        contest_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        name: Some("Example option 3".into()),
        name_i18n:None,
        alias:None,alias_i18n:None,
                        description: None,
        description_i18n: None,
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
                    },
                ],
                presentation: Some(ContestPresentation {
                    i18n: None,
                    allow_writeins: Some(true),
                    base32_writeins: Some(true),
                    invalid_vote_policy: Some(InvalidVotePolicy::ALLOWED),
                    blank_vote_policy: Some(EBlankVotePolicy::WARN),
                    over_vote_policy: None,
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
                    under_vote_policy: Some(EUnderVotePolicy::WARN),
                    columns: None,
                }),
            },
            raw_ballot: RawBallotContest {
                bases: vec![2u64, 2u64, 2u64, 2u64],
                choices: vec![0u64, 0u64, 0u64, 0u64],
            },
            plaintext: DecodedVoteContest {
                contest_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".to_string(),
                is_explicit_invalid: false,
                choices: vec![
                    DecodedVoteChoice {
                        id: 0.to_string(),
                        selected: -1,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 1.to_string(),
                        selected: -1,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 2.to_string(),
                        selected: -1,
                        write_in_text: None,
                    }
                ],
                invalid_errors: vec![
                    InvalidPlaintextError {
                        error_type: InvalidPlaintextErrorType::Implicit,
                        candidate_id: None,
                        message: Some("errors.implicit.selectedMin".to_string()),
                        message_map: HashMap::from([
                            ("numSelected".to_string(), 0.to_string()),
                            ("min".to_string(), 1.to_string()),
                        ]),
                    },
                ],
                invalid_alerts: vec![
                    InvalidPlaintextError {
                        error_type: InvalidPlaintextErrorType::Implicit,
                        candidate_id: None,
                        message: Some("errors.implicit.blankVote".to_string()),
                        message_map: HashMap::from([
                            ("numSelected".to_string(), 0.to_string()),
                            ("type".to_string(), "alert".to_string()),
                        ]),
                    },
                ],
            },
            encoded_ballot_bigint: "0".to_string(),
            encoded_ballot: vec_to_30_array(&vec![1, 0]).unwrap(),
            expected_errors: None
        },
        BallotCodecFixture {
            title: "example_4_implicit_empty_blank_vote".to_string(),
            contest: Contest {
        created_at:None,
                id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into(),
                tenant_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                election_event_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                election_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                name: Some("Test contest title".into()),
        name_i18n:None,
        alias:None,alias_i18n:None,
                description: None,
        description_i18n: None,
                max_votes: (1),
                min_votes: (1),
                winning_candidates_num: (1),
                voting_type: Some("first-past-the-post".into()),
                counting_algorithm: Some(CountingAlgType::PluralityAtLarge),
                is_encrypted: (true),
                annotations: None,
                candidates: vec![
                    Candidate {
                        id: "0".into(),
                        tenant_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        election_event_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        election_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        contest_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        name: Some("Example option 1".into()),
        name_i18n:None,
        alias:None,alias_i18n:None,
                        description: Some("This is an option with an simple example description.".into()),
        description_i18n: None,
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
                    },
                    Candidate {
                        id: "1".into(),
                        tenant_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        election_event_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        election_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        contest_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        name: Some("Example option 2".into()),
        name_i18n:None,
        alias:None,alias_i18n:None,
                        description: Some("An option can contain a description. You can add simple html like <strong>bold</strong> or <a href=\"https://sequentech.io\" rel=\"nofollow\">links to websites</a>. You can also set an image url below, but be sure it&#39;s HTTPS or else it won&#39;t load.\n\n<br /><br />You need to use two br element for new paragraphs.".into()),
        description_i18n: None,
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
                    },
                    Candidate {
                        id: "2".into(),
                        tenant_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        election_event_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        election_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        contest_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        name: Some("Example option 3".into()),
        name_i18n:None,
        alias:None,alias_i18n:None,
                        description: None,
        description_i18n: None,
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
                    },
                ],
                presentation: Some(ContestPresentation {
                    i18n: None,
                    allow_writeins: Some(true),
                    base32_writeins: Some(true),
                    invalid_vote_policy: Some(InvalidVotePolicy::ALLOWED),
                    blank_vote_policy: Some(EBlankVotePolicy::NOT_ALLOWED),
                    over_vote_policy: None,
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
            },
            raw_ballot: RawBallotContest {
                bases: vec![2u64, 2u64, 2u64, 2u64],
                choices: vec![0u64, 0u64, 0u64, 0u64],
            },
            plaintext: DecodedVoteContest {
                contest_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".to_string(),
                is_explicit_invalid: false,
                choices: vec![
                    DecodedVoteChoice {
                        id: 0.to_string(),
                        selected: -1,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 1.to_string(),
                        selected: -1,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 2.to_string(),
                        selected: -1,
                        write_in_text: None,
                    }
                ],
                invalid_errors: vec![
                    InvalidPlaintextError {
                        error_type: InvalidPlaintextErrorType::Implicit,
                        candidate_id: None,
                        message: Some("errors.implicit.selectedMin".to_string()),
                        message_map: HashMap::from([
                            ("min".to_string(), 1.to_string()),
                            ("numSelected".to_string(), 0.to_string()),
                        ]),
                    },
                    InvalidPlaintextError {
                        error_type: InvalidPlaintextErrorType::Implicit,
                        candidate_id: None,
                        message: Some("errors.implicit.blankVote".to_string()),
                        message_map: HashMap::from([
                            ("numSelected".to_string(), 0.to_string()),
                            ("type".to_string(), "alert".to_string()),
                        ]),
                    },
                ],
                invalid_alerts: vec![],
            },
            encoded_ballot_bigint: "0".to_string(),
            encoded_ballot: vec_to_30_array(&vec![1, 0]).unwrap(),
            expected_errors: None
        },
        BallotCodecFixture {
            title: "example_4_implicit_invented_candidate".to_string(),
            contest: Contest {
        created_at:None,
                id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into(),
                tenant_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                election_event_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                election_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                name: Some("Test contest title".into()),
        name_i18n:None,
        alias:None,alias_i18n:None,
                description: None,
        description_i18n: None,
                max_votes: (1),
                min_votes: (1),
                winning_candidates_num: (1),
                voting_type: Some("first-past-the-post".into()),
                counting_algorithm: Some(CountingAlgType::PluralityAtLarge),
                is_encrypted: (true),
                annotations: None,
                candidates: vec![
                    Candidate {
                        id: "0".into(),
                        tenant_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        election_event_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        election_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        contest_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        name: Some("Example option 1".into()),
        name_i18n:None,
        alias:None,alias_i18n:None,
                        description: Some("This is an option with an simple example description.".into()),
        description_i18n: None,
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
                    },
                    Candidate {
                        id: "1".into(),
                        tenant_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        election_event_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        election_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        contest_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        name: Some("Example option 2".into()),
        name_i18n:None,
        alias:None,alias_i18n:None,
                        description: Some("An option can contain a description. You can add simple html like <strong>bold</strong> or <a href=\"https://sequentech.io\" rel=\"nofollow\">links to websites</a>. You can also set an image url below, but be sure it&#39;s HTTPS or else it won&#39;t load.\n\n<br /><br />You need to use two br element for new paragraphs.".into()),
        description_i18n: None,
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
                    },
                    Candidate {
                        id: "2".into(),
                        tenant_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        election_event_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        election_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        contest_id: ("1fc963b1-f93b-4151-93d6-bbe0ea5eac46".into()),
                        name: Some("Example option 3".into()),
        name_i18n:None,
        alias:None,alias_i18n:None,
                        description: None,
        description_i18n: None,
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
                    },
                ],
                presentation: Some(ContestPresentation {
                    i18n: None,
                    allow_writeins: Some(true),
                    base32_writeins: Some(true),
                    invalid_vote_policy: Some(InvalidVotePolicy::ALLOWED),
                    blank_vote_policy: None,
                    over_vote_policy: None,
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
            },
            raw_ballot: RawBallotContest {
                bases: vec![2u64, 2u64, 2u64, 2u64, 2u64],
                choices: vec![0u64, 0u64, 0u64, 0u64, 1u64],
            },
            plaintext: DecodedVoteContest {
                contest_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".to_string(),
                is_explicit_invalid: false,
                choices: vec![
                    DecodedVoteChoice {
                        id: 0.to_string(),
                        selected: -1,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 1.to_string(),
                        selected: -1,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 2.to_string(),
                        selected: -1,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 3.to_string(),
                        selected: 0,
                        write_in_text: None,
                    }
                ],
                invalid_errors: vec![
                    InvalidPlaintextError {
                        error_type: InvalidPlaintextErrorType::EncodingError,
                        candidate_id: None,
                        message: Some("errors.encoding.ballotTooLarge".to_string()),
                        message_map: HashMap::new(),
                    },
                    InvalidPlaintextError {
                        error_type: InvalidPlaintextErrorType::Implicit,
                        candidate_id: None,
                        message: Some("errors.implicit.selectedMin".to_string()),
                        message_map: HashMap::from([
                            ("min".to_string(), 1.to_string()),
                            ("numSelected".to_string(), 0.to_string()),
                        ]),
                    }
                ],
                invalid_alerts: vec![],
            },
            encoded_ballot_bigint: "16".to_string(),
            encoded_ballot: vec_to_30_array(&vec![1, 16]).unwrap(),
            expected_errors: Some(HashMap::from([
                ("contest_bases".to_string(), "".to_string()),
                ("contest_encode_plaintext".to_string(), "choice id is not a valid candidate".to_string()),
                ("contest_encode_to_raw_ballot".to_string(), "choice id is not a valid candidate".to_string()),
                ("contest_decode_plaintext".to_string(), "decode_choices".to_string()),
                ("encoding_plaintext_bigint".to_string(), "choice id is not a valid candidate".to_string()),
            ]))
        },
        BallotCodecFixture {
            title: "plurality with two selections".to_string(),
            contest: get_configurable_contest(3, 7, CountingAlgType::PluralityAtLarge, false, None, true),
            raw_ballot: RawBallotContest {
                bases: vec![2, 2, 2, 2, 2, 2, 2, 2],
                choices: vec![0, 0, 1, 0, 0, 0, 1, 0],
            },
            plaintext: DecodedVoteContest {
                contest_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".to_string(),
                is_explicit_invalid: false,
                invalid_errors: vec![],
                invalid_alerts: vec![],
                choices: vec![
                    DecodedVoteChoice {
                        id: 0.to_string(),
                        selected: -1,
                        write_in_text: None
                    },
                    DecodedVoteChoice {
                        id: 1.to_string(),
                        selected: 0,
                        write_in_text: None
                    },
                    DecodedVoteChoice {
                        id: 2.to_string(),
                        selected: -1,
                        write_in_text: None
                    },
                    DecodedVoteChoice {
                        id: 3.to_string(),
                        selected: -1,
                        write_in_text: None
                    },
                    DecodedVoteChoice {
                        id: 4.to_string(),
                        selected: -1,
                        write_in_text: None
                    },
                    DecodedVoteChoice {
                        id: 5.to_string(),
                        selected: 1,
                        write_in_text: None
                    },
                    DecodedVoteChoice {
                        id: 6.to_string(),
                        selected: -1,
                        write_in_text: None
                    }
                ]
            },
            encoded_ballot_bigint: "68".to_string(),
            encoded_ballot: vec_to_30_array(&vec![1, 68]).unwrap(),
            expected_errors: Some(HashMap::from([
                ("contest_decode_plaintext".to_string(), "decode_choices".to_string()),
            ]))
        },
        BallotCodecFixture {
            title: "plurality with three selections".to_string(),
            contest: get_configurable_contest(3, 7, CountingAlgType::PluralityAtLarge, false, None, true),
            raw_ballot: RawBallotContest {
                bases: vec![2, 2, 2, 2, 2, 2, 2, 2],
                choices: vec![0, 1, 1, 0, 0, 0, 1, 0],
            },
            plaintext: DecodedVoteContest {
                contest_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".to_string(),
                is_explicit_invalid: false,
                invalid_errors: vec![],
                invalid_alerts: vec![],
                choices: vec![
                    DecodedVoteChoice {
                        id: 0.to_string(),
                        selected: 0,
                        write_in_text: None
                    },
                    DecodedVoteChoice {
                        id: 1.to_string(),
                        selected: 0,
                        write_in_text: None
                    },
                    DecodedVoteChoice {
                        id: 2.to_string(),
                        selected: -1,
                        write_in_text: None
                    },
                    DecodedVoteChoice {
                        id: 3.to_string(),
                        selected: -1,
                        write_in_text: None
                    },
                    DecodedVoteChoice {
                        id: 4.to_string(),
                        selected: -1,
                        write_in_text: None
                    },
                    DecodedVoteChoice {
                        id: 5.to_string(),
                        selected: 1,
                        write_in_text: None
                    },
                    DecodedVoteChoice {
                        id: 6.to_string(),
                        selected: -1,
                        write_in_text: None
                    }
                ]
            },
            encoded_ballot_bigint: "70".to_string(),
            encoded_ballot: vec_to_30_array(&vec![1, 70]).unwrap(),
            expected_errors: Some(HashMap::from([
                ("contest_decode_plaintext".to_string(), "decode_choices".to_string()),
            ]))
        },
        BallotCodecFixture {
            title: "borda with three selections".to_string(),
            contest: get_configurable_contest(3, 7, CountingAlgType::Borda, false, None, true),
            raw_ballot: RawBallotContest {
                bases: vec![2, 4, 4, 4, 4, 4, 4, 4],
                choices: vec![0, 1, 3, 0, 0, 0, 2, 0]
            },
            plaintext: DecodedVoteContest {
                contest_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".to_string(),
                is_explicit_invalid: false,
                invalid_errors: vec![],
                invalid_alerts: vec![],
                choices: vec![
                    DecodedVoteChoice {
                        id: 0.to_string(),
                        selected: 0,
                        write_in_text: None
                    },
                    DecodedVoteChoice {
                        id: 1.to_string(),
                        selected: 2,
                        write_in_text: None
                    },
                    DecodedVoteChoice {
                        id: 2.to_string(),
                        selected: -1,
                        write_in_text: None
                    },
                    DecodedVoteChoice {
                        id: 3.to_string(),
                        selected: -1,
                        write_in_text: None
                    },
                    DecodedVoteChoice {
                        id: 4.to_string(),
                        selected: -1,
                        write_in_text: None
                    },
                    DecodedVoteChoice {
                        id: 5.to_string(),
                        selected: 1,
                        write_in_text: None
                    },
                    DecodedVoteChoice {
                        id: 6.to_string(),
                        selected: -1,
                        write_in_text: None
                    }
                ]
            },
            encoded_ballot_bigint: "4122".to_string(),
            encoded_ballot: vec_to_30_array(&vec![2, 26, 16]).unwrap(),
            expected_errors: None
        },
        BallotCodecFixture {
            title: "plurality explicit invalid and one selection".to_string(),
            contest: get_configurable_contest(2, 2, CountingAlgType::PluralityAtLarge, false, None, true),
            raw_ballot: RawBallotContest {
                bases: vec![2, 2, 2],
                choices: vec![1, 1, 0]
            },
            plaintext: DecodedVoteContest {
                contest_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".to_string(),
                is_explicit_invalid: true,
                invalid_errors: vec![
                    InvalidPlaintextError {
                        error_type: InvalidPlaintextErrorType::Explicit,
                        candidate_id: None,
                        message: Some("errors.explicit.notAllowed".to_string()),
                        message_map: HashMap::new(),
                    }
                ],
                invalid_alerts: vec![],
                choices: vec![
                    DecodedVoteChoice {
                        id: 0.to_string(),
                        selected: 0,
                        write_in_text: None
                    },
                    DecodedVoteChoice {
                        id: 1.to_string(),
                        selected: -1,
                        write_in_text: None
                    },
                ]
            },
            encoded_ballot_bigint: "3".to_string(),
            encoded_ballot: vec_to_30_array(&vec![1, 3]).unwrap(),
            expected_errors: None
        },
        BallotCodecFixture {
            title: "two write ins, an explicit invalid ballot, one of the write-ins is not selected".to_string(),
            contest: get_configurable_contest(2, 6, CountingAlgType::Borda, true, None, true),
            raw_ballot: RawBallotContest {
                bases: vec!  [2, 3, 3, 3, 3, 3, 3, 32, 32, 32],
                choices: vec![1, 1, 0, 0, 1, 2, 0, 4, 0, 0]
            },
            plaintext: DecodedVoteContest {
                contest_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".to_string(),
                is_explicit_invalid: true,
                invalid_errors: vec![
                    InvalidPlaintextError {
                        error_type: InvalidPlaintextErrorType::Explicit,
                        candidate_id: None,
                        message: Some("errors.explicit.notAllowed".to_string()),
                        message_map: HashMap::new(),
                    },
                    InvalidPlaintextError {
                        error_type: InvalidPlaintextErrorType::Implicit,
                        candidate_id: None,
                        message: Some("errors.implicit.selectedMax".to_string()),
                        message_map: HashMap::from([
                            ("numSelected".to_string(), "3".to_string()),
                            ("max".to_string(), "2".to_string())
                        ]),
                    }
                ],
                invalid_alerts: vec![],
                choices: vec![
                    DecodedVoteChoice {
                        id: 0.to_string(),
                        selected: 0,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 1.to_string(),
                        selected: -1,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 2.to_string(),
                        selected: -1,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 3.to_string(),
                        selected: 0,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 4.to_string(),
                        selected: 1,
                        write_in_text: Some("D".to_string())
                    },
                    DecodedVoteChoice {
                        id: 5.to_string(),
                        selected: -1,
                        write_in_text: Some("".to_string())
                    }
                ]
            },
            encoded_ballot_bigint: "6213".to_string(),
            encoded_ballot: vec_to_30_array(&vec![2, 69, 24]).unwrap(),
            expected_errors: Some(HashMap::from([
                ("contest_bases".to_string(), "bases don't cover write-ins".to_string()),
            ]))
        },
        BallotCodecFixture {
            title: "three write ins, a valid ballot, one of the write-ins is not selected".to_string(),
            contest: get_configurable_contest(3, 7, CountingAlgType::PluralityAtLarge, true, None, true),
            raw_ballot: RawBallotContest {
                bases: vec![2, 2, 2, 2, 2, 2, 2, 2, 32, 32, 32, 32, 32, 32, 32, 32],
                choices: vec![0, 1, 0, 0, 0, 1, 0, 1, 5, 0, 0, 1, 27, 2, 3, 0]
            },
            plaintext: DecodedVoteContest {
                contest_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".to_string(),
                is_explicit_invalid: false,
                invalid_errors: vec![],
                invalid_alerts: vec![],
                choices: vec![
                    DecodedVoteChoice {
                        id: 0.to_string(),
                        selected: 1,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 1.to_string(),
                        selected: -1,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 2.to_string(),
                        selected: -1,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 3.to_string(),
                        selected: -1,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 4.to_string(),
                        selected: 1,
                        write_in_text: Some("E".to_string()),
                    },
                    DecodedVoteChoice {
                        id: 5.to_string(),
                        selected: -1,
                        write_in_text: Some("".to_string()),
                    },
                    DecodedVoteChoice {
                        id: 6.to_string(),
                        selected: 1,
                        write_in_text: Some("A BC".to_string()),
                    }
                ]
            },
            encoded_ballot_bigint: "849069737378".to_string(),
            encoded_ballot: vec_to_30_array(&vec![5, 162, 5, 128, 176, 197]).unwrap(),
            expected_errors: Some(HashMap::from([
                ("contest_bases".to_string(), "bases don't cover write-ins".to_string()),
            ]))
        },
        BallotCodecFixture {
            title: "Not enough choices to decode".to_string(),
            contest: get_configurable_contest(2, 3, CountingAlgType::PluralityAtLarge, true, None, true),
            raw_ballot: RawBallotContest {
                bases: vec![2, 2, 2, 2],
                choices: vec![0, 1, 0],
            },
            plaintext: DecodedVoteContest {
                contest_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".to_string(),
                is_explicit_invalid: false,
                invalid_errors: vec![
                    InvalidPlaintextError {
                        error_type: InvalidPlaintextErrorType::EncodingError,
                        candidate_id: None,
                        message: Some("errors.encoding.notEnoughChoices".to_string()),
                        message_map: HashMap::new(),
                    }
                ],
                invalid_alerts: vec![],
                choices: vec![
                    DecodedVoteChoice {
                        id: 0.to_string(),
                        selected: 0,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 1.to_string(),
                        selected: -1,
                        write_in_text: None,
                    }
                ]
            },
            encoded_ballot_bigint: "2".to_string(),
            encoded_ballot: vec_to_30_array(&vec![1, 2]).unwrap(),
            expected_errors: Some(HashMap::from([
                ("contest_encode_raw_ballot".to_string(), "Invalid parameters: 'valueList' (size = 3) and 'baseList' (size = 4) must have the same length.".to_string()),
                ("contest_encode_plaintext".to_string(), "Invalid parameters: 'valueList' (size = 3) and 'baseList' (size = 4) must have the same length.".to_string()),
                ("contest_decode_plaintext".to_string(), "invalid_errors,decode_choices".to_string()),
                ("encoding_plaintext_bigint".to_string(), "Invalid parameters: 'valueList' (size = 3) and 'baseList' (size = 4) must have the same length.".to_string()),
            ]))
        },
        BallotCodecFixture {
            title: "simple vote".to_string(),
            contest: get_configurable_contest(2, 3, CountingAlgType::PluralityAtLarge, true, Some(vec![0]), true),
            raw_ballot: RawBallotContest {
                bases:   vec![2, 2, 2, 2],
                choices: vec![0, 1, 0, 0],
            },
            plaintext: DecodedVoteContest {
                contest_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".to_string(),
                is_explicit_invalid: false,
                invalid_errors: vec![],
                invalid_alerts: vec![],
                choices: vec![
                    DecodedVoteChoice {
                        id: 0.to_string(),
                        selected: 0,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 1.to_string(),
                        selected: -1,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 2.to_string(),
                        selected: -1,
                        write_in_text: None,
                    }
                ]
            },
            encoded_ballot_bigint: "2".to_string(),
            encoded_ballot: vec_to_30_array(&vec![1, 2]).unwrap(),
            expected_errors: Some(HashMap::from([
                ("contest_bases".to_string(),  "bases don't cover write-ins".to_string()),
                ("contest_encode_to_raw_ballot".to_string(),  "disabled".to_string()),
                ("contest_encode_plaintext".to_string(),  "disabled".to_string()),
            ]))
        },
        BallotCodecFixture {
            title: "Write in doesn't end on 0".to_string(),
            contest: get_configurable_contest(2, 3, CountingAlgType::PluralityAtLarge, true, Some(vec![0]), true),
            raw_ballot: RawBallotContest {
                bases:   vec![2, 2, 2, 2, 32],
                choices: vec![0, 1, 0, 0, 1],
            },
            plaintext: DecodedVoteContest {
                contest_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".to_string(),
                is_explicit_invalid: false,
                invalid_errors: vec![
                    InvalidPlaintextError {
                        error_type: InvalidPlaintextErrorType::EncodingError,
                        candidate_id: Some(0.to_string()),
                        message: Some("errors.encoding.writeInNotEndInZero".to_string()),
                        message_map: HashMap::new(),
                    }
                ],
                invalid_alerts: vec![],
                choices: vec![
                    DecodedVoteChoice {
                        id: 0.to_string(),
                        selected: 0,
                        write_in_text: Some("A".to_string()),
                    },
                    DecodedVoteChoice {
                        id: 1.to_string(),
                        selected: -1,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 2.to_string(),
                        selected: -1,
                        write_in_text: None,
                    }
                ]
            },
            encoded_ballot_bigint: "18".to_string(),
            encoded_ballot: vec_to_30_array(&vec![1, 18]).unwrap(),
            expected_errors: Some(HashMap::from([
                ("contest_encode_to_raw_ballot".to_string(),  "disabled".to_string()),
                ("contest_decode_plaintext".to_string(),  "invalid_errors, decode_choices".to_string()),
            ]))
        },
        BallotCodecFixture {
            title: "Ballot larger than expected".to_string(),
            contest: get_configurable_contest(2, 3, CountingAlgType::PluralityAtLarge, true, Some(vec![]), true),
            raw_ballot: RawBallotContest {
                bases: vec![2, 2, 2, 2, 32],
                choices: vec![0, 1, 0, 0, 24],
            },
            plaintext: DecodedVoteContest {
                contest_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".to_string(),
                is_explicit_invalid: false,
                invalid_errors: vec![
                    InvalidPlaintextError {
                        error_type: InvalidPlaintextErrorType::EncodingError,
                        candidate_id: None,
                        message: Some("errors.encoding.ballotTooLarge".to_string()),
                        message_map: HashMap::new(),
                    }
                ],
                invalid_alerts: vec![],
                choices: vec![
                    DecodedVoteChoice {
                        id: 0.to_string(),
                        selected: 0,
                        write_in_text: None
                    },
                    DecodedVoteChoice {
                        id: 1.to_string(),
                        selected: -1,
                        write_in_text: None
                    },
                    DecodedVoteChoice {
                        id: 2.to_string(),
                        selected: -1,
                        write_in_text: None
                    }
                ]
            },
            encoded_ballot_bigint: "386".to_string(),
            encoded_ballot: vec_to_30_array(&vec![2, 130, 1]).unwrap(),
            expected_errors: Some(HashMap::from([
                ("contest_bases".to_string(),  "bases don't cover write-ins".to_string()),
                ("contest_encode_to_raw_ballot".to_string(),  "disabled".to_string()),
                ("contest_encode_plaintext".to_string(),  "disabled".to_string()),
                ("encoding_plaintext_bigint".to_string(),  "disabled".to_string()),
            ]))
        },
    ]
}

pub fn bases_fixture() -> Vec<BasesFixture> {
    vec![
        BasesFixture {
            contest: get_configurable_contest(
                3,
                7,
                CountingAlgType::PluralityAtLarge,
                false,
                None,
                true,
            ),
            bases: vec![2, 2, 2, 2, 2, 2, 2, 2],
        },
        BasesFixture {
            contest: get_configurable_contest(
                1,
                1,
                CountingAlgType::PluralityAtLarge,
                false,
                None,
                true,
            ),
            bases: vec![2, 2],
        },
        BasesFixture {
            contest: get_configurable_contest(
                1,
                1,
                CountingAlgType::Borda,
                false,
                None,
                true,
            ),
            bases: vec![2, 2],
        },
        BasesFixture {
            contest: get_configurable_contest(
                2,
                3,
                CountingAlgType::Borda,
                false,
                None,
                true,
            ),
            bases: vec![2, 3, 3, 3],
        },
    ]
}
