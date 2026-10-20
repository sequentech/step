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

/// Closed public purposes approved for newly requested pseudonym resolutions.
/// Historical signed reasons remain strings and are never rewritten.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum PseudonymResolutionReason {
    Investigation,
    ComplianceReview,
    TestValidation,
}

impl PseudonymResolutionReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Investigation => "investigation",
            Self::ComplianceReview => "compliance-review",
            Self::TestValidation => "test-validation",
        }
    }
}

impl std::str::FromStr for PseudonymResolutionReason {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        const INVALID: &str =
            "Choose investigation, compliance-review, or test-validation";
        if value.chars().any(char::is_control) {
            return Err(INVALID);
        }
        match value.trim() {
            "investigation" => Ok(Self::Investigation),
            "compliance-review" | "compliance review" => {
                Ok(Self::ComplianceReview)
            }
            "test-validation" | "test validation" => Ok(Self::TestValidation),
            _ => Err(INVALID),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolution_categories_have_only_the_approved_public_values() {
        for (input, canonical) in [
            ("investigation", "investigation"),
            (" compliance review ", "compliance-review"),
            ("compliance-review", "compliance-review"),
            ("test validation", "test-validation"),
            ("test-validation", "test-validation"),
        ] {
            let reason = input.parse::<PseudonymResolutionReason>().unwrap();
            assert_eq!(reason.as_str(), canonical);
            assert_eq!(serde_json::to_value(reason).unwrap(), canonical);
        }
        for rejected in [
            "",
            "Jane Doe",
            "jane@example.test",
            "11111111-1111-4111-8111-111111111111",
            "AAAAAAAA-AAAA-4AAA-8AAA-AAAAAAAAAAAA",
            "198.51.100.7",
            "AQ",
            "Mozilla/5.0",
            "investigation\n",
            "other",
            "Election protest",
        ] {
            assert!(rejected.parse::<PseudonymResolutionReason>().is_err());
        }
        assert!(
            serde_json::from_str::<PseudonymResolutionReason>("\"other\"")
                .is_err()
        );
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
