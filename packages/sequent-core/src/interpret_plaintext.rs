// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::ballot::*;
use crate::plaintext::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
pub enum ContestState {
    ElectionChooserScreen,
    ReceivingElection,
    ErrorScreen,
    HelpScreen,
    StartScreen,
    MultiContest,
    PairwiseBeta,
    DraftsElectionScreen,
    AuditBallotScreen,
    PcandidatesElectionScreen,
    TwoContestsConditionalScreen,
    SimultaneousContestsScreen,
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
    contest: &Contest,
) -> Option<ContestLayoutProperties> {
    /*if contest.layout == "conditional-accordion" {
        return Some(ContestLayoutProperties {
            state: ContestState::ConditionalAccordionScreen,
            sorted: true,
            ordered: true,
        });
    } else if contest.layout == "pcandidates-election" {
        return Some(ContestLayoutProperties {
            state: ContestState::PcandidatesElectionScreen,
            sorted: true,
            ordered: true,
        });
    } else if contest.layout == "simultaneous-contests" {
        return Some(ContestLayoutProperties {
            state: ContestState::SimultaneousContestsScreen,
            sorted: false,
            ordered: false,
        });
    }*/

    match contest.get_counting_algorithm().as_str() {
        "plurality-at-large" => Some(ContestLayoutProperties {
            state: ContestState::MultiContest,
            sorted: true,
            ordered: false,
        }),
        "borda-nauru" => Some(ContestLayoutProperties {
            state: ContestState::MultiContest,
            sorted: true,
            ordered: true,
        }),
        "borda" => Some(ContestLayoutProperties {
            state: ContestState::MultiContest,
            sorted: true,
            ordered: true,
        }),
        "borda-mas-madrid" => Some(ContestLayoutProperties {
            state: ContestState::MultiContest,
            sorted: true,
            ordered: true,
        }),
        "pairwise-beta" => Some(ContestLayoutProperties {
            state: ContestState::PairwiseBeta,
            sorted: true,
            ordered: true,
        }),
        "desborda3" => Some(ContestLayoutProperties {
            state: ContestState::MultiContest,
            sorted: true,
            ordered: true,
        }),
        "desborda2" => Some(ContestLayoutProperties {
            state: ContestState::MultiContest,
            sorted: true,
            ordered: true,
        }),
        "desborda" => Some(ContestLayoutProperties {
            state: ContestState::MultiContest,
            sorted: true,
            ordered: true,
        }),
        "cumulative" => Some(ContestLayoutProperties {
            state: ContestState::SimultaneousContestsScreen,
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
    contest: &Contest,
    candidate: &DecodedVoteChoice,
) -> Option<i64> {
    if !&contest.show_points() {
        return Some(0);
    }
    if candidate.selected < 0 {
        return Some(0);
    }
    match contest.get_counting_algorithm().as_str() {
        "plurality-at-large" => Some(1),
        "borda" => Some((contest.max_votes as i64) - candidate.selected),
        // "borda-mas-madrid" => return scope.contest.max -
        // scope.option.selected
        "borda-nauru" => Some(1 + candidate.selected), /* 1 / (1 + candidate. */
        // selected)
        "pairwise-beta" => None,
        /*"desborda3" => Some(cmp::max(
            1,
            (((contest.num_winners as f64) * 1.3) - (candidate.selected as f64))
                .trunc() as i64,
        )),
        "desborda2" => Some(cmp::max(
            1,
            (((contest.num_winners as f64) * 1.3) - (candidate.selected as f64))
                .trunc() as i64,
        )),*/
        "desborda" => Some(80 - candidate.selected),
        "cummulative" => Some(candidate.selected + 1),
        _ => None,
    }
}

pub fn check_is_blank(decoded_contest: DecodedVoteContest) -> bool {
    !decoded_contest.is_explicit_invalid
        && decoded_contest
            .choices
            .iter()
            .all(|choice| choice.selected < 0)
}
