// SPDX-FileCopyrightText: 2024 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

use crate::encrypt::hash_ballot_style;
use crate::error::BallotError;
use crate::serialization::base64::{Base64Deserialize, Base64Serialize};
use strand::{backend::ristretto::RistrettoCtx, context::Ctx};

use crate::ballot::BallotStyle;
use crate::ballot::TYPES_VERSION;

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct AuditablePlaintextBallot {
    pub version: u32,
    pub issue_date: String,
    pub config: BallotStyle,
    // String serialization of AuditablePlaintextBallotContests through
    //
    // self::serialize_contests can be deserialized with
    // self::deserialize_contests
    pub contests: String,
    pub ballot_hash: String,
}

#[derive(BorshSerialize, BorshDeserialize, PartialEq, Eq, Debug, Clone)]
pub struct AuditablePlaintextBallotContests<C: Ctx> {
    pub contest_ids: Vec<String>,
    pub plaintext: C::P,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct HashablePlaintextBallot {
    pub version: u32,
    pub issue_date: String,
    // String serialization of HashablePlaintextBallotContests through
    //
    // self::serialize_contests can be deserialized with
    // self::deserialize_contests
    pub contests: String,
    pub config: String,
    pub ballot_style_hash: String,
}

#[derive(BorshSerialize, BorshDeserialize, PartialEq, Eq, Debug, Clone)]
pub struct HashablePlaintextBallotContests<C: Ctx> {
    pub contest_ids: Vec<String>,
    pub plaintext: C::P,
}

#[derive(BorshSerialize, BorshDeserialize, PartialEq, Eq, Debug, Clone)]
pub struct RawHashablePlaintextBallot<C: Ctx> {
    pub version: u32,
    pub issue_date: String,
    pub contests: HashablePlaintextBallotContests<C>,
}

impl AuditablePlaintextBallot {
    pub fn deserialize_contests<C: Ctx>(
        &self,
    ) -> Result<AuditablePlaintextBallotContests<C>, BallotError> {
        let ret = Base64Deserialize::deserialize(self.contests.clone())
            .map_err(|err| BallotError::Serialization(err.to_string()));

        ret
    }

    pub fn serialize_contests<C: Ctx>(
        contests: &AuditablePlaintextBallotContests<C>,
    ) -> Result<String, BallotError> {
        Base64Serialize::serialize(&contests)
    }
}

impl HashablePlaintextBallot {
    pub fn deserialize_contests<C: Ctx>(
        &self,
    ) -> Result<HashablePlaintextBallotContests<C>, BallotError> {
        let ret = Base64Deserialize::deserialize(self.contests.clone())
            .map_err(|err| BallotError::Serialization(err.to_string()));

        ret
    }

    pub fn serialize_contests<C: Ctx>(
        contest: &HashablePlaintextBallotContests<C>,
    ) -> Result<String, BallotError> {
        Base64Serialize::serialize(&contest)
    }
}

impl TryFrom<&AuditablePlaintextBallot> for HashablePlaintextBallot {
    type Error = BallotError;

    fn try_from(value: &AuditablePlaintextBallot) -> Result<Self, Self::Error> {
        if TYPES_VERSION != value.version {
            return Err(BallotError::Serialization(format!(
                "Unexpected version {}, expected {}",
                value.version.to_string(),
                TYPES_VERSION
            )));
        }

        let contests = value.deserialize_contests::<RistrettoCtx>()?;
        let hashable_ballot_contests =
            HashablePlaintextBallotContests::<RistrettoCtx>::from(&contests);

        let ballot_style_hash =
            hash_ballot_style(&value.config).map_err(|error| {
                BallotError::Serialization(format!(
                    "Failed to hash ballot style: {}",
                    error
                ))
            })?;

        Ok(HashablePlaintextBallot {
            version: TYPES_VERSION,
            issue_date: value.issue_date.clone(),
            contests: HashablePlaintextBallot::serialize_contests::<
                RistrettoCtx,
            >(&hashable_ballot_contests)?,
            config: value.config.id.clone(),
            ballot_style_hash: ballot_style_hash,
        })
    }
}

impl<C: Ctx> TryFrom<&HashablePlaintextBallot>
    for RawHashablePlaintextBallot<C>
{
    type Error = BallotError;

    fn try_from(value: &HashablePlaintextBallot) -> Result<Self, Self::Error> {
        let contests = value.deserialize_contests::<C>()?;
        Ok(RawHashablePlaintextBallot {
            version: value.version,
            issue_date: value.issue_date.clone(),
            contests: contests,
        })
    }
}

impl<C: Ctx> From<&AuditablePlaintextBallotContests<C>>
    for HashablePlaintextBallotContests<C>
{
    fn from(
        value: &AuditablePlaintextBallotContests<C>,
    ) -> HashablePlaintextBallotContests<C> {
        HashablePlaintextBallotContests {
            contest_ids: value.contest_ids.clone(),
            plaintext: value.plaintext.clone(),
        }
    }
}
