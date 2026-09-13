// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! These policy values are embedded in hashed ballot styles. Pin their public
//! JSON names and Borsh discriminants so reordering an enum cannot silently
//! change the meaning or signature of an already published election.

use borsh::{BorshDeserialize, BorshSerialize};
use sequent_core::ballot::*;
use serde::{de::DeserializeOwned, Serialize};
use std::fmt::Debug;

fn assert_policy_wire<T>(value: T, name: &str, discriminant: u8)
where
    T: Serialize
        + DeserializeOwned
        + BorshSerialize
        + BorshDeserialize
        + PartialEq
        + Debug,
{
    let json = serde_json::Value::String(name.into());
    assert_eq!(serde_json::to_value(&value).unwrap(), json);
    assert_eq!(serde_json::from_value::<T>(json).unwrap(), value);
    assert_eq!(borsh::to_vec(&value).unwrap(), vec![discriminant]);
    assert_eq!(borsh::from_slice::<T>(&[discriminant]).unwrap(), value);
    assert!(
        borsh::from_slice::<T>(&[255]).is_err(),
        "unknown discriminants must not fall back to another policy"
    );
}

#[test]
fn encrypted_ballot_layout_policies_keep_their_published_wire_values() {
    assert_policy_wire(
        ContestEncryptionPolicy::MULTIPLE_CONTESTS,
        "multiple-contests",
        0,
    );
    assert_policy_wire(
        ContestEncryptionPolicy::SINGLE_CONTEST,
        "single-contest",
        1,
    );
    assert_policy_wire(MultiContestEncodingMode::LEGACY, "legacy", 0);
    assert_policy_wire(
        MultiContestEncodingMode::EXPANDED_CAPACITY,
        "expanded-capacity",
        1,
    );
    assert_policy_wire(DecodedBallotsInclusionPolicy::INCLUDED, "included", 0);
    assert_policy_wire(
        DecodedBallotsInclusionPolicy::NOT_INCLUDED,
        "not-included",
        1,
    );
    assert_policy_wire(DeclineToVotePolicy::DISABLED, "disabled", 0);
    assert_policy_wire(DeclineToVotePolicy::ENABLED, "enabled", 1);
    assert_policy_wire(BlankBallotsPolicy::DISABLED, "disabled", 0);
    assert_policy_wire(BlankBallotsPolicy::ENABLED, "enabled", 1);
}

#[test]
fn voting_weight_and_tie_policies_keep_distinct_wire_values() {
    assert_policy_wire(
        WeightedVotingPolicy::DISABLED_WEIGHTED_VOTING,
        "disabled-weighted-voting",
        0,
    );
    assert_policy_wire(
        WeightedVotingPolicy::AREAS_WEIGHTED_VOTING,
        "areas-weighted-voting",
        1,
    );
    assert_policy_wire(
        WeightedVotingPolicy::VOTERS_WEIGHTED_VOTING,
        "voters-weighted-voting",
        2,
    );
    assert_policy_wire(DelegatedVotingPolicy::DISABLED, "disabled", 0);
    assert_policy_wire(DelegatedVotingPolicy::ENABLED, "enabled", 1);
    assert_policy_wire(TieBreakingPolicy::RANDOM, "random", 0);
    assert_policy_wire(
        TieBreakingPolicy::EXTERNAL_PROCEDURE,
        "external-procedure",
        1,
    );
}

#[test]
fn language_and_voting_state_wire_values_do_not_change_between_surfaces() {
    assert_policy_wire(
        LanguageDetectionPolicy::BROWSER_DETECT,
        "browser-detect",
        0,
    );
    assert_policy_wire(
        LanguageDetectionPolicy::FORCE_DEFAULT,
        "force-default",
        1,
    );
    assert_policy_wire(VotingStatus::NOT_STARTED, "NOT_STARTED", 0);
    assert_policy_wire(VotingStatus::OPEN, "OPEN", 1);
    assert_policy_wire(VotingStatus::PAUSED, "PAUSED", 2);
    assert_policy_wire(VotingStatus::CLOSED, "CLOSED", 3);
    assert_policy_wire(VotingStatusChannel::ONLINE, "ONLINE", 0);
    assert_policy_wire(VotingStatusChannel::KIOSK, "KIOSK", 1);
    assert_policy_wire(VotingStatusChannel::EARLY_VOTING, "EARLY_VOTING", 2);
    assert_policy_wire(VotingStatusChannel::TELEPHONE, "TELEPHONE", 3);
}
