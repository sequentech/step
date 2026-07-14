// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::{
    BallotClassification, Engagement, EvaluationContext, EvaluationPhase,
    FindingCode, FindingParameters, FindingSeverity, InteractionGate,
    PolicyError, PolicyOutcome, PresentedFinding,
};
use crate::ballot::{Contest, InvalidVotePolicy};
use crate::ballot_codec::checker::check_min_vote_policy;
use crate::plaintext::DecodedVoteContest;

const MINIMUM_SELECTIONS_MESSAGE: &str = "errors.implicit.selectedMin";

pub(crate) fn evaluate_contest(
    contest: &Contest,
    vote: &DecodedVoteContest,
    context: EvaluationContext,
) -> Result<PolicyOutcome, PolicyError> {
    if contest.id != vote.contest_id {
        return Err(PolicyError::ContestMismatch {
            expected: contest.id.clone(),
            actual: vote.contest_id.clone(),
        });
    }

    let minimum = usize::try_from(contest.min_votes).map_err(|_| {
        PolicyError::InvalidMinimum {
            value: contest.min_votes,
        }
    })?;

    if vote.is_decline_to_vote {
        return Ok(PolicyOutcome {
            classification: BallotClassification::Declined,
            gate: InteractionGate::Open,
            findings: vec![],
            effects: vec![],
        });
    }

    let selected = selected_count(contest, vote);
    let result = check_min_vote_policy(selected, minimum);

    if !result.invalid_alerts.is_empty()
        || result.invalid_errors.len() > 1
        || result.invalid_errors.iter().any(|error| {
            error.message.as_deref() != Some(MINIMUM_SELECTIONS_MESSAGE)
        })
    {
        return Err(PolicyError::UnexpectedLegacyResult);
    }

    let minimum_not_met = !result.invalid_errors.is_empty();
    let classification = if vote.is_explicit_invalid {
        BallotClassification::ExplicitlyInvalid
    } else if minimum_not_met {
        BallotClassification::ImplicitlyInvalid
    } else {
        BallotClassification::Countable
    };

    let invalid_vote_policy = contest.get_invalid_vote_policy();
    let gate = if minimum_not_met || vote.is_explicit_invalid {
        interaction_gate(invalid_vote_policy, context.phase)
    } else {
        InteractionGate::Open
    };

    let show_finding = minimum_not_met
        && !(context.phase == EvaluationPhase::InteractiveSelection
            && context.engagement == Engagement::Untouched);
    let findings = if show_finding {
        vec![PresentedFinding {
            code: FindingCode::MinimumSelectionsNotMet,
            severity: FindingSeverity::Error,
            parameters: FindingParameters::MinimumSelections {
                selected,
                minimum,
            },
        }]
    } else {
        vec![]
    };

    Ok(PolicyOutcome {
        classification,
        gate,
        findings,
        effects: vec![],
    })
}

fn selected_count(contest: &Contest, vote: &DecodedVoteContest) -> usize {
    let selected = vote
        .choices
        .iter()
        .filter(|choice| choice.is_selected())
        .count();
    let selected_explicit_invalid_marker = vote.choices.iter().any(|choice| {
        choice.is_selected()
            && contest.candidates.iter().any(|candidate| {
                candidate.id == choice.id && candidate.is_explicit_invalid()
            })
    });

    selected
        + usize::from(
            vote.is_explicit_invalid && !selected_explicit_invalid_marker,
        )
}

fn interaction_gate(
    invalid_vote_policy: InvalidVotePolicy,
    phase: EvaluationPhase,
) -> InteractionGate {
    if matches!(
        phase,
        EvaluationPhase::InteractiveSelection
            | EvaluationPhase::PostDecryption
            | EvaluationPhase::Audit
    ) {
        return InteractionGate::Open;
    }

    match invalid_vote_policy {
        InvalidVotePolicy::ALLOWED => InteractionGate::Open,
        InvalidVotePolicy::WARN
        | InvalidVotePolicy::WARN_INVALID_IMPLICIT_AND_EXPLICIT => {
            InteractionGate::AcknowledgementRequired
        }
        InvalidVotePolicy::NOT_ALLOWED => InteractionGate::Closed,
    }
}
