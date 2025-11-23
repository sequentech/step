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
use strand::serialization::StrandSerialize;
use strand::signature::StrandSignature;
use strand::signature::StrandSignaturePk;
use strand::signature::StrandSignatureSk;
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
    pub voter_signing_pk: Option<String>,
    pub voter_ballot_signature: Option<String>,
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
#[derive(
    BorshSerialize, Serialize, Deserialize, PartialEq, Eq, Debug, Clone,
)]
pub struct HashableBallot {
    pub version: u32,
    pub issue_date: String,
    pub contests: Vec<String>, // Vec<HashableBallotContest<C>>,
    pub config: String,
    pub ballot_style_hash: String,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct SignedHashableBallot {
    pub version: u32,
    pub issue_date: String,
    pub contests: Vec<String>,
    pub config: String,
    pub ballot_style_hash: String,
    pub voter_signing_pk: Option<String>,
    pub voter_ballot_signature: Option<String>,
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

impl SignedHashableBallot {
    pub fn deserialize_contests<C: Ctx>(
        &self,
    ) -> Result<Vec<HashableBallotContest<C>>, BallotError> {
        let hashable_ballot = HashableBallot::try_from(self)?;

        hashable_ballot.deserialize_contests()
    }

    pub fn serialize_contests<C: Ctx>(
        contests: &Vec<HashableBallotContest<C>>,
    ) -> Result<Vec<String>, BallotError> {
        HashableBallot::serialize_contests(contests)
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

impl TryFrom<&AuditableBallot> for SignedHashableBallot {
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
        Ok(SignedHashableBallot {
            version: TYPES_VERSION,
            issue_date: value.issue_date.clone(),
            contests: HashableBallot::serialize_contests::<RistrettoCtx>(
                &hashable_ballot_contest,
            )?,
            config: value.config.id.clone(),
            ballot_style_hash: ballot_style_hash,
            voter_signing_pk: value.voter_signing_pk.clone(),
            voter_ballot_signature: value.voter_ballot_signature.clone(),
        })
    }
}

impl TryFrom<&SignedHashableBallot> for HashableBallot {
    type Error = BallotError;
    fn try_from(value: &SignedHashableBallot) -> Result<Self, Self::Error> {
        if TYPES_VERSION != value.version {
            return Err(BallotError::Serialization(format!(
                "Unexpected version {}, expected {}",
                value.version.to_string(),
                TYPES_VERSION
            )));
        }

        Ok(HashableBallot {
            version: TYPES_VERSION,
            issue_date: value.issue_date.clone(),
            contests: value.contests.clone(),
            config: value.config.clone(),
            ballot_style_hash: value.ballot_style_hash.clone(),
        })
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct SignedContent {
    pub public_key: String,
    pub signature: String,
}

pub fn sign_hashable_ballot_with_ephemeral_voter_signing_key(
    ballot_id: &str,
    election_id: &str,
    hashable_ballot: &HashableBallot,
) -> Result<SignedContent, String> {
    // Get ballot_bytes_for_signing
    let content_bytes = hashable_ballot
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

// Returns Some(StrandSignature) if the signature was verified or None if there
// was no signature to verify.
pub fn verify_ballot_signature(
    ballot_id: &str,
    election_id: &str,
    signed_hashable_ballot: &SignedHashableBallot,
) -> Result<Option<(StrandSignaturePk, StrandSignature)>, String> {
    let (voter_ballot_signature, voter_signing_pk) =
        if let (Some(voter_ballot_signature), Some(voter_signing_pk)) = (
            signed_hashable_ballot.voter_ballot_signature.clone(),
            signed_hashable_ballot.voter_signing_pk.clone(),
        ) {
            (voter_ballot_signature, voter_signing_pk)
        } else {
            return Ok(None);
        };

    let voter_signing_pk = StrandSignaturePk::from_der_b64_string(
        &voter_signing_pk,
    )
    .map_err(|err| {
        format!(
            "Failed to deserialize signature from hashable ballot: {}",
            err
        )
    })?;

    let hashable_ballot: HashableBallot =
        signed_hashable_ballot.try_into().map_err(|err| {
            format!("Failed to convert to hashable ballot: {}", err)
        })?;

    let content = hashable_ballot.strand_serialize().map_err(|err| {
        format!(
            "Failed to get bytes for signing from hashable ballot: {}",
            err
        )
    })?;

    let ballot_bytes =
        get_ballot_bytes_for_signing(ballot_id, election_id, &content);

    let ballot_signature = StrandSignature::from_b64_string(
        &voter_ballot_signature,
    )
    .map_err(|err| {
        format!(
            "Failed to deserialize signature from hashable ballot: {}",
            err
        )
    })?;

    voter_signing_pk
        .verify(&ballot_signature, &ballot_bytes)
        .map_err(|err| format!("Failed to verify signature: {err}"))?;

    Ok(Some((voter_signing_pk, ballot_signature)))
}

pub fn get_ballot_bytes_for_signing(
    ballot_id: &str,
    election_id: &str,
    content: &[u8],
) -> Vec<u8> {
    let mut ret: Vec<u8> = vec![];

    let bytes = ballot_id.as_bytes();
    let length = (bytes.len() as u64).to_le_bytes();
    ret.extend_from_slice(&length);
    ret.extend_from_slice(&bytes);

    let bytes = election_id.as_bytes();
    let length = (bytes.len() as u64).to_le_bytes();
    ret.extend_from_slice(&length);
    ret.extend_from_slice(&bytes);

    let bytes = content;
    let length = (bytes.len() as u64).to_le_bytes();
    ret.extend_from_slice(&length);
    ret.extend_from_slice(&bytes);

    ret
}
