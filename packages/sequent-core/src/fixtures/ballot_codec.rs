// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::ballot::BallotStyle;
use crate::ballot::*;
use crate::ballot_codec::{vec_to_30_array, RawBallotContest};
use crate::plaintext::{
    DecodedVoteChoice, DecodedVoteContest, InvalidPlaintextError,
    InvalidPlaintextErrorType,
};
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
    let contest_str = r#"{
        "id":"1fc963b1-f93b-4151-93d6-bbe0ea5eac46",
        "description":"Elige quien quieres que sea tu Secretario General en tu municipio",
         "layout":"",
         "min":0,
         "max":1,
         "num_winners":1,
         "title":"Secretario General",
         "tally_type":"plurality-at-large",
         "answer_total_votes_percentage":"over-total-valid-votes",
         "answers":[
            {
                "id":"0",
               "category":"Candidaturas no agrupadas",
               "details":"",
               "sort_order":0,
               "urls":[
                  
               ],
               "text":"José Rabano Pimiento"
            },
            {
                "id":"1",
               "category":"Candidaturas no agrupadas",
               "details":"",
               "sort_order":1,
               "urls":[
                  
               ],
               "text":"Miguel Pimentel Inventado"
            },
            {
               "category":"Candidaturas no agrupadas",
               "text":"Juan Iglesias Torquemada",
               "sort_order":2,
               "details":"",
               "urls":[
                  
               ],
               "id":"2"
            },
            {
               "category":"Candidaturas no agrupadas",
               "text":"Mari Pili Hernández Ordoñez",
               "sort_order":3,
               "details":"",
               "urls":[
                  
               ],
               "id":"3"
            },
            {
               "category":"Candidaturas no agrupadas",
               "text":"Juan Y Medio",
               "sort_order":4,
               "details":"",
               "urls":[
                  
               ],
               "id":"4"
            }
         ],
         "extra_options":{
            "base32_writeins":true
         }
      }"#;
    let contest: Contest = serde_json::from_str(contest_str).unwrap();
    contest
}

fn get_contest_borda() -> Contest {
    let mut contest = get_contest_plurality();
    contest.tally_type = String::from("borda");
    contest.max = 4;
    contest
}

pub fn get_test_decoded_vote_contest() -> DecodedVoteContest {
    DecodedVoteContest {
        contest_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".to_string(),
        is_explicit_invalid: false,
        invalid_errors: vec![],
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
    let contest_str = r#"{
        "id": "9570d82a-d92a-44d7-b483-d5a6c8c398a8",
        "configuration": {
            "id": "9570d82a-d92a-44d7-b483-d5a6c8c398a8",
            "layout": "simple",
            "director": "6xx-a1",
            "authorities": ["6xx-a2"],
            "title": "Write-ins simple",
            "description": "",
            "contests": [
                {
                    "id": "1c1500ac-173e-4e78-a59d-91bfa3678c5a",
                    "description": "",
                    "layout": "simultaneous-contests",
                    "max": 2,
                    "min": 1,
                    "num_winners": 1,
                    "title": "Test contest title",
                    "tally_type": "plurality-at-large",
                    "answer_total_votes_percentage": "over-total-valid-votes",
                    "answers": [
                        {
                            "id": "f257cd3a-d1cf-4b97-91f8-2dfe156b015c",
                            "category": "",
                            "details": "This is an option with an simple example description.",
                            "sort_order": 0,
                            "urls": [],
                            "text": "Example option 1"
                        },
                        {
                            "id": "17325099-f5ab-4c48-a142-6d7ed721e9bb",
                            "category": "",
                            "details": "An option can contain a description.",
                            "sort_order": 1,
                            "urls": [
                                {
                                    "title": "URL",
                                    "url": "https://sequentech.io"
                                },
                                {
                                    "title": "Image URL",
                                    "url": "/XFQwVFL.jpg"
                                }
                            ],
                            "text": "Example option 2"
                        },
                        {
                            "id": "61320aac-0d78-4001-845e-a2f2bd8e800b",
                            "category": "",
                            "details": "",
                            "sort_order": 2,
                            "urls": [
                                {
                                    "title": "isWriteIn",
                                    "url": "true"
                                }
                            ],
                            "text": ""
                        },
                        {
                            "id": "e9ad3ed1-4fd5-4498-a0e7-3a3c22ef57d5",
                            "category": "",
                            "details": "",
                            "sort_order": 3,
                            "urls": [
                                {
                                    "title": "isWriteIn",
                                    "url": "true"
                                }
                            ],
                            "text": ""
                        }
                    ],
                    "extra_options": {
                        "shuffle_categories": true,
                        "shuffle_all_options": true,
                        "shuffle_category_list": [],
                        "show_points": false,
                        "allow_writeins": true,
                        "base32_writeins": true
                    }
                }
            ],
            "presentation": {
                "share_text": [
                    {
                        "network": "Twitter",
                        "button_text": "",
                        "social_message": "I have just voted in election __URL__, you can too! #sequent"
                    }
                ],
                "theme": "default",
                "urls": [],
                "theme_css": ""
            },
            "extra_data": "{}",
            "tallyPipesConfig": "",
            "ballotBoxesResultsConfig": "",
            "virtual": false,
            "tally_allowed": false,
            "publicCandidates": true,
            "virtualSubelections": [],
            "logo_url": ""
        },
        "state": "created",
        "public_key": {
            "public_key": "ajR/I9RqyOwbpsVRucSNOgXVLCvLpfQxCgPoXGQ2RF4",
            "is_demo": false
        },
        "tallyPipesConfig": "",
        "ballotBoxesResultsConfig": "",
        "virtual": false,
        "tallyAllowed": false,
        "publicCandidates": true,
        "logo_url": "",
        "trusteeKeysState": [
            {
                "id": "6xx-a1",
                "state": "initial"
            },
            {
                "id": "6xx-a2",
                "state": "initial"
            }
        ]
    }"#;
    let contest: BallotStyle = serde_json::from_str(contest_str).unwrap();
    contest
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
    }
}

pub fn get_test_contest() -> Contest {
    let contest_str = r#"{
        "id":"1fc963b1-f93b-4151-93d6-bbe0ea5eac46",
        "description":"This is the description of this contest. You can have multiple contests. You can add simple html like.",
        "layout":"simultaneous-contests",
        "max":3,
        "min":1,
        "num_winners":1,
        "title":"Test contest title",
        "tally_type":"plurality-at-large",
        "answer_total_votes_percentage":"over-total-valid-votes",
        "answers":[
           {
              "id":"38df9caf-2dc8-472c-87f2-f003241e9510",
              "category":"",
              "details":"This is an option with an simple example description.",
              "sort_order":0,
              "urls":[
                 {
                    "title":"Image URL",
                    "url":"https://i.imgur.com/XFQwVFL.jpg"
                 }
              ],
              "text":"Example option 1"
           },
           {
              "id":"97ac7d0a-e0f5-4e51-a1ee-6614c0836fec",
              "category":"",
              "details":"An option can contain a description. You can add simple html like ",
              "sort_order":1,
              "urls":[
                 {
                    "title":"URL",
                    "url":"https://sequentech.io"
                 },
                 {
                    "title":"Image URL",
                    "url":"/XFQwVFL.jpg"
                 }
              ],
              "text":"Example option 2"
           },
           {
              "id":"94c9eafa-ebc6-4594-a176-24788f761ced",
              "category":"",
              "details":"",
              "sort_order":2,
              "urls":[
                 
              ],
              "text":"Example option 3"
           }
        ],
        "extra_options":{
           "shuffle_categories":true,
           "shuffle_all_options":true,
           "shuffle_category_list":[
              
           ],
           "show_points":false,
           "base32_writeins":true
        }
     }"#;
    let contest: Contest = serde_json::from_str(contest_str).unwrap();
    contest
}

pub(crate) fn get_configurable_contest(
    max: i64,
    num_answers: usize,
    tally_type: String,
    enable_writeins: bool,
    write_in_contests: Option<Vec<usize>>,
    base32_writeins: bool,
) -> Contest {
    let contest_str = r#"{
        "id": "1fc963b1-f93b-4151-93d6-bbe0ea5eac46",
        "layout":"",
        "description":"Elige quien quieres que sea tu Secretario General en tu municipio",
        "min":0,
        "max":3,
        "tally_type":"plurality-at-large",
        "answers":[
           {
              "category":"Candidaturas no agrupadas",
              "text":"José Rabano Pimiento",
              "sort_order":0,
              "details":"",
              "urls":[
                 
              ],
              "id":"0"
           },
           {
              "category":"Candidaturas no agrupadas",
              "text":"Miguel Pimentel Inventado",
              "sort_order":1,
              "details":"",
              "urls":[
                 
              ],
              "id":"1"
           },
           {
              "category":"Candidaturas no agrupadas",
              "text":"Juan Iglesias Torquemada",
              "sort_order":2,
              "details":"",
              "urls":[
                 
              ],
              "id":"2"
           },
           {
              "category":"Candidaturas no agrupadas",
              "text":"Mari Pili Hernández Ordoñez",
              "sort_order":3,
              "details":"",
              "urls":[
                 
              ],
              "id":"3"
           },
           {
              "category":"Candidaturas no agrupadas",
              "text":"Juan Y Medio",
              "sort_order":4,
              "details":"",
              "urls":[
                 
              ],
              "id":"4"
           },
           {
              "category":"Candidaturas no agrupadas",
              "text":"Juan Y Medio",
              "sort_order":5,
              "details":"",
              "urls":[
                 
              ],
              "id":"5"
           },
           {
              "category":"Candidaturas no agrupadas",
              "text":"Juan Y Medio",
              "sort_order":6,
              "details":"",
              "urls":[
                 
              ],
              "id":"6"
           }
        ],
        "num_winners":1,
        "title":"Secretario General",
        "randomize_answer_order":true,
        "answer_total_votes_percentage":"over-total-valid-votes",
        "extra_options": {
            "base32_writeins": true
        }
     }"#;
    let mut contest: Contest = serde_json::from_str(contest_str).unwrap();

    contest.tally_type = tally_type;
    contest.max = max;
    if enable_writeins {
        let mut extra_options =
            contest.extra_options.unwrap_or(ContestExtra::new());
        extra_options.allow_writeins = Some(true);

        contest.extra_options = Some(extra_options);
        let write_in_indexes =
            write_in_contests.unwrap_or_else(|| vec![4, 5, 6]);
        for write_in_index in write_in_indexes {
            if write_in_index < contest.answers.len() {
                contest.answers[write_in_index].set_is_write_in(true);
            }
        }
    }
    // set base32_writeins
    let mut extra_options =
        contest.extra_options.unwrap_or(ContestExtra::new());
    extra_options.base32_writeins = Some(base32_writeins);
    contest.extra_options = Some(extra_options);

    contest.answers = contest.answers[0..num_answers].to_vec();
    contest
}

pub fn get_fixtures() -> Vec<BallotCodecFixture> {
    vec![
        BallotCodecFixture {
            title: "plurality_fixture".to_string(),
            contest: get_contest_plurality(),
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
                        answer_id: None,
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
            },
            encoded_ballot_bigint: "2756".to_string(),
            encoded_ballot: vec_to_30_array(&vec![2, 196, 10]).unwrap(),
            expected_errors: None
        },
        BallotCodecFixture {
            title: "example_3_explicit_and_implicit_invalid".to_string(),
            contest: Contest {
                id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".to_string(),
                description: "".to_string(),
                layout: "simultaneous-contests".to_string(),
                min: 0,
                max: 1,
                num_winners: 1,
                title: "Poste de maire(sse)".to_string(),
                tally_type: "plurality-at-large".to_string(),
                answer_total_votes_percentage: "over-total-valid-votes".to_string(),
                answers: vec![
                    Answer {
                        id: 0.to_string(),
                        category: "".to_string(),
                        text: "Chloe HUTCHISON".to_string(),
                        sort_order: 0,
                        details: "Independent".to_string(),
                        urls: vec![],
                    },
                    Answer {
                        id: 1.to_string(),
                        category: "".to_string(),
                        text: "Helen KURGANSKY".to_string(),
                        sort_order: 0,
                        details: "Political Affiliation 1".to_string(),
                        urls: vec![],
                    },
                    Answer {
                        id: 2.to_string(),
                        category: "".to_string(),
                        text: "Jamie NICHOLLS".to_string(),
                        sort_order: 0,
                        details: "Political Affiliation 2".to_string(),
                        urls: vec![],
                    },
                    Answer {
                        id: 3.to_string(),
                        category: "".to_string(),
                        text: "".to_string(),
                        sort_order: 0,
                        details: "".to_string(),
                        urls: vec![
                            Url {
                                title: "invalidVoteFlag".to_string(),
                                url: "true".to_string(),
                            }
                        ],
                    },
                ],
                extra_options: {
                    let mut extra = ContestExtra::new();
                    extra.invalid_vote_policy =  Some("allowed".to_string());
                    extra.base32_writeins = Some(true);
                    Some(extra)
                },
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
                        answer_id: None,
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
                id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".to_string(),
                layout: "simultaneous-contests".to_string(),
                description: "".to_string(),
                min: 0,
                max: 1,
                tally_type: "plurality-at-large".to_string(),
                answer_total_votes_percentage: "over-total-valid-votes".to_string(),
                answers: vec![
                    Answer {
                        id: 0.to_string(),
                        category: "".to_string(),
                        text: "Chloe HUTCHISON".to_string(),
                        sort_order: 0,
                        details: "Independent".to_string(),
                        urls: vec![],
                    },
                    Answer {
                        id: 1.to_string(),
                        category: "".to_string(),
                        text: "Helen KURGANSKY".to_string(),
                        sort_order: 0,
                        details: "Political Affiliation 1".to_string(),
                        urls: vec![],
                    },
                    Answer {
                        id: 2.to_string(),
                        category: "".to_string(),
                        text: "Jamie NICHOLLS".to_string(),
                        sort_order: 0,
                        details: "Political Affiliation 2".to_string(),
                        urls: vec![],
                    },
                    Answer {
                        id: 3.to_string(),
                        category: "".to_string(),
                        text: "".to_string(),
                        sort_order: 0,
                        details: "".to_string(),
                        urls: vec![
                            Url {
                                title: "invalidVoteFlag".to_string(),
                                url: "true".to_string(),
                            }
                        ],
                    },
                ],
                num_winners: 1,
                title: "Poste de maire(sse)".to_string(),
                extra_options: {
                    let mut extra = ContestExtra::new();
                    extra.invalid_vote_policy =  Some("allowed".to_string());
                    Some(extra)
                },
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
            },
            encoded_ballot_bigint: "3".to_string(),
            encoded_ballot: vec_to_30_array(&vec![1, 3]).unwrap(),
            expected_errors: None
        },
        BallotCodecFixture {
            title: "example_3_implicit_too_many".to_string(),
            contest: Contest {
                id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".to_string(),
                layout: "simultaneous-contests".to_string(),
                description: "".to_string(),
                min: 0,
                max: 1,
                tally_type: "plurality-at-large".to_string(),
                answers: vec![
                    Answer {
                        id: 0.to_string(),
                        category: "".to_string(),
                        text: "Chloe HUTCHISON".to_string(),
                        sort_order: 0,
                        details: "Independent".to_string(),
                        urls: vec![],
                    },
                    Answer {
                        id: 1.to_string(),
                        category: "".to_string(),
                        text: "Helen KURGANSKY".to_string(),
                        sort_order: 0,
                        details: "Political Affiliation 1".to_string(),
                        urls: vec![],
                    },
                    Answer {
                        id: 2.to_string(),
                        category: "".to_string(),
                        text: "Jamie NICHOLLS".to_string(),
                        sort_order: 0,
                        details: "Political Affiliation 2".to_string(),
                        urls: vec![],
                    },
                    Answer {
                        id: 3.to_string(),
                        category: "".to_string(),
                        text: "".to_string(),
                        sort_order: 0,
                        details: "".to_string(),
                        urls: vec![
                            Url {
                                title: "invalidVoteFlag".to_string(),
                                url: "true".to_string(),
                            }
                        ],
                    },
                ],
                num_winners: 1,
                title: "Poste de maire(sse)".to_string(),
                answer_total_votes_percentage: "over-total-valid-votes".to_string(),
                extra_options: {
                    let mut extra = ContestExtra::new();
                    extra.invalid_vote_policy =  Some("allowed".to_string());
                    Some(extra)
                },
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
                        answer_id: None,
                        message: Some("errors.implicit.selectedMax".to_string()),
                        message_map: HashMap::from([
                            ("numSelected".to_string(), 3.to_string()),
                            ("max".to_string(), 1.to_string()),
                        ]),
                    }
                ],
            },
            encoded_ballot_bigint: "14".to_string(),
            encoded_ballot: vec_to_30_array(&vec![1, 14]).unwrap(),
            expected_errors: None
        },
        BallotCodecFixture {
            title: "example_4_implicit_empty".to_string(),
            contest: Contest {
                id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".to_string(),
                layout: "accordion".to_string(),
                description: "".to_string(),
                min: 1,
                max: 1,
                tally_type: "plurality-at-large".to_string(),
                answers: vec![
                    Answer {
                        id: 0.to_string(),
                        category: "".to_string(),
                        text: "Example option 1".to_string(),
                        sort_order: 0,
                        details: "This is an option with an simple example description.".to_string(),
                        urls: vec![],
                    },
                    Answer {
                        id: 1.to_string(),
                        category: "".to_string(),
                        text: "Example option 2".to_string(),
                        sort_order: 0,
                        details: "An option can contain a description. You can add simple html like <strong>bold</strong> or <a href=\"https://sequentech.io\" rel=\"nofollow\">links to websites</a>. You can also set an image url below, but be sure it&#39;s HTTPS or else it won&#39;t load.\n\n<br /><br />You need to use two br element for new paragraphs.".to_string(),
                        urls: vec![],
                    },
                    Answer {
                        id: 2.to_string(),
                        category: "".to_string(),
                        text: "Example option 3".to_string(),
                        sort_order: 0,
                        details: "".to_string(),
                        urls: vec![],
                    }
                ],
                num_winners: 1,
                title: "Test contest title".to_string(),
                answer_total_votes_percentage: "over-total-valid-votes".to_string(),
                extra_options: {
                    let mut extra = ContestExtra::new();
                    extra.invalid_vote_policy =  Some("allowed".to_string());
                    Some(extra)
                },
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
                        answer_id: None,
                        message: Some("errors.implicit.selectedMin".to_string()),
                        message_map: HashMap::from([
                            ("numSelected".to_string(), 0.to_string()),
                            ("min".to_string(), 1.to_string()),
                        ]),
                    }
                ],
            },
            encoded_ballot_bigint: "0".to_string(),
            encoded_ballot: vec_to_30_array(&vec![1, 0]).unwrap(),
            expected_errors: None
        },
        BallotCodecFixture {
            title: "example_4_implicit_invented_answer".to_string(),
            contest: Contest {
                id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".to_string(),
                layout: "accordion".to_string(),
                description: "".to_string(),
                min: 1,
                max: 1,
                tally_type: "plurality-at-large".to_string(),
                answers: vec![
                    Answer {
                        id: 0.to_string(),
                        category: "".to_string(),
                        text: "Example option 1".to_string(),
                        sort_order: 0,
                        details: "This is an option with an simple example description.".to_string(),
                        urls: vec![],
                    },
                    Answer {
                        id: 1.to_string(),
                        category: "".to_string(),
                        text: "Example option 2".to_string(),
                        sort_order: 0,
                        details: "An option can contain a description. You can add simple html like <strong>bold</strong> or <a href=\"https://sequentech.io\" rel=\"nofollow\">links to websites</a>. You can also set an image url below, but be sure it&#39;s HTTPS or else it won&#39;t load.\n\n<br /><br />You need to use two br element for new paragraphs.".to_string(),
                        urls: vec![],
                    },
                    Answer {
                        id: 2.to_string(),
                        category: "".to_string(),
                        text: "Example option 3".to_string(),
                        sort_order: 0,
                        details: "".to_string(),
                        urls: vec![],
                    }
                ],
                num_winners: 1,
                title: "Test contest title".to_string(),
                answer_total_votes_percentage: "over-total-valid-votes".to_string(),
                extra_options: {
                    let mut extra = ContestExtra::new();
                    extra.invalid_vote_policy =  Some("allowed".to_string());
                    Some(extra)
                },
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
                        answer_id: None,
                        message: Some("errors.encoding.ballotTooLarge".to_string()),
                        message_map: HashMap::new(),
                    },
                    InvalidPlaintextError {
                        error_type: InvalidPlaintextErrorType::Implicit,
                        answer_id: None,
                        message: Some("errors.implicit.selectedMin".to_string()),
                        message_map: HashMap::from([
                            ("numSelected".to_string(), 0.to_string()),
                            ("min".to_string(), 1.to_string()),
                        ]),
                    }
                ],
            },
            encoded_ballot_bigint: "16".to_string(),
            encoded_ballot: vec_to_30_array(&vec![1, 16]).unwrap(),
            expected_errors: Some(HashMap::from([
                ("contest_bases".to_string(), "".to_string()),
                ("contest_encode_plaintext".to_string(), "choice id is not a valid answer".to_string()),
                ("contest_encode_to_raw_ballot".to_string(), "choice id is not a valid answer".to_string()),
                ("contest_decode_plaintext".to_string(), "decode_choices".to_string()),
                ("encoding_plaintext_bigint".to_string(), "choice id is not a valid answer".to_string()),
            ]))
        },
        BallotCodecFixture {
            title: "plurality with two selections".to_string(),
            contest: get_configurable_contest(3, 7, "plurality-at-large".to_string(), false, None, true),
            raw_ballot: RawBallotContest {
                bases: vec![2, 2, 2, 2, 2, 2, 2, 2],
                choices: vec![0, 0, 1, 0, 0, 0, 1, 0],
            },
            plaintext: DecodedVoteContest {
                contest_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".to_string(),
                is_explicit_invalid: false,
                invalid_errors: vec![],
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
            contest: get_configurable_contest(3, 7, "plurality-at-large".to_string(), false, None, true),
            raw_ballot: RawBallotContest {
                bases: vec![2, 2, 2, 2, 2, 2, 2, 2],
                choices: vec![0, 1, 1, 0, 0, 0, 1, 0],
            },
            plaintext: DecodedVoteContest {
                contest_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".to_string(),
                is_explicit_invalid: false,
                invalid_errors: vec![],
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
            contest: get_configurable_contest(3, 7, "borda".to_string(), false, None, true),
            raw_ballot: RawBallotContest {
                bases: vec![2, 4, 4, 4, 4, 4, 4, 4],
                choices: vec![0, 1, 3, 0, 0, 0, 2, 0]
            },
            plaintext: DecodedVoteContest {
                contest_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".to_string(),
                is_explicit_invalid: false,
                invalid_errors: vec![],
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
            contest: get_configurable_contest(2, 2, "plurality-at-large".to_string(), false, None, true),
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
                        answer_id: None,
                        message: Some("errors.explicit.notAllowed".to_string()),
                        message_map: HashMap::new(),
                    }
                ],
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
            contest: get_configurable_contest(2, 6, "borda".to_string(), true, None, true),
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
                        answer_id: None,
                        message: Some("errors.explicit.notAllowed".to_string()),
                        message_map: HashMap::new(),
                    },
                    InvalidPlaintextError {
                        error_type: InvalidPlaintextErrorType::Implicit,
                        answer_id: None,
                        message: Some("errors.implicit.selectedMax".to_string()),
                        message_map: HashMap::from([
                            ("numSelected".to_string(), "3".to_string()),
                            ("max".to_string(), "2".to_string())
                        ]),
                    }
                ],
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
            contest: get_configurable_contest(3, 7, "plurality-at-large".to_string(), true, None, true),
            raw_ballot: RawBallotContest {
                bases: vec![2, 2, 2, 2, 2, 2, 2, 2, 32, 32, 32, 32, 32, 32, 32, 32],
                choices: vec![0, 1, 0, 0, 0, 1, 0, 1, 5, 0, 0, 1, 27, 2, 3, 0]
            },
            plaintext: DecodedVoteContest {
                contest_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".to_string(),
                is_explicit_invalid: false,
                invalid_errors: vec![],
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
            contest: get_configurable_contest(2, 3, "plurality-at-large".to_string(), true, None, true),
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
                        answer_id: None,
                        message: Some("errors.encoding.notEnoughChoices".to_string()),
                        message_map: HashMap::new(),
                    }
                ],
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
            contest: get_configurable_contest(2, 3, "plurality-at-large".to_string(), true, Some(vec![0]), true),
            raw_ballot: RawBallotContest {
                bases:   vec![2, 2, 2, 2],
                choices: vec![0, 1, 0, 0],
            },
            plaintext: DecodedVoteContest {
                contest_id: "1fc963b1-f93b-4151-93d6-bbe0ea5eac46".to_string(),
                is_explicit_invalid: false,
                invalid_errors: vec![],
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
            contest: get_configurable_contest(2, 3, "plurality-at-large".to_string(), true, Some(vec![0]), true),
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
                        answer_id: Some(0.to_string()),
                        message: Some("errors.encoding.writeInNotEndInZero".to_string()),
                        message_map: HashMap::new(),
                    }
                ],
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
            contest: get_configurable_contest(2, 3, "plurality-at-large".to_string(), true, Some(vec![]), true),
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
                        answer_id: None,
                        message: Some("errors.encoding.ballotTooLarge".to_string()),
                        message_map: HashMap::new(),
                    }
                ],
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
                "plurality-at-large".to_string(),
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
                "plurality-at-large".to_string(),
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
                "borda".to_string(),
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
                "borda".to_string(),
                false,
                None,
                true,
            ),
            bases: vec![2, 3, 3, 3],
        },
    ]
}
