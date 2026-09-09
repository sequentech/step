// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

#[cfg(test)]
use sequent_core::ballot::Candidate;

use crate::pipes::do_tally::ExtendedMetricsContest;
use sequent_core::plaintext::DecodedVoteContest;
use sequent_core::{
    ballot::{BallotStyle, Contest, Weight},
    types::ceremonies::{CountingAlgType, TallyOperation},
};
use std::collections::HashSet;
use std::str::FromStr;
use tracing::instrument;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BallotClass {
    ExplicitInvalid,
    ImplicitInvalid,
    ExplicitBlank,
    ImplicitBlank,
    Declined,
    Valid,
}

pub fn get_explicit_blank_candidate_ids(contest: &Contest) -> HashSet<String> {
    contest
        .candidates
        .iter()
        .filter(|candidate| candidate.is_explicit_blank())
        .map(|candidate| candidate.id.clone())
        .collect()
}

fn is_explicit_blank_choice(
    choice_id: &str,
    explicit_blank_candidate_ids: &HashSet<String>,
) -> bool {
    explicit_blank_candidate_ids.contains(choice_id)
}

/// Computes in a single pass over the vote's choices whether the explicit
/// blank candidate is selected and whether any non-blank candidate is
/// selected. Callers classifying a ballot always need both flags.
fn explicit_blank_selection_flags(
    vote: &DecodedVoteContest,
    explicit_blank_candidate_ids: &HashSet<String>,
) -> (bool, bool) {
    let mut has_explicit_blank = false;
    let mut has_non_explicit_blank = false;
    for choice in &vote.choices {
        if choice.selected > -1 {
            if is_explicit_blank_choice(&choice.id, explicit_blank_candidate_ids) {
                has_explicit_blank = true;
            } else {
                has_non_explicit_blank = true;
            }
            if has_explicit_blank && has_non_explicit_blank {
                break;
            }
        }
    }
    (has_explicit_blank, has_non_explicit_blank)
}

pub fn is_explicit_blank_vote(
    vote: &DecodedVoteContest,
    explicit_blank_candidate_ids: &HashSet<String>,
) -> bool {
    explicit_blank_selection_flags(vote, explicit_blank_candidate_ids).0
}

pub fn has_selected_non_explicit_blank_choice(
    vote: &DecodedVoteContest,
    explicit_blank_candidate_ids: &HashSet<String>,
) -> bool {
    explicit_blank_selection_flags(vote, explicit_blank_candidate_ids).1
}

pub fn classify_ballot(
    vote: &DecodedVoteContest,
    explicit_blank_candidate_ids: &HashSet<String>,
) -> BallotClass {
    // Decline-to-vote takes precedence over the invalid checks: a declined
    // ballot is intentionally empty, so it must not be reclassified as
    // invalid. A declined ballot with actual content (selections, an
    // explicit invalid marker or policy errors) is not blank and classifies
    // as implicit invalid.
    if vote.is_decline_to_vote() {
        if vote.is_blank() {
            BallotClass::Declined
        } else {
            BallotClass::ImplicitInvalid
        }
    } else if vote.is_invalid() {
        if vote.is_explicit_invalid {
            BallotClass::ExplicitInvalid
        } else {
            BallotClass::ImplicitInvalid
        }
    } else {
        let (has_explicit_blank, has_non_explicit_blank_selection) =
            explicit_blank_selection_flags(vote, explicit_blank_candidate_ids);

        if has_explicit_blank && has_non_explicit_blank_selection {
            BallotClass::ImplicitInvalid
        } else if has_explicit_blank {
            BallotClass::ExplicitBlank
        } else if vote.is_blank() {
            BallotClass::ImplicitBlank
        } else {
            BallotClass::Valid
        }
    }
}

fn count_actual_votes(
    vote: &DecodedVoteContest,
    explicit_blank_candidate_ids: &HashSet<String>,
) -> u64 {
    vote.choices.iter().fold(0u64, |acc, choice| {
        if choice.selected > -1
            && !is_explicit_blank_choice(&choice.id, explicit_blank_candidate_ids)
        {
            acc + 1
        } else {
            acc
        }
    })
}

fn calculate_undervotes(actual_votes: u64, contest: &Contest) -> u64 {
    // Calculate undervotes based on max_votes
    let max_votes = contest.max_votes as u64;
    max_votes.saturating_sub(actual_votes)
}

fn calculate_valid_votes(actual_votes: u64, contest: &Contest) -> u64 {
    // Check if votes are within valid range
    if actual_votes >= (contest.min_votes as u64) && actual_votes <= (contest.max_votes as u64) {
        actual_votes
    } else {
        0
    }
}

fn calculate_overvotes(actual_votes: u64, contest: &Contest) -> u64 {
    // Calculate overvotes if actual votes exceed max_votes
    actual_votes.saturating_sub(contest.max_votes as u64)
}

#[instrument(skip_all)]
pub fn update_extended_metrics(
    vote: &DecodedVoteContest,
    current_metrics: &ExtendedMetricsContest,
    contest: &Contest,
    explicit_blank_candidate_ids: &HashSet<String>,
) -> ExtendedMetricsContest {
    let mut metrics = current_metrics.clone();

    // Count the actual (non marker) votes once; all derived metrics below
    // are computed from this count.
    let actual_votes = count_actual_votes(vote, explicit_blank_candidate_ids);

    // Calculate valid votes first
    let valid_votes = calculate_valid_votes(actual_votes, contest);
    metrics.votes_actually += valid_votes;

    // Calculate undervotes if not a decline to vote
    if !vote.is_decline_to_vote() {
        let undervotes = calculate_undervotes(actual_votes, contest);
        metrics.under_votes += undervotes;
    }

    // Calculate overvotes
    let overvotes = calculate_overvotes(actual_votes, contest);
    metrics.over_votes += overvotes;

    // Expected votes is always max_votes per ballot
    metrics.expected_votes += contest.max_votes as u64;

    metrics
}

#[instrument(skip_all)]
pub fn get_contest_tally_operation(contest: &Contest) -> TallyOperation {
    let default_tally_op = contest
        .get_counting_algorithm()
        .get_default_tally_operation_for_contest();
    let annotations = contest.annotations.clone().unwrap_or_default();
    let operation = annotations
        .get("tally_operation")
        .cloned()
        .unwrap_or_default();
    TallyOperation::from_str(&operation).unwrap_or(default_tally_op)
}

#[instrument(skip_all)]
pub fn get_area_tally_operation(
    ballot_styles: &[BallotStyle],
    counting_alg: CountingAlgType,
    area_id: &Uuid,
) -> TallyOperation {
    let area_ballot_style: Option<&BallotStyle> = ballot_styles
        .iter()
        .find(|bs| bs.area_id == area_id.to_string());

    match area_ballot_style
        .and_then(|bs| bs.area_annotations.as_ref())
        .and_then(|area_annotations| area_annotations.tally_operation)
    {
        Some(tally_op) => tally_op,
        None => counting_alg.get_default_tally_operation_for_area(),
    }
}

#[instrument(skip_all)]
pub fn get_area_weight(ballot_styles: &[BallotStyle], area_id: &Uuid) -> Weight {
    let area_ballot_style: Option<&BallotStyle> = ballot_styles
        .iter()
        .find(|bs| bs.area_id == area_id.to_string());

    area_ballot_style
        .and_then(|bs| {
            bs.area_annotations
                .as_ref()
                .map(|area_annotations| area_annotations.get_weight())
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    use sequent_core::ballot::CandidatePresentation;
    use sequent_core::plaintext::{
        DecodedVoteChoice, InvalidPlaintextError, InvalidPlaintextErrorType,
    };
    use std::collections::HashMap;

    fn ballot_style_with(area_id: &Uuid, weight: Option<u64>) -> BallotStyle {
        BallotStyle {
            id: Uuid::new_v4().to_string(),
            tenant_id: Uuid::new_v4().to_string(),
            election_event_id: Uuid::new_v4().to_string(),
            election_id: Uuid::new_v4().to_string(),
            num_allowed_revotes: None,
            description: None,
            public_key: None,
            area_id: area_id.to_string(),
            area_presentation: None,
            contests: vec![],
            election_event_annotations: Default::default(),
            election_annotations: Default::default(),
            election_event_presentation: None,
            election_presentation: None,
            election_dates: None,
            area_annotations: weight.map(|weight| {
                serde_json::from_value(serde_json::json!({ "weight": weight }))
                    .expect("area annotations parse")
            }),
            multi_contest_encoding_mode: None,
        }
    }

    /// An area weight is applied whenever the published ballot style carries
    /// one, with no reference to the weighted voting policy. That is why
    /// `create_tally_ceremony` refuses to start a voters-weighted tally while
    /// any published ballot style still carries a weight: nothing downstream
    /// would stop the two being applied on top of each other.
    #[test]
    fn area_weight_is_applied_whenever_the_ballot_style_carries_one() {
        let area_id = Uuid::new_v4();
        let ballot_styles = vec![ballot_style_with(&area_id, Some(5))];
        assert_eq!(*get_area_weight(&ballot_styles, &area_id), Some(5));
    }

    /// An area with no ballot style, and one whose ballot style carries no
    /// annotations, both fall back to the neutral weight rather than zero.
    #[test]
    fn area_weight_falls_back_to_the_neutral_weight() {
        let area_id = Uuid::new_v4();
        assert_eq!(*get_area_weight(&[], &area_id), Some(1));

        let ballot_styles = vec![ballot_style_with(&area_id, None)];
        assert_eq!(*get_area_weight(&ballot_styles, &area_id), Some(1));
    }

    fn candidate(id: &str, is_explicit_blank: bool) -> Candidate {
        Candidate {
            id: id.to_string(),
            presentation: Some(CandidatePresentation {
                is_explicit_blank: Some(is_explicit_blank),
                ..CandidatePresentation::default()
            }),
            ..Candidate::default()
        }
    }

    fn contest() -> Contest {
        Contest {
            candidates: vec![candidate("normal", false), candidate("blank", true)],
            ..Contest::default()
        }
    }

    fn vote(normal_selected: bool, blank_selected: bool) -> DecodedVoteContest {
        DecodedVoteContest {
            contest_id: "contest".to_string(),
            is_explicit_invalid: false,
            is_decline_to_vote: false,
            is_blank_ballot: false,
            invalid_errors: vec![],
            invalid_alerts: vec![],
            choices: vec![
                DecodedVoteChoice {
                    id: "normal".to_string(),
                    selected: if normal_selected { 0 } else { -1 },
                    write_in_text: None,
                },
                DecodedVoteChoice {
                    id: "blank".to_string(),
                    selected: if blank_selected { 0 } else { -1 },
                    write_in_text: None,
                },
            ],
        }
    }

    fn implicit_invalid_error() -> InvalidPlaintextError {
        InvalidPlaintextError {
            error_type: InvalidPlaintextErrorType::Implicit,
            candidate_id: None,
            message: None,
            message_map: HashMap::new(),
        }
    }

    #[test]
    fn classifies_explicit_invalid() {
        let contest = contest();
        let explicit_blank_candidate_ids = get_explicit_blank_candidate_ids(&contest);
        let mut explicit_invalid = vote(false, false);
        explicit_invalid.is_explicit_invalid = true;

        assert_eq!(
            classify_ballot(&explicit_invalid, &explicit_blank_candidate_ids),
            BallotClass::ExplicitInvalid
        );
    }

    #[test]
    fn classifies_implicit_invalid_from_errors() {
        let contest = contest();
        let explicit_blank_candidate_ids = get_explicit_blank_candidate_ids(&contest);
        let mut implicit_invalid = vote(false, false);
        implicit_invalid.invalid_errors = vec![implicit_invalid_error()];

        assert_eq!(
            classify_ballot(&implicit_invalid, &explicit_blank_candidate_ids),
            BallotClass::ImplicitInvalid
        );
    }

    #[test]
    fn classifies_declined_blank_ballot() {
        let contest = contest();
        let explicit_blank_candidate_ids = get_explicit_blank_candidate_ids(&contest);
        let mut declined = vote(false, false);
        declined.is_decline_to_vote = true;

        assert_eq!(
            classify_ballot(&declined, &explicit_blank_candidate_ids),
            BallotClass::Declined
        );
    }

    #[test]
    fn classifies_declined_non_blank_ballot_as_implicit_invalid() {
        let contest = contest();
        let explicit_blank_candidate_ids = get_explicit_blank_candidate_ids(&contest);
        let mut declined = vote(true, false);
        declined.is_decline_to_vote = true;

        assert_eq!(
            classify_ballot(&declined, &explicit_blank_candidate_ids),
            BallotClass::ImplicitInvalid
        );
    }

    #[test]
    fn classifies_declined_before_invalid() {
        // Decline-to-vote takes precedence over the invalid checks: a
        // declined ballot carrying an explicit invalid marker is not blank,
        // so it classifies as implicit invalid (not explicit invalid).
        let contest = contest();
        let explicit_blank_candidate_ids = get_explicit_blank_candidate_ids(&contest);
        let mut declined = vote(false, false);
        declined.is_decline_to_vote = true;
        declined.is_explicit_invalid = true;

        assert_eq!(
            classify_ballot(&declined, &explicit_blank_candidate_ids),
            BallotClass::ImplicitInvalid
        );
    }

    #[test]
    fn classifies_mixed_explicit_blank_as_implicit_invalid() {
        let contest = contest();
        let explicit_blank_candidate_ids = get_explicit_blank_candidate_ids(&contest);

        assert_eq!(
            classify_ballot(&vote(true, true), &explicit_blank_candidate_ids),
            BallotClass::ImplicitInvalid
        );
    }

    #[test]
    fn classifies_explicit_blank() {
        let contest = contest();
        let explicit_blank_candidate_ids = get_explicit_blank_candidate_ids(&contest);

        assert_eq!(
            classify_ballot(&vote(false, true), &explicit_blank_candidate_ids),
            BallotClass::ExplicitBlank
        );
    }

    #[test]
    fn classifies_implicit_blank() {
        let contest = contest();
        let explicit_blank_candidate_ids = get_explicit_blank_candidate_ids(&contest);

        assert_eq!(
            classify_ballot(&vote(false, false), &explicit_blank_candidate_ids),
            BallotClass::ImplicitBlank
        );
    }

    #[test]
    fn classifies_regular_valid_vote() {
        let contest = contest();
        let explicit_blank_candidate_ids = get_explicit_blank_candidate_ids(&contest);

        assert_eq!(
            classify_ballot(&vote(true, false), &explicit_blank_candidate_ids),
            BallotClass::Valid
        );
    }

    #[test]
    fn detects_explicit_blank_mixed_with_normal_selection() {
        let contest = contest();
        let explicit_blank_candidate_ids = get_explicit_blank_candidate_ids(&contest);

        let explicit_blank_only = vote(false, true);
        assert!(is_explicit_blank_vote(
            &explicit_blank_only,
            &explicit_blank_candidate_ids
        ));
        assert!(!has_selected_non_explicit_blank_choice(
            &explicit_blank_only,
            &explicit_blank_candidate_ids
        ));

        let normal_only = vote(true, false);
        assert!(!is_explicit_blank_vote(
            &normal_only,
            &explicit_blank_candidate_ids
        ));
        assert!(has_selected_non_explicit_blank_choice(
            &normal_only,
            &explicit_blank_candidate_ids
        ));

        let mixed = vote(true, true);
        assert!(is_explicit_blank_vote(
            &mixed,
            &explicit_blank_candidate_ids
        ));
        assert!(has_selected_non_explicit_blank_choice(
            &mixed,
            &explicit_blank_candidate_ids
        ));
    }
}
