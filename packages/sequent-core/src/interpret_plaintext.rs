// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::ballot::*;
use crate::plaintext::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::cmp;

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
pub enum ContestState {
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
pub struct ContestLayoutProperties {
    state: ContestState,
    sorted: bool,
    ordered: bool,
}

pub fn get_layout_properties(
    question: &Contest,
) -> Option<ContestLayoutProperties> {
    /*if question.layout == "conditional-accordion" {
        return Some(ContestLayoutProperties {
            state: ContestState::ConditionalAccordionScreen,
            sorted: true,
            ordered: true,
        });
    } else if question.layout == "pcandidates-election" {
        return Some(ContestLayoutProperties {
            state: ContestState::PcandidatesElectionScreen,
            sorted: true,
            ordered: true,
        });
    } else if question.layout == "simultaneous-questions" {
        return Some(ContestLayoutProperties {
            state: ContestState::SimultaneousQuestionsScreen,
            sorted: false,
            ordered: false,
        });
    }*/

    match question.get_counting_algorithm().as_str() {
        "plurality-at-large" => Some(ContestLayoutProperties {
            state: ContestState::MultiQuestion,
            sorted: true,
            ordered: false,
        }),
        "borda-nauru" => Some(ContestLayoutProperties {
            state: ContestState::MultiQuestion,
            sorted: true,
            ordered: true,
        }),
        "borda" => Some(ContestLayoutProperties {
            state: ContestState::MultiQuestion,
            sorted: true,
            ordered: true,
        }),
        "borda-mas-madrid" => Some(ContestLayoutProperties {
            state: ContestState::MultiQuestion,
            sorted: true,
            ordered: true,
        }),
        "pairwise-beta" => Some(ContestLayoutProperties {
            state: ContestState::PairwiseBeta,
            sorted: true,
            ordered: true,
        }),
        "desborda3" => Some(ContestLayoutProperties {
            state: ContestState::MultiQuestion,
            sorted: true,
            ordered: true,
        }),
        "desborda2" => Some(ContestLayoutProperties {
            state: ContestState::MultiQuestion,
            sorted: true,
            ordered: true,
        }),
        "desborda" => Some(ContestLayoutProperties {
            state: ContestState::MultiQuestion,
            sorted: true,
            ordered: true,
        }),
        "cumulative" => Some(ContestLayoutProperties {
            state: ContestState::SimultaneousQuestionsScreen,
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
    question: &Contest,
    answer: &DecodedVoteChoice,
) -> Option<i64> {
    if !&question.show_points() {
        return Some(0);
    }
    if answer.selected < 0 {
        return Some(0);
    }

    match question.get_counting_algorithm().as_str() {
        "plurality-at-large" => Some(1),
        "borda" => Some((question.max_votes as i64) - answer.selected),
        // "borda-mas-madrid" => return scope.question.max -
        // scope.option.selected
        "borda-nauru" => Some(1 + answer.selected), /* 1 / (1 + answer. */
        // selected)
        "pairwise-beta" => None,
        /*"desborda3" => Some(cmp::max(
            1,
            (((question.num_winners as f64) * 1.3) - (answer.selected as f64))
                .trunc() as i64,
        )),
        "desborda2" => Some(cmp::max(
            1,
            (((question.num_winners as f64) * 1.3) - (answer.selected as f64))
                .trunc() as i64,
        )),*/
        "desborda" => Some(80 - answer.selected),
        "cummulative" => Some(answer.selected + 1),
        _ => None,
    }
}
