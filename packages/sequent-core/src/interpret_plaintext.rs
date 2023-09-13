// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::ballot::*;
use crate::plaintext::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::cmp;

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
pub enum QuestionState {
    ElectionChooserScreen,
    ReceivingElection,
    ErrorScreen,
    HelpScreen,
    StartScreen,
    MultiQuestion,
    PairwiseBeta,
    DraftsElectionScreen,
    AuditBallotScreen,
    PcandidatesElectionScreen,
    TwoQuestionsConditionalScreen,
    SimultaneousQuestionsScreen,
    ConditionalAccordionScreen,
    EncryptingBallotScreen,
    CastOrCancelScreen,
    ReviewScreen,
    CastingBallotScreen,
    SuccessScreen,
    ShowPdf,
}

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
pub struct QuestionLayoutProperties {
    state: QuestionState,
    sorted: bool,
    ordered: bool,
}

pub fn get_layout_properties(
    question: &Question,
) -> Option<QuestionLayoutProperties> {
    if question.layout == "conditional-accordion" {
        return Some(QuestionLayoutProperties {
            state: QuestionState::ConditionalAccordionScreen,
            sorted: true,
            ordered: true,
        });
    } else if question.layout == "pcandidates-election" {
        return Some(QuestionLayoutProperties {
            state: QuestionState::PcandidatesElectionScreen,
            sorted: true,
            ordered: true,
        });
    } else if question.layout == "simultaneous-questions" {
        return Some(QuestionLayoutProperties {
            state: QuestionState::SimultaneousQuestionsScreen,
            sorted: false,
            ordered: false,
        });
    }

    match question.tally_type.as_str() {
        "plurality-at-large" => Some(QuestionLayoutProperties {
            state: QuestionState::MultiQuestion,
            sorted: true,
            ordered: false,
        }),
        "borda-nauru" => Some(QuestionLayoutProperties {
            state: QuestionState::MultiQuestion,
            sorted: true,
            ordered: true,
        }),
        "borda" => Some(QuestionLayoutProperties {
            state: QuestionState::MultiQuestion,
            sorted: true,
            ordered: true,
        }),
        "borda-mas-madrid" => Some(QuestionLayoutProperties {
            state: QuestionState::MultiQuestion,
            sorted: true,
            ordered: true,
        }),
        "pairwise-beta" => Some(QuestionLayoutProperties {
            state: QuestionState::PairwiseBeta,
            sorted: true,
            ordered: true,
        }),
        "desborda3" => Some(QuestionLayoutProperties {
            state: QuestionState::MultiQuestion,
            sorted: true,
            ordered: true,
        }),
        "desborda2" => Some(QuestionLayoutProperties {
            state: QuestionState::MultiQuestion,
            sorted: true,
            ordered: true,
        }),
        "desborda" => Some(QuestionLayoutProperties {
            state: QuestionState::MultiQuestion,
            sorted: true,
            ordered: true,
        }),
        "cumulative" => Some(QuestionLayoutProperties {
            state: QuestionState::SimultaneousQuestionsScreen,
            sorted: false,
            ordered: false,
        }),
        _ => None,
    }
}

/**
 * @returns number of points this ballot is giving to this option
 */
pub fn get_points(
    question: &Question,
    answer: &DecodedVoteChoice,
) -> Option<i64> {
    if !&question.show_points() {
        return Some(0);
    }
    if answer.selected < 0 {
        return Some(0);
    }

    match question.tally_type.as_str() {
        "plurality-at-large" => Some(1),
        "borda" => Some((question.max as i64) - answer.selected),
        // "borda-mas-madrid" => return scope.question.max -
        // scope.option.selected
        "borda-nauru" => Some(1 + answer.selected), /* 1 / (1 + answer. */
        // selected)
        "pairwise-beta" => None,
        "desborda3" => Some(cmp::max(
            1,
            (((question.num_winners as f64) * 1.3) - (answer.selected as f64))
                .trunc() as i64,
        )),
        "desborda2" => Some(cmp::max(
            1,
            (((question.num_winners as f64) * 1.3) - (answer.selected as f64))
                .trunc() as i64,
        )),
        "desborda" => Some(80 - answer.selected),
        "cummulative" => Some(answer.selected + 1),
        _ => None,
    }
}
