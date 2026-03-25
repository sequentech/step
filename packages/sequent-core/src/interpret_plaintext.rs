// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::ballot::Contest;
use crate::plaintext::{DecodedVoteChoice, DecodedVoteContest};
use crate::types::ceremonies::CountingAlgType;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Enum representing the different states of the contest UI.
#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
#[allow(missing_docs)]
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

/// Struct representing the layout properties of a contest, including its state and whether it is sorted or ordered.
#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
pub struct ContestLayoutProperties {
    /// The state of the contest UI.
    state: ContestState,
    /// Whether the contest is sorted.
    sorted: bool,
    /// Whether the contest is ordered.
    ordered: bool,
}

#[must_use]
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
        CountingAlgType::InstantRunoff
        | CountingAlgType::BordaNauru
        | CountingAlgType::Borda
        | CountingAlgType::BordaMasMadrid
        | CountingAlgType::Desborda3
        | CountingAlgType::Desborda2
        | CountingAlgType::Desborda => Some(ContestLayoutProperties {
            state: ContestState::MultiContest,
            sorted: true,
            ordered: true,
        }),
        CountingAlgType::PairwiseBeta => Some(ContestLayoutProperties {
            state: ContestState::PairwiseBeta,
            sorted: true,
            ordered: true,
        }),
        CountingAlgType::Cumulative => Some(ContestLayoutProperties {
            state: ContestState::SimultaneousContestsScreen,
            sorted: false,
            ordered: false,
        }),
    }
}

/**
 * @returns number of points this ballot is giving to this option
 */
#[must_use]
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
            contest.max_votes.checked_sub(candidate.selected)
        }
        // "borda-mas-madrid" => return scope.contest.max -
        // scope.option.selected
        CountingAlgType::BordaNauru | CountingAlgType::Cumulative => {
            candidate.selected.checked_add(1)
        } /* 1 / (1 + candidate. */
        // selected)
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
        CountingAlgType::Desborda => 80i64.checked_sub(candidate.selected),
        _ => None,
    }
}

/// Checks if the given decoded contest is blank, meaning it has no explicit invalidity and all choices are unselected.
#[must_use]
pub fn check_is_blank(decoded_contest: &DecodedVoteContest) -> bool {
    !decoded_contest.is_explicit_invalid
        && decoded_contest
            .choices
            .iter()
            .all(|choice| choice.selected < 0)
}
