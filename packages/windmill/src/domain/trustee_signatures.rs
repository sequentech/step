// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Which trustees count for a key ceremony or a tally when the event's rule
//! makes each trustee sign their key step.

use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum TrusteeSignatures {
    /// The rule needs no signatures: every trustee who took the step counts.
    #[default]
    NotNeeded,
    /// Only trustees whose signed step was used count.
    Needed {
        /// Trustees with a used signed step, besides the one taken now.
        signed: HashSet<String>,
        /// The trustee whose signed step is being taken now.
        signing: Option<String>,
    },
}

impl TrusteeSignatures {
    /// Whether the trustee's step counts.
    pub fn counts(&self, trustee: &str) -> bool {
        match self {
            TrusteeSignatures::NotNeeded => true,
            TrusteeSignatures::Needed { signed, signing } => {
                signed.contains(trustee) || signing.as_deref() == Some(trustee)
            }
        }
    }

    /// Whether a trustee who took the step without a signature (before the
    /// rule needed one) may take it again, now signed.
    pub fn may_redo(&self, trustee: &str) -> bool {
        match self {
            TrusteeSignatures::NotNeeded => false,
            TrusteeSignatures::Needed { signed, .. } => !signed.contains(trustee),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn needed(signed: &[&str], signing: Option<&str>) -> TrusteeSignatures {
        TrusteeSignatures::Needed {
            signed: signed.iter().map(|name| name.to_string()).collect(),
            signing: signing.map(str::to_owned),
        }
    }

    #[test]
    fn without_the_rule_everyone_counts_and_nobody_redoes() {
        assert!(TrusteeSignatures::NotNeeded.counts("alice"));
        assert!(!TrusteeSignatures::NotNeeded.may_redo("alice"));
    }

    #[test]
    fn with_the_rule_only_signed_steps_count_and_unsigned_ones_are_redone() {
        let signatures = needed(&["alice"], Some("bob"));
        assert!(signatures.counts("alice"));
        assert!(signatures.counts("bob"));
        assert!(!signatures.counts("carol"));
        assert!(!signatures.may_redo("alice"));
        assert!(signatures.may_redo("bob"));
        assert!(signatures.may_redo("carol"));
    }
}
