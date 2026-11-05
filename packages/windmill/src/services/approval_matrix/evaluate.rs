// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Deciding an enrollment with an approval matrix. The same functions decide
//! real enrollments and the dry runs of the matrix editor's test panel.

use super::{
    ApprovalMatrix, FieldMatch, IdentityMethod, RuleConditions, RuleOutcome, BUILT_IN_VERSION,
};
use crate::types::application::{ApplicationRejectReason, ApplicationStatus};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use strum_macros::{Display, EnumString};

/// Where the matrix that decided an enrollment came from.
#[allow(non_camel_case_types)]
#[derive(Display, EnumString, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatrixSource {
    BUILT_IN,
    SAVED,
}

/// A matrix and the version that identifies it in decision records.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct MatrixVersion {
    pub version: i32,
    pub source: MatrixSource,
    pub matrix: ApprovalMatrix,
}

impl MatrixVersion {
    pub fn built_in(compared_fields: Vec<String>) -> Self {
        MatrixVersion {
            version: BUILT_IN_VERSION,
            source: MatrixSource::BUILT_IN,
            matrix: ApprovalMatrix::built_in(compared_fields),
        }
    }
}

/// What the rules see of an enrollment compared with one registry voter, or
/// with none when the registry has no candidate.
#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RuleInputs {
    /// Empty when the enrollment flow doesn't report how the identity was
    /// established.
    #[serde(default)]
    pub identity: Option<IdentityMethod>,
    pub voter_found: bool,
    #[serde(default)]
    pub already_enrolled: bool,
    #[serde(default)]
    pub valid_id: Option<String>,
    /// A compared field that is missing here is not compared for the
    /// identity document: it counts as differing in field conditions, but
    /// not in `differing`.
    #[serde(default)]
    pub fields: BTreeMap<String, FieldMatch>,
    #[serde(default)]
    pub differing: usize,
}

impl RuleInputs {
    /// Counts the differing fields from the per-field results.
    pub fn with_counted_differences(mut self) -> Self {
        self.differing = self
            .fields
            .values()
            .filter(|result| **result == FieldMatch::DIFFERS)
            .count();
        self
    }
}

/// A rule the evaluator overrides whatever the matrix says.
#[allow(non_camel_case_types)]
#[derive(Display, EnumString, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Invariant {
    /// An identity entered manually is never accepted automatically.
    MANUAL_ENTRY_NOT_ACCEPTED,
    /// A voter who is already enrolled is never accepted again.
    ALREADY_ENROLLED_NOT_ACCEPTED,
    /// Nobody is accepted without a registry voter.
    NO_VOTER_NOT_ACCEPTED,
    /// The last rule can't accept.
    OTHERWISE_NOT_ACCEPTED,
}

/// The rule that decided for one set of inputs.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct RuleDecision {
    /// The rule's position starting at 1; empty for the last rule
    /// (Otherwise).
    pub rule: Option<usize>,
    pub conditions: Option<RuleConditions>,
    pub outcome: RuleOutcome,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invariant: Option<Invariant>,
}

fn applies(conditions: &RuleConditions, inputs: &RuleInputs) -> bool {
    if let Some(identity) = conditions.identity {
        if inputs.identity != Some(identity) {
            return false;
        }
    }
    if let Some(voter_found) = conditions.voter_found {
        if inputs.voter_found != voter_found {
            return false;
        }
    }
    if let Some(already_enrolled) = conditions.already_enrolled {
        if inputs.already_enrolled != already_enrolled {
            return false;
        }
    }
    if let Some(valid_id) = &conditions.valid_id {
        if inputs.valid_id.as_ref() != Some(valid_id) {
            return false;
        }
    }
    if conditions.compares_fields() && !inputs.voter_found {
        return false;
    }
    if let Some(differing) = conditions.differing {
        if !differing.includes(inputs.differing) {
            return false;
        }
    }
    conditions.fields.iter().all(|(field, expected)| {
        inputs.fields.get(field).unwrap_or(&FieldMatch::DIFFERS) == expected
    })
}

fn enforce_invariants(
    rule: Option<usize>,
    outcome: RuleOutcome,
    inputs: &RuleInputs,
) -> (RuleOutcome, Option<Invariant>) {
    if outcome.decision != ApplicationStatus::ACCEPTED {
        return (outcome, None);
    }
    if !inputs.voter_found {
        return (
            RuleOutcome::rejected(ApplicationRejectReason::NO_VOTER),
            Some(Invariant::NO_VOTER_NOT_ACCEPTED),
        );
    }
    if inputs.already_enrolled {
        return (
            RuleOutcome::rejected(ApplicationRejectReason::ALREADY_APPROVED),
            Some(Invariant::ALREADY_ENROLLED_NOT_ACCEPTED),
        );
    }
    if inputs.identity == Some(IdentityMethod::MANUAL_ENTRY) {
        return (
            RuleOutcome::pending(ApplicationRejectReason::IDENTITY_NOT_VERIFIED),
            Some(Invariant::MANUAL_ENTRY_NOT_ACCEPTED),
        );
    }
    if rule.is_none() {
        return (
            RuleOutcome::pending(ApplicationRejectReason::OTHER),
            Some(Invariant::OTHERWISE_NOT_ACCEPTED),
        );
    }
    (outcome, None)
}

/// The first rule that applies to the inputs decides; the last rule
/// (Otherwise) decides when none does.
pub fn decide(matrix: &ApprovalMatrix, inputs: &RuleInputs) -> RuleDecision {
    let found = matrix
        .rules
        .iter()
        .enumerate()
        .find(|(_, rule)| applies(&rule.when, inputs));
    let (rule, conditions, outcome) = match found {
        Some((index, rule)) => (Some(index + 1), Some(rule.when.clone()), rule.then.clone()),
        None => (None, None, matrix.otherwise.clone()),
    };
    let (outcome, invariant) = enforce_invariants(rule, outcome, inputs);
    RuleDecision {
        rule,
        conditions,
        outcome,
        invariant,
    }
}

/// A registry voter the lookup returned, compared with the enrollment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub user_id: Option<String>,
    pub username: Option<String>,
    pub inputs: RuleInputs,
}

/// The matrix, rule and inputs that decided an application, stored with it.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct DecisionRecord {
    pub matrix_version: i32,
    pub matrix_source: MatrixSource,
    pub rule: Option<usize>,
    pub conditions: Option<RuleConditions>,
    pub decision: ApplicationStatus,
    pub reason: Option<ApplicationRejectReason>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invariant: Option<Invariant>,
    pub inputs: RuleInputs,
    /// How many registry voters were compared, and how many of them the
    /// rules accepted.
    pub candidates: usize,
    pub accepted_candidates: usize,
}

/// The decision for an enrollment and the registry voter it refers to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Evaluation {
    /// The registry voter the recorded inputs describe.
    pub compared: Option<Candidate>,
    /// The voter to enroll when accepted, or the already enrolled voter when
    /// rejected for that reason.
    pub voter: Option<Candidate>,
    pub record: DecisionRecord,
}

/// Decides an enrollment from the registry voters the lookup returned. The
/// result doesn't depend on the order of `candidates`.
///
/// Exactly one accepted voter accepts the enrollment for that voter; several
/// send it to manual review; otherwise any voter in manual review does so;
/// otherwise it is rejected, preferring a voter who is already enrolled.
pub fn evaluate(
    matrix: &MatrixVersion,
    identity: Option<IdentityMethod>,
    valid_id: Option<String>,
    mut candidates: Vec<Candidate>,
) -> Evaluation {
    let record = |decision: RuleDecision, inputs: RuleInputs, candidates, accepted_candidates| {
        DecisionRecord {
            matrix_version: matrix.version,
            matrix_source: matrix.source,
            rule: decision.rule,
            conditions: decision.conditions,
            decision: decision.outcome.decision,
            reason: decision.outcome.reason,
            invariant: decision.invariant,
            inputs,
            candidates,
            accepted_candidates,
        }
    };

    if candidates.is_empty() {
        let inputs = RuleInputs {
            identity,
            valid_id,
            voter_found: false,
            ..Default::default()
        };
        let decision = decide(&matrix.matrix, &inputs);
        return Evaluation {
            compared: None,
            voter: None,
            record: record(decision, inputs, 0, 0),
        };
    }

    candidates.sort_by(|a, b| {
        (a.inputs.differing, &a.user_id, &a.username).cmp(&(
            b.inputs.differing,
            &b.user_id,
            &b.username,
        ))
    });
    let total = candidates.len();
    let mut decided: Vec<(Candidate, RuleDecision)> = candidates
        .into_iter()
        .map(|candidate| {
            let decision = decide(&matrix.matrix, &candidate.inputs);
            (candidate, decision)
        })
        .collect();
    let accepted = decided
        .iter()
        .filter(|(_, decision)| decision.outcome.decision == ApplicationStatus::ACCEPTED)
        .count();
    let position = |chosen: &dyn Fn(&RuleDecision) -> bool| {
        decided.iter().position(|(_, decision)| chosen(decision))
    };

    let (index, names_voter) = if let Some(index) =
        position(&|d| d.outcome.decision == ApplicationStatus::ACCEPTED)
    {
        (index, accepted == 1)
    } else if let Some(index) = position(&|d| d.outcome.decision == ApplicationStatus::PENDING) {
        (index, false)
    } else if let Some(index) =
        position(&|d| d.outcome.reason == Some(ApplicationRejectReason::ALREADY_APPROVED))
    {
        (index, true)
    } else {
        (0, false)
    };

    let (candidate, mut decision) = decided.swap_remove(index);
    if accepted > 1 {
        decision.outcome = RuleOutcome::pending(ApplicationRejectReason::OTHER);
    }
    Evaluation {
        voter: names_voter.then(|| candidate.clone()),
        record: record(decision, candidate.inputs.clone(), total, accepted),
        compared: Some(candidate),
    }
}

#[cfg(test)]
#[path = "evaluate_tests.rs"]
mod evaluate_tests;
