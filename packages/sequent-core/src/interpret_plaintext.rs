// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::ballot::*;
use crate::plaintext::*;
use crate::types::ceremonies::CountingAlgType;
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

    match contest.get_counting_algorithm() {
        CountingAlgType::PluralityAtLarge => Some(ContestLayoutProperties {
            state: ContestState::MultiContest,
            sorted: true,
            ordered: false,
        }),
        CountingAlgType::InstantRunoff => Some(ContestLayoutProperties {
            state: ContestState::MultiContest,
            sorted: true,
            ordered: true,
        }),
        CountingAlgType::BordaNauru => Some(ContestLayoutProperties {
            state: ContestState::MultiContest,
            sorted: true,
            ordered: true,
        }),
        CountingAlgType::Borda => Some(ContestLayoutProperties {
            state: ContestState::MultiContest,
            sorted: true,
            ordered: true,
        }),
        CountingAlgType::BordaMasMadrid => Some(ContestLayoutProperties {
            state: ContestState::MultiContest,
            sorted: true,
            ordered: true,
        }),
        CountingAlgType::PairwiseBeta => Some(ContestLayoutProperties {
            state: ContestState::PairwiseBeta,
            sorted: true,
            ordered: true,
        }),
        CountingAlgType::Desborda3 => Some(ContestLayoutProperties {
            state: ContestState::MultiContest,
            sorted: true,
            ordered: true,
        }),
        CountingAlgType::Desborda2 => Some(ContestLayoutProperties {
            state: ContestState::MultiContest,
            sorted: true,
            ordered: true,
        }),
        CountingAlgType::Desborda => Some(ContestLayoutProperties {
            state: ContestState::MultiContest,
            sorted: true,
            ordered: true,
        }),
        CountingAlgType::Cumulative => Some(ContestLayoutProperties {
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
    match contest.get_counting_algorithm() {
        CountingAlgType::PluralityAtLarge => Some(1),
        CountingAlgType::Borda => {
            Some((contest.max_votes as i64) - candidate.selected)
        }
        // "borda-mas-madrid" => return scope.contest.max -
        // scope.option.selected
        CountingAlgType::BordaNauru => Some(1 + candidate.selected), /* 1 / (1 + candidate. */
        // selected)
        CountingAlgType::PairwiseBeta => None,
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
        CountingAlgType::Desborda => Some(80 - candidate.selected),
        CountingAlgType::Cumulative => Some(candidate.selected + 1),
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

/// Check the ballot for its validity according to the its contest counting
/// algorithm rules.
pub fn check_ballot_validity(
    contests: &[Contest],
    decoded_contests: &[DecodedVoteContest],
) -> bool {
    for decoded_contest in decoded_contests.iter() {
        let contest =
            contests.iter().find(|c| c.id == decoded_contest.contest_id);
        match contest {
            Some(contest) if contest.is_valid_ballot(decoded_contest) => {}
            _ => return false,
        }
    }
    true
}
