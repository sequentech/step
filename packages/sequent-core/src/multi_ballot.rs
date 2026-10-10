// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use borsh::{BorshDeserialize, BorshSerialize};
use cryptography::utils::serialization::{Deserializable, Serializable};
use cryptography::Canonical;
use serde::{Deserialize, Serialize};
use voting_crypto::ballot::BallotCiphertext;
use voting_crypto::signing::{Signature, SigningKey, VerificationKey};
use voting_crypto::RecordSerialize;

use crate::encrypt::hash_ballot_style;
use crate::error::BallotError;
use crate::serialization::base64::{Base64Deserialize, Base64Serialize};

use crate::ballot::get_ballot_bytes_for_signing;
use crate::ballot::SignedContent;
use crate::ballot::INCOMPLETE_BALLOT_SIGNATURE_ERROR;
use crate::ballot::TYPES_VERSION;
use crate::ballot::{BallotStyle, ReplicationChoice};
use base64::engine::general_purpose;
use base64::Engine;

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

#[derive(PartialEq, Debug, Clone, Canonical)]
pub struct AuditableMultiBallotContests {
    pub contest_ids: Vec<String>,
    pub choice: ReplicationChoice,
}

#[derive(
    BorshSerialize, Serialize, Deserialize, PartialEq, Eq, Debug, Clone,
)]
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

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct SignedHashableMultiBallot {
    pub version: u32,
    pub issue_date: String,
    pub contests: String,
    pub config: String,
    pub ballot_style_hash: String,
    pub voter_signing_pk: Option<String>,
    pub voter_ballot_signature: Option<String>,
}

pub type HashableMultiBallotContests = BallotCiphertext;

#[derive(PartialEq, Debug, Clone, Canonical)]
pub struct RawHashableMultiBallot {
    pub version: u32,
    pub issue_date: String,
    pub contests: HashableMultiBallotContests,
}

impl AuditableMultiBallot {
    pub fn deserialize_contests(
        &self,
    ) -> Result<AuditableMultiBallotContests, BallotError> {
        let ret = Base64Deserialize::deserialize(self.contests.clone())
            .map_err(|err| BallotError::Serialization(err.to_string()));

        ret
    }

    pub fn serialize_contests(
        contests: &AuditableMultiBallotContests,
    ) -> Result<String, BallotError> {
        Base64Serialize::serialize(&contests)
    }
}

impl HashableMultiBallot {
    pub fn deserialize_contests(
        &self,
    ) -> Result<HashableMultiBallotContests, BallotError> {
        let ret = Base64Deserialize::deserialize(self.contests.clone())
            .map_err(|err| BallotError::Serialization(err.to_string()));

        ret
    }

    pub fn serialize_contests(
        contest: &HashableMultiBallotContests,
    ) -> Result<String, BallotError> {
        Base64Serialize::serialize(&contest)
    }
}

impl SignedHashableMultiBallot {
    pub fn deserialize_contests(
        &self,
    ) -> Result<HashableMultiBallotContests, BallotError> {
        let hashable_ballot = HashableMultiBallot::try_from(self)?;

        hashable_ballot.deserialize_contests()
    }

    pub fn serialize_contests(
        contest: &HashableMultiBallotContests,
    ) -> Result<String, BallotError> {
        HashableMultiBallot::serialize_contests(contest)
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

        let contests = value.deserialize_contests()?;
        let hashable_ballot_contests =
            HashableMultiBallotContests::from(&contests);

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
            contests: HashableMultiBallot::serialize_contests(
                &hashable_ballot_contests,
            )?,
            config: value.config.id.clone(),
            ballot_style_hash: ballot_style_hash,
        })
    }
}

impl TryFrom<&AuditableMultiBallot> for SignedHashableMultiBallot {
    type Error = BallotError;

    fn try_from(value: &AuditableMultiBallot) -> Result<Self, Self::Error> {
        if TYPES_VERSION != value.version {
            return Err(BallotError::Serialization(format!(
                "Unexpected version {}, expected {}",
                value.version.to_string(),
                TYPES_VERSION
            )));
        }

        let contests = value.deserialize_contests()?;
        let hashable_ballot_contests =
            HashableMultiBallotContests::from(&contests);

        let ballot_style_hash =
            hash_ballot_style(&value.config).map_err(|error| {
                BallotError::Serialization(format!(
                    "Failed to hash ballot style: {}",
                    error
                ))
            })?;

        Ok(SignedHashableMultiBallot {
            version: TYPES_VERSION,
            issue_date: value.issue_date.clone(),
            contests: HashableMultiBallot::serialize_contests(
                &hashable_ballot_contests,
            )?,
            config: value.config.id.clone(),
            ballot_style_hash: ballot_style_hash,
            voter_signing_pk: value.voter_signing_pk.clone(),
            voter_ballot_signature: value.voter_ballot_signature.clone(),
        })
    }
}

impl TryFrom<&SignedHashableMultiBallot> for HashableMultiBallot {
    type Error = BallotError;
    fn try_from(
        value: &SignedHashableMultiBallot,
    ) -> Result<Self, Self::Error> {
        if TYPES_VERSION != value.version {
            return Err(BallotError::Serialization(format!(
                "Unexpected version {}, expected {}",
                value.version.to_string(),
                TYPES_VERSION
            )));
        }

        Ok(HashableMultiBallot {
            version: TYPES_VERSION,
            issue_date: value.issue_date.clone(),
            contests: value.contests.clone(),
            config: value.config.clone(),
            ballot_style_hash: value.ballot_style_hash.clone(),
        })
    }
}

impl TryFrom<&HashableMultiBallot> for RawHashableMultiBallot {
    type Error = BallotError;

    fn try_from(value: &HashableMultiBallot) -> Result<Self, Self::Error> {
        let contests = value.deserialize_contests()?;
        Ok(RawHashableMultiBallot {
            version: value.version,
            issue_date: value.issue_date.clone(),
            contests: contests,
        })
    }
}

impl From<&AuditableMultiBallotContests> for HashableMultiBallotContests {
    fn from(
        value: &AuditableMultiBallotContests,
    ) -> HashableMultiBallotContests {
        HashableMultiBallotContests {
            contest_ids: value.contest_ids.clone(),
            ciphertext: value.choice.ciphertext.clone(),
        }
    }
}

pub fn sign_hashable_multi_ballot_with_ephemeral_voter_signing_key(
    ballot_id: &str,
    election_id: &str,
    hashable_multi_ballot: &HashableMultiBallot,
) -> Result<SignedContent, String> {
    let secret_key = SigningKey::generate()
        .map_err(|err| format!("Error generating secret key: {err}"))?;
    sign_hashable_multi_ballot_with_voter_signing_key(
        &secret_key,
        ballot_id,
        election_id,
        hashable_multi_ballot,
    )
}

pub fn sign_hashable_multi_ballot_with_voter_signing_key(
    secret_key: &SigningKey,
    ballot_id: &str,
    election_id: &str,
    hashable_multi_ballot: &HashableMultiBallot,
) -> Result<SignedContent, String> {
    // Get ballot_bytes_for_signing
    let content_bytes = hashable_multi_ballot
        .record_serialize()
        .map_err(|err| format!("Error getting signature bytes: {err}"))?;
    let ballot_bytes =
        get_ballot_bytes_for_signing(ballot_id, election_id, &content_bytes);

    let public_key = VerificationKey::from_sk(secret_key)
        .map_err(|err| format!("Error generating public key: {err}"))?;

    let ballot_signature = secret_key
        .sign(&ballot_bytes)
        .map_err(|err| format!("Failed to sign the ballot: {err}"))?;

    let public_key = public_key
        .to_b64_string()
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
    signed_hashable_multi_ballot: &SignedHashableMultiBallot,
) -> Result<Option<(VerificationKey, Signature)>, String> {
    let (signature, public_key) =
        if let (Some(voter_ballot_signature), Some(voter_signing_pk)) = (
            signed_hashable_multi_ballot.voter_ballot_signature.clone(),
            signed_hashable_multi_ballot.voter_signing_pk.clone(),
        ) {
            (voter_ballot_signature, voter_signing_pk)
        } else if signed_hashable_multi_ballot
            .voter_ballot_signature
            .is_none()
            && signed_hashable_multi_ballot.voter_signing_pk.is_none()
        {
            return Ok(None);
        } else {
            return Err(INCOMPLETE_BALLOT_SIGNATURE_ERROR.into());
        };

    let voter_signing_pk = VerificationKey::from_b64_string(&public_key)
        .map_err(|err| {
            format!(
                "Failed to deserialize signature from hashable multi ballot: {}",
                err
            )
        })?;

    let hashable_multi_ballot: HashableMultiBallot =
        signed_hashable_multi_ballot.try_into().map_err(|err| {
            format!("Failed to convert to hashable multi ballot: {}", err)
        })?;

    let content = hashable_multi_ballot.record_serialize().map_err(|err| {
        format!(
            "Failed to deserialize signature from hashable multi ballot: {}",
            err
        )
    })?;

    let ballot_bytes =
        get_ballot_bytes_for_signing(ballot_id, election_id, &content);

    let ballot_signature = Signature::from_b64_string(&signature)
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
