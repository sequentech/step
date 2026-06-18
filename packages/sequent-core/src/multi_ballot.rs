// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
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
use strand::serialization::StrandSerialize;
use strand::signature::StrandSignature;
use strand::signature::StrandSignaturePk;
use strand::signature::StrandSignatureSk;

/// Represents a fully auditable multi-contest ballot.
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct AuditableMultiBallot {
    /// [`crate::ballot::TYPES_VERSION`] of the ballot JSON schema.
    pub version: u32,
    /// ISO 8601 timestamp when the ballot was generated.
    pub issue_date: String,
    /// Ballot style defining contests, keys, and presentation for this voter.
    pub config: BallotStyle,
    /// String serialization of `AuditableMultiBallotContests` through
    /// `serialize_contests` can be deserialized with `deserialize_contests`
    pub contests: String,
    /// Hash fingerprint of the ballot contents for tracking and verification.
    pub ballot_hash: String,
    /// Optional voter ephemeral signing public key.
    pub voter_signing_pk: Option<String>,
    /// Optional Ed25519 signature over the hashable multi-ballot bytes.
    pub voter_ballot_signature: Option<String>,
}

/// Encrypted choices for multiple contests encoded as a single mixed-radix value.
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Eq, Debug, Clone)]
pub struct AuditableMultiBallotContests<C: Ctx> {
    /// Identifiers of all contests covered by the combined ciphertext.
    pub contest_ids: Vec<String>,
    /// Combined encrypted choice with plaintext and randomness for voter audit.
    pub choice: ReplicationChoice<C>,
    /// Proof that the ciphertext is a valid encryption of the plaintext.
    pub proof: Schnorr<C>,
}

/// Hashable multi-contest ballot submitted for casting (no audit material).
#[derive(
    BorshSerialize, Serialize, Deserialize, PartialEq, Eq, Debug, Clone,
)]
pub struct HashableMultiBallot {
    /// [`crate::ballot::TYPES_VERSION`] of the ballot schema.
    pub version: u32,
    /// ISO 8601 timestamp when the ballot was generated.
    pub issue_date: String,
    /// String serialization of `HashableMultiBallotContests` through
    /// `serialize_contests` can be deserialized with `deserialize_contests`
    pub contests: String,
    /// Ballot style identifier binding this ballot to a voter's configuration.
    pub config: String,
    /// Hash of the ballot style.
    pub ballot_style_hash: String,
}

/// Hashable multi-contest ballot with optional voter signature fields.
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct SignedHashableMultiBallot {
    /// [`crate::ballot::TYPES_VERSION`] of the ballot schema.
    pub version: u32,
    /// ISO 8601 timestamp when the ballot was generated.
    pub issue_date: String,
    /// Base64-encoded multi-contest hashable payload.
    pub contests: String,
    /// Serialized ballot style identifier.
    pub config: String,
    /// Hash of the ballot style.
    pub ballot_style_hash: String,
    /// Voter ephemeral signing public key, when the ballot was signed.
    pub voter_signing_pk: Option<String>,
    /// Ed25519 signature over the canonical multi-ballot signing bytes.
    pub voter_ballot_signature: Option<String>,
}

/// One multi-contest entry in a hashable ballot (ciphertext only).
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Eq, Debug, Clone)]
pub struct HashableMultiBallotContests<C: Ctx> {
    /// Identifiers of all contests covered by the combined ciphertext.
    pub contest_ids: Vec<String>,
    /// Combined `ElGamal` ciphertext for all contest selections.
    pub ciphertext: Ciphertext<C>,
    /// Proof that the ciphertext is a valid encryption.
    pub proof: Schnorr<C>,
}

/// In-memory hashable multi-ballot with a typed contest entry.
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Eq, Debug, Clone)]
pub struct RawHashableMultiBallot<C: Ctx> {
    /// [`crate::ballot::TYPES_VERSION`] of the ballot schema.
    pub version: u32,
    /// ISO 8601 timestamp when the ballot was generated.
    pub issue_date: String,
    /// Decoded multi-contest ciphertext ready for hashing.
    pub contests: HashableMultiBallotContests<C>,
}

impl AuditableMultiBallot {
    /// Decodes the base64 contests blob into a typed multi-contest structure.
    ///
    /// # Errors
    ///
    /// Returns [`BallotError::Serialization`] when the payload cannot be decoded.
    pub fn deserialize_contests<C: Ctx>(
        &self,
    ) -> Result<AuditableMultiBallotContests<C>, BallotError> {
        let ret = Base64Deserialize::deserialize(self.contests.clone())
            .map_err(|err| BallotError::Serialization(err.to_string()));

        ret
    }

    /// Encodes multi-contest auditable data as a base64 string for JSON transport.
    ///
    /// # Errors
    ///
    /// Returns [`BallotError::Serialization`] when the payload cannot be encoded.
    pub fn serialize_contests<C: Ctx>(
        contests: &AuditableMultiBallotContests<C>,
    ) -> Result<String, BallotError> {
        Base64Serialize::serialize(&contests)
    }
}

impl HashableMultiBallot {
    /// Decodes the base64 contests blob into a typed hashable multi-contest structure.
    ///
    /// # Errors
    ///
    /// Returns [`BallotError::Serialization`] when the payload cannot be decoded.
    pub fn deserialize_contests<C: Ctx>(
        &self,
    ) -> Result<HashableMultiBallotContests<C>, BallotError> {
        let ret = Base64Deserialize::deserialize(self.contests.clone())
            .map_err(|err| BallotError::Serialization(err.to_string()));

        ret
    }

    /// Encodes hashable multi-contest data as a base64 string for JSON transport.
    ///
    /// # Errors
    ///
    /// Returns [`BallotError::Serialization`] when the payload cannot be encoded.
    pub fn serialize_contests<C: Ctx>(
        contest: &HashableMultiBallotContests<C>,
    ) -> Result<String, BallotError> {
        Base64Serialize::serialize(&contest)
    }
}

impl SignedHashableMultiBallot {
    /// Decodes contests via the intermediate [`HashableMultiBallot`] representation.
    ///
    /// # Errors
    ///
    /// Returns [`BallotError`] when contest decoding fails.
    pub fn deserialize_contests<C: Ctx>(
        &self,
    ) -> Result<HashableMultiBallotContests<C>, BallotError> {
        let hashable_ballot = HashableMultiBallot::try_from(self)?;

        hashable_ballot.deserialize_contests()
    }

    /// Encodes hashable multi-contest data (delegates to [`HashableMultiBallot`]).
    ///
    /// # Errors
    ///
    /// Returns [`BallotError::Serialization`] when the payload cannot be encoded.
    pub fn serialize_contests<C: Ctx>(
        contest: &HashableMultiBallotContests<C>,
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

        Ok(SignedHashableMultiBallot {
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

/// Signs a hashable multi-ballot with a freshly generated ephemeral voter key pair.
///
/// # Errors
///
/// Returns an error when signing bytes cannot be produced or the ephemeral key
/// cannot be generated or serialized.
pub fn sign_hashable_multi_ballot_with_ephemeral_voter_signing_key(
    ballot_id: &str,
    election_id: &str,
    hashable_multi_ballot: &HashableMultiBallot,
) -> Result<SignedContent, String> {
    // Get ballot_bytes_for_signing
    let content_bytes = hashable_multi_ballot
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

/// Verify the signature on a signed hashable multi-ballot.
///
/// # Errors
///
/// Returns an error when signature or public-key material is invalid or
/// verification fails.
pub fn verify_multi_ballot_signature(
    ballot_id: &str,
    election_id: &str,
    signed_hashable_multi_ballot: &SignedHashableMultiBallot,
) -> Result<Option<(StrandSignaturePk, StrandSignature)>, String> {
    let (signature, public_key) =
        if let (Some(voter_ballot_signature), Some(voter_signing_pk)) = (
            signed_hashable_multi_ballot.voter_ballot_signature.clone(),
            signed_hashable_multi_ballot.voter_signing_pk.clone(),
        ) {
            (voter_ballot_signature, voter_signing_pk)
        } else {
            return Ok(None);
        };

    let voter_signing_pk = StrandSignaturePk::from_der_b64_string(&public_key)
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

    let content = hashable_multi_ballot.strand_serialize().map_err(|err| {
        format!(
            "Failed to deserialize signature from hashable multi ballot: {}",
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
