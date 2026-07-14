// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    JsonSchema,
)]
pub enum PolicyDefinition {
    #[default]
    LegacyV1,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema,
)]
pub struct EvaluationContext {
    pub phase: EvaluationPhase,
    pub engagement: Engagement,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema,
)]
pub enum EvaluationPhase {
    InteractiveSelection,
    ContestConfirmation,
    BallotReview,
    PreCast,
    PostDecryption,
    Audit,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema,
)]
pub enum Engagement {
    Untouched,
    Touched,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PolicyOutcome {
    pub classification: BallotClassification,
    pub gate: InteractionGate,
    pub findings: Vec<PresentedFinding>,
    pub effects: Vec<InteractionEffect>,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema,
)]
pub enum BallotClassification {
    Countable,
    ImplicitlyInvalid,
    ExplicitlyInvalid,
    Declined,
    ConfigurationInvalid,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema,
)]
pub enum InteractionGate {
    Open,
    AcknowledgementRequired,
    Closed,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema,
)]
pub enum InteractionEffect {
    SelectionLimitReached,
    NoAdditionalSelections,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PresentedFinding {
    pub code: FindingCode,
    pub severity: FindingSeverity,
    pub parameters: FindingParameters,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema,
)]
pub enum FindingCode {
    MinimumSelectionsNotMet,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema,
)]
pub enum FindingSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum FindingParameters {
    MinimumSelections { selected: usize, minimum: usize },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyError {
    ContestMismatch { expected: String, actual: String },
    InvalidMinimum { value: i64 },
    UnexpectedLegacyResult,
}

impl Display for PolicyError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ContestMismatch { expected, actual } => write!(
                formatter,
                "contest mismatch: expected {expected}, got {actual}"
            ),
            Self::InvalidMinimum { value } => {
                write!(formatter, "invalid minimum selection count: {value}")
            }
            Self::UnexpectedLegacyResult => {
                write!(formatter, "unexpected legacy min-vote result")
            }
        }
    }
}

impl Error for PolicyError {}
