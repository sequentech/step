// SPDX-FileCopyrightText: 2025 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::ballot::{BallotStyle, TYPES_VERSION};
use crate::ballot_codec::PlaintextCodec;
use crate::encrypt::hash_ballot_style;
use crate::error::BallotError;
use crate::serialization::base64::{Base64Deserialize, Base64Serialize};
use crate::serialization::deserialize_with_path::deserialize_value;
use crate::types::hasura::core::{self, ElectionEvent};
use crate::types::scheduled_event::EventProcessors;
use borsh::{BorshDeserialize, BorshSerialize};
use chrono::DateTime;
use chrono::Utc;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_path_to_error::Error;
use std::hash::Hash;
use std::{collections::HashMap, default::Default};
use strand::elgamal::Ciphertext;
use strand::zkp::Schnorr;
use strand::{backend::ristretto::RistrettoCtx, context::Ctx};

#[derive(BorshSerialize, BorshDeserialize, PartialEq, Eq, Debug, Clone)]
pub struct ReplicationChoice<C: Ctx> {
    pub ciphertext: Ciphertext<C>,
    pub plaintext: C::P,
    pub randomness: C::X,
}

#[derive(BorshSerialize, BorshDeserialize, PartialEq, Eq, Debug, Clone)]
pub struct AuditableBallotContest<C: Ctx> {
    pub contest_id: String,
    pub choice: ReplicationChoice<C>,
    pub proof: Schnorr<C>,
}

/*
FIXME: why does this exist
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Eq, Debug, Clone)]
pub struct RawAuditableBallot<C: Ctx> {
    pub election_url: String,
    pub issue_date: String,
    pub contests: Vec<AuditableBallotContest<C>>,
    pub ballot_hash: String,
}*/

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct AuditableBallot {
    pub version: u32,
    pub issue_date: String,
    pub config: BallotStyle,
    pub contests: Vec<String>, // Vec<AuditableBallotContest<C>>,
    pub ballot_hash: String,
}

impl AuditableBallot {
    pub fn deserialize_contests<C: Ctx>(
        &self,
    ) -> Result<Vec<AuditableBallotContest<C>>, BallotError> {
        self.contests
            .clone()
            .into_iter()
            .map(|auditable_ballot_contest_serialized| {
                Base64Deserialize::deserialize(
                    auditable_ballot_contest_serialized.clone(),
                )
                .map_err(|err| BallotError::Serialization(err.to_string()))
            })
            .collect()
    }

    pub fn serialize_contests<C: Ctx>(
        contests: &Vec<AuditableBallotContest<C>>,
    ) -> Result<Vec<String>, BallotError> {
        contests
            .clone()
            .into_iter()
            .map(|auditable_ballot_contest| {
                Base64Serialize::serialize(&auditable_ballot_contest)
            })
            .collect::<Vec<Result<String, BallotError>>>()
            .into_iter()
            .collect()
    }
}

#[derive(BorshSerialize, BorshDeserialize, PartialEq, Eq, Debug, Clone)]
pub struct HashableBallotContest<C: Ctx> {
    pub contest_id: String,
    pub ciphertext: Ciphertext<C>,
    pub proof: Schnorr<C>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct HashableBallot {
    pub version: u32,
    pub issue_date: String,
    pub contests: Vec<String>, // Vec<HashableBallotContest<C>>,
    pub config: String,
    pub ballot_style_hash: String,
}

#[derive(BorshSerialize, BorshDeserialize, PartialEq, Eq, Debug, Clone)]
pub struct RawHashableBallot<C: Ctx> {
    pub version: u32,
    pub issue_date: String,
    pub contests: Vec<HashableBallotContest<C>>,
}

impl HashableBallot {
    pub fn deserialize_contests<C: Ctx>(
        &self,
    ) -> Result<Vec<HashableBallotContest<C>>, BallotError> {
        self.contests
            .clone()
            .into_iter()
            .map(|hashable_ballot_contest_serialized| {
                Base64Deserialize::deserialize(
                    hashable_ballot_contest_serialized.clone(),
                )
                .map_err(|err| BallotError::Serialization(err.to_string()))
            })
            .collect()
    }

    pub fn serialize_contests<C: Ctx>(
        contests: &Vec<HashableBallotContest<C>>,
    ) -> Result<Vec<String>, BallotError> {
        contests
            .clone()
            .into_iter()
            .map(|hashable_ballot_contest| {
                Base64Serialize::serialize(&hashable_ballot_contest)
            })
            .collect::<Vec<Result<String, BallotError>>>()
            .into_iter()
            .collect()
    }
}

impl<C: Ctx> TryFrom<&HashableBallot> for RawHashableBallot<C> {
    type Error = BallotError;

    fn try_from(value: &HashableBallot) -> Result<Self, Self::Error> {
        let contests = value.deserialize_contests::<C>()?;
        Ok(RawHashableBallot {
            version: value.version,
            issue_date: value.issue_date.clone(),
            contests: contests,
        })
    }
}

impl<C: Ctx> From<&AuditableBallotContest<C>> for HashableBallotContest<C> {
    fn from(value: &AuditableBallotContest<C>) -> HashableBallotContest<C> {
        HashableBallotContest {
            contest_id: value.contest_id.clone(),
            ciphertext: value.choice.ciphertext.clone(),
            proof: value.proof.clone(),
        }
    }
}

impl TryFrom<&AuditableBallot> for HashableBallot {
    type Error = BallotError;

    fn try_from(value: &AuditableBallot) -> Result<Self, Self::Error> {
        if TYPES_VERSION != value.version {
            return Err(BallotError::Serialization(format!(
                "Unexpected version {}, expected {}",
                value.version.to_string(),
                TYPES_VERSION
            )));
        }

        let contests = value.deserialize_contests::<RistrettoCtx>()?;
        let hashable_ballot_contest: Vec<HashableBallotContest<RistrettoCtx>> =
            contests
                .iter()
                .map(|auditable_ballot_contest| {
                    let hashable_ballot_contest =
                        HashableBallotContest::<RistrettoCtx>::from(
                            auditable_ballot_contest,
                        );
                    hashable_ballot_contest
                })
                .collect();
        let ballot_style_hash =
            hash_ballot_style(&value.config).map_err(|error| {
                BallotError::Serialization(format!(
                    "Failed to hash ballot style: {}",
                    error
                ))
            })?;
        Ok(HashableBallot {
            version: TYPES_VERSION,
            issue_date: value.issue_date.clone(),
            contests: HashableBallot::serialize_contests::<RistrettoCtx>(
                &hashable_ballot_contest,
            )?,
            config: value.config.id.clone(),
            ballot_style_hash: ballot_style_hash,
        })
    }
}
