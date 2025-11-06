// SPDX-FileCopyrightText: 2024 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use strand::signature::{
    StrandSignature, StrandSignaturePk, StrandSignatureSk,
};

use crate::encrypt::hash_ballot_style;
use crate::error::BallotError;
use crate::serialization::base64::{Base64Deserialize, Base64Serialize};
use strand::{backend::ristretto::RistrettoCtx, context::Ctx};

use crate::ballot::TYPES_VERSION;
use crate::ballot::{BallotStyle, SignedContent};

use crate::ballot::get_ballot_bytes_for_signing;
use strand::serialization::StrandSerialize;

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct AuditablePlaintextBallot {
    pub version: u32,
    pub issue_date: String,
    pub config: BallotStyle,
    // String serialization of AuditablePlaintextBallotContests through
    //
    // self::serialize_contests can be deserialized with
    // self::deserialize_contests
    pub contests: Vec<String>,
    pub ballot_hash: String,
    pub voter_signing_pk: Option<String>,
    pub voter_ballot_signature: Option<String>,
}

#[derive(BorshSerialize, BorshDeserialize, PartialEq, Eq, Debug, Clone)]
pub struct AuditablePlaintextBallotContest<C: Ctx> {
    pub contest_id: String,
    pub plaintext: C::P,
}

#[derive(
    BorshSerialize, Serialize, Deserialize, PartialEq, Eq, Debug, Clone,
)]
pub struct HashablePlaintextBallot {
    pub version: u32,
    pub issue_date: String,
    // String serialization of HashablePlaintextBallotContests through
    //
    // self::serialize_contests can be deserialized with
    // self::deserialize_contests
    pub contests: Vec<String>,
    pub config: String,
    pub ballot_style_hash: String,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct SignedHashablePlaintextBallot {
    pub version: u32,
    pub issue_date: String,
    // String serialization of HashablePlaintextBallotContests through
    //
    // self::serialize_contests can be deserialized with
    // self::deserialize_contests
    pub contests: Vec<String>,
    pub config: String,
    pub ballot_style_hash: String,
    pub voter_signing_pk: Option<String>,
    pub voter_ballot_signature: Option<String>,
}

#[derive(BorshSerialize, BorshDeserialize, PartialEq, Eq, Debug, Clone)]
pub struct HashablePlaintextBallotContest<C: Ctx> {
    pub contest_id: String,
    pub plaintext: C::P,
}

#[derive(BorshSerialize, BorshDeserialize, PartialEq, Eq, Debug, Clone)]
pub struct RawHashablePlaintextBallot<C: Ctx> {
    pub version: u32,
    pub issue_date: String,
    pub contests: Vec<HashablePlaintextBallotContest<C>>,
}

impl AuditablePlaintextBallot {
    pub fn deserialize_contests<C: Ctx>(
        &self,
    ) -> Result<Vec<AuditablePlaintextBallotContest<C>>, BallotError> {
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
        contests: &Vec<AuditablePlaintextBallotContest<C>>,
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

impl HashablePlaintextBallot {
    pub fn deserialize_contests<C: Ctx>(
        &self,
    ) -> Result<Vec<HashablePlaintextBallotContest<C>>, BallotError> {
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
        contests: &Vec<HashablePlaintextBallotContest<C>>,
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

impl SignedHashablePlaintextBallot {
    pub fn deserialize_contests<C: Ctx>(
        &self,
    ) -> Result<Vec<HashablePlaintextBallotContest<C>>, BallotError> {
        let hashable_ballot = HashablePlaintextBallot::try_from(self)?;

        hashable_ballot.deserialize_contests()
    }

    pub fn serialize_contests<C: Ctx>(
        contests: &Vec<HashablePlaintextBallotContest<C>>,
    ) -> Result<Vec<String>, BallotError> {
        HashablePlaintextBallot::serialize_contests(contests)
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
        let hashable_ballot_contests = contests
            .iter()
            .map(|auditable_ballot_contest| {
                let hashable_ballot_contest =
                    HashablePlaintextBallotContest::<RistrettoCtx>::from(
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

impl<C: Ctx> From<&AuditablePlaintextBallotContest<C>>
    for HashablePlaintextBallotContest<C>
{
    fn from(
        value: &AuditablePlaintextBallotContest<C>,
    ) -> HashablePlaintextBallotContest<C> {
        HashablePlaintextBallotContest {
            contest_id: value.contest_id.clone(),
            plaintext: value.plaintext.clone(),
        }
    }
}

impl TryFrom<&AuditablePlaintextBallot> for SignedHashablePlaintextBallot {
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
        let hashable_ballot_contest: Vec<
            HashablePlaintextBallotContest<RistrettoCtx>,
        > = contests
            .iter()
            .map(|auditable_ballot_contest| {
                let hashable_ballot_contest =
                    HashablePlaintextBallotContest::<RistrettoCtx>::from(
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
        Ok(SignedHashablePlaintextBallot {
            version: TYPES_VERSION,
            issue_date: value.issue_date.clone(),
            contests: HashablePlaintextBallot::serialize_contests::<
                RistrettoCtx,
            >(&hashable_ballot_contest)?,
            config: value.config.id.clone(),
            ballot_style_hash: ballot_style_hash,
            voter_signing_pk: value.voter_signing_pk.clone(),
            voter_ballot_signature: value.voter_ballot_signature.clone(),
        })
    }
}

impl TryFrom<&SignedHashablePlaintextBallot> for HashablePlaintextBallot {
    type Error = BallotError;
    fn try_from(
        value: &SignedHashablePlaintextBallot,
    ) -> Result<Self, Self::Error> {
        if TYPES_VERSION != value.version {
            return Err(BallotError::Serialization(format!(
                "Unexpected version {}, expected {}",
                value.version.to_string(),
                TYPES_VERSION
            )));
        }

        Ok(HashablePlaintextBallot {
            version: TYPES_VERSION,
            issue_date: value.issue_date.clone(),
            contests: value.contests.clone(),
            config: value.config.clone(),
            ballot_style_hash: value.ballot_style_hash.clone(),
        })
    }
}

pub fn sign_hashable_plaintext_ballot_with_ephemeral_voter_signing_key(
    ballot_id: &str,
    election_id: &str,
    hashable_plaintext_ballot: &HashablePlaintextBallot,
) -> Result<SignedContent, String> {
    // Get ballot_bytes_for_signing
    let content_bytes = hashable_plaintext_ballot
        .strand_serialize()
        .map_err(|err| format!("Error getting signature bytes: {err}"))?;
    let ballot_bytes =
        get_ballot_bytes_for_signing(ballot_id, election_id, &content_bytes);

    // Generate voter ephemeral key for signing
    let secret_key = StrandSignatureSk::gen()
        .map_err(|err| format!("Error generating secret key: {err}"))?;
    let public_key = StrandSignaturePk::from_sk(&secret_key)
        .map_err(|err| format!("Error generating public key: {err}"))?;

    let ballot_signature = secret_key
        .sign(&ballot_bytes)
        .map_err(|err| format!("Failed to sign the ballot: {err}"))?;

    let public_key = public_key
        .to_der_b64_string()
        .map_err(|err| format!("Failed to serialize the public key: {err}"))?;

    let signature = ballot_signature
        .to_b64_string()
        .map_err(|err| format!("Failed to serialize signature: {err}"))?;

    Ok(SignedContent {
        public_key,
        signature,
    })
}

pub fn verify_plaintext_ballot_signature(
    ballot_id: &str,
    election_id: &str,
    signed_hashable_plaintext_ballot: &SignedHashablePlaintextBallot,
) -> Result<Option<(StrandSignaturePk, StrandSignature)>, String> {
    let (signature, public_key) =
        if let (Some(voter_ballot_signature), Some(voter_signing_pk)) = (
            signed_hashable_plaintext_ballot
                .voter_ballot_signature
                .clone(),
            signed_hashable_plaintext_ballot.voter_signing_pk.clone(),
        ) {
            (voter_ballot_signature, voter_signing_pk)
        } else {
            return Ok(None);
        };

    let voter_signing_pk = StrandSignaturePk::from_der_b64_string(&public_key)
        .map_err(|err| {
            format!(
                "Failed to deserialize signature from hashable plaintext ballot: {}",
                err
            )
        })?;

    let hashable_plaintext_ballot: HashablePlaintextBallot =
        signed_hashable_plaintext_ballot.try_into().map_err(|err| {
            format!("Failed to convert to hashable plaintext ballot: {}", err)
        })?;

    let content = hashable_plaintext_ballot.strand_serialize().map_err(|err| {
        format!(
            "Failed to deserialize signature from hashable plaintext ballot: {}",
            err
        )
    })?;

    let ballot_bytes =
        get_ballot_bytes_for_signing(ballot_id, election_id, &content);

    let ballot_signature = StrandSignature::from_b64_string(&signature)
        .map_err(|err| {
            format!(
                "Failed to deserialize signature from hashable multi ballot: {}",
                err
            )
        })?;

    voter_signing_pk
        .verify(&ballot_signature, &ballot_bytes)
        .map_err(|err| format!("Failed to verify signature: {err}"))?;

    Ok(Some((voter_signing_pk, ballot_signature)))
}
