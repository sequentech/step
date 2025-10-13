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

use crate::ballot::get_ballot_bytes_for_signing;
use crate::ballot::SignedContent;
use crate::ballot::TYPES_VERSION;
use crate::ballot::{BallotStyle, ReplicationChoice};
use base64::engine::general_purpose;
use base64::Engine;
use strand::signature::StrandSignature;
use strand::signature::StrandSignaturePk;
use strand::signature::StrandSignatureSk;

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
    pub voter_signing_pk: Option<String>,
    pub voter_ballot_signature: Option<String>,
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
    pub voter_signing_pk: Option<String>,
    pub voter_ballot_signature: Option<String>,
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

    pub fn get_bytes_for_signing(&self) -> Result<Vec<u8>, BallotError> {
        let mut ret: Vec<u8> = vec![];

        let bytes = self.version.to_le_bytes();
        let length = (bytes.len() as u64).to_le_bytes();
        ret.extend_from_slice(&length);
        ret.extend_from_slice(&bytes);

        let bytes = self.issue_date.as_bytes();
        let length = (bytes.len() as u64).to_le_bytes();
        ret.extend_from_slice(&length);
        ret.extend_from_slice(&bytes);

        let bytes = self.contests.as_bytes();
        let length = (bytes.len() as u64).to_le_bytes();
        ret.extend_from_slice(&length);
        ret.extend_from_slice(&bytes);

        let bytes = self.config.as_bytes();
        let length = (bytes.len() as u64).to_le_bytes();
        ret.extend_from_slice(&length);
        ret.extend_from_slice(&bytes);

        let bytes = self.ballot_style_hash.as_bytes();
        let length = (bytes.len() as u64).to_le_bytes();
        ret.extend_from_slice(&length);
        ret.extend_from_slice(&bytes);

        Ok(ret)
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
            voter_signing_pk: value.voter_signing_pk.clone(),
            voter_ballot_signature: value.voter_ballot_signature.clone(),
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

pub fn sign_hashable_multi_ballot_with_ephemeral_voter_signing_key(
    ballot_id: &str,
    election_id: &str,
    hashable_multi_ballot: &HashableMultiBallot,
) -> Result<SignedContent, String> {
    // Get ballot_bytes_for_signing
    let content_bytes = hashable_multi_ballot
        .get_bytes_for_signing()
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

pub fn verify_multi_ballot_signature(
    ballot_id: &str,
    election_id: &str,
    hashable_multi_ballot: &HashableMultiBallot,
) -> Result<bool, String> {
    let (signature, public_key) =
        if let (Some(voter_ballot_signature), Some(voter_signing_pk)) = (
            hashable_multi_ballot.voter_ballot_signature.clone(),
            hashable_multi_ballot.voter_signing_pk.clone(),
        ) {
            (voter_ballot_signature, voter_signing_pk)
        } else {
            return Ok(false);
        };

    let voter_signing_pk = StrandSignaturePk::from_der_b64_string(&public_key)
        .map_err(|err| {
            format!(
                "Failed to deserialize signature from hashable ballot: {}",
                err
            )
        })?;

    let content =
        hashable_multi_ballot
            .get_bytes_for_signing()
            .map_err(|err| {
                format!(
                    "Failed to deserialize signature from hashable ballot: {}",
                    err
                )
            })?;

    let ballot_bytes =
        get_ballot_bytes_for_signing(ballot_id, election_id, &content);

    let ballot_signature = StrandSignature::from_b64_string(&signature)
        .map_err(|err| {
            format!(
                "Failed to deserialize signature from hashable ballot: {}",
                err
            )
        })?;

    voter_signing_pk
        .verify(&ballot_signature, &ballot_bytes)
        .map_err(|err| format!("Failed to verify signature: {err}"))?;

    Ok(true)
}
