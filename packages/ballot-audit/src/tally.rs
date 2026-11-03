// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Rules that map a saved tally to audit scopes. export-ballot-audit checks a
//! supplied scope with them, and audit-tally derives scopes with them.

use crate::{incomplete, invalid_input, MultiplicityPolicy};
use anyhow::{ensure, Context, Result};
use sequent_core::{
    ballot::{ContestEncryptionPolicy, DelegatedVotingPolicy, WeightedVotingPolicy},
    types::{hasura::core::TallySessionConfiguration, keycloak::VOTE_WEIGHT_BATCHES},
};
use serde_json::Value;

/// The multiplicity policy and contest encryption policy of a saved tally
/// session configuration. A null configuration uses the platform defaults.
pub fn tally_policies(
    configuration: &Value,
) -> Result<(MultiplicityPolicy, ContestEncryptionPolicy)> {
    let configuration: Option<TallySessionConfiguration> =
        serde_json::from_value(configuration.clone())
            .map_err(|_| invalid_input("unsupported saved tally configuration"))?;
    let configuration = configuration.unwrap_or_default();
    let weighted =
        configuration.get_weighted_voting_policy() == WeightedVotingPolicy::VOTERS_WEIGHTED_VOTING;
    let delegated = configuration.get_delegated_voting_policy() == DelegatedVotingPolicy::ENABLED;
    let policy = match (weighted, delegated) {
        (true, true) => {
            return Err(invalid_input(
                "combined weighted and delegated voting is unsupported",
            ))
        }
        (true, false) => MultiplicityPolicy::Weighted,
        (false, true) => MultiplicityPolicy::Delegated,
        (false, false) => MultiplicityPolicy::Unweighted,
    };
    Ok((policy, configuration.get_contest_encryption_policy()))
}

/// The ballot batches of one saved tally contest row, in ascending order.
/// Weighted tallies use one batch per bit of the saved weight bit mask.
pub fn tally_batches(
    policy: MultiplicityPolicy,
    base_batch: u32,
    annotations: &Value,
) -> Result<Vec<u32>> {
    if policy != MultiplicityPolicy::Weighted {
        return Ok(vec![base_batch]);
    }
    let mask = annotations["weight_bit_mask"]
        .as_u64()
        .ok_or_else(|| incomplete("weighted saved mapping lacks a weight bit mask"))?;
    ensure!(
        mask > 0 && mask < (1u64 << VOTE_WEIGHT_BATCHES),
        invalid_input("invalid saved weight bit mask")
    );
    (0..VOTE_WEIGHT_BATCHES)
        .filter(|bit| mask & (1u64 << bit) != 0)
        .map(|bit| base_batch.checked_add(bit).context("saved batch overflow"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ExitStatus;
    use serde_json::json;

    #[test]
    fn policies_follow_the_saved_configuration() {
        assert_eq!(
            tally_policies(&Value::Null).unwrap(),
            (
                MultiplicityPolicy::Unweighted,
                ContestEncryptionPolicy::SINGLE_CONTEST
            )
        );
        assert_eq!(
            tally_policies(&json!({"contest_encryption_policy": "multiple-contests",
                "weighted_voting_policy": "voters-weighted-voting"}))
            .unwrap(),
            (
                MultiplicityPolicy::Weighted,
                ContestEncryptionPolicy::MULTIPLE_CONTESTS
            )
        );
        assert_eq!(
            tally_policies(&json!({"delegated_voting_policy": "enabled"}))
                .unwrap()
                .0,
            MultiplicityPolicy::Delegated
        );
        let combined = json!({"weighted_voting_policy": "voters-weighted-voting",
            "delegated_voting_policy": "enabled"});
        let error = tally_policies(&combined).unwrap_err();
        assert_eq!(ExitStatus::of(&error), ExitStatus::InvalidInput);
        let error = tally_policies(&json!("not an object")).unwrap_err();
        assert_eq!(ExitStatus::of(&error), ExitStatus::InvalidInput);
    }

    #[test]
    fn batches_follow_the_weight_bit_mask() {
        assert_eq!(
            tally_batches(MultiplicityPolicy::Unweighted, 0, &json!({})).unwrap(),
            vec![0]
        );
        assert_eq!(
            tally_batches(MultiplicityPolicy::Delegated, 7, &Value::Null).unwrap(),
            vec![7]
        );
        assert_eq!(
            tally_batches(
                MultiplicityPolicy::Weighted,
                10,
                &json!({"weight_bit_mask": 5})
            )
            .unwrap(),
            vec![10, 12]
        );
        let error = tally_batches(MultiplicityPolicy::Weighted, 0, &json!({})).unwrap_err();
        assert_eq!(ExitStatus::of(&error), ExitStatus::Incomplete);
        let error = tally_batches(
            MultiplicityPolicy::Weighted,
            0,
            &json!({"weight_bit_mask": 0}),
        )
        .unwrap_err();
        assert_eq!(ExitStatus::of(&error), ExitStatus::InvalidInput);
        assert!(tally_batches(
            MultiplicityPolicy::Weighted,
            u32::MAX,
            &json!({"weight_bit_mask": 2})
        )
        .is_err());
    }
}
