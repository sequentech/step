// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

mod legacy;
mod types;

pub use types::*;

use crate::ballot::Contest;
use crate::plaintext::DecodedVoteContest;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct BallotPolicyEngine {
    definition: PolicyDefinition,
}

impl BallotPolicyEngine {
    pub const fn new(definition: PolicyDefinition) -> Self {
        Self { definition }
    }

    pub const fn definition(&self) -> PolicyDefinition {
        self.definition
    }

    pub fn evaluate_contest(
        &self,
        contest: &Contest,
        vote: &DecodedVoteContest,
        context: EvaluationContext,
    ) -> Result<PolicyOutcome, PolicyError> {
        match self.definition {
            PolicyDefinition::LegacyV1 => {
                legacy::evaluate_contest(contest, vote, context)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ballot::{Contest, ContestPresentation, InvalidVotePolicy};
    use crate::plaintext::{
        DecodedVoteChoice, DecodedVoteContest, InvalidPlaintextError,
        InvalidPlaintextErrorType,
    };
    use std::collections::HashMap;

    fn contest(
        minimum: i64,
        invalid_vote_policy: InvalidVotePolicy,
    ) -> Contest {
        Contest {
            id: "contest-1".to_string(),
            min_votes: minimum,
            presentation: Some(ContestPresentation {
                invalid_vote_policy: Some(invalid_vote_policy),
                ..ContestPresentation::default()
            }),
            ..Contest::default()
        }
    }

    fn vote(selections: &[i64]) -> DecodedVoteContest {
        DecodedVoteContest {
            contest_id: "contest-1".to_string(),
            is_explicit_invalid: false,
            is_decline_to_vote: false,
            invalid_errors: vec![],
            invalid_alerts: vec![],
            choices: selections
                .iter()
                .enumerate()
                .map(|(index, selected)| DecodedVoteChoice {
                    id: format!("candidate-{index}"),
                    selected: *selected,
                    write_in_text: None,
                })
                .collect(),
        }
    }

    const fn context(
        phase: EvaluationPhase,
        engagement: Engagement,
    ) -> EvaluationContext {
        EvaluationContext { phase, engagement }
    }

    #[test]
    fn returns_open_countable_outcome_when_minimum_is_met() {
        let outcome = BallotPolicyEngine::default()
            .evaluate_contest(
                &contest(2, InvalidVotePolicy::NOT_ALLOWED),
                &vote(&[0, 1]),
                context(
                    EvaluationPhase::ContestConfirmation,
                    Engagement::Touched,
                ),
            )
            .expect("minimum evaluation should succeed");

        assert_eq!(outcome.classification, BallotClassification::Countable);
        assert_eq!(outcome.gate, InteractionGate::Open);
        assert!(outcome.findings.is_empty());
        assert!(outcome.effects.is_empty());
    }

    #[test]
    fn returns_typed_finding_and_closed_gate_when_minimum_is_not_met() {
        let outcome = BallotPolicyEngine::default()
            .evaluate_contest(
                &contest(2, InvalidVotePolicy::NOT_ALLOWED),
                &vote(&[0, -1]),
                context(
                    EvaluationPhase::ContestConfirmation,
                    Engagement::Touched,
                ),
            )
            .expect("minimum evaluation should succeed");

        assert_eq!(
            outcome.classification,
            BallotClassification::ImplicitlyInvalid
        );
        assert_eq!(outcome.gate, InteractionGate::Closed);
        assert_eq!(
            outcome.findings,
            vec![PresentedFinding {
                code: FindingCode::MinimumSelectionsNotMet,
                severity: FindingSeverity::Error,
                parameters: FindingParameters::MinimumSelections {
                    selected: 1,
                    minimum: 2,
                },
            }]
        );
    }

    #[test]
    fn invalid_vote_policy_controls_progression_separately_from_classification()
    {
        let engine = BallotPolicyEngine::default();
        let vote = vote(&[0, -1]);
        let confirmation =
            context(EvaluationPhase::ContestConfirmation, Engagement::Touched);

        let allowed = engine
            .evaluate_contest(
                &contest(2, InvalidVotePolicy::ALLOWED),
                &vote,
                confirmation,
            )
            .expect("allowed policy evaluation should succeed");
        let warning = engine
            .evaluate_contest(
                &contest(2, InvalidVotePolicy::WARN),
                &vote,
                confirmation,
            )
            .expect("warning policy evaluation should succeed");

        assert_eq!(
            allowed.classification,
            BallotClassification::ImplicitlyInvalid
        );
        assert_eq!(allowed.gate, InteractionGate::Open);
        assert_eq!(
            warning.classification,
            BallotClassification::ImplicitlyInvalid
        );
        assert_eq!(warning.gate, InteractionGate::AcknowledgementRequired);
    }

    #[test]
    fn hides_premature_finding_while_contest_is_untouched() {
        let outcome = BallotPolicyEngine::default()
            .evaluate_contest(
                &contest(1, InvalidVotePolicy::NOT_ALLOWED),
                &vote(&[-1]),
                context(
                    EvaluationPhase::InteractiveSelection,
                    Engagement::Untouched,
                ),
            )
            .expect("minimum evaluation should succeed");

        assert_eq!(
            outcome.classification,
            BallotClassification::ImplicitlyInvalid
        );
        assert_eq!(outcome.gate, InteractionGate::Open);
        assert!(outcome.findings.is_empty());
    }

    #[test]
    fn ignores_findings_derived_by_the_caller() {
        let mut vote = vote(&[0]);
        vote.invalid_errors.push(InvalidPlaintextError {
            error_type: InvalidPlaintextErrorType::Implicit,
            candidate_id: None,
            message: Some("caller.injected".to_string()),
            message_map: HashMap::new(),
        });

        let outcome = BallotPolicyEngine::default()
            .evaluate_contest(
                &contest(1, InvalidVotePolicy::NOT_ALLOWED),
                &vote,
                context(EvaluationPhase::PreCast, Engagement::Touched),
            )
            .expect("minimum evaluation should succeed");

        assert_eq!(outcome.classification, BallotClassification::Countable);
        assert_eq!(outcome.gate, InteractionGate::Open);
        assert!(outcome.findings.is_empty());
    }

    #[test]
    fn rejects_invalid_minimum_configuration() {
        let error = BallotPolicyEngine::default()
            .evaluate_contest(
                &contest(-1, InvalidVotePolicy::NOT_ALLOWED),
                &vote(&[]),
                context(EvaluationPhase::PreCast, Engagement::Touched),
            )
            .expect_err("negative minimum must fail");

        assert_eq!(error, PolicyError::InvalidMinimum { value: -1 });
    }

    #[test]
    fn rejects_mismatched_contest_input() {
        let mut vote = vote(&[0]);
        vote.contest_id = "contest-2".to_string();

        let error = BallotPolicyEngine::default()
            .evaluate_contest(
                &contest(1, InvalidVotePolicy::NOT_ALLOWED),
                &vote,
                context(EvaluationPhase::PreCast, Engagement::Touched),
            )
            .expect_err("contest mismatch must fail");

        assert_eq!(
            error,
            PolicyError::ContestMismatch {
                expected: "contest-1".to_string(),
                actual: "contest-2".to_string(),
            }
        );
    }
}
