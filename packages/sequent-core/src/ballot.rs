// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
#![allow(non_snake_case)]
#![allow(dead_code)]
use crate::encrypt::hash_ballot_style;
use crate::error::BallotError;
use crate::plaintext::{
    DecodedVoteChoice, DecodedVoteContest, PreferencialOrderErrorType,
};
use crate::serialization::base64::{Base64Deserialize, Base64Serialize};
use crate::serialization::deserialize_with_path::deserialize_value;
use crate::types::ceremonies::TallySessionResolutionData;
use crate::types::ceremonies::{
    CeremoniesPolicy, CountingAlgType, TallyOperation,
};
use crate::types::hasura::core::{self, Area, ElectionEvent};
use ::core::convert::TryInto;
use anyhow::anyhow;
use borsh::{BorshDeserialize, BorshSerialize};
use chrono::DateTime;
use chrono::Utc;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_path_to_error::Error;
use std::hash::Hash;
use std::ops::Deref;
use std::{collections::HashMap, default::Default};
use strand::elgamal::Ciphertext;
use strand::serialization::StrandSerialize;
use strand::signature::StrandSignature;
use strand::signature::StrandSignaturePk;
use strand::signature::StrandSignatureSk;
use strand::zkp::Schnorr;
use strand::{backend::ristretto::RistrettoCtx, context::Ctx};
use strum_macros::{Display, EnumString, IntoStaticStr};

/// Schema version for serializable ballot and election types in this crate.
///
/// Bumped when breaking changes are introduced to serialized ballot structures.
pub const TYPES_VERSION: u32 = 1;

/// Localized content keyed by BCP 47 language tag (e.g. `"en"`, `"es"`).
pub type I18nContent<T = Option<String>> = HashMap<String, T>;

/// Custom string metadata attached to elections, contests, or candidates.
pub type Annotations = HashMap<String, String>;

/// A voter's encrypted choice.
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Eq, Debug, Clone)]
pub struct ReplicationChoice<C: Ctx> {
    /// ElGamal ciphertext encoding the voter's selection for one contest.
    pub ciphertext: Ciphertext<C>,
    /// Plaintext vote encoded in the contest's mixed-radix representation.
    pub plaintext: C::P,
    /// Encryption randomness, exposed for voter-side auditability.
    pub randomness: C::X,
}

#[derive(
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    JsonSchema,
    PartialEq,
    Eq,
    Debug,
    Clone,
)]
/// Election public-key material bundled with a demo-mode indicator.
pub struct PublicKeyConfig {
    /// Base64-encoded election public key used to encrypt ballots.
    pub public_key: String,
    /// When true, the election uses demo keys and is not production-safe.
    pub is_demo: bool,
}

/// One contest entry inside an auditable ballot, including audit data.
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Eq, Debug, Clone)]
pub struct AuditableBallotContest<C: Ctx> {
    /// Identifier of the contest this ciphertext belongs to.
    pub contest_id: String,
    /// Encrypted choice with plaintext and randomness for voter verification.
    pub choice: ReplicationChoice<C>,
    /// Schnorr proof that the ciphertext is a valid encryption of the plaintext.
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

/// Ballot representation for end-to-end verifiability.
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct AuditableBallot {
    /// [`TYPES_VERSION`] of the ballot JSON schema.
    pub version: u32,
    /// ISO 8601 timestamp when the ballot was generated.
    pub issue_date: String,
    /// Ballot style defining which contests and keys apply to this voter.
    pub config: BallotStyle,
    /// Base64-encoded auditable contest payloads.
    pub contests: Vec<String>, // Vec<AuditableBallotContest<C>>,
    /// Hash fingerprint of the ballot contents for tracking and verification.
    pub ballot_hash: String,
    /// Optional voter ephemeral signing public key, if the ballot was signed.
    pub voter_signing_pk: Option<String>,
    /// Optional Ed25519 signature over the hashable ballot bytes.
    pub voter_ballot_signature: Option<String>,
}

impl AuditableBallot {
    /// Decodes each base64 contest string into a typed auditable contest.
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

    /// Encodes auditable contests as base64 strings for JSON transport.
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

/// One contest entry in a hashable ballot (ciphertext only, no audit material).
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Eq, Debug, Clone)]
pub struct HashableBallotContest<C: Ctx> {
    /// Identifier of the contest this ciphertext belongs to.
    pub contest_id: String,
    /// ElGamal ciphertext for the voter's selection.
    pub ciphertext: Ciphertext<C>,
    /// Proof that the ciphertext is a valid encryption.
    pub proof: Schnorr<C>,
}

/// Ballot representation used for hashing and cast submission.
#[derive(
    BorshSerialize, Serialize, Deserialize, PartialEq, Eq, Debug, Clone,
)]
pub struct HashableBallot {
    /// [`TYPES_VERSION`] of the ballot schema.
    pub version: u32,
    /// ISO 8601 timestamp when the ballot was generated.
    pub issue_date: String,
    /// Base64-encoded hashable contest payloads.
    pub contests: Vec<String>, // Vec<HashableBallotContest<C>>,
    /// Serialized ballot style configuration.
    pub config: String,
    /// Hash of the ballot style, binding the ciphertexts to a specific layout.
    pub ballot_style_hash: String,
}

/// Hashable ballot with optional voter signature.
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct SignedHashableBallot {
    /// [`TYPES_VERSION`] of the ballot schema.
    pub version: u32,
    /// ISO 8601 timestamp when the ballot was generated.
    pub issue_date: String,
    /// Base64-encoded hashable contest payloads.
    pub contests: Vec<String>,
    /// Serialized ballot style configuration.
    pub config: String,
    /// Hash of the ballot style.
    pub ballot_style_hash: String,
    /// Voter ephemeral signing public key, when the ballot was signed.
    pub voter_signing_pk: Option<String>,
    /// Ed25519 signature over the canonical ballot signing bytes.
    pub voter_ballot_signature: Option<String>,
}

/// In-memory hashable ballot with typed contest entries (not base64-wrapped).
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Eq, Debug, Clone)]
pub struct RawHashableBallot<C: Ctx> {
    /// [`TYPES_VERSION`] of the ballot schema.
    pub version: u32,
    /// ISO 8601 timestamp when the ballot was generated.
    pub issue_date: String,
    /// Decoded contest ciphertexts ready for hashing or further processing.
    pub contests: Vec<HashableBallotContest<C>>,
}

impl HashableBallot {
    /// Decodes each base64 contest string into a typed hashable contest.
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

    /// Encodes hashable contests as base64 strings for JSON transport.
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
    /// Decodes contests via the intermediate [`HashableBallot`] representation.
    pub fn deserialize_contests<C: Ctx>(
        &self,
    ) -> Result<Vec<HashableBallotContest<C>>, BallotError> {
        let hashable_ballot = HashableBallot::try_from(self)?;

        hashable_ballot.deserialize_contests()
    }

    /// Encodes hashable contests as base64 strings (delegates to [`HashableBallot`]).
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

/// Ephemeral voter signing key material produced when signing a ballot.
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct SignedContent {
    /// Base64-encoded Ed25519 public key of the ephemeral voter signing key.
    pub public_key: String,
    /// Base64-encoded signature over the canonical ballot signing bytes.
    pub signature: String,
}

/// Signs a hashable ballot with a freshly generated ephemeral voter key pair.
///
/// The signed payload binds `ballot_id`, `election_id`, and the serialized
/// hashable ballot bytes. Returns the public key and signature to attach to
/// the cast ballot.
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

/// Verifies the voter signature on a signed hashable ballot, if present.
///
/// Returns `Ok(None)` when no signature fields are set. On success returns the
/// deserialized public key and signature that were verified.
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

/// Builds the canonical byte sequence signed by the voter.
///
/// Concatenates length-prefixed `ballot_id`, `election_id`, and serialized
/// ballot content so signatures are bound to a specific ballot instance and
/// election.
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

#[derive(
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    JsonSchema,
    PartialEq,
    Eq,
    Debug,
    Clone,
)]
/// External link or media resource associated with a candidate.
pub struct CandidateUrl {
    /// Target URL (image, website, social profile, etc.).
    pub url: String,
    /// Semantic kind of link (e.g. `"website"`, `"image"`).
    pub kind: Option<String>,
    /// Accessible title or label for the link.
    pub title: Option<String>,
    /// When true, the URL points to an image resource.
    pub is_image: bool,
}

/// UI and behavioral flags that control how a candidate is displayed and behaves.
#[derive(
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    JsonSchema,
    PartialEq,
    Eq,
    Debug,
    Clone,
    Default,
)]
pub struct CandidatePresentation {
    /// Nested localized labels for candidate-specific UI strings.
    pub i18n: Option<I18nContent<I18nContent<Option<String>>>>,
    /// Marks a dedicated "invalid vote" choice shown to the voter.
    pub is_explicit_invalid: Option<bool>,
    /// Marks a dedicated "blank vote" choice shown to the voter.
    pub is_explicit_blank: Option<bool>,
    /// When true, the candidate cannot be selected.
    pub is_disabled: Option<bool>,
    /// When true, the row is a non-selectable category heading.
    pub is_category_list: Option<bool>,
    /// Placement of the explicit invalid option (`"top"` or `"bottom"`).
    pub invalid_vote_position: Option<String>,
    /// When true, the row accepts free-text write-in input.
    pub is_write_in: Option<bool>,
    /// Admin-defined ordering index within the contest candidate list.
    pub sort_order: Option<i64>,
    /// Optional media and reference URLs for the candidate.
    pub urls: Option<Vec<CandidateUrl>>,
    /// Presentation subtype for specialized rendering (e.g. party list entries).
    pub subtype: Option<String>,
}

impl CandidatePresentation {
    /// Returns a presentation with conservative defaults (no special flags set).
    pub fn new() -> CandidatePresentation {
        CandidatePresentation {
            i18n: None,
            is_explicit_invalid: Some(false),
            is_explicit_blank: Some(false),
            is_disabled: Some(false),
            is_category_list: Some(false),
            invalid_vote_position: None,
            is_write_in: Some(false),
            sort_order: None,
            urls: None,
            subtype: None,
        }
    }
}

#[derive(
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    JsonSchema,
    PartialEq,
    Eq,
    Debug,
    Clone,
    Default,
)]
/// A selectable option (or special row) within a contest.
pub struct Candidate {
    /// Unique candidate identifier.
    pub id: String,
    /// Owning tenant identifier.
    pub tenant_id: String,
    /// Parent election event identifier.
    pub election_event_id: String,
    /// Parent election identifier.
    pub election_id: String,
    /// Contest this candidate belongs to.
    pub contest_id: String,
    /// Default display name (non-localized fallback).
    pub name: Option<String>,
    /// Localized display names keyed by language tag.
    pub name_i18n: Option<I18nContent>,
    /// Default description text.
    pub description: Option<String>,
    /// Localized descriptions keyed by language tag.
    pub description_i18n: Option<I18nContent>,
    /// Short label or acronym shown in compact layouts.
    pub alias: Option<String>,
    /// Localized aliases keyed by language tag.
    pub alias_i18n: Option<I18nContent>,
    /// Candidate classification (person, party, option, etc.).
    pub candidate_type: Option<String>,
    /// Rendering and interaction flags for the voting UI.
    pub presentation: Option<CandidatePresentation>,
    /// Admin-defined metadata attached by administrators.
    pub annotations: Option<Annotations>,
}

impl Candidate {
    /// Returns whether this row is a category heading rather than a choice.
    pub fn is_category_list(&self) -> bool {
        self.presentation
            .as_ref()
            .map(|presentation| presentation.is_category_list)
            .flatten()
            .unwrap_or(false)
    }

    /// Returns whether this row is the explicit invalid-vote option.
    pub fn is_explicit_invalid(&self) -> bool {
        self.presentation
            .as_ref()
            .map(|presentation| presentation.is_explicit_invalid)
            .flatten()
            .unwrap_or(false)
    }

    /// Returns whether this row is the explicit blank-vote option.
    pub fn is_explicit_blank(&self) -> bool {
        self.presentation
            .as_ref()
            .map(|presentation| presentation.is_explicit_blank)
            .flatten()
            .unwrap_or(false)
    }

    /// Returns whether this candidate is disabled and cannot be selected.
    pub fn is_disabled(&self) -> bool {
        self.presentation
            .as_ref()
            .map(|presentation| presentation.is_disabled)
            .flatten()
            .unwrap_or(false)
    }

    /// Returns whether this row accepts free-text write-in input.
    pub fn is_write_in(&self) -> bool {
        self.presentation
            .as_ref()
            .map(|presentation| presentation.is_write_in)
            .flatten()
            .unwrap_or(false)
    }

    /// Updates the write-in flag, creating a default presentation if needed.
    pub fn set_is_write_in(&mut self, is_write_in: bool) {
        let mut presentation =
            self.presentation.clone().unwrap_or(Default::default());
        presentation.is_write_in = Some(is_write_in);
        self.presentation = Some(presentation);
    }
}

#[derive(
    Debug,
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    JsonSchema,
    Clone,
    EnumString,
    Display,
    Default,
)]
/// How candidates are ordered in the voting UI for a contest.
pub enum CandidatesOrder {
    /// Pseudorandom order, typically seeded per voter session.
    #[strum(serialize = "random")]
    #[serde(rename = "random")]
    Random,
    /// Order defined by administrator `sort_order` values.
    #[strum(serialize = "custom")]
    #[serde(rename = "custom")]
    Custom,
    /// Alphabetical order by display name.
    #[strum(serialize = "alphabetical")]
    #[serde(rename = "alphabetical")]
    #[default]
    Alphabetical,
}

/// Whether voters may cast ballots before the official voting period opens.
#[derive(
    Debug,
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    JsonSchema,
    Clone,
    Copy,
    EnumString,
    Display,
    Default,
)]
pub enum EarlyVotingPolicy {
    /// Early voting is permitted according to election schedule rules.
    #[strum(serialize = "allow_early_voting")]
    #[serde(rename = "allow_early_voting")]
    AllowEarlyVoting,
    /// Ballots may only be cast during the configured voting window.
    #[strum(serialize = "no_early_voting")]
    #[serde(rename = "no_early_voting")]
    #[default]
    NoEarlyVoting,
}

/// How contests are ordered in the voting UI within an election.
#[derive(
    Debug,
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    JsonSchema,
    Clone,
    EnumString,
    Display,
    Default,
)]
pub enum ContestsOrder {
    /// Pseudorandom order, typically seeded per voter session.
    #[strum(serialize = "random")]
    #[serde(rename = "random")]
    Random,
    /// Order defined by administrator configuration.
    #[strum(serialize = "custom")]
    #[serde(rename = "custom")]
    Custom,
    /// Alphabetical order by contest title.
    #[strum(serialize = "alphabetical")]
    #[serde(rename = "alphabetical")]
    #[default]
    Alphabetical,
}

/// Whether the cast-vote confirmation screen uses a high-contrast "gold" style.
#[derive(
    Debug,
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    JsonSchema,
    Clone,
    EnumString,
    Display,
    Default,
)]
pub enum CastVoteGoldLevelPolicy {
    /// Show the gold-level confirmation styling.
    #[strum(serialize = "gold-level")]
    #[serde(rename = "gold-level")]
    GoldLevel,
    /// Use the standard confirmation styling.
    #[strum(serialize = "no-gold-level")]
    #[serde(rename = "no-gold-level")]
    #[default]
    NoGoldLevel,
}

/// Which name appears as the title on the voting portal start screen.
#[derive(
    Debug,
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    JsonSchema,
    Clone,
    EnumString,
    Display,
    Default,
)]
pub enum StartScreenTitlePolicy {
    /// Show the current election name.
    #[strum(serialize = "election")]
    #[serde(rename = "election")]
    #[default]
    Election,
    /// Show the parent election event name.
    #[strum(serialize = "election-event")]
    #[serde(rename = "election-event")]
    ElectionEvent,
}

/// Whether voters must acknowledge e-security information before voting.
#[derive(
    Debug,
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    JsonSchema,
    Clone,
    EnumString,
    Display,
    Default,
)]
pub enum ESecurityConfirmationPolicy {
    /// No e-security confirmation step is required.
    #[strum(serialize = "none")]
    #[serde(rename = "none")]
    #[default]
    NONE,
    /// Voters must confirm e-security information before proceeding.
    #[strum(serialize = "mandatory")]
    #[serde(rename = "mandatory")]
    MANDATORY,
}

/// Visibility of the ballot audit button in the voting UI.
#[allow(non_camel_case_types)]
#[derive(
    Debug,
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    JsonSchema,
    Clone,
    EnumString,
    Display,
    Default,
)]
pub enum AuditButtonCfg {
    /// Show the audit button in the main voting chrome.
    #[strum(serialize = "show")]
    #[serde(rename = "show")]
    #[default]
    SHOW,
    /// Hide the audit button entirely.
    #[strum(serialize = "not-show")]
    #[serde(rename = "not-show")]
    NOT_SHOW,
    /// Show the audit entry point only inside the help section.
    #[strum(serialize = "show-in-help")]
    #[serde(rename = "show-in-help")]
    SHOW_IN_HELP,
}

/// Whether the cast-vote screen exposes a technical logs tab to voters.
#[derive(
    Debug,
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    JsonSchema,
    Clone,
    EnumString,
    Display,
    Default,
)]
pub enum ShowCastVoteLogs {
    /// Display a logs tab with cast-vote diagnostic details.
    #[strum(serialize = "show-logs-tab")]
    #[serde(rename = "show-logs-tab")]
    ShowLogsTab,
    /// Hide the logs tab from the cast-vote screen.
    #[strum(serialize = "hide-logs-tab")]
    #[serde(rename = "hide-logs-tab")]
    #[default]
    HideLogsTab,
}

#[derive(
    Debug,
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    JsonSchema,
    Clone,
    EnumString,
    Display,
    Default,
)]
/// How elections are ordered on the voting portal election list.
pub enum ElectionsOrder {
    /// Pseudorandom order, typically seeded per voter session.
    #[strum(serialize = "random")]
    #[serde(rename = "random")]
    Random,
    /// Order defined by administrator `sort_order` values.
    #[strum(serialize = "custom")]
    #[serde(rename = "custom")]
    Custom,
    /// Alphabetical order by election title.
    #[strum(serialize = "alphabetical")]
    #[serde(rename = "alphabetical")]
    #[default]
    Alphabetical,
}

/// A single election within an election event, including its contests.
#[derive(
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    JsonSchema,
    PartialEq,
    Eq,
    Debug,
    Clone,
)]
pub struct Election {
    /// Unique election identifier.
    pub id: String,
    /// Parent election event identifier.
    pub election_event_id: String,
    /// Owning tenant identifier.
    pub tenant_id: String,
    /// Default display name (non-localized fallback).
    pub name: Option<String>,
    /// Localized display names keyed by language tag.
    pub name_i18n: Option<I18nContent>,
    /// Default description text.
    pub description: Option<String>,
    /// Localized descriptions keyed by language tag.
    pub description_i18n: Option<I18nContent>,
    /// Short label or acronym shown in compact layouts.
    pub alias: Option<String>,
    /// Localized aliases keyed by language tag.
    pub alias_i18n: Option<I18nContent>,
    /// Reference to an uploaded image document for the election.
    pub image_document_id: Option<String>,
    /// Contests that voters can vote on within this election.
    pub contests: Vec<Contest>,
    /// JSON presentation configuration controlling voting UI behavior.
    pub presentation: Option<ElectionPresentation>,
    /// Admin-defined metadata attached by administrators.
    pub annotations: Option<Annotations>,
}

#[allow(non_camel_case_types)]
#[derive(
    Debug,
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    JsonSchema,
    Clone,
    EnumString,
    Display,
    Default,
)]
/// How invalid votes are handled.
pub enum InvalidVotePolicy {
    /// Invalid votes are accepted without warning.
    #[strum(serialize = "allowed")]
    #[serde(rename = "allowed")]
    #[default]
    ALLOWED,
    /// Warn the voter but allow casting an invalid vote.
    #[strum(serialize = "warn")]
    #[serde(rename = "warn")]
    WARN,
    /// Warn for both implicit and explicit invalid vote patterns.
    #[strum(serialize = "warn-invalid-implicit-and-explicit")]
    #[serde(rename = "warn-invalid-implicit-and-explicit")]
    WARN_INVALID_IMPLICIT_AND_EXPLICIT,
    /// Block invalid votes from being cast.
    #[strum(serialize = "not-allowed")]
    #[serde(rename = "not-allowed")]
    NOT_ALLOWED,
}

/// How selecting one candidate affects other selections in the same contest.
#[derive(
    Debug,
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    JsonSchema,
    Clone,
    EnumString,
    Display,
)]
pub enum CandidatesSelectionPolicy {
    /// Single-choice behavior: selecting one option deselects the previous one.
    #[strum(serialize = "radio")]
    #[serde(rename = "radio")]
    RADIO, // if you select one, the previously selected one gets unselected
    /// Multi-choice behavior: selections accumulate up to the contest maximum.
    #[strum(serialize = "cumulative")]
    #[serde(rename = "cumulative")]
    CUMULATIVE, // default behaviour
}

/// Icon style used for candidate selection controls in the voting UI.
#[derive(
    Debug,
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    JsonSchema,
    Clone,
    EnumString,
    Display,
    Default,
)]
pub enum CandidatesIconCheckboxPolicy {
    /// Square checkbox icon (default for multi-select contests).
    #[strum(serialize = "square-checkbox")]
    #[serde(rename = "square-checkbox")]
    #[default]
    SQUARE_CHECKBOX, // Checkbox icon by default
    /// Round radio-button icon (default for single-select contests).
    #[strum(serialize = "round-checkbox")]
    #[serde(rename = "round-checkbox")]
    ROUND_CHECKBOX, // RadioButton icon
}

/// Scope at which trustee key-generation ceremonies are configured.
#[allow(non_camel_case_types)]
#[derive(
    Debug,
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    JsonSchema,
    Clone,
    EnumString,
    Display,
    Default,
)]
pub enum KeysCeremonyPolicy {
    /// Keys are generated once per election event and shared across elections.
    #[strum(serialize = "ELECTION_EVENT")]
    #[serde(rename = "ELECTION_EVENT")]
    #[default]
    ELECTION_EVENT,
    /// Each election has its own independent key ceremony.
    #[strum(serialize = "ELECTION")]
    #[serde(rename = "ELECTION")]
    ELECTION,
}

/// Feature flags for downloadable election-event materials.
#[derive(
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    JsonSchema,
    PartialEq,
    Eq,
    Debug,
    Clone,
    Default,
)]
pub struct ElectionEventMaterials {
    /// When true, supplementary materials are available to voters.
    pub activated: Option<bool>,
}

/// Language settings that apply across an entire election event.
#[derive(
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    JsonSchema,
    PartialEq,
    Eq,
    Debug,
    Clone,
    Default,
)]
pub struct ElectionEventLanguageConf {
    /// BCP 47 language codes offered in the voting portal.
    pub enabled_language_codes: Option<Vec<String>>,
    /// Language code used when no voter preference is available.
    pub default_language_code: Option<String>,
    /// How the portal picks a language when the voter has not chosen one.
    pub language_detection_policy: Option<LanguageDetectionPolicy>,
}

/// Voting portal presentation and policy settings for an election event.
#[derive(
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    JsonSchema,
    PartialEq,
    Eq,
    Debug,
    Clone,
    Default,
)]
pub struct ElectionEventPresentation {
    /// Nested localized UI strings for the election event.
    pub i18n: Option<I18nContent<I18nContent<Option<String>>>>,
    /// Controls availability of downloadable event materials.
    pub materials: Option<ElectionEventMaterials>,
    /// Language configuration for the voting portal.
    pub language_conf: Option<ElectionEventLanguageConf>,
    /// URL of the logo shown in the voting portal header.
    pub logo_url: Option<String>,
    /// URL to redirect the voter to after finishing all elections.
    pub redirect_finish_url: Option<String>,
    /// Custom CSS injected into the voting portal.
    pub css: Option<String>,
    /// When true, skip the election list and go directly to voting.
    pub skip_election_list: Option<bool>,
    /// When true, show the voter profile menu (defaults to true).
    pub show_user_profile: Option<bool>, // default is true
    /// Whether to expose cast-vote diagnostic logs to voters.
    pub show_cast_vote_logs: Option<ShowCastVoteLogs>,
    /// Ordering of elections on the portal election list.
    pub elections_order: Option<ElectionsOrder>,
    /// Countdown behavior before the voting period ends.
    pub voting_portal_countdown_policy: Option<VotingPortalCountdownPolicy>,
    /// Custom URL overrides for portal pages.
    pub custom_urls: Option<CustomUrls>,
    /// Whether keys are generated per event or per election.
    pub keys_ceremony_policy: Option<KeysCeremonyPolicy>,
    /// Whether contests are encrypted individually or as a batch.
    pub contest_encryption_policy: Option<ContestEncryptionPolicy>,
    /// Whether decoded ballots are included in exported data.
    pub decoded_ballot_inclusion_policy: Option<DecodedBallotsInclusionPolicy>,
    /// Restricts portal access to authorized voters only.
    pub locked_down: Option<LockedDown>,
    /// Controls when election results and data are published.
    pub publish_policy: Option<Publish>,
    /// Voter enrollment configuration for the event.
    pub enrollment: Option<Enrollment>,
    /// One-time-password authentication settings.
    pub otp: Option<Otp>,
    /// Whether and how voters must sign their cast ballots.
    pub voter_signing_policy: Option<VoterSigningPolicy>,
    /// Requirements for voter certificate authentication.
    pub voter_certificate_policy: Option<VoterCertificatePolicy>,
    /// Weighted voting rules when voters have unequal vote weights.
    pub weighted_voting_policy: Option<WeightedVotingPolicy>,
    /// Trustee ceremony configuration (tally, keys, etc.).
    pub ceremonies_policy: Option<CeremoniesPolicy>,
    /// Rules for delegated/proxy voting, if enabled.
    pub delegated_voting_policy: Option<DelegatedVotingPolicy>,
}

impl ElectionEvent {
    /// Deserializes the raw JSON presentation into a typed configuration struct.
    pub fn get_presentation(
        &self,
    ) -> Result<Option<ElectionEventPresentation>, Error<serde_json::Error>>
    {
        self.presentation
            .clone()
            .map(|presentation_value| deserialize_value(presentation_value))
            .transpose()
    }
}

#[allow(non_camel_case_types)]
#[derive(
    Debug,
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    JsonSchema,
    Clone,
    EnumString,
    Display,
)]
/// Policy for grace period.
pub enum EGracePeriodPolicy {
    /// No grace period after the official end time.
    #[strum(serialize = "no-grace-period")]
    #[serde(rename = "no-grace-period")]
    NO_GRACE_PERIOD,
    /// Allow casting during a grace window without alerting the voter.
    #[strum(serialize = "grace-period-without-alert")]
    #[serde(rename = "grace-period-without-alert")]
    GRACE_PERIOD_WITHOUT_ALERT,
}

/// Scheduled start and end timestamps for a voting period.
#[derive(
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    JsonSchema,
    PartialEq,
    Eq,
    Debug,
    Clone,
    Default,
)]
pub struct VotingPeriodDates {
    /// ISO 8601 start of the voting period.
    pub start_date: Option<String>,
    /// ISO 8601 end of the voting period.
    pub end_date: Option<String>,
}

/// Policy for whether Initialize Report is required to start voting.
#[allow(non_camel_case_types)]
#[derive(
    Debug,
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    JsonSchema,
    Clone,
    EnumString,
    Display,
)]
pub enum EInitializeReportPolicy {
    /// Initialize Report is required
    #[strum(serialize = "required")]
    #[serde(rename = "required")]
    REQUIRED,
    /// Initialize Report is optional.
    #[strum(serialize = "not-required")]
    #[serde(rename = "not-required")]
    NOT_REQUIRED,
}

impl Default for EInitializeReportPolicy {
    fn default() -> Self {
        EInitializeReportPolicy::NOT_REQUIRED
    }
}

#[derive(
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    JsonSchema,
    PartialEq,
    Eq,
    Debug,
    Clone,
    Default,
)]
/// Countdown timer configuration shown as the voting period approaches its end.
pub struct VotingPortalCountdownPolicy {
    /// Which countdown mode to use (none, silent, or with alert).
    pub policy: Option<ECountdownPolicy>,
    /// Seconds before period end when the countdown becomes visible.
    pub countdown_anticipation_secs: Option<u64>,
    /// Seconds before period end when an alert is shown to the voter.
    pub countdown_alert_anticipation_secs: Option<u64>,
}

/// Countdown display mode for the end of a voting period.
#[allow(non_camel_case_types)]
#[derive(
    Debug,
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    JsonSchema,
    Clone,
    EnumString,
    Display,
)]
pub enum ECountdownPolicy {
    /// No countdown is shown.
    NO_COUNTDOWN,
    /// Show a countdown timer without an alert dialog.
    COUNTDOWN,
    /// Show a countdown timer and alert the voter as time runs out.
    COUNTDOWN_WITH_ALERT,
}

/// Policy for under-votes.
#[allow(non_camel_case_types)]
#[derive(
    Debug,
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    JsonSchema,
    Copy,
    Clone,
    EnumString,
    Display,
    Default,
)]
pub enum EUnderVotePolicy {
    /// Under-votes are accepted without warning.
    #[strum(serialize = "allowed")]
    #[serde(rename = "allowed")]
    #[default]
    ALLOWED,
    /// Warn the voter about unfilled selections.
    #[strum(serialize = "warn")]
    #[serde(rename = "warn")]
    WARN,
    /// Warn only on the ballot review screen, not while voting.
    #[strum(serialize = "warn-only-in-review")]
    #[serde(rename = "warn-only-in-review")]
    WARN_ONLY_IN_REVIEW,
    /// Warn and show an alert dialog before allowing the under-vote.
    #[strum(serialize = "warn-and-alert")]
    #[serde(rename = "warn-and-alert")]
    WARN_AND_ALERT,
}

/// Policy for blank votes.
#[allow(non_camel_case_types)]
#[derive(
    Debug,
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    JsonSchema,
    Copy,
    Clone,
    EnumString,
    Display,
    Default,
)]
pub enum EBlankVotePolicy {
    /// Blank votes are accepted without warning.
    #[strum(serialize = "allowed")]
    #[serde(rename = "allowed")]
    #[default]
    ALLOWED,
    /// Warn the voter before accepting a blank vote.
    #[strum(serialize = "warn")]
    #[serde(rename = "warn")]
    WARN,
    /// Warn only on the ballot review screen.
    #[strum(serialize = "warn-only-in-review")]
    #[serde(rename = "warn-only-in-review")]
    WARN_ONLY_IN_REVIEW,
    /// Blank votes are not permitted.
    #[strum(serialize = "not-allowed")]
    #[serde(rename = "not-allowed")]
    NOT_ALLOWED,
}

/// How the voting UI handles selections exceeding the contest maximum.
#[allow(non_camel_case_types)]
#[derive(
    Debug,
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    JsonSchema,
    Copy,
    Clone,
    EnumString,
    Display,
    Default,
)]
pub enum EOverVotePolicy {
    /// Over-votes are silently accepted.
    #[strum(serialize = "allowed")]
    #[serde(rename = "allowed")]
    ALLOWED,
    /// Over-votes are accepted but a message is shown.
    #[strum(serialize = "allowed-with-msg")]
    #[serde(rename = "allowed-with-msg")]
    ALLOWED_WITH_MSG,
    /// Over-votes are accepted with a message and alert dialog.
    #[strum(serialize = "allowed-with-msg-and-alert")]
    #[serde(rename = "allowed-with-msg-and-alert")]
    #[default]
    ALLOWED_WITH_MSG_AND_ALERT,
    /// Over-votes are blocked with a message and alert dialog.
    #[strum(serialize = "not-allowed-with-msg-and-alert")]
    #[serde(rename = "not-allowed-with-msg-and-alert")]
    NOT_ALLOWED_WITH_MSG_AND_ALERT,
    /// Over-votes are blocked, the excess selection is disabled, and a message is shown.
    #[strum(serialize = "not-allowed-with-msg-and-disable")]
    #[serde(rename = "not-allowed-with-msg-and-disable")]
    NOT_ALLOWED_WITH_MSG_AND_DISABLE,
}

/// How preferential contests handle duplicate rank assignments.
#[allow(non_camel_case_types)]
#[derive(
    Debug,
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    JsonSchema,
    Copy,
    Clone,
    EnumString,
    Display,
    Default,
)]
pub enum EDuplicatedRankPolicy {
    /// Duplicate ranks are allowed after a warning dialog.
    #[strum(serialize = "allowed-warn-and-dialog")]
    #[serde(rename = "allowed-warn-and-dialog")]
    #[default]
    ALLOWED_WARN_AND_DIALOG,
    /// Duplicate ranks are rejected after a warning dialog.
    #[strum(serialize = "not-allowed-warn-and-dialog")]
    #[serde(rename = "not-allowed-warn-and-dialog")]
    NOT_ALLOWED_WARN_AND_DIALOG,
}

/// How preferential contests handle gaps in the ranking sequence (e.g. 1, 3 without 2).
#[allow(non_camel_case_types)]
#[derive(
    Debug,
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    JsonSchema,
    Copy,
    Clone,
    EnumString,
    Display,
    Default,
)]
pub enum EPreferenceGapsPolicy {
    /// Gaps are allowed after a warning dialog.
    #[strum(serialize = "allowed-warn-and-dialog")]
    #[serde(rename = "allowed-warn-and-dialog")]
    #[default]
    ALLOWED_WARN_AND_DIALOG,
    /// Gaps are rejected after a warning dialog.
    #[strum(serialize = "not-allowed-warn-and-dialog")]
    #[serde(rename = "not-allowed-warn-and-dialog")]
    NOT_ALLOWED_WARN_AND_DIALOG,
}

/// Presentation and policy settings for a single election.
#[derive(
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    JsonSchema,
    PartialEq,
    Eq,
    Debug,
    Clone,
)]
pub struct ElectionPresentation {
    /// Nested localized UI strings for this election.
    pub i18n: Option<I18nContent<I18nContent<Option<String>>>>,
    /// Scheduled voting period start and end dates.
    pub dates: Option<VotingPeriodDates>,
    /// Language settings overriding the election-event defaults.
    pub language_conf: Option<ElectionEventLanguageConf>,
    /// Ordering of contests within this election.
    pub contests_order: Option<ContestsOrder>,
    /// Visibility of the ballot audit button.
    pub audit_button_cfg: Option<AuditButtonCfg>,
    /// Admin-defined position in the election list.
    pub sort_order: Option<i64>,
    /// When true, require explicit confirmation before casting.
    pub cast_vote_confirm: Option<bool>,
    /// High-contrast "gold level" styling on the cast-vote screen.
    pub cast_vote_gold_level: Option<CastVoteGoldLevelPolicy>,
    /// Which title to show on the voting portal start screen.
    pub start_screen_title_policy: Option<StartScreenTitlePolicy>,
    /// Legacy grace-period flag (prefer [`EGracePeriodPolicy`]).
    pub is_grace_priod: Option<bool>,
    /// How the grace period after voting end is handled.
    pub grace_period_policy: Option<EGracePeriodPolicy>,
    /// Duration of the post-deadline grace period in seconds.
    pub grace_period_secs: Option<u64>,
    /// Whether voters may file an initialization report.
    pub init_report: Option<InitReport>,
    /// Whether administrators may manually open the voting period.
    pub manual_start_voting_period: Option<ManualStartVotingPeriod>,
    /// Whether administrators may manually close the voting period.
    pub voting_period_end: Option<VotingPeriodEnd>,
    /// When tallying is permitted relative to the voting schedule.
    pub tally: Option<Tally>,
    /// Whether an initialization report is required before voting.
    pub initialization_report_policy: Option<EInitializeReportPolicy>,
    /// Whether voters must acknowledge e-security information.
    pub security_confirmation_policy: Option<ESecurityConfirmationPolicy>,
    /// How consolidated reports are generated for this election.
    pub consolidated_report_policy: Option<ConsolidatedReportPolicy>,
    /// The policy to determine if the voter can decline to vote for an election level.
    pub decline_to_vote_policy: Option<DeclineToVotePolicy>,
}

impl core::Election {
    /// Deserializes the raw JSON presentation, returning `None` on parse failure.
    pub fn get_presentation(&self) -> Option<ElectionPresentation> {
        let election_presentation: Option<ElectionPresentation> = self
            .presentation
            .clone()
            .map(|value| deserialize_value(value).ok())
            .flatten();

        election_presentation
    }
}

impl Default for ElectionPresentation {
    fn default() -> ElectionPresentation {
        ElectionPresentation {
            init_report: Some(InitReport::ALLOWED),
            manual_start_voting_period: Some(ManualStartVotingPeriod::ALLOWED),
            voting_period_end: Some(VotingPeriodEnd::DISALLOWED),
            tally: Some(Tally::ALWAYS_ALLOW),
            i18n: None,
            dates: None,
            language_conf: None,
            contests_order: None,
            audit_button_cfg: None,
            sort_order: None,
            cast_vote_confirm: None,
            cast_vote_gold_level: Some(CastVoteGoldLevelPolicy::NoGoldLevel),
            start_screen_title_policy: Some(StartScreenTitlePolicy::Election),
            is_grace_priod: None,
            grace_period_policy: None,
            grace_period_secs: None,
            initialization_report_policy: None,
            security_confirmation_policy: None,
            consolidated_report_policy: Some(
                ConsolidatedReportPolicy::default(),
            ),
            decline_to_vote_policy: Some(DeclineToVotePolicy::default()),
        }
    }
}

#[derive(
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    JsonSchema,
    PartialEq,
    Eq,
    Debug,
    Clone,
    Default,
)]
/// Presentation settings scoped to a geographic or organizational area.
pub struct AreaPresentation {
    /// Whether voters in this area may cast before the official voting period.
    pub allow_early_voting: Option<EarlyVotingPolicy>,
}

impl AreaPresentation {
    /// Returns true when early voting is explicitly enabled for this area.
    pub fn is_early_voting(&self) -> bool {
        self.allow_early_voting.clone().unwrap_or_default()
            == EarlyVotingPolicy::AllowEarlyVoting
    }
}

#[derive(
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    JsonSchema,
    PartialEq,
    Eq,
    Debug,
    Clone,
    Default,
)]
/// Localized labels for a candidate subtype within a typed contest.
pub struct SubtypePresentation {
    /// Default display name for the subtype.
    pub name: Option<String>,
    /// Localized subtype names keyed by language tag.
    pub name_i18n: Option<I18nContent<Option<String>>>,
    /// Ordering index among subtypes of the same candidate type.
    pub sort_order: Option<i64>,
}

/// Localized labels for a candidate type grouping within a contest.
#[derive(
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    JsonSchema,
    PartialEq,
    Eq,
    Debug,
    Clone,
    Default,
)]
pub struct TypePresentation {
    /// Default display name for the candidate type.
    pub name: Option<String>,
    /// Localized type names keyed by language tag.
    pub name_i18n: Option<I18nContent<Option<String>>>,
    /// Ordering index among candidate types in the contest.
    pub sort_order: Option<i64>,
    /// Per-subtype presentation overrides keyed by subtype identifier.
    pub subtypes_presentation:
        Option<HashMap<String, Option<SubtypePresentation>>>,
}

/// Presentation and policy settings for a single contest.
#[derive(
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    JsonSchema,
    PartialEq,
    Eq,
    Debug,
    Clone,
)]
pub struct ContestPresentation {
    /// Nested localized UI strings for this contest.
    pub i18n: Option<I18nContent<I18nContent<Option<String>>>>,
    /// Whether free-text write-in candidates are accepted.
    pub allow_writeins: Option<bool>,
    /// Restrict write-in characters to Base32 alphabet.
    pub base32_writeins: Option<bool>,
    /// How implicit and explicit invalid votes are handled.
    pub invalid_vote_policy: Option<InvalidVotePolicy>,
    /// How under-votes (fewer than max selections) are handled.
    pub under_vote_policy: Option<EUnderVotePolicy>,
    /// How intentionally blank votes are handled.
    pub blank_vote_policy: Option<EBlankVotePolicy>,
    /// How over-votes (more than max selections) are handled.
    pub over_vote_policy: Option<EOverVotePolicy>,
    /// How duplicate ranks in preferential voting are handled.
    pub duplicated_rank_policy: Option<EDuplicatedRankPolicy>,
    /// How gaps in preferential ranking sequences are handled.
    pub preference_gaps_policy: Option<EPreferenceGapsPolicy>,
    /// Candidate list pagination mode.
    pub pagination_policy: Option<String>,
    /// Number of checkboxes shown per candidate in cumulative contests.
    pub cumulative_number_of_checkboxes: Option<u64>,
    /// When true, shuffle category groupings on the ballot.
    pub shuffle_categories: Option<bool>,
    /// Explicit list of category IDs to shuffle (when shuffling is enabled).
    pub shuffle_category_list: Option<Vec<String>>,
    /// When true, show point values next to candidates (e.g. Borda contests).
    pub show_points: Option<bool>,
    /// Whether voters can select lists, candidates, or both (`disabled`,
    /// `allow-selecting-candidates-and-lists`, allow-selecting-candidates|allow-selecting-lists).
    pub enable_checkable_lists: Option<String>,
    /// Collapsible list behavior (`disabled`, `enabled-expanded`, `enabled-collapsed`).
    pub collapsible_lists: Option<String>,
    /// Ordering of candidates within the contest.
    pub candidates_order: Option<CandidatesOrder>,
    /// Single-choice vs cumulative selection behavior.
    pub candidates_selection_policy: Option<CandidatesSelectionPolicy>,
    /// Checkbox vs radio icon style for selection controls.
    pub candidates_icon_checkbox_policy: Option<CandidatesIconCheckboxPolicy>,
    /// Maximum selections allowed per candidate type.
    pub max_selections_per_type: Option<u64>,
    /// Per-type presentation overrides keyed by candidate type identifier.
    pub types_presentation: Option<HashMap<String, Option<TypePresentation>>>,
    /// Admin-defined ordering index within the election contest list.
    pub sort_order: Option<i64>,
    /// Number of columns used to lay out candidates in the voting UI.
    pub columns: Option<u64>,
}

impl ContestPresentation {
    /// Returns a presentation with permissive defaults suitable for new contests.
    pub fn new() -> ContestPresentation {
        ContestPresentation {
            i18n: None,
            allow_writeins: Some(true),
            base32_writeins: Some(true),
            invalid_vote_policy: Some(InvalidVotePolicy::ALLOWED),
            blank_vote_policy: Some(EBlankVotePolicy::ALLOWED),
            over_vote_policy: Some(EOverVotePolicy::ALLOWED),
            pagination_policy: Some("".to_owned()),
            cumulative_number_of_checkboxes: None,
            shuffle_categories: Some(false),
            shuffle_category_list: None,
            show_points: Some(false),
            enable_checkable_lists: None,
            collapsible_lists: None,
            candidates_order: None,
            candidates_selection_policy: None,
            candidates_icon_checkbox_policy: None,
            max_selections_per_type: None,
            types_presentation: None,
            sort_order: None,
            under_vote_policy: Some(EUnderVotePolicy::ALLOWED),
            duplicated_rank_policy: Some(EDuplicatedRankPolicy::default()),
            preference_gaps_policy: Some(EPreferenceGapsPolicy::default()),
            columns: None,
        }
    }
}

impl Default for ContestPresentation {
    fn default() -> Self {
        ContestPresentation::new()
    }
}

#[derive(
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    JsonSchema,
    PartialEq,
    Eq,
    Debug,
    Clone,
    Default,
)]
/// A contest within an election.
pub struct Contest {
    /// Unique contest identifier.
    pub id: String,
    /// Owning tenant identifier.
    pub tenant_id: String,
    /// Parent election event identifier.
    pub election_event_id: String,
    /// Parent election identifier.
    pub election_id: String,
    /// Default display title (non-localized fallback).
    pub name: Option<String>,
    /// Localized titles keyed by language tag.
    pub name_i18n: Option<I18nContent>,
    /// Default description text.
    pub description: Option<String>,
    /// Localized descriptions keyed by language tag.
    pub description_i18n: Option<I18nContent>,
    /// Short label or acronym shown in compact layouts.
    pub alias: Option<String>,
    /// Localized aliases keyed by language tag.
    pub alias_i18n: Option<I18nContent>,
    /// Maximum number of selections a voter may make.
    pub max_votes: i64,
    /// Minimum number of selections required for a valid vote.
    pub min_votes: i64,
    /// Number of candidates that will be elected (winners).
    pub winning_candidates_num: i64,
    /// Voting interaction type (e.g. preferential, cumulative).
    pub voting_type: Option<String>,
    /// Algorithm used to count ballots and determine winners.
    /// plurality-at-large|borda-nauru|borda|borda-mas-madrid|desborda3|desborda2|desborda|cumulative
    pub counting_algorithm: Option<CountingAlgType>,
    /// When true, ballot choices for this contest are encrypted before casting.
    pub is_encrypted: bool,
    /// Candidates for this contest.
    pub candidates: Vec<Candidate>,
    /// Presentation configuration for this contest.
    pub presentation: Option<ContestPresentation>,
    /// ISO 8601 creation timestamp.
    pub created_at: Option<String>,
    /// Admin-defined metadata (e.g. tie-resolution data) attached by administrators.
    pub annotations: Option<Annotations>,
    /// How ties are broken during counting.
    pub tie_breaking_policy: Option<TieBreakingPolicy>,
}

impl Contest {
    /// Returns whether write-in candidates are allowed (defaults to false).
    pub fn allow_writeins(&self) -> bool {
        self.presentation
            .as_ref()
            .map(|presentation| presentation.allow_writeins)
            .flatten()
            .unwrap_or(false)
    }

    /// Returns the configured counting algorithm, defaulting to plurality-at-large.
    pub fn get_counting_algorithm(&self) -> CountingAlgType {
        self.counting_algorithm.unwrap_or_default()
    }

    /// Returns whether write-ins are restricted to Base32 characters (defaults to true).
    pub fn base32_writeins(&self) -> bool {
        self.presentation
            .as_ref()
            .map(|presentation| presentation.base32_writeins)
            .flatten()
            .unwrap_or(true)
    }

    /// Get the invalid vote policy configuration value from the presentation.
    /// If the value or the parent object is not set, return the default value.
    pub fn get_invalid_vote_policy(&self) -> InvalidVotePolicy {
        match self
            .presentation
            .as_ref()
            .map(|presentation| &presentation.invalid_vote_policy)
        {
            Some(policy) => policy.clone().unwrap_or_default(),
            _ => InvalidVotePolicy::default(),
        }
    }

    /// Returns the number of cumulative checkboxes per candidate (defaults to 1).
    pub fn cumulative_number_of_checkboxes(&self) -> u64 {
        self.presentation
            .as_ref()
            .map(|presentation| {
                presentation.cumulative_number_of_checkboxes.unwrap_or(1)
            })
            .unwrap_or(1)
    }

    /// Returns whether point values are shown next to candidates.
    pub fn show_points(&self) -> bool {
        self.presentation
            .as_ref()
            .map(|presentation| presentation.show_points)
            .flatten()
            .unwrap_or(false)
    }

    /// Returns IDs of candidates marked as explicit invalid-vote options.
    pub fn get_invalid_candidate_ids(&self) -> Vec<String> {
        self.candidates
            .iter()
            .filter(|candidate| candidate.is_explicit_invalid())
            .collect::<Vec<&Candidate>>()
            .iter()
            .map(|candidate| candidate.id.clone())
            .collect()
    }

    /// Get the tie-breaking policy configuration value.
    /// If the value is not set, return the default value (RANDOM).
    pub fn get_tie_breaking_policy(&self) -> TieBreakingPolicy {
        self.tie_breaking_policy.clone().unwrap_or_default()
    }

    /// Get per-round tie resolutions from contest annotations.
    pub fn get_tie_resolutions(&self) -> Vec<TallySessionResolutionData> {
        self.annotations
            .as_ref()
            .and_then(|annotations| annotations.get("tie_resolutions"))
            .and_then(|json_str| {
                // Since Annotations stores strings, we just parse the string directly into our Vec
                serde_json::from_str::<Vec<TallySessionResolutionData>>(
                    json_str,
                )
                .ok()
            })
            .unwrap_or_default()
    }

    /// Stores per-round tie resolutions in the contest `annotations` map.
    pub fn insert_tie_resolutions(
        contest: &mut Contest,
        contest_tie_resolutions: &Vec<TallySessionResolutionData>,
    ) -> anyhow::Result<()> {
        // Only inject if there is actually data to add
        if !contest_tie_resolutions.is_empty() {
            // Serialize the data back into a JSON string
            let tie_res_json_string =
                serde_json::to_string(&contest_tie_resolutions)?;

            // Clone existing annotations or create a new map if it's None
            let mut annotations =
                contest.annotations.clone().unwrap_or_default();

            // Insert the stringified JSON into the annotations map
            annotations
                .insert("tie_resolutions".to_string(), tie_res_json_string);

            contest.annotations = Some(annotations);
        }

        Ok(())
    }
}

#[allow(non_camel_case_types)]
#[derive(
    BorshSerialize,
    BorshDeserialize,
    Default,
    Display,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Eq,
    Clone,
    EnumString,
    JsonSchema,
)]
/// Whether self-service voter enrollment is enabled for the election event.
pub enum Enrollment {
    /// Voters may enroll themselves through the portal.
    #[default]
    #[strum(serialize = "enabled")]
    #[serde(rename = "enabled")]
    ENABLED,
    /// Enrollment is disabled; voters must be pre-registered.
    #[strum(serialize = "disabled")]
    #[serde(rename = "disabled")]
    DISABLED,
}

/// Whether one-time-password authentication is required for voters.
#[allow(non_camel_case_types)]
#[derive(
    BorshSerialize,
    BorshDeserialize,
    Default,
    Display,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Eq,
    Clone,
    EnumString,
    JsonSchema,
)]
pub enum Otp {
    /// OTP authentication is active.
    #[default]
    #[strum(serialize = "enabled")]
    #[serde(rename = "enabled")]
    ENABLED,
    /// OTP authentication is not used.
    #[strum(serialize = "disabled")]
    #[serde(rename = "disabled")]
    DISABLED,
}

/// Whether decoded (plaintext) ballots are included in exported election data.
#[allow(non_camel_case_types)]
#[derive(
    BorshSerialize,
    BorshDeserialize,
    Default,
    Display,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Eq,
    Clone,
    EnumString,
    JsonSchema,
)]
pub enum DecodedBallotsInclusionPolicy {
    /// Decoded ballots are included in exports.
    #[strum(serialize = "included")]
    #[serde(rename = "included")]
    INCLUDED,
    /// Only encrypted ballot data is exported.
    #[default]
    #[strum(serialize = "not-included")]
    #[serde(rename = "not-included")]
    NOT_INCLUDED,
}

/// Whether the voter encrypts all contests in one operation or one at a time.
#[allow(non_camel_case_types)]
#[derive(
    BorshSerialize,
    BorshDeserialize,
    Default,
    Display,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Eq,
    Clone,
    EnumString,
    JsonSchema,
)]
pub enum ContestEncryptionPolicy {
    /// Encrypt multiple contests in a single client-side operation.
    #[strum(serialize = "multiple-contests")]
    #[serde(rename = "multiple-contests")]
    MULTIPLE_CONTESTS,
    /// Encrypt one contest at a time as the voter progresses.
    #[default]
    #[strum(serialize = "single-contest")]
    #[serde(rename = "single-contest")]
    SINGLE_CONTEST,
}

/// Configuration for voter signing policy.
#[allow(non_camel_case_types)]
#[derive(
    BorshSerialize,
    BorshDeserialize,
    Default,
    Display,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Eq,
    Clone,
    EnumString,
    JsonSchema,
)]
pub enum VoterSigningPolicy {
    /// Votes are not signed with the voter's signature.
    #[default]
    #[strum(serialize = "no-signature")]
    #[serde(rename = "no-signature")]
    NO_SIGNATURE,
    /// Votes are signed with the voter's signature.
    #[strum(serialize = "with-signature")]
    #[serde(rename = "with-signature")]
    WITH_SIGNATURE,
}

/// Whether voters must authenticate with a digital certificate.
#[allow(non_camel_case_types)]
#[derive(
    BorshSerialize,
    BorshDeserialize,
    Default,
    Display,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Eq,
    Clone,
    EnumString,
    JsonSchema,
)]
pub enum VoterCertificatePolicy {
    /// Certificate-based voter authentication is not required.
    #[default]
    #[strum(serialize = "disabled")]
    #[serde(rename = "disabled")]
    DISABLED,
    /// Voters must present a valid certificate to access the portal.
    #[strum(serialize = "enabled")]
    #[serde(rename = "enabled")]
    ENABLED,
}

/// Whether the election event is in lockdown.
#[allow(non_camel_case_types)]
#[derive(
    BorshSerialize,
    BorshDeserialize,
    Default,
    Display,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Eq,
    Clone,
    EnumString,
    JsonSchema,
)]
pub enum LockedDown {
    /// Event configuration is frozen and access is restricted.
    #[strum(serialize = "locked-down")]
    #[serde(rename = "locked-down")]
    LOCKED_DOWN,
    /// Normal operation; configuration may still be edited.
    #[default]
    #[strum(serialize = "not-locked-down")]
    #[serde(rename = "not-locked-down")]
    NOT_LOCKED_DOWN,
}

/// Configuration for whether able to publish.
#[allow(non_camel_case_types)]
#[derive(
    BorshSerialize,
    BorshDeserialize,
    Default,
    Display,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Eq,
    Clone,
    EnumString,
    JsonSchema,
)]
pub enum Publish {
    /// Results are published as soon as they are available.
    #[default]
    #[strum(serialize = "always")]
    #[serde(rename = "always")]
    ALWAYS,
    /// Publishing is enabled only after the election event is locked down.
    #[strum(serialize = "after-lockdown")]
    #[serde(rename = "after-lockdown")]
    AFTER_LOCKDOWN,
}

/// Runtime voting state for an election event across all channels.
#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
#[serde(default)]
pub struct ElectionEventStatus {
    /// Whether results and public data have been published.
    pub is_published: Option<bool>,
    /// Voting status for the online channel.
    pub voting_status: VotingStatus,
    /// Voting status for kiosk (in-person) channel.
    pub kiosk_voting_status: VotingStatus,
    /// Voting status for the early-voting channel.
    pub early_voting_status: VotingStatus,
    /// Scheduled dates for the online voting period.
    pub voting_period_dates: PeriodDates,
    /// Scheduled dates for the kiosk voting period.
    pub kiosk_voting_period_dates: PeriodDates,
    /// Scheduled dates for the early-voting period.
    pub early_voting_period_dates: PeriodDates,
}

impl Default for ElectionEventStatus {
    fn default() -> Self {
        ElectionEventStatus {
            is_published: Some(false),
            voting_status: VotingStatus::NOT_STARTED,
            kiosk_voting_status: VotingStatus::NOT_STARTED,
            early_voting_status: VotingStatus::NOT_STARTED,
            voting_period_dates: Default::default(),
            kiosk_voting_period_dates: Default::default(),
            early_voting_period_dates: Default::default(),
        }
    }
}

impl ElectionEventStatus {
    /// Returns the current [`VotingStatus`] for the given delivery channel.
    pub fn status_by_channel(
        &self,
        channel: VotingStatusChannel,
    ) -> VotingStatus {
        match channel {
            VotingStatusChannel::ONLINE => self.voting_status.clone(),
            VotingStatusChannel::KIOSK => self.kiosk_voting_status.clone(),
            VotingStatusChannel::EARLY_VOTING => {
                self.early_voting_status.clone()
            }
        }
    }

    /// Close EARLY_VOTING channel's status automatically if the new online
    /// status is OPEN or CLOSED
    pub fn close_early_voting_if_online_status_change(
        &mut self,
        channel: VotingStatusChannel,
        new_status: VotingStatus,
    ) {
        let should_close_early_voting = channel == VotingStatusChannel::ONLINE
            && (new_status == VotingStatus::OPEN
                || new_status == VotingStatus::CLOSED);

        if should_close_early_voting
            && self.status_by_channel(VotingStatusChannel::EARLY_VOTING)
                != VotingStatus::NOT_STARTED
        {
            self.set_status_by_channel(
                VotingStatusChannel::EARLY_VOTING,
                VotingStatus::CLOSED,
            );
        }
    }

    /// Updates the status for a channel and records the transition timestamp.
    pub fn set_status_by_channel(
        &mut self,
        channel: VotingStatusChannel,
        new_status: VotingStatus,
    ) {
        let mut period_dates = match channel {
            VotingStatusChannel::ONLINE => {
                self.voting_status = new_status.clone();
                &mut self.voting_period_dates
            }
            VotingStatusChannel::KIOSK => {
                self.kiosk_voting_status = new_status.clone();
                &mut self.kiosk_voting_period_dates
            }
            VotingStatusChannel::EARLY_VOTING => {
                self.early_voting_status = new_status.clone();
                &mut self.early_voting_period_dates
            }
        };
        period_dates.update_period_dates(&new_status);
    }
}

#[allow(non_camel_case_types)]
#[derive(
    BorshSerialize,
    BorshDeserialize,
    Display,
    Default,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Eq,
    Clone,
    Copy,
    EnumString,
    JsonSchema,
    IntoStaticStr,
)]
/// Lifecycle state of a voting channel (online, kiosk, or early voting).
pub enum VotingStatus {
    /// Voting has not been opened on this channel yet.
    #[default]
    NOT_STARTED,
    /// Voters may cast ballots on this channel.
    OPEN,
    /// Voting is temporarily suspended; no new ballots are accepted.
    PAUSED,
    /// Voting has ended on this channel.
    CLOSED,
}

impl VotingStatus {
    /// Returns true when the channel has never been opened.
    pub fn is_not_started(&self) -> bool {
        match self {
            VotingStatus::NOT_STARTED => true,
            VotingStatus::OPEN => false,
            VotingStatus::PAUSED => false,
            VotingStatus::CLOSED => false,
        }
    }

    /// Returns true once the channel has left [`NOT_STARTED`] at least once.
    pub fn is_started(&self) -> bool {
        !self.is_not_started()
    }

    /// Returns true when the channel accepts ballots ([`OPEN`] or [`PAUSED`]).
    pub fn is_open(&self) -> bool {
        match self {
            VotingStatus::NOT_STARTED => false,
            VotingStatus::OPEN => true,
            VotingStatus::PAUSED => true,
            VotingStatus::CLOSED => false,
        }
    }

    /// Returns true when voting is suspended but may resume.
    pub fn is_paused(&self) -> bool {
        match self {
            VotingStatus::NOT_STARTED => false,
            VotingStatus::OPEN => false,
            VotingStatus::PAUSED => true,
            VotingStatus::CLOSED => false,
        }
    }

    /// Returns true when voting has permanently ended on this channel.
    pub fn is_closed(&self) -> bool {
        match self {
            VotingStatus::NOT_STARTED => false,
            VotingStatus::OPEN => false,
            VotingStatus::PAUSED => false,
            VotingStatus::CLOSED => true,
        }
    }

    /// Returns true for [`NOT_STARTED`] or [`CLOSED`] — channels that reject new ballots.
    pub fn is_closed_or_never_started(&self) -> bool {
        match self {
            VotingStatus::NOT_STARTED => true,
            VotingStatus::OPEN => false,
            VotingStatus::PAUSED => false,
            VotingStatus::CLOSED => true,
        }
    }
}

#[allow(non_camel_case_types)]
#[derive(
    BorshSerialize,
    BorshDeserialize,
    Default,
    Display,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Eq,
    Clone,
    EnumString,
    JsonSchema,
    IntoStaticStr,
)]
/// Whether administrators may start the tally ceremony for an election.
pub enum AllowTallyStatus {
    /// Tallying may begin regardless of voting status.
    #[default]
    #[strum(serialize = "allowed")]
    #[serde(rename = "allowed")]
    ALLOWED,
    /// Tallying is blocked until an administrator explicitly allows it.
    #[strum(serialize = "disallowed")]
    #[serde(rename = "disallowed")]
    DISALLOWED,
    /// Tallying is permitted only after all voting channels have closed.
    #[strum(serialize = "requires-voting-period-end")]
    #[serde(rename = "requires-voting-period-end")]
    REQUIRES_VOTING_PERIOD_END,
}

/// Delivery channel whose voting status is tracked independently.
#[allow(non_camel_case_types)]
#[derive(
    BorshSerialize,
    BorshDeserialize,
    Display,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Eq,
    Clone,
    Copy,
    EnumString,
    JsonSchema,
    IntoStaticStr,
)]
pub enum VotingStatusChannel {
    /// Standard remote voting through the web portal.
    ONLINE,
    /// In-person voting at a physical kiosk.
    KIOSK,
    /// Voting before the official period opens (when enabled by policy).
    EARLY_VOTING,
}

impl VotingStatusChannel {
    /// Looks up whether this channel is enabled in the election's channel configuration.
    pub fn channel_from(
        &self,
        channels: &core::VotingChannels,
    ) -> Option<bool> {
        match self {
            &VotingStatusChannel::ONLINE => channels.online.clone(),
            &VotingStatusChannel::KIOSK => channels.kiosk.clone(),
            &VotingStatusChannel::EARLY_VOTING => channels.early_voting.clone(),
        }
    }
}

#[derive(
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    Debug,
    Clone,
)]
/// Counters for voter notification messages sent across an election event.
pub struct ElectionEventStatistics {
    /// Total invitation or notification emails dispatched.
    pub num_emails_sent: Option<i64>,
    /// Total SMS notifications dispatched.
    pub num_sms_sent: Option<i64>,
}

impl Default for ElectionEventStatistics {
    fn default() -> Self {
        ElectionEventStatistics {
            num_emails_sent: Some(0),
            num_sms_sent: Some(0),
        }
    }
}

#[derive(
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    Debug,
    Clone,
)]
/// Counters for voter notification messages sent within a single election.
pub struct ElectionStatistics {
    /// Total invitation or notification emails dispatched.
    pub num_emails_sent: Option<i64>,
    /// Total SMS notifications dispatched.
    pub num_sms_sent: Option<i64>,
}

impl Default for ElectionStatistics {
    fn default() -> Self {
        ElectionStatistics {
            num_emails_sent: Some(0),
            num_sms_sent: Some(0),
        }
    }
}

#[allow(non_camel_case_types)]
#[derive(
    BorshSerialize,
    BorshDeserialize,
    Default,
    Display,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Eq,
    Clone,
    EnumString,
    JsonSchema,
)]
/// Whether voters may file an initialization report before casting.
pub enum InitReport {
    /// Initialization reports are permitted.
    #[default]
    #[strum(serialize = "allowed")]
    #[serde(rename = "allowed")]
    ALLOWED,
    /// Initialization reports are not available.
    #[strum(serialize = "disallowed")]
    #[serde(rename = "disallowed")]
    DISALLOWED,
}

/// Whether administrators may manually open the voting period.
#[allow(non_camel_case_types)]
#[derive(
    BorshSerialize,
    BorshDeserialize,
    Default,
    Display,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Eq,
    Clone,
    EnumString,
    JsonSchema,
)]
pub enum ManualStartVotingPeriod {
    /// Administrators may start voting at any time.
    #[default]
    #[strum(serialize = "allowed")]
    #[serde(rename = "allowed")]
    ALLOWED,
    /// Manual start is allowed only after an initialization report has been filed.
    #[strum(serialize = "only-when-initialization-report-has-been-performed")]
    #[serde(rename = "only-when-initialization-report-has-been-performed")]
    ONLY_WHEN_INITIALIZATION_REPORT_HAS_BEEN_PERFORMED,
}

/// Whether administrators may manually close the voting period.
#[allow(non_camel_case_types)]
#[derive(
    BorshSerialize,
    BorshDeserialize,
    Default,
    Display,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Eq,
    Clone,
    EnumString,
    JsonSchema,
)]
pub enum VotingPeriodEnd {
    /// Administrators may end voting before the scheduled deadline.
    #[default]
    #[strum(serialize = "allowed")]
    #[serde(rename = "allowed")]
    ALLOWED,
    /// Voting ends only at the scheduled time.
    #[strum(serialize = "disallowed")]
    #[serde(rename = "disallowed")]
    DISALLOWED,
}

/// When the tally ceremony may be started relative to the voting schedule.
#[allow(non_camel_case_types)]
#[derive(
    BorshSerialize,
    BorshDeserialize,
    Default,
    Display,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Eq,
    Clone,
    EnumString,
    JsonSchema,
)]
pub enum Tally {
    /// Tallying may begin at any time.
    #[default]
    #[strum(serialize = "always-allow")]
    #[serde(rename = "always-allow")]
    ALWAYS_ALLOW,
    /// Tallying is allowed only after the voting period has ended.
    #[strum(serialize = "allow-when-voting-period-ends")]
    #[serde(rename = "allow-when-voting-period-ends")]
    ONLY_WHEN_VOTING_PERIOD_ENDS,
}

/// Timestamps recording voting-period state transitions for a single channel.
#[derive(
    Serialize, Deserialize, PartialEq, Eq, JsonSchema, Debug, Clone, Default,
)]
pub struct PeriodDates {
    /// First time this channel was opened.
    pub first_started_at: Option<DateTime<Utc>>,
    /// Most recent time this channel was opened or resumed.
    pub last_started_at: Option<DateTime<Utc>>,
    /// First time this channel was paused.
    pub first_paused_at: Option<DateTime<Utc>>,
    /// Most recent time this channel was paused.
    pub last_paused_at: Option<DateTime<Utc>>,
    /// First time this channel was closed.
    pub first_stopped_at: Option<DateTime<Utc>>,
    /// Most recent time this channel was closed.
    pub last_stopped_at: Option<DateTime<Utc>>,
}

#[derive(
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    JsonSchema,
    Debug,
    Clone,
    Default,
)]
/// RFC 3339 string representation of [`PeriodDates`] for JSON transport.
pub struct StringifiedPeriodDates {
    /// First open timestamp as an ISO 8601 string.
    pub first_started_at: Option<String>,
    /// Most recent open/resume timestamp as an ISO 8601 string.
    pub last_started_at: Option<String>,
    /// First pause timestamp as an ISO 8601 string.
    pub first_paused_at: Option<String>,
    /// Most recent pause timestamp as an ISO 8601 string.
    pub last_paused_at: Option<String>,
    /// First close timestamp as an ISO 8601 string.
    pub first_stopped_at: Option<String>,
    /// Most recent close timestamp as an ISO 8601 string.
    pub last_stopped_at: Option<String>,
    /// Scheduled automation events keyed by event identifier.
    pub scheduled_event_dates: Option<HashMap<String, ScheduledEventDates>>,
}

/// Date range metadata embedded in generated PDF reports.
#[derive(
    Serialize, Deserialize, PartialEq, Eq, JsonSchema, Debug, Clone, Default,
)]
pub struct ReportDates {
    /// Report coverage period start (ISO 8601).
    pub start_date: String,
    /// Report coverage period end (ISO 8601).
    pub end_date: String,
    /// Election day label used in the report header.
    pub election_date: String,
}

/// Execution timestamps for a scheduled automation event.
#[derive(
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    JsonSchema,
    Debug,
    Clone,
    Default,
)]
pub struct ScheduledEventDates {
    /// When the event was scheduled to run (ISO 8601).
    pub scheduled_at: Option<String>,
    /// When the event actually completed or was cancelled (ISO 8601).
    pub stopped_at: Option<String>,
}

impl PeriodDates {
    fn update_period_dates(&mut self, new_status: &VotingStatus) {
        let (first, last) = match new_status {
            VotingStatus::NOT_STARTED => {
                // nothing to do
                return;
            }
            VotingStatus::OPEN => {
                (&mut self.first_started_at, &mut self.last_started_at)
            }
            VotingStatus::PAUSED => {
                (&mut self.first_paused_at, &mut self.last_paused_at)
            }
            VotingStatus::CLOSED => {
                (&mut self.first_stopped_at, &mut self.last_stopped_at)
            }
        };
        *last = Some(Utc::now());
        if first.is_none() {
            *first = last.clone();
        }
    }

    /// Converts typed timestamps to RFC 3339 strings for ballot-style JSON.
    pub fn to_string_fields(&self) -> StringifiedPeriodDates {
        StringifiedPeriodDates {
            first_started_at: format_date_opt(&self.first_started_at),
            last_started_at: format_date_opt(&self.last_started_at),
            first_paused_at: format_date_opt(&self.first_paused_at),
            last_paused_at: format_date_opt(&self.last_paused_at),
            first_stopped_at: format_date_opt(&self.first_stopped_at),
            last_stopped_at: format_date_opt(&self.last_stopped_at),
            scheduled_event_dates: Default::default(),
        }
    }
}

/// Formats a UTC timestamp as RFC 3339, or returns `default` when absent.
pub fn format_date(date: &Option<DateTime<Utc>>, default: &str) -> String {
    date.map_or(default.to_string(), |d| d.to_rfc3339())
}

/// Formats a UTC timestamp as RFC 3339, returning `None` when absent.
pub fn format_date_opt(date: &Option<DateTime<Utc>>) -> Option<String> {
    date.map(|d| d.to_rfc3339())
}

/// Runtime voting state for a single election across all channels.
#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
#[serde(default)]
pub struct ElectionStatus {
    /// Whether results and public data have been published.
    pub is_published: Option<bool>,
    /// Voting status for the online channel.
    pub voting_status: VotingStatus,
    /// Whether initialization reports are allowed for this election.
    pub init_report: InitReport,
    /// Voting status for the kiosk channel.
    pub kiosk_voting_status: VotingStatus,
    /// Voting status for the early-voting channel.
    pub early_voting_status: VotingStatus,
    /// Transition timestamps for the online voting period.
    pub voting_period_dates: PeriodDates,
    /// Transition timestamps for the kiosk voting period.
    pub kiosk_voting_period_dates: PeriodDates,
    /// Transition timestamps for the early-voting period.
    pub early_voting_period_dates: PeriodDates,
    /// Whether administrators may start tallying for this election.
    pub allow_tally: AllowTallyStatus,
}

impl Default for ElectionStatus {
    fn default() -> Self {
        ElectionStatus {
            is_published: Some(false),
            voting_status: VotingStatus::NOT_STARTED,
            init_report: InitReport::ALLOWED,
            kiosk_voting_status: VotingStatus::NOT_STARTED,
            early_voting_status: VotingStatus::NOT_STARTED,
            voting_period_dates: Default::default(),
            kiosk_voting_period_dates: Default::default(),
            early_voting_period_dates: Default::default(),
            allow_tally: Default::default(),
        }
    }
}

impl ElectionStatus {
    /// Returns the current [`VotingStatus`] for the given delivery channel.
    pub fn status_by_channel(
        &self,
        channel: VotingStatusChannel,
    ) -> VotingStatus {
        match channel {
            VotingStatusChannel::ONLINE => self.voting_status.clone(),
            VotingStatusChannel::KIOSK => self.kiosk_voting_status.clone(),
            VotingStatusChannel::EARLY_VOTING => {
                self.early_voting_status.clone()
            }
        }
    }

    /// Returns the [`PeriodDates`] audit trail for the given channel.
    pub fn dates_by_channel(
        &self,
        channel: VotingStatusChannel,
    ) -> PeriodDates {
        match channel {
            VotingStatusChannel::ONLINE => self.voting_period_dates.clone(),
            VotingStatusChannel::KIOSK => {
                self.kiosk_voting_period_dates.clone()
            }
            VotingStatusChannel::EARLY_VOTING => {
                self.early_voting_period_dates.clone()
            }
        }
    }

    /// Close EARLY_VOTING channel's status automatically if the new online
    /// status is OPEN or CLOSED
    pub fn close_early_voting_if_online_status_change(
        &mut self,
        channel: VotingStatusChannel,
        new_status: VotingStatus,
    ) {
        let should_close_early_voting = channel == VotingStatusChannel::ONLINE
            && (new_status.is_open() || new_status.is_closed());

        if should_close_early_voting
            && self
                .status_by_channel(VotingStatusChannel::EARLY_VOTING)
                .is_started()
        {
            self.set_status_by_channel(
                VotingStatusChannel::EARLY_VOTING,
                VotingStatus::CLOSED,
            );
        }
    }

    /// Updates the status for a channel and records the transition timestamp.
    pub fn set_status_by_channel(
        &mut self,
        channel: VotingStatusChannel,
        new_status: VotingStatus,
    ) {
        let period_dates = match channel {
            VotingStatusChannel::ONLINE => {
                self.voting_status = new_status.clone();
                &mut self.voting_period_dates
            }
            VotingStatusChannel::KIOSK => {
                self.kiosk_voting_status = new_status.clone();
                &mut self.kiosk_voting_period_dates
            }
            VotingStatusChannel::EARLY_VOTING => {
                self.early_voting_status = new_status.clone();
                &mut self.early_voting_period_dates
            }
        };
        period_dates.update_period_dates(&new_status);
    }
}

/// Complete configuration delivered to a voter for casting a ballot.
///
/// Assembled by [`crate::ballot_style::create_ballot_style`] from Hasura data and
/// embedded in auditable ballots as the `config` field. Bundles the contests a
/// voter may access, encryption keys, presentation policies, and annotations.
#[derive(
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    Debug,
    Clone,
)]
pub struct BallotStyle {
    /// Unique ballot-style identifier.
    pub id: String,
    /// Owning tenant identifier.
    pub tenant_id: String,
    /// Parent election event identifier.
    pub election_event_id: String,
    /// Election this ballot style belongs to.
    pub election_id: String,
    /// Maximum number of times the voter may recast (revote).
    pub num_allowed_revotes: Option<i64>,
    /// Human-readable label for administrators.
    pub description: Option<String>,
    /// Election public key used to encrypt ballot choices.
    pub public_key: Option<PublicKeyConfig>,
    /// Geographic or organizational area scoping this ballot style.
    pub area_id: String,
    /// Area-level presentation overrides (e.g. early voting).
    pub area_presentation: Option<AreaPresentation>,
    /// Contests included in this ballot, with candidates and presentation.
    pub contests: Vec<Contest>,
    /// Event-wide portal presentation and policy settings.
    pub election_event_presentation: Option<ElectionEventPresentation>,
    /// Election-specific portal presentation and policy settings.
    pub election_presentation: Option<ElectionPresentation>,
    /// Voting-period timestamps serialized for the voting portal.
    pub election_dates: Option<StringifiedPeriodDates>,
    /// Event-level metadata passed through to the portal.
    pub election_event_annotations: Option<HashMap<String, String>>,
    /// Election-level metadata passed through to the portal.
    pub election_annotations: Option<HashMap<String, String>>,
    /// Parsed area-level metadata (weight, tally operation, etc.).
    pub area_annotations: Option<AreaAnnotations>,
}

#[derive(
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    JsonSchema,
    PartialEq,
    Eq,
    Debug,
    Clone,
    Default,
)]
/// Custom URL overrides for authentication flows in the voting portal.
pub struct CustomUrls {
    /// Override URL for the login page.
    pub login: Option<String>,
    /// Override URL for the voter enrollment page.
    pub enrollment: Option<String>,
    /// Override URL for SAML-based single sign-on.
    pub saml: Option<String>,
}

/// Vote weight for weighted-voting elections (defaults to 1 when unset).
#[derive(
    PartialEq,
    Eq,
    Debug,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    BorshSerialize,
    BorshDeserialize,
)]
pub struct Weight(Option<u64>);

impl Default for Weight {
    fn default() -> Self {
        Self { 0: Some(1) } // default weight is 1
    }
}

impl Deref for Weight {
    type Target = Option<u64>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(
    PartialEq,
    Eq,
    Debug,
    Clone,
    Serialize,
    Deserialize,
    BorshSerialize,
    BorshDeserialize,
    Default,
)]
/// Structured metadata stored on a geographic area.
pub struct AreaAnnotations {
    /// Vote weight applied when [`WeightedVotingPolicy::AREAS_WEIGHTED_VOTING`] is active.
    pub weight: Option<Weight>,
    /// How this area's ballots are combined during tallying.
    pub tally_operation: Option<TallyOperation>,
}

impl AreaAnnotations {
    /// Returns the configured weight, defaulting to 1.
    pub fn get_weight(&self) -> Weight {
        self.weight.unwrap_or_default()
    }
}

impl Area {
    /// Parses the raw JSON `annotations` field into structured area metadata.
    pub fn read_annotations(
        &self,
    ) -> Result<Option<AreaAnnotations>, Error<serde_json::Error>> {
        self.annotations
            .as_ref()
            .map(|v| {
                deserialize_value::<AreaAnnotations>(v.clone()).map_err(|e| {
                    anyhow!("failed to deserialize AreaAnnotations: error={e} raw={v}");
                    e
                })
            })
            .transpose()
    }
}

#[derive(
    BorshSerialize,
    BorshDeserialize,
    Display,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Eq,
    Clone,
    EnumString,
    Default,
    JsonSchema,
)]
/// Whether ballot choices carry unequal weight based on the voter's area.
pub enum WeightedVotingPolicy {
    /// All ballots count equally regardless of area.
    #[default]
    #[serde(rename = "disabled-weighted-voting")]
    DISABLED_WEIGHTED_VOTING,
    /// Each area's [`AreaAnnotations::weight`] scales the voter's ballot.
    #[serde(rename = "areas-weighted-voting")]
    AREAS_WEIGHTED_VOTING,
}

/// Whether proxy (delegated) voting is enabled for the election event.
#[derive(
    BorshSerialize,
    BorshDeserialize,
    Display,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Eq,
    Clone,
    EnumString,
    Default,
    JsonSchema,
)]
pub enum DelegatedVotingPolicy {
    /// Delegated voting is not available.
    #[default]
    #[serde(rename = "disabled")]
    DISABLED,
    /// Voters may cast ballots on behalf of delegates.
    #[serde(rename = "enabled")]
    ENABLED,
}

/// Whether a single consolidated PDF report is generated for the election.
#[derive(
    BorshSerialize,
    BorshDeserialize,
    Display,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Eq,
    Clone,
    EnumString,
    Default,
    JsonSchema,
)]
pub enum ConsolidatedReportPolicy {
    /// No consolidated report is produced.
    #[default]
    #[strum(serialize = "do-not-generate")]
    #[serde(rename = "do-not-generate")]
    DO_NOT_GENERATE,
    /// A consolidated report is generated after tallying.
    #[strum(serialize = "generate")]
    #[serde(rename = "generate")]
    GENERATE,
}

/// How tied results are resolved when counting produces equal scores.
#[allow(non_camel_case_types)]
#[derive(
    BorshSerialize,
    BorshDeserialize,
    Default,
    Display,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Eq,
    Clone,
    EnumString,
    JsonSchema,
)]
pub enum TieBreakingPolicy {
    /// Break ties randomly during counting.
    #[default]
    #[strum(serialize = "random")]
    #[serde(rename = "random")]
    RANDOM,
    /// Defer tie resolution to an external procedure recorded in annotations.
    #[strum(serialize = "external-procedure")]
    #[serde(rename = "external-procedure")]
    EXTERNAL_PROCEDURE,
}

#[allow(non_camel_case_types)]
#[derive(
    BorshSerialize,
    BorshDeserialize,
    Default,
    Display,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Eq,
    Clone,
    EnumString,
    JsonSchema,
)]
/// Language detection policy.
/// Used to determine which language to use initially across all surfaces
pub enum LanguageDetectionPolicy {
    #[default]
    #[strum(serialize = "browser-detect")]
    #[serde(rename = "browser-detect")]
    /// detect user's language through their browser.
    BROWSER_DETECT,
    /// skip browser detection, use default language code
    #[strum(serialize = "force-default")]
    #[serde(rename = "force-default")]
    FORCE_DEFAULT,
}

#[allow(non_camel_case_types)]
#[derive(
    BorshSerialize,
    BorshDeserialize,
    Display,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Eq,
    Clone,
    EnumString,
    Default,
    JsonSchema,
)]
/// Used to determine if the user can decline to vote.
pub enum DeclineToVotePolicy {
    #[default]
    #[strum(serialize = "disabled")]
    #[serde(rename = "disabled")]
    /// The user cannot decline to vote.
    DISABLED,
    #[strum(serialize = "enabled")]
    #[serde(rename = "enabled")]
    /// The user can decline to vote at the election level (Ballot Level)
    ENABLED,
}
