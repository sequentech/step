// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use strand::hash::{Hash, HashWrapper};

#[derive(
    BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, Debug,
)]
pub struct EventIdString(pub String);

#[derive(
    BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, Debug,
)]
pub struct ElectionsIdsString(pub Option<Vec<String>>);

#[derive(
    BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, Debug,
)]
pub struct ElectionIdString(pub Option<String>);
#[derive(
    BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, Debug,
)]
pub struct ErrorMessageString(pub String);

#[derive(
    BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, Debug,
)]
pub struct KeycloakEventTypeString(pub String);

#[derive(
    BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, Debug,
)]
pub struct ContestIdString(pub String);

#[derive(
    BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, Debug,
)]
pub struct TrusteeNameString(pub String);

#[derive(
    BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, Debug,
)]
pub struct BallotPublicationIdString(pub String);

#[derive(
    BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, Debug,
)]
pub struct CastVoteErrorString(pub String);

#[derive(
    BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, Debug,
)]
pub struct PseudonymHash(pub HashWrapper);
#[derive(
    BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, Debug,
)]
pub struct PublicKeyDerB64(pub String);

#[derive(
    BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, Debug,
)]
pub struct TenantIdString(pub String);

#[derive(
    BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, Debug,
)]
pub struct AdminUserIdString(pub String);

impl PseudonymHash {
    // Provide methods to work with HashWrapper as needed
    pub fn new(hash: Hash) -> Self {
        PseudonymHash(HashWrapper::new(hash))
    }
}

#[derive(
    BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, Debug,
)]
pub struct CastVoteHash(pub HashWrapper);

impl CastVoteHash {
    // Provide methods to work with HashWrapper as needed
    pub fn new(hash: Hash) -> Self {
        CastVoteHash(HashWrapper::new(hash))
    }
}

pub type Timestamp = u64;

#[derive(
    BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, Debug,
)]
pub struct VoterIpString(pub String);

#[derive(
    BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, Debug,
)]
pub struct VoterCountryString(pub String);

#[derive(
    BorshSerialize, BorshDeserialize, Deserialize, Serialize, Clone, PartialEq, Eq, Hash, Debug,
)]
pub struct VotingChannelString(pub String);
