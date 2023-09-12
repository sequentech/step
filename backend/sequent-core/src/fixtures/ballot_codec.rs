// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::ballot::{Answer, Question, QuestionExtra, Url};
use crate::ballot_codec::RawBallotQuestion;
use crate::plaintext::{
    DecodedVoteChoice, DecodedVoteQuestion, InvalidPlaintextError,
    InvalidPlaintextErrorType,
};
use crate::util::read_ballot_fixture;
use std::collections::HashMap;

pub struct BallotCodecFixture {
    pub title: String,
    pub question: Question,
    pub raw_ballot: RawBallotQuestion,
    pub plaintext: DecodedVoteQuestion,
    pub encoded_ballot: String,
    pub expected_errors: Option<HashMap<String, String>>,
}
pub struct BasesFixture {
    pub question: Question,
    pub bases: Vec<u64>,
}

fn get_question_plurality() -> Question {
    let ballot = read_ballot_fixture();
    ballot.config.configuration.questions[0].clone()
}

fn get_question_borda() -> Question {
    let mut question = get_question_plurality();
    question.tally_type = String::from("borda");
    question.max = 4;
    question
}

fn get_configurable_question(
    max: i64,
    num_answers: usize,
    tally_type: String,
    enable_writeins: bool,
    write_in_questions: Option<Vec<usize>>,
) -> Question {
    let question_str = r#"{
        "id": "fae0b09e-1b78-4118-b99c-7955f8ef2a52",
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
              "id":0
           },
           {
              "category":"Candidaturas no agrupadas",
              "text":"Miguel Pimentel Inventado",
              "sort_order":1,
              "details":"",
              "urls":[
                 
              ],
              "id":1
           },
           {
              "category":"Candidaturas no agrupadas",
              "text":"Juan Iglesias Torquemada",
              "sort_order":2,
              "details":"",
              "urls":[
                 
              ],
              "id":2
           },
           {
              "category":"Candidaturas no agrupadas",
              "text":"Mari Pili Hernández Ordoñez",
              "sort_order":3,
              "details":"",
              "urls":[
                 
              ],
              "id":3
           },
           {
              "category":"Candidaturas no agrupadas",
              "text":"Juan Y Medio",
              "sort_order":4,
              "details":"",
              "urls":[
                 
              ],
              "id":4
           },
           {
              "category":"Candidaturas no agrupadas",
              "text":"Juan Y Medio",
              "sort_order":5,
              "details":"",
              "urls":[
                 
              ],
              "id":5
           },
           {
              "category":"Candidaturas no agrupadas",
              "text":"Juan Y Medio",
              "sort_order":6,
              "details":"",
              "urls":[
                 
              ],
              "id":6
           }
        ],
        "num_winners":1,
        "title":"Secretario General",
        "randomize_answer_order":true,
        "answer_total_votes_percentage":"over-total-valid-votes"
     }"#;
    let mut question: Question = serde_json::from_str(question_str).unwrap();

    question.tally_type = tally_type;
    question.max = max;
    if enable_writeins {
        let mut extra_options =
            question.extra_options.unwrap_or(QuestionExtra::new());
        extra_options.allow_writeins = Some(true);

        question.extra_options = Some(extra_options);
        let write_in_indexes =
            write_in_questions.unwrap_or_else(|| vec![4, 5, 6]);
        for write_in_index in write_in_indexes {
            if write_in_index < question.answers.len() {
                question.answers[write_in_index].set_is_write_in(true);
            }
        }
    }
    question.answers = question.answers[0..num_answers].to_vec();
    question
}

pub fn get_fixtures() -> Vec<BallotCodecFixture> {
    vec![
        BallotCodecFixture {
            title: "plurality_fixture".to_string(),
            question: get_question_plurality(),
            raw_ballot: RawBallotQuestion {
                bases: vec![2u64, 2u64, 2u64, 2u64, 2u64, 2u64],
                choices: vec![0u64, 1u64, 0u64, 0u64, 1u64, 1u64],
            },
            plaintext: DecodedVoteQuestion {
                is_explicit_invalid: false,
                choices: vec![
                    DecodedVoteChoice {
                        id: 0,
                        selected: 0,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 1,
                        selected: -1,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 2,
                        selected: -1,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 3,
                        selected: 0,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 4,
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
            encoded_ballot: "50".to_string(),
            expected_errors: None
        },
        BallotCodecFixture {
            title: "borda_fixture".to_string(),
            question: get_question_borda(),
            raw_ballot: RawBallotQuestion {
                bases: vec![2u64, 5u64, 5u64, 5u64, 5u64, 5u64],
                choices: vec![0u64, 3u64, 0u64, 0u64, 1u64, 2u64],
            },
            plaintext: DecodedVoteQuestion {
                is_explicit_invalid: false,
                choices: vec![
                    DecodedVoteChoice {
                        id: 0,
                        selected: 2,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 1,
                        selected: -1,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 2,
                        selected: -1,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 3,
                        selected: 0,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 4,
                        selected: 1,
                        write_in_text: None,
                    },
                ],
                invalid_errors: vec![],
            },
            encoded_ballot: "2756".to_string(),
            expected_errors: None
        },
        BallotCodecFixture {
            title: "example_3_explicit_and_implicit_invalid".to_string(),
            question: Question {
                description: "".to_string(),
                layout: "simultaneous-questions".to_string(),
                min: 0,
                max: 1,
                num_winners: 1,
                title: "Poste de maire(sse)".to_string(),
                tally_type: "plurality-at-large".to_string(),
                answer_total_votes_percentage: "over-total-valid-votes".to_string(),
                answers: vec![
                    Answer {
                        id: 0,
                        category: "".to_string(),
                        text: "Chloe HUTCHISON".to_string(),
                        sort_order: 0,
                        details: "Independent".to_string(),
                        urls: vec![],
                    },
                    Answer {
                        id: 1,
                        category: "".to_string(),
                        text: "Helen KURGANSKY".to_string(),
                        sort_order: 0,
                        details: "Political Affiliation 1".to_string(),
                        urls: vec![],
                    },
                    Answer {
                        id: 2,
                        category: "".to_string(),
                        text: "Jamie NICHOLLS".to_string(),
                        sort_order: 0,
                        details: "Political Affiliation 2".to_string(),
                        urls: vec![],
                    },
                    Answer {
                        id: 3,
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
                    let mut extra = QuestionExtra::new();
                    extra.invalid_vote_policy =  Some("allowed".to_string());
                    Some(extra)
                },
            },
            raw_ballot: RawBallotQuestion {
                bases: vec![2u64, 2u64, 2u64, 2u64],
                choices: vec![1u64, 1u64, 1u64, 1u64],
            },
            plaintext: DecodedVoteQuestion {
                is_explicit_invalid: true,
                choices: vec![
                    DecodedVoteChoice {
                        id: 0,
                        selected: 0,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 1,
                        selected: 0,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 2,
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
            encoded_ballot: "15".to_string(),
            expected_errors: None
        },
        BallotCodecFixture {
            title: "example_3_explicit_invalid".to_string(),
            question: Question {
                layout: "simultaneous-questions".to_string(),
                description: "".to_string(),
                min: 0,
                max: 1,
                tally_type: "plurality-at-large".to_string(),
                answer_total_votes_percentage: "over-total-valid-votes".to_string(),
                answers: vec![
                    Answer {
                        id: 0,
                        category: "".to_string(),
                        text: "Chloe HUTCHISON".to_string(),
                        sort_order: 0,
                        details: "Independent".to_string(),
                        urls: vec![],
                    },
                    Answer {
                        id: 1,
                        category: "".to_string(),
                        text: "Helen KURGANSKY".to_string(),
                        sort_order: 0,
                        details: "Political Affiliation 1".to_string(),
                        urls: vec![],
                    },
                    Answer {
                        id: 2,
                        category: "".to_string(),
                        text: "Jamie NICHOLLS".to_string(),
                        sort_order: 0,
                        details: "Political Affiliation 2".to_string(),
                        urls: vec![],
                    },
                    Answer {
                        id: 3,
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
                    let mut extra = QuestionExtra::new();
                    extra.invalid_vote_policy =  Some("allowed".to_string());
                    Some(extra)
                },
            },
            raw_ballot: RawBallotQuestion {
                bases: vec![2u64, 2u64, 2u64, 2u64],
                choices: vec![1u64, 1u64, 0u64, 0u64],
            },
            plaintext: DecodedVoteQuestion {
                is_explicit_invalid: true,
                choices: vec![
                    DecodedVoteChoice {
                        id: 0,
                        selected: 0,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 1,
                        selected: -1,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 2,
                        selected: -1,
                        write_in_text: None,
                    }
                ],
                invalid_errors: vec![],
            },
            encoded_ballot: "3".to_string(),
            expected_errors: None
        },
        BallotCodecFixture {
            title: "example_3_implicit_too_many".to_string(),
            question: Question {
                layout: "simultaneous-questions".to_string(),
                description: "".to_string(),
                min: 0,
                max: 1,
                tally_type: "plurality-at-large".to_string(),
                answers: vec![
                    Answer {
                        id: 0,
                        category: "".to_string(),
                        text: "Chloe HUTCHISON".to_string(),
                        sort_order: 0,
                        details: "Independent".to_string(),
                        urls: vec![],
                    },
                    Answer {
                        id: 1,
                        category: "".to_string(),
                        text: "Helen KURGANSKY".to_string(),
                        sort_order: 0,
                        details: "Political Affiliation 1".to_string(),
                        urls: vec![],
                    },
                    Answer {
                        id: 2,
                        category: "".to_string(),
                        text: "Jamie NICHOLLS".to_string(),
                        sort_order: 0,
                        details: "Political Affiliation 2".to_string(),
                        urls: vec![],
                    },
                    Answer {
                        id: 3,
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
                    let mut extra = QuestionExtra::new();
                    extra.invalid_vote_policy =  Some("allowed".to_string());
                    Some(extra)
                },
            },
            raw_ballot: RawBallotQuestion {
                bases: vec![2u64, 2u64, 2u64, 2u64],
                choices: vec![0u64, 1u64, 1u64, 1u64],
            },
            plaintext: DecodedVoteQuestion {
                is_explicit_invalid: false,
                choices: vec![
                    DecodedVoteChoice {
                        id: 0,
                        selected: 0,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 1,
                        selected: 0,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 2,
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
            encoded_ballot: "14".to_string(),
            expected_errors: None
        },
        BallotCodecFixture {
            title: "example_4_implicit_empty".to_string(),
            question: Question {
                layout: "accordion".to_string(),
                description: "".to_string(),
                min: 1,
                max: 1,
                tally_type: "plurality-at-large".to_string(),
                answers: vec![
                    Answer {
                        id: 0,
                        category: "".to_string(),
                        text: "Example option 1".to_string(),
                        sort_order: 0,
                        details: "This is an option with an simple example description.".to_string(),
                        urls: vec![],
                    },
                    Answer {
                        id: 1,
                        category: "".to_string(),
                        text: "Example option 2".to_string(),
                        sort_order: 0,
                        details: "An option can contain a description. You can add simple html like <strong>bold</strong> or <a href=\"https://sequentech.io\" rel=\"nofollow\">links to websites</a>. You can also set an image url below, but be sure it&#39;s HTTPS or else it won&#39;t load.\n\n<br /><br />You need to use two br element for new paragraphs.".to_string(),
                        urls: vec![],
                    },
                    Answer {
                        id: 2,
                        category: "".to_string(),
                        text: "Example option 3".to_string(),
                        sort_order: 0,
                        details: "".to_string(),
                        urls: vec![],
                    }
                ],
                num_winners: 1,
                title: "Test question title".to_string(),
                answer_total_votes_percentage: "over-total-valid-votes".to_string(),
                extra_options: {
                    let mut extra = QuestionExtra::new();
                    extra.invalid_vote_policy =  Some("allowed".to_string());
                    Some(extra)
                },
            },
            raw_ballot: RawBallotQuestion {
                bases: vec![2u64, 2u64, 2u64, 2u64],
                choices: vec![0u64, 0u64, 0u64, 0u64],
            },
            plaintext: DecodedVoteQuestion {
                is_explicit_invalid: false,
                choices: vec![
                    DecodedVoteChoice {
                        id: 0,
                        selected: -1,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 1,
                        selected: -1,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 2,
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
            encoded_ballot: "0".to_string(),
            expected_errors: None
        },
        BallotCodecFixture {
            title: "example_4_implicit_invented_answer".to_string(),
            question: Question {
                layout: "accordion".to_string(),
                description: "".to_string(),
                min: 1,
                max: 1,
                tally_type: "plurality-at-large".to_string(),
                answers: vec![
                    Answer {
                        id: 0,
                        category: "".to_string(),
                        text: "Example option 1".to_string(),
                        sort_order: 0,
                        details: "This is an option with an simple example description.".to_string(),
                        urls: vec![],
                    },
                    Answer {
                        id: 1,
                        category: "".to_string(),
                        text: "Example option 2".to_string(),
                        sort_order: 0,
                        details: "An option can contain a description. You can add simple html like <strong>bold</strong> or <a href=\"https://sequentech.io\" rel=\"nofollow\">links to websites</a>. You can also set an image url below, but be sure it&#39;s HTTPS or else it won&#39;t load.\n\n<br /><br />You need to use two br element for new paragraphs.".to_string(),
                        urls: vec![],
                    },
                    Answer {
                        id: 2,
                        category: "".to_string(),
                        text: "Example option 3".to_string(),
                        sort_order: 0,
                        details: "".to_string(),
                        urls: vec![],
                    }
                ],
                num_winners: 1,
                title: "Test question title".to_string(),
                answer_total_votes_percentage: "over-total-valid-votes".to_string(),
                extra_options: {
                    let mut extra = QuestionExtra::new();
                    extra.invalid_vote_policy =  Some("allowed".to_string());
                    Some(extra)
                },
            },
            raw_ballot: RawBallotQuestion {
                bases: vec![2u64, 2u64, 2u64, 2u64, 2u64],
                choices: vec![0u64, 0u64, 0u64, 0u64, 1u64],
            },
            plaintext: DecodedVoteQuestion {
                is_explicit_invalid: false,
                choices: vec![
                    DecodedVoteChoice {
                        id: 0,
                        selected: -1,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 1,
                        selected: -1,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 2,
                        selected: -1,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 3,
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
            encoded_ballot: "16".to_string(),
            expected_errors: Some(HashMap::from([
                ("question_bases".to_string(), "".to_string()),
                ("question_encode_plaintext".to_string(), "choice id is not a valid answer".to_string()),
                ("question_encode_to_raw_ballot".to_string(), "choice id is not a valid answer".to_string()),
                ("question_decode_plaintext".to_string(), "decode_choices".to_string()),
            ]))
        },
        BallotCodecFixture {
            title: "plurality with two selections".to_string(),
            question: get_configurable_question(3, 7, "plurality-at-large".to_string(), false, None),
            raw_ballot: RawBallotQuestion {
                bases: vec![2, 2, 2, 2, 2, 2, 2, 2],
                choices: vec![0, 0, 1, 0, 0, 0, 1, 0],
            },
            plaintext: DecodedVoteQuestion {
                is_explicit_invalid: false,
                invalid_errors: vec![],
                choices: vec![
                    DecodedVoteChoice {
                        id: 0,
                        selected: -1,
                        write_in_text: None
                    },
                    DecodedVoteChoice {
                        id: 1,
                        selected: 0,
                        write_in_text: None
                    },
                    DecodedVoteChoice {
                        id: 2,
                        selected: -1,
                        write_in_text: None
                    },
                    DecodedVoteChoice {
                        id: 3,
                        selected: -1,
                        write_in_text: None
                    },
                    DecodedVoteChoice {
                        id: 4,
                        selected: -1,
                        write_in_text: None
                    },
                    DecodedVoteChoice {
                        id: 5,
                        selected: 1,
                        write_in_text: None
                    },
                    DecodedVoteChoice {
                        id: 6,
                        selected: -1,
                        write_in_text: None
                    }
                ]
            },
            encoded_ballot: "68".to_string(),
            expected_errors: Some(HashMap::from([
                ("question_decode_plaintext".to_string(), "decode_choices".to_string()),
            ]))
        },
        BallotCodecFixture {
            title: "plurality with three selections".to_string(),
            question: get_configurable_question(3, 7, "plurality-at-large".to_string(), false, None),
            raw_ballot: RawBallotQuestion {
                bases: vec![2, 2, 2, 2, 2, 2, 2, 2],
                choices: vec![0, 1, 1, 0, 0, 0, 1, 0],
            },
            plaintext: DecodedVoteQuestion {
                is_explicit_invalid: false,
                invalid_errors: vec![],
                choices: vec![
                    DecodedVoteChoice {
                        id: 0,
                        selected: 0,
                        write_in_text: None
                    },
                    DecodedVoteChoice {
                        id: 1,
                        selected: 0,
                        write_in_text: None
                    },
                    DecodedVoteChoice {
                        id: 2,
                        selected: -1,
                        write_in_text: None
                    },
                    DecodedVoteChoice {
                        id: 3,
                        selected: -1,
                        write_in_text: None
                    },
                    DecodedVoteChoice {
                        id: 4,
                        selected: -1,
                        write_in_text: None
                    },
                    DecodedVoteChoice {
                        id: 5,
                        selected: 1,
                        write_in_text: None
                    },
                    DecodedVoteChoice {
                        id: 6,
                        selected: -1,
                        write_in_text: None
                    }
                ]
            },
            encoded_ballot: "70".to_string(),
            expected_errors: Some(HashMap::from([
                ("question_decode_plaintext".to_string(), "decode_choices".to_string()),
            ]))
        },
        BallotCodecFixture {
            title: "borda with three selections".to_string(),
            question: get_configurable_question(3, 7, "borda".to_string(), false, None),
            raw_ballot: RawBallotQuestion {
                bases: vec![2, 4, 4, 4, 4, 4, 4, 4],
                choices: vec![0, 1, 3, 0, 0, 0, 2, 0]
            },
            plaintext: DecodedVoteQuestion {
                is_explicit_invalid: false,
                invalid_errors: vec![],
                choices: vec![
                    DecodedVoteChoice {
                        id: 0,
                        selected: 0,
                        write_in_text: None
                    },
                    DecodedVoteChoice {
                        id: 1,
                        selected: 2,
                        write_in_text: None
                    },
                    DecodedVoteChoice {
                        id: 2,
                        selected: -1,
                        write_in_text: None
                    },
                    DecodedVoteChoice {
                        id: 3,
                        selected: -1,
                        write_in_text: None
                    },
                    DecodedVoteChoice {
                        id: 4,
                        selected: -1,
                        write_in_text: None
                    },
                    DecodedVoteChoice {
                        id: 5,
                        selected: 1,
                        write_in_text: None
                    },
                    DecodedVoteChoice {
                        id: 6,
                        selected: -1,
                        write_in_text: None
                    }
                ]
            },
            encoded_ballot: "4122".to_string(),
            expected_errors: None
        },
        BallotCodecFixture {
            title: "plurality explicit invalid and one selection".to_string(),
            question: get_configurable_question(2, 2, "plurality-at-large".to_string(), false, None),
            raw_ballot: RawBallotQuestion {
                bases: vec![2, 2, 2],
                choices: vec![1, 1, 0]
            },
            plaintext: DecodedVoteQuestion {
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
                        id: 0,
                        selected: 0,
                        write_in_text: None
                    },
                    DecodedVoteChoice {
                        id: 1,
                        selected: -1,
                        write_in_text: None
                    },
                ]
            },
            encoded_ballot: "3".to_string(),
            expected_errors: None
        },
        BallotCodecFixture {
            title: "two write ins, an explicit invalid ballot, one of the write-ins is not selected".to_string(),
            question: get_configurable_question(2, 6, "borda".to_string(), true, None),
            raw_ballot: RawBallotQuestion {
                bases: vec![2, 3, 3, 3, 3, 3, 3, 256, 256, 256],
                choices: vec![1, 1, 0, 0, 1, 2, 0, 68, 0, 0]
            },
            plaintext: DecodedVoteQuestion {
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
                        id: 0,
                        selected: 0,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 1,
                        selected: -1,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 2,
                        selected: -1,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 3,
                        selected: 0,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 4,
                        selected: 1,
                        write_in_text: Some("D".to_string())
                    },
                    DecodedVoteChoice {
                        id: 5,
                        selected: -1,
                        write_in_text: Some("".to_string())
                    }
                ]
            },
            encoded_ballot: "99525".to_string(),
            expected_errors: Some(HashMap::from([
                ("question_bases".to_string(), "bases don't cover write-ins".to_string()),
            ]))
        },
        BallotCodecFixture {
            title: "three write ins, a valid ballot, one of the write-ins is not selected".to_string(),
            question: get_configurable_question(3, 7, "plurality-at-large".to_string(), true, None),
            raw_ballot: RawBallotQuestion {
                bases: vec![2, 2, 2, 2, 2, 2, 2, 2, 256, 256, 256, 256, 256, 256, 256, 256, 256],
                choices: vec![0, 1, 0, 0, 0, 1, 0, 1, 69, 0, 0, 195, 132, 32, 98, 99, 0]
            },
            plaintext: DecodedVoteQuestion {
                is_explicit_invalid: false,
                invalid_errors: vec![],
                choices: vec![
                    DecodedVoteChoice {
                        id: 0,
                        selected: 1,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 1,
                        selected: -1,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 2,
                        selected: -1,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 3,
                        selected: -1,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 4,
                        selected: 1,
                        write_in_text: Some("E".to_string()),
                    },
                    DecodedVoteChoice {
                        id: 5,
                        selected: -1,
                        write_in_text: Some("".to_string()),
                    },
                    DecodedVoteChoice {
                        id: 6,
                        selected: 1,
                        write_in_text: Some("Ä bc".to_string()),
                    }
                ]
            },
            encoded_ballot: "1833298460685270795682".to_string(),
            expected_errors: Some(HashMap::from([
                ("question_bases".to_string(), "bases don't cover write-ins".to_string()),
            ]))
        },
        BallotCodecFixture {
            title: "Not enough choices to decode".to_string(),
            question: get_configurable_question(2, 3, "plurality-at-large".to_string(), true, None),
            raw_ballot: RawBallotQuestion {
                bases: vec![2, 2, 2, 2],
                choices: vec![0, 1, 0],
            },
            plaintext: DecodedVoteQuestion {
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
                        id: 0,
                        selected: 0,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 1,
                        selected: -1,
                        write_in_text: None,
                    }
                ]
            },
            encoded_ballot: "2".to_string(),
            expected_errors: Some(HashMap::from([
                ("question_encode_raw_ballot".to_string(), "Invalid parameters: 'valueList' (size = 3) and 'baseList' (size = 4) must have the same length.".to_string()),
                ("question_encode_plaintext".to_string(), "Invalid parameters: 'valueList' (size = 3) and 'baseList' (size = 4) must have the same length.".to_string()),
                ("question_decode_plaintext".to_string(), "invalid_errors,decode_choices".to_string()),
            ]))
        },
        BallotCodecFixture {
            title: "invalid utf-8 sequence".to_string(),
            question: get_configurable_question(2, 3, "plurality-at-large".to_string(), true, Some(vec![0])),
            raw_ballot: RawBallotQuestion {
                bases:   vec![2, 2, 2, 2, 256, 256],
                choices: vec![0, 1, 0, 0, 150, 0],
            },
            plaintext: DecodedVoteQuestion {
                is_explicit_invalid: false,
                invalid_errors: vec![
                    InvalidPlaintextError {
                        error_type: InvalidPlaintextErrorType::EncodingError,
                        answer_id: Some(0),
                        message: Some("errors.encoding.bytesToUtf8Conversion".to_string()),
                        message_map: HashMap::from([
                            ("errorMessage".to_string(), "invalid utf-8 sequence of 1 bytes from index 0".to_string())
                        ]),
                    }
                ],
                choices: vec![
                    DecodedVoteChoice {
                        id: 0,
                        selected: 0,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 1,
                        selected: -1,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 2,
                        selected: -1,
                        write_in_text: None,
                    }
                ]
            },
            encoded_ballot: "2402".to_string(),
            expected_errors: Some(HashMap::from([
                ("question_bases".to_string(),  "bases don't cover write-ins".to_string()),
                ("question_encode_to_raw_ballot".to_string(),  "disabled".to_string()),
                ("question_encode_plaintext".to_string(),  "disabled".to_string()),
            ]))
        },
        BallotCodecFixture {
            title: "Write in doesn't end on 0".to_string(),
            question: get_configurable_question(2, 3, "plurality-at-large".to_string(), true, Some(vec![0])),
            raw_ballot: RawBallotQuestion {
                bases:   vec![2, 2, 2, 2, 256],
                choices: vec![0, 1, 0, 0, 97],
            },
            plaintext: DecodedVoteQuestion {
                is_explicit_invalid: false,
                invalid_errors: vec![
                    InvalidPlaintextError {
                        error_type: InvalidPlaintextErrorType::EncodingError,
                        answer_id: Some(0),
                        message: Some("errors.encoding.writeInNotEndInZero".to_string()),
                        message_map: HashMap::new(),
                    }
                ],
                choices: vec![
                    DecodedVoteChoice {
                        id: 0,
                        selected: 0,
                        write_in_text: Some("a".to_string()),
                    },
                    DecodedVoteChoice {
                        id: 1,
                        selected: -1,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 2,
                        selected: -1,
                        write_in_text: None,
                    }
                ]
            },
            encoded_ballot: "1554".to_string(),
            expected_errors: Some(HashMap::from([
                ("question_encode_to_raw_ballot".to_string(),  "disabled".to_string()),
                ("question_decode_plaintext".to_string(),  "invalid_errors, decode_choices".to_string()),
            ]))
        },
        BallotCodecFixture {
            title: "Ballot larger than expected".to_string(),
            question: get_configurable_question(2, 3, "plurality-at-large".to_string(), true, Some(vec![])),
            raw_ballot: RawBallotQuestion {
                bases: vec![2, 2, 2, 2, 256],
                choices: vec![0, 1, 0, 0, 24],
            },
            plaintext: DecodedVoteQuestion {
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
                        id: 0,
                        selected: 0,
                        write_in_text: None
                    },
                    DecodedVoteChoice {
                        id: 1,
                        selected: -1,
                        write_in_text: None
                    },
                    DecodedVoteChoice {
                        id: 2,
                        selected: -1,
                        write_in_text: None
                    }
                ]
            },
            encoded_ballot: "386".to_string(),
            expected_errors: Some(HashMap::from([
                ("question_bases".to_string(),  "bases don't cover write-ins".to_string()),
                ("question_encode_to_raw_ballot".to_string(),  "disabled".to_string()),
                ("question_encode_plaintext".to_string(),  "disabled".to_string()),
            ]))
        },
        // see https://hsivonen.fi/string-length/
        BallotCodecFixture {
            title: "write in fixture with utf-8 characters".to_string(),
            question: get_configurable_question(2, 6, "borda".to_string(), true, None),
            raw_ballot: RawBallotQuestion {
                bases: vec![2, 3, 3, 3, 3, 3, 3, 256, 256, 256, 256, 256, 256, 256, 256, 256, 256, 256, 256, 256, 256, 256, 256, 256, 256, 256],
                choices: vec![1, 1, 0, 0, 1, 2, 0, 240, 159, 164, 166, 240, 159, 143, 188, 226, 128, 141, 226, 153, 130, 239, 184, 143, 0, 0]
            },
            plaintext: DecodedVoteQuestion {
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
                        id: 0,
                        selected: 0,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 1,
                        selected: -1,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 2,
                        selected: -1,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 3,
                        selected: 0,
                        write_in_text: None,
                    },
                    DecodedVoteChoice {
                        id: 4,
                        selected: 1,
                        write_in_text: Some("🤦🏼‍♂️".to_string())
                    },
                    DecodedVoteChoice {
                        id: 5,
                        selected: -1,
                        write_in_text: Some("".to_string())
                    }
                ]
            },
            encoded_ballot: "71305239641951160318911622492115035225515613".to_string(),
            expected_errors: Some(HashMap::from([
                ("question_bases".to_string(), "bases don't cover write-ins".to_string()),
            ]))
        },
    ]
}

pub fn bases_fixture() -> Vec<BasesFixture> {
    vec![
        BasesFixture {
            question: get_configurable_question(
                3,
                7,
                "plurality-at-large".to_string(),
                false,
                None,
            ),
            bases: vec![2, 2, 2, 2, 2, 2, 2, 2],
        },
        BasesFixture {
            question: get_configurable_question(
                1,
                1,
                "plurality-at-large".to_string(),
                false,
                None,
            ),
            bases: vec![2, 2],
        },
        BasesFixture {
            question: get_configurable_question(
                1,
                1,
                "borda".to_string(),
                false,
                None,
            ),
            bases: vec![2, 2],
        },
        BasesFixture {
            question: get_configurable_question(
                2,
                3,
                "borda".to_string(),
                false,
                None,
            ),
            bases: vec![2, 3, 3, 3],
        },
    ]
}
