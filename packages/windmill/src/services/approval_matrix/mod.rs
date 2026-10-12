// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The enrollment approval matrix: the ordered rules that decide whether an
//! enrollment is accepted, sent to manual review or rejected.

pub mod evaluate;
pub mod store;

use crate::types::application::{ApplicationRejectReason, ApplicationStatus};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use strum_macros::{Display, EnumString};

/// The version of the built-in matrix, used by election events that have
/// not saved one.
pub const BUILT_IN_VERSION: i32 = 1;

const FIELD_EMBASSY: &str = "embassy";
const FIELD_MIDDLE_NAME: &str = "middleName";
const FIELD_LAST_NAME: &str = "lastName";

/// How the applicant's identity was established before enrolling.
#[allow(non_camel_case_types)]
#[derive(Display, EnumString, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityMethod {
    VERIFIED,
    MANUAL_ENTRY,
}

/// The result of comparing one field with the registry voter.
#[derive(Display, EnumString, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldMatch {
    MATCHES,
    DIFFERS,
}

/// How many compared fields differ from the registry voter.
#[derive(Display, EnumString, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum DifferingFields {
    #[serde(rename = "none")]
    #[strum(serialize = "none")]
    None,
    #[serde(rename = "exactly_1")]
    #[strum(serialize = "exactly_1")]
    Exactly1,
    #[serde(rename = "at_most_1")]
    #[strum(serialize = "at_most_1")]
    AtMost1,
    #[serde(rename = "exactly_2")]
    #[strum(serialize = "exactly_2")]
    Exactly2,
    #[serde(rename = "at_most_2")]
    #[strum(serialize = "at_most_2")]
    AtMost2,
    #[serde(rename = "at_least_3")]
    #[strum(serialize = "at_least_3")]
    AtLeast3,
}

impl DifferingFields {
    pub fn includes(&self, differing: usize) -> bool {
        match self {
            DifferingFields::None => differing == 0,
            DifferingFields::Exactly1 => differing == 1,
            DifferingFields::AtMost1 => differing <= 1,
            DifferingFields::Exactly2 => differing == 2,
            DifferingFields::AtMost2 => differing <= 2,
            DifferingFields::AtLeast3 => differing >= 3,
        }
    }
}

/// What a rule requires of an enrollment. A condition left out accepts any
/// value.
#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RuleConditions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity: Option<IdentityMethod>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voter_found: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub already_enrolled: Option<bool>,
    /// The type of identity document presented.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valid_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub differing: Option<DifferingFields>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fields: BTreeMap<String, FieldMatch>,
}

impl RuleConditions {
    /// Whether the rule compares the enrollment with a registry voter.
    pub fn compares_fields(&self) -> bool {
        self.differing.is_some() || !self.fields.is_empty()
    }
}

/// What a rule decides, and the reason shown to the voter.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RuleOutcome {
    pub decision: ApplicationStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<ApplicationRejectReason>,
}

impl RuleOutcome {
    pub fn accepted() -> Self {
        RuleOutcome {
            decision: ApplicationStatus::ACCEPTED,
            reason: None,
        }
    }

    pub fn pending(reason: ApplicationRejectReason) -> Self {
        RuleOutcome {
            decision: ApplicationStatus::PENDING,
            reason: Some(reason),
        }
    }

    pub fn rejected(reason: ApplicationRejectReason) -> Self {
        RuleOutcome {
            decision: ApplicationStatus::REJECTED,
            reason: Some(reason),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ApprovalRule {
    pub when: RuleConditions,
    pub then: RuleOutcome,
}

/// The fields compared with the registry, the rules in the order they are
/// checked, and the decision when no rule applies.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ApprovalMatrix {
    pub compared_fields: Vec<String>,
    pub rules: Vec<ApprovalRule>,
    pub otherwise: RuleOutcome,
}

/// Why a matrix can't be saved.
#[allow(non_camel_case_types)]
#[derive(Display, EnumString, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatrixErrorCode {
    NO_COMPARED_FIELDS,
    DUPLICATE_COMPARED_FIELD,
    UNKNOWN_FIELD,
    ACCEPTS_MANUAL_ENTRY,
    ACCEPTS_ALREADY_ENROLLED,
    ACCEPTS_WITHOUT_VOTER,
    OTHERWISE_ACCEPTS,
    MISSING_REASON,
    UNEXPECTED_REASON,
}

/// A validation error. `rule` is the rule's position starting at 1, and is
/// empty for the last rule (Otherwise) and for the matrix itself.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct MatrixError {
    pub code: MatrixErrorCode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rule: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
}

impl std::fmt::Display for MatrixError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.rule {
            Some(rule) => write!(f, "rule {rule}: {}", self.code)?,
            None => write!(f, "{}", self.code)?,
        }
        match &self.field {
            Some(field) => write!(f, " ({field})"),
            None => Ok(()),
        }
    }
}

fn outcome_errors(outcome: &RuleOutcome, rule: Option<usize>) -> Option<MatrixError> {
    let code = match (&outcome.decision, &outcome.reason) {
        (ApplicationStatus::ACCEPTED, Some(_)) => MatrixErrorCode::UNEXPECTED_REASON,
        (ApplicationStatus::PENDING | ApplicationStatus::REJECTED, None) => {
            MatrixErrorCode::MISSING_REASON
        }
        _ => return None,
    };
    Some(MatrixError {
        code,
        rule,
        field: None,
    })
}

impl ApprovalMatrix {
    /// The matrix used when an election event has not saved one: the
    /// decisions enrollment made before matrices were configurable, plus
    /// manual review for identities entered manually.
    pub fn built_in(compared_fields: Vec<String>) -> Self {
        let rule = |when: RuleConditions, then: RuleOutcome| ApprovalRule { when, then };
        let fields = |fields: &[(&str, FieldMatch)]| -> BTreeMap<String, FieldMatch> {
            fields
                .iter()
                .map(|(name, result)| (name.to_string(), *result))
                .collect()
        };
        let mut rules = vec![
            rule(
                RuleConditions {
                    already_enrolled: Some(true),
                    differing: Some(DifferingFields::AtMost1),
                    ..Default::default()
                },
                RuleOutcome::rejected(ApplicationRejectReason::ALREADY_APPROVED),
            ),
            rule(
                RuleConditions {
                    identity: Some(IdentityMethod::MANUAL_ENTRY),
                    ..Default::default()
                },
                RuleOutcome::pending(ApplicationRejectReason::IDENTITY_NOT_VERIFIED),
            ),
            rule(
                RuleConditions {
                    differing: Some(DifferingFields::None),
                    ..Default::default()
                },
                RuleOutcome::accepted(),
            ),
            rule(
                RuleConditions {
                    differing: Some(DifferingFields::Exactly1),
                    fields: fields(&[(FIELD_EMBASSY, FieldMatch::DIFFERS)]),
                    ..Default::default()
                },
                RuleOutcome::accepted(),
            ),
            rule(
                RuleConditions {
                    differing: Some(DifferingFields::Exactly1),
                    fields: fields(&[(FIELD_EMBASSY, FieldMatch::MATCHES)]),
                    ..Default::default()
                },
                RuleOutcome::pending(ApplicationRejectReason::NO_VOTER),
            ),
            rule(
                RuleConditions {
                    differing: Some(DifferingFields::Exactly2),
                    fields: fields(&[(FIELD_EMBASSY, FieldMatch::DIFFERS)]),
                    ..Default::default()
                },
                RuleOutcome::pending(ApplicationRejectReason::NO_VOTER),
            ),
            rule(
                RuleConditions {
                    differing: Some(DifferingFields::Exactly2),
                    fields: fields(&[
                        (FIELD_MIDDLE_NAME, FieldMatch::DIFFERS),
                        (FIELD_LAST_NAME, FieldMatch::DIFFERS),
                    ]),
                    ..Default::default()
                },
                RuleOutcome::pending(ApplicationRejectReason::NO_VOTER),
            ),
        ];
        if !compared_fields.iter().any(|field| field == FIELD_EMBASSY) {
            // Without the post in the comparison, a single difference can't
            // be told apart from a post difference: it goes to manual review.
            rules.retain(|rule| {
                rule.then.decision != ApplicationStatus::ACCEPTED
                    || !rule.when.fields.contains_key(FIELD_EMBASSY)
            });
            for rule in rules.iter_mut() {
                if rule.when.differing == Some(DifferingFields::Exactly1) {
                    rule.when.fields.remove(FIELD_EMBASSY);
                }
            }
        }
        ApprovalMatrix {
            compared_fields,
            rules,
            otherwise: RuleOutcome::rejected(ApplicationRejectReason::NO_VOTER),
        }
    }

    /// The errors that keep the matrix from being saved, in rule order.
    pub fn validate(&self) -> Vec<MatrixError> {
        let mut errors = vec![];
        let matrix_error = |code| MatrixError {
            code,
            rule: None,
            field: None,
        };

        let mut compared = BTreeSet::new();
        for field in &self.compared_fields {
            if field.trim().is_empty() || field.trim() != field {
                errors.push(MatrixError {
                    field: Some(field.clone()),
                    ..matrix_error(MatrixErrorCode::UNKNOWN_FIELD)
                });
            } else if !compared.insert(field.as_str()) {
                errors.push(MatrixError {
                    field: Some(field.clone()),
                    ..matrix_error(MatrixErrorCode::DUPLICATE_COMPARED_FIELD)
                });
            }
        }
        if self.compared_fields.is_empty() {
            errors.push(matrix_error(MatrixErrorCode::NO_COMPARED_FIELDS));
        }

        for (index, rule) in self.rules.iter().enumerate() {
            let number = Some(index + 1);
            let rule_error = |code| MatrixError {
                code,
                rule: number,
                field: None,
            };
            for field in rule.when.fields.keys() {
                if !compared.contains(field.as_str()) {
                    errors.push(MatrixError {
                        field: Some(field.clone()),
                        ..rule_error(MatrixErrorCode::UNKNOWN_FIELD)
                    });
                }
            }
            if rule.then.decision == ApplicationStatus::ACCEPTED {
                if rule.when.identity == Some(IdentityMethod::MANUAL_ENTRY) {
                    errors.push(rule_error(MatrixErrorCode::ACCEPTS_MANUAL_ENTRY));
                }
                if rule.when.already_enrolled == Some(true) {
                    errors.push(rule_error(MatrixErrorCode::ACCEPTS_ALREADY_ENROLLED));
                }
                if rule.when.voter_found == Some(false) {
                    errors.push(rule_error(MatrixErrorCode::ACCEPTS_WITHOUT_VOTER));
                }
            }
            errors.extend(outcome_errors(&rule.then, number));
        }

        if self.otherwise.decision == ApplicationStatus::ACCEPTED {
            errors.push(matrix_error(MatrixErrorCode::OTHERWISE_ACCEPTS));
        } else {
            errors.extend(outcome_errors(&self.otherwise, None));
        }

        errors
    }
}

#[cfg(test)]
#[path = "model_tests.rs"]
mod model_tests;
