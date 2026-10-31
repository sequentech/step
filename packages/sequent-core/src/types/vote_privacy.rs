// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Backwards-compatible event privacy policies. Identity mode is immutable;
//! network retention is an independently configurable policy.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum BallotIdentity {
    #[default]
    VoterId,
    Pseudonym,
}

#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum NetworkDataPolicy {
    ForensicStore,
    #[default]
    DoNotStore,
}

/// Free-text justification recorded in the signed resolution audit entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(try_from = "String")]
pub struct PseudonymResolutionReason(String);

const MAX_RESOLUTION_REASON_BYTES: usize = 512;

impl PseudonymResolutionReason {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::str::FromStr for PseudonymResolutionReason {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.chars().any(char::is_control) {
            return Err("Reason must not contain control characters");
        }
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err("Enter a reason for resolving this pseudonym");
        }
        if trimmed.len() > MAX_RESOLUTION_REASON_BYTES {
            return Err("Reason must not exceed 512 UTF-8 bytes");
        }
        Ok(Self(trimmed.to_owned()))
    }
}

impl TryFrom<String> for PseudonymResolutionReason {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolution_reasons_preserve_free_text() {
        for text in [
            "Election protest EP-2028-014, order of 19 May 2028",
            "Court order 28/2028",
            "Revisión electoral – expediente 28",
            "investigation",
            "compliance-review",
            "compliance review",
            "test-validation",
        ] {
            let reason = format!("  {text}  ")
                .parse::<PseudonymResolutionReason>()
                .unwrap();
            assert_eq!(reason.as_str(), text);
            assert_eq!(serde_json::to_value(&reason).unwrap(), text);
            let decoded: PseudonymResolutionReason =
                serde_json::from_value(serde_json::json!(text)).unwrap();
            assert_eq!(decoded.as_str(), text);
        }
    }

    #[test]
    fn resolution_reasons_require_bounded_text_without_control_characters() {
        for text in ["x".repeat(512), "é".repeat(256)] {
            assert_eq!(
                text.parse::<PseudonymResolutionReason>().unwrap().as_str(),
                text
            );
        }
        for rejected in [
            "".to_owned(),
            "   ".to_owned(),
            "first\nsecond".to_owned(),
            "first\rsecond".to_owned(),
            "first\tsecond".to_owned(),
            "reason\0".to_owned(),
            "reason\u{007f}".to_owned(),
            "x".repeat(513),
            "é".repeat(257),
        ] {
            assert!(rejected.parse::<PseudonymResolutionReason>().is_err());
            assert!(serde_json::from_value::<PseudonymResolutionReason>(
                serde_json::json!(rejected)
            )
            .is_err());
        }
    }

    #[test]
    fn policy_values_are_explicit_and_unknown_values_are_rejected() {
        assert_eq!(
            serde_json::to_string(&BallotIdentity::Pseudonym).unwrap(),
            "\"pseudonym\""
        );
        assert_eq!(
            serde_json::to_string(&NetworkDataPolicy::DoNotStore).unwrap(),
            "\"do-not-store\""
        );
        assert!(serde_json::from_str::<NetworkDataPolicy>("true").is_err());
        assert!(serde_json::from_str::<BallotIdentity>("\"unknown\"").is_err());
        assert_eq!(BallotIdentity::default(), BallotIdentity::VoterId);
        assert_eq!(NetworkDataPolicy::default(), NetworkDataPolicy::DoNotStore);
    }
}
