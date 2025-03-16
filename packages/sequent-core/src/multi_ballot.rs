// SPDX-FileCopyrightText: 2024 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

use crate::encrypt::hash_ballot_style;
use crate::error::BallotError;
use crate::serialization::base64::{Base64Deserialize, Base64Serialize};
use strand::elgamal::Ciphertext;
use strand::zkp::Schnorr;
use strand::{backend::ristretto::RistrettoCtx, context::Ctx};

use crate::ballot::TYPES_VERSION;
use crate::ballot::{BallotStyle, ReplicationChoice};

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct AuditableMultiBallot {
    pub version: u32,
    pub issue_date: String,
    pub config: BallotStyle,
    // String serialization of AuditableMultiBallotContests through
    //
    // self::serialize_contests can be deserialized with
    // self::deserialize_contests
    pub contests: String,
    pub ballot_hash: String,
}

#[derive(BorshSerialize, BorshDeserialize, PartialEq, Eq, Debug, Clone)]
pub struct AuditableMultiBallotContests<C: Ctx> {
    pub contest_ids: Vec<String>,
    pub choice: ReplicationChoice<C>,
    pub proof: Schnorr<C>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct HashableMultiBallot {
    pub version: u32,
    pub issue_date: String,
    // String serialization of HashableMultiBallotContests through
    //
    // self::serialize_contests can be deserialized with
    // self::deserialize_contests
    pub contests: String,
    pub config: String,
    pub ballot_style_hash: String,
}

#[derive(BorshSerialize, BorshDeserialize, PartialEq, Eq, Debug, Clone)]
pub struct HashableMultiBallotContests<C: Ctx> {
    pub contest_ids: Vec<String>,
    pub ciphertext: Ciphertext<C>,
    pub proof: Schnorr<C>,
}

#[derive(BorshSerialize, BorshDeserialize, PartialEq, Eq, Debug, Clone)]
pub struct RawHashableMultiBallot<C: Ctx> {
    pub version: u32,
    pub issue_date: String,
    pub contests: HashableMultiBallotContests<C>,
}

impl AuditableMultiBallot {
    pub fn deserialize_contests<C: Ctx>(
        &self,
    ) -> Result<AuditableMultiBallotContests<C>, BallotError> {
        let ret = Base64Deserialize::deserialize(self.contests.clone())
            .map_err(|err| BallotError::Serialization(err.to_string()));

        ret
    }

    pub fn serialize_contests<C: Ctx>(
        contests: &AuditableMultiBallotContests<C>,
    ) -> Result<String, BallotError> {
        Base64Serialize::serialize(&contests)
    }
}

impl HashableMultiBallot {
    pub fn deserialize_contests<C: Ctx>(
        &self,
    ) -> Result<HashableMultiBallotContests<C>, BallotError> {
        let ret = Base64Deserialize::deserialize(self.contests.clone())
            .map_err(|err| BallotError::Serialization(err.to_string()));

        ret
    }

    pub fn serialize_contests<C: Ctx>(
        contest: &HashableMultiBallotContests<C>,
    ) -> Result<String, BallotError> {
        Base64Serialize::serialize(&contest)
    }
}

impl TryFrom<&AuditableMultiBallot> for HashableMultiBallot {
    type Error = BallotError;

    fn try_from(value: &AuditableMultiBallot) -> Result<Self, Self::Error> {
        if TYPES_VERSION != value.version {
            return Err(BallotError::Serialization(format!(
                "Unexpected version {}, expected {}",
                value.version.to_string(),
                TYPES_VERSION
            )));
        }

        let contests = value.deserialize_contests::<RistrettoCtx>()?;
        let hashable_ballot_contests =
            HashableMultiBallotContests::<RistrettoCtx>::from(&contests);

        let ballot_style_hash =
            hash_ballot_style(&value.config).map_err(|error| {
                BallotError::Serialization(format!(
                    "Failed to hash ballot style: {}",
                    error
                ))
            })?;

        Ok(HashableMultiBallot {
            version: TYPES_VERSION,
            issue_date: value.issue_date.clone(),
            contests: HashableMultiBallot::serialize_contests::<RistrettoCtx>(
                &hashable_ballot_contests,
            )?,
            config: value.config.id.clone(),
            ballot_style_hash: ballot_style_hash,
        })
    }
}

impl<C: Ctx> TryFrom<&HashableMultiBallot> for RawHashableMultiBallot<C> {
    type Error = BallotError;

    fn try_from(value: &HashableMultiBallot) -> Result<Self, Self::Error> {
        let contests = value.deserialize_contests::<C>()?;
        Ok(RawHashableMultiBallot {
            version: value.version,
            issue_date: value.issue_date.clone(),
            contests: contests,
        })
    }
}

impl<C: Ctx> From<&AuditableMultiBallotContests<C>>
    for HashableMultiBallotContests<C>
{
    fn from(
        value: &AuditableMultiBallotContests<C>,
    ) -> HashableMultiBallotContests<C> {
        HashableMultiBallotContests {
            contest_ids: value.contest_ids.clone(),
            ciphertext: value.choice.ciphertext.clone(),
            proof: value.proof.clone(),
        }
    }
}
