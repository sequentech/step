// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The review screen must interpret a decoded vote consistently with its
//! counting algorithm. Expected values below are explicit election examples.

#![cfg(feature = "default_features")]

use sequent_core::ballot::*;
use sequent_core::interpret_plaintext::{
    check_is_blank, get_layout_properties, get_points,
};
use sequent_core::plaintext::{DecodedVoteChoice, DecodedVoteContest};
use sequent_core::types::ceremonies::CountingAlgType;
use sequent_core::util::voting_screen::get_contest_plurality;
use serde_json::json;

fn contest() -> Contest {
    get_contest_plurality(
        EOverVotePolicy::ALLOWED,
        EBlankVotePolicy::ALLOWED,
        InvalidVotePolicy::ALLOWED,
        Some(3),
    )
}

#[test]
fn counting_algorithms_choose_ranked_or_checkbox_layouts() {
    use CountingAlgType::*;
    for (algorithm, state, sorted, ordered) in [
        (PluralityAtLarge, "MultiContest", true, false),
        (InstantRunoff, "MultiContest", true, true),
        (BordaNauru, "MultiContest", true, true),
        (Borda, "MultiContest", true, true),
        (BordaMasMadrid, "MultiContest", true, true),
        (PairwiseBeta, "PairwiseBeta", true, true),
        (Desborda3, "MultiContest", true, true),
        (Desborda2, "MultiContest", true, true),
        (Desborda, "MultiContest", true, true),
        (Cumulative, "SimultaneousContestsScreen", false, false),
    ] {
        let mut contest = contest();
        contest.counting_algorithm = Some(algorithm);
        assert_eq!(
            serde_json::to_value(get_layout_properties(&contest).unwrap())
                .unwrap(),
            json!({"state": state, "sorted": sorted, "ordered": ordered}),
            "{algorithm:?}"
        );
    }
}

#[test]
fn displayed_points_apply_the_selected_rank_only_when_enabled() {
    use CountingAlgType::*;
    let mut contest = contest();
    contest.max_votes = 5;
    let choice = DecodedVoteChoice {
        id: "candidate".into(),
        selected: 2,
        write_in_text: None,
    };
    assert_eq!(get_points(&contest, &choice), Some(0));
    contest.presentation.as_mut().unwrap().show_points = Some(true);
    for (algorithm, expected) in [
        (PluralityAtLarge, Some(1)),
        (Borda, Some(3)),
        (BordaNauru, Some(3)),
        (Desborda, Some(78)),
        (Cumulative, Some(3)),
        (PairwiseBeta, None),
        (InstantRunoff, None),
        (BordaMasMadrid, None),
        (Desborda2, None),
        (Desborda3, None),
    ] {
        contest.counting_algorithm = Some(algorithm);
        assert_eq!(get_points(&contest, &choice), expected, "{algorithm:?}");
        let unselected = DecodedVoteChoice {
            selected: -1,
            ..choice.clone()
        };
        assert_eq!(get_points(&contest, &unselected), Some(0));
    }
}

#[test]
fn an_explicit_invalid_vote_is_never_presented_as_blank() {
    for (ranks, invalid, blank) in [
        (vec![], false, true),
        (vec![-1, -1], false, true),
        (vec![-1, 0], false, false),
        (vec![-1, -1], true, false),
    ] {
        let decoded = DecodedVoteContest {
            contest_id: "mayor".into(),
            is_explicit_invalid: invalid,
            is_decline_to_vote: false,
            is_blank_ballot: false,
            invalid_errors: vec![],
            invalid_alerts: vec![],
            choices: ranks
                .into_iter()
                .map(|selected| DecodedVoteChoice {
                    id: "candidate".into(),
                    selected,
                    write_in_text: None,
                })
                .collect(),
        };
        assert_eq!(check_is_blank(decoded), blank);
    }
}

#[test]
fn unrepresentable_point_counts_return_no_value_instead_of_panicking_or_wrapping(
) {
    let mut contest = contest();
    contest.presentation.as_mut().unwrap().show_points = Some(true);
    let choice = DecodedVoteChoice {
        id: "candidate".into(),
        selected: i64::MAX,
        write_in_text: None,
    };
    for algorithm in [CountingAlgType::Cumulative, CountingAlgType::BordaNauru]
    {
        contest.counting_algorithm = Some(algorithm);
        assert_eq!(get_points(&contest, &choice), None);
    }
    contest.counting_algorithm = Some(CountingAlgType::Borda);
    contest.max_votes = i64::MIN;
    let first = DecodedVoteChoice {
        selected: 1,
        ..choice
    };
    assert_eq!(get_points(&contest, &first), None);
}
