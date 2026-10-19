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

#[cfg(test)]
mod tests {
    use super::*;

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
