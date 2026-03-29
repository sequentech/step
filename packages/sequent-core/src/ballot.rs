// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
#![allow(non_snake_case)]
#![allow(dead_code)]
use crate::encrypt::hash_ballot_style;
use crate::error::BallotError;
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

/// Version number for ballot.
pub const TYPES_VERSION: u32 = 1;

/// Internationalized content map, keyed by language code.
pub type I18nContent<T = Option<String>> = HashMap<String, T>;

/// Annotations for ballots or contests, as key-value pairs.
pub type Annotations = HashMap<String, String>;

#[derive(BorshSerialize, BorshDeserialize, PartialEq, Eq, Debug, Clone)]
/// Represents a choice in a contest.
pub struct ReplicationChoice<C: Ctx> {
    /// Encrypted choice.
    pub ciphertext: Ciphertext<C>,
    /// Plaintext value of the choice.
    pub plaintext: C::P,
    /// Randomness used for encryption.
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
/// Configuration for a public key.
pub struct PublicKeyConfig {
    /// Public key as a string.
    pub public_key: String,
    /// Whether this is a demo key.
    pub is_demo: bool,
}

#[derive(BorshSerialize, BorshDeserialize, PartialEq, Eq, Debug, Clone)]
/// An auditable contest on a ballot.
pub struct AuditableBallotContest<C: Ctx> {
    /// Contest identifier.
    pub contest_id: String,
    /// The selected choice for the contest.
    pub choice: ReplicationChoice<C>,
    /// Proof for the choice.
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
/// An auditable ballot.
pub struct AuditableBallot {
    /// Ballot version.
    pub version: u32,
    /// Date the ballot was issued.
    pub issue_date: String,
    /// Ballot style configuration.
    pub config: BallotStyle,
    /// Serialized contests.
    pub contests: Vec<String>, // Vec<AuditableBallotContest<C>>,
    /// Hash of the ballot.
    pub ballot_hash: String,
    /// Voter's public signing key (if present).
    pub voter_signing_pk: Option<String>,
    /// Voter's ballot signature (if present).
    pub voter_ballot_signature: Option<String>,
}

impl AuditableBallot {
    /// Deserialize the stored contest strings into a vector of contests ballot.
    ///
    /// # Errors
    /// Returns `BallotError::Serialization` if any contest ballot fails to deserialize.
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

    /// Serialize a slice of contests ballot into base64 strings.
    ///
    /// # Errors
    /// Returns `BallotError::Serialization` if serialization fails.
    pub fn serialize_contests<C: Ctx>(
        contests: &[AuditableBallotContest<C>],
    ) -> Result<Vec<String>, BallotError> {
        contests
            .iter()
            .map(|auditable_ballot_contest| {
                Base64Serialize::serialize(auditable_ballot_contest)
            })
            .collect::<Vec<Result<String, BallotError>>>()
            .into_iter()
            .collect()
    }
}

#[derive(BorshSerialize, BorshDeserialize, PartialEq, Eq, Debug, Clone)]
/// Contest data for hashable ballots.
pub struct HashableBallotContest<C: Ctx> {
    /// Contest identifier.
    pub contest_id: String,
    /// Encrypted contest data.
    pub ciphertext: Ciphertext<C>,
    /// Proof for the contest.
    pub proof: Schnorr<C>,
}

#[derive(
    BorshSerialize, Serialize, Deserialize, PartialEq, Eq, Debug, Clone,
)]
/// Hashable ballot.
pub struct HashableBallot {
    /// Ballot version.
    pub version: u32,
    /// Date the ballot was issued.
    pub issue_date: String,
    /// Serialized contests.
    pub contests: Vec<String>, // Vec<HashableBallotContest<C>>,
    /// Ballot style configuration as a string.
    pub config: String,
    /// Hash of the ballot style.
    pub ballot_style_hash: String,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
/// Signed hashable ballot.
pub struct SignedHashableBallot {
    /// Ballot version.
    pub version: u32,
    /// Date the ballot was issued.
    pub issue_date: String,
    /// Serialized contests (base64-encoded).
    pub contests: Vec<String>,
    /// Configuration as a string.
    pub config: String,
    /// Hash of the ballot style.
    pub ballot_style_hash: String,
    /// Voter's public signing key (if present).
    pub voter_signing_pk: Option<String>,
    /// Voter's ballot signature (if present).
    pub voter_ballot_signature: Option<String>,
}

#[derive(BorshSerialize, BorshDeserialize, PartialEq, Eq, Debug, Clone)]
/// Raw hashable ballot data.
pub struct RawHashableBallot<C: Ctx> {
    /// Ballot version.
    pub version: u32,
    /// Date the ballot was issued.
    pub issue_date: String,
    /// Contests data for the ballot.
    pub contests: Vec<HashableBallotContest<C>>,
}

impl HashableBallot {
    /// # Errors
    /// Returns an error if deserialization of any ballot contest fails.
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

    /// Serialize a slice of hashable ballot contests.
    ///
    /// # Errors
    /// Returns `BallotError::Serialization` if serialization fails.
    pub fn serialize_contests<C: Ctx>(
        contests: &[HashableBallotContest<C>],
    ) -> Result<Vec<String>, BallotError> {
        contests
            .iter()
            .map(|hashable_ballot_contest| {
                Base64Serialize::serialize(hashable_ballot_contest)
            })
            .collect::<Vec<Result<String, BallotError>>>()
            .into_iter()
            .collect()
    }
}

impl SignedHashableBallot {
    /// Deserialize contests from the signed hashable ballot.
    ///
    /// # Errors
    /// Returns `BallotError::Serialization` on deserialization failure.
    pub fn deserialize_contests<C: Ctx>(
        &self,
    ) -> Result<Vec<HashableBallotContest<C>>, BallotError> {
        let hashable_ballot = HashableBallot::try_from(self)?;

        hashable_ballot.deserialize_contests()
    }

    /// # Errors
    /// Returns `BallotError::Serialization` if contest serialization fails.
    pub fn serialize_contests<C: Ctx>(
        contests: &[HashableBallotContest<C>],
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
            contests,
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
                "Unexpected version {:?}, expected {}",
                value.version, TYPES_VERSION
            )));
        }

        let contests = value.deserialize_contests::<RistrettoCtx>()?;
        let hashable_ballot_contest: Vec<HashableBallotContest<RistrettoCtx>> =
            contests
                .iter()
                .map(|auditable_ballot_contest| {
                    HashableBallotContest::<RistrettoCtx>::from(
                        auditable_ballot_contest,
                    )
                })
                .collect();
        let ballot_style_hash =
            hash_ballot_style(&value.config).map_err(|error| {
                BallotError::Serialization(format!(
                    "Failed to hash ballot style: {error}"
                ))
            })?;
        Ok(SignedHashableBallot {
            version: TYPES_VERSION,
            issue_date: value.issue_date.clone(),
            contests: HashableBallot::serialize_contests::<RistrettoCtx>(
                &hashable_ballot_contest,
            )?,
            config: value.config.id.clone(),
            ballot_style_hash,
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
                "Unexpected version {:?}, expected {}",
                value.version, TYPES_VERSION
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
/// Content for a signed ballot, including the public key and signature.
pub struct SignedContent {
    /// Public key used for signing.
    pub public_key: String,
    /// Signature value.
    pub signature: String,
}

/// # Errors
/// Returns an error if key generation, serialization, or signing fails.
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
/// # Errors
/// Returns an error if signature deserialization, hashing, or verification fails.
pub fn verify_ballot_signature(
    ballot_id: &str,
    election_id: &str,
    signed_hashable_ballot: &SignedHashableBallot,
) -> Result<Option<(StrandSignaturePk, StrandSignature)>, String> {
    let Some(voter_ballot_signature) =
        signed_hashable_ballot.voter_ballot_signature.as_ref()
    else {
        return Ok(None);
    };
    let Some(voter_signing_pk) =
        signed_hashable_ballot.voter_signing_pk.as_ref()
    else {
        return Ok(None);
    };
    let voter_ballot_signature = voter_ballot_signature.clone();
    let voter_signing_pk = voter_signing_pk.clone();

    let voter_signing_pk = StrandSignaturePk::from_der_b64_string(
        &voter_signing_pk,
    )
    .map_err(|err| {
        format!("Failed to deserialize signature from hashable ballot: {err}")
    })?;

    let hashable_ballot: HashableBallot =
        signed_hashable_ballot.try_into().map_err(|err| {
            format!("Failed to convert to hashable ballot: {err}")
        })?;

    let content = hashable_ballot.strand_serialize().map_err(|err| {
        format!("Failed to get bytes for signing from hashable ballot: {err}")
    })?;

    let ballot_bytes =
        get_ballot_bytes_for_signing(ballot_id, election_id, &content);

    let ballot_signature = StrandSignature::from_b64_string(
        &voter_ballot_signature,
    )
    .map_err(|err| {
        format!("Failed to deserialize signature from hashable ballot: {err}")
    })?;

    voter_signing_pk
        .verify(&ballot_signature, &ballot_bytes)
        .map_err(|err| format!("Failed to verify signature: {err}"))?;

    Ok(Some((voter_signing_pk, ballot_signature)))
}

#[must_use]
/// Get bytes of ballot content
pub fn get_ballot_bytes_for_signing(
    ballot_id: &str,
    election_id: &str,
    content: &[u8],
) -> Vec<u8> {
    let mut ret: Vec<u8> = vec![];

    let ballot_id_bytes = ballot_id.as_bytes();
    let ballot_id_length = (ballot_id_bytes.len() as u64).to_le_bytes();
    ret.extend_from_slice(&ballot_id_length);
    ret.extend_from_slice(ballot_id_bytes);

    let election_id_bytes = election_id.as_bytes();
    let election_id_length = (election_id_bytes.len() as u64).to_le_bytes();
    ret.extend_from_slice(&election_id_length);
    ret.extend_from_slice(election_id_bytes);

    let content_length = (content.len() as u64).to_le_bytes();
    ret.extend_from_slice(&content_length);
    ret.extend_from_slice(content);

    ret
}

/// URL and metadata for a candidate's resource (e.g., website or image).
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
pub struct CandidateUrl {
    /// The URL string.
    pub url: String,
    /// The kind/type of the URL.
    pub kind: Option<String>,
    /// The title or label for the URL.
    pub title: Option<String>,
    /// True if the URL points to an image resource.
    pub is_image: bool,
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
/// Presentation configuration for a candidate, including i18n, status, and display options.
pub struct CandidatePresentation {
    /// Internationalized content for the candidate.
    pub i18n: Option<I18nContent<I18nContent<Option<String>>>>,
    /// True if the candidate is explicitly marked as invalid.
    pub is_explicit_invalid: Option<bool>,
    /// True if the candidate is explicitly marked as blank.
    pub is_explicit_blank: Option<bool>,
    /// True if the candidate is disabled.
    pub is_disabled: Option<bool>,
    /// True if the candidate is a category list.
    pub is_category_list: Option<bool>,
    /// Position for invalid votes ("top" or "bottom").
    pub invalid_vote_position: Option<String>,
    /// True if the candidate is a write-in.
    pub is_write_in: Option<bool>,
    /// Sort order for display.
    pub sort_order: Option<i64>,
    /// List of URLs associated with the candidate.
    pub urls: Option<Vec<CandidateUrl>>,
    /// Subtype identifier for the candidate.
    pub subtype: Option<String>,
}

impl CandidatePresentation {
    /// Create a default candidate presentation config.
    #[must_use]
    pub const fn new() -> CandidatePresentation {
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
/// Candidate data structure.
pub struct Candidate {
    /// Unique candidate identifier.
    pub id: String,
    /// Tenant identifier.
    pub tenant_id: String,
    /// Election event identifier.
    pub election_event_id: String,
    /// Election identifier.
    pub election_id: String,
    /// Contest identifier.
    pub contest_id: String,
    /// Candidate name.
    pub name: Option<String>,
    /// Internationalized candidate name.
    pub name_i18n: Option<I18nContent>,
    /// Candidate description.
    pub description: Option<String>,
    /// Internationalized candidate description.
    pub description_i18n: Option<I18nContent>,
    /// Candidate alias.
    pub alias: Option<String>,
    /// Internationalized candidate alias.
    pub alias_i18n: Option<I18nContent>,
    /// Candidate type.
    pub candidate_type: Option<String>,
    /// Presentation configuration for the candidate.
    pub presentation: Option<CandidatePresentation>,
    /// Annotations for the candidate.
    pub annotations: Option<Annotations>,
}

impl Candidate {
    #[must_use]
    /// Checks if the candidate is a category list based on its presentation configuration.
    pub fn is_category_list(&self) -> bool {
        self.presentation
            .as_ref()
            .and_then(|presentation| presentation.is_category_list)
            .unwrap_or(false)
    }

    #[must_use]
    /// Checks if the candidate is explicitly marked as invalid based on its presentation configuration.
    pub fn is_explicit_invalid(&self) -> bool {
        self.presentation
            .as_ref()
            .and_then(|presentation| presentation.is_explicit_invalid)
            .unwrap_or(false)
    }

    #[must_use]
    /// Checks if the candidate is explicitly marked as blank based on its presentation configuration.
    pub fn is_explicit_blank(&self) -> bool {
        self.presentation
            .as_ref()
            .and_then(|presentation| presentation.is_explicit_blank)
            .unwrap_or(false)
    }

    #[must_use]
    /// Checks if the candidate is disabled based on its presentation configuration.
    pub fn is_disabled(&self) -> bool {
        self.presentation
            .as_ref()
            .and_then(|presentation| presentation.is_disabled)
            .unwrap_or(false)
    }

    #[must_use]
    /// Checks if the candidate is a write-in based on its presentation configuration.
    pub fn is_write_in(&self) -> bool {
        self.presentation
            .as_ref()
            .and_then(|presentation| presentation.is_write_in)
            .unwrap_or(false)
    }

    /// Sets the write-in status for the candidate.
    pub fn set_is_write_in(&mut self, is_write_in: bool) {
        let mut presentation = self.presentation.clone().unwrap_or_default();
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
/// Specifies the order in which candidates are displayed on the ballot.
pub enum CandidatesOrder {
    /// Candidates order is randomized.
    #[strum(serialize = "random")]
    #[serde(rename = "random")]
    Random,
    /// Candidates order is custom-defined.
    #[strum(serialize = "custom")]
    #[serde(rename = "custom")]
    Custom,
    /// Candidates order is alphabetical (default).
    #[strum(serialize = "alphabetical")]
    #[serde(rename = "alphabetical")]
    #[default]
    Alphabetical,
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
    Copy,
    EnumString,
    Display,
    Default,
)]
/// Policy for allowing or disallowing early voting.
pub enum EarlyVotingPolicy {
    /// Early voting is allowed.
    #[strum(serialize = "allow_early_voting")]
    #[serde(rename = "allow_early_voting")]
    AllowEarlyVoting,
    /// Early voting is not allowed (default).
    #[strum(serialize = "no_early_voting")]
    #[serde(rename = "no_early_voting")]
    #[default]
    NoEarlyVoting,
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
/// Specifies the order in which contests are displayed on the ballot.
pub enum ContestsOrder {
    /// Contests order is randomized.
    #[strum(serialize = "random")]
    #[serde(rename = "random")]
    Random,
    /// Contests order is custom-defined.
    #[strum(serialize = "custom")]
    #[serde(rename = "custom")]
    Custom,
    /// Contests order is alphabetical (default).
    #[strum(serialize = "alphabetical")]
    #[serde(rename = "alphabetical")]
    #[default]
    Alphabetical,
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
/// Policy for requiring gold level authentication when casting a vote.
pub enum CastVoteGoldLevelPolicy {
    /// Gold level Authentication is required for cast vote.
    #[strum(serialize = "gold-level")]
    #[serde(rename = "gold-level")]
    GoldLevel,
    /// Gold level Authentication is not required for cast vote (default).
    #[strum(serialize = "no-gold-level")]
    #[serde(rename = "no-gold-level")]
    #[default]
    NoGoldLevel,
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
/// Policy for the title shown on the start screen of the voting portal.
pub enum StartScreenTitlePolicy {
    /// Start screen title should be of the election (default).
    #[strum(serialize = "election")]
    #[serde(rename = "election")]
    #[default]
    Election,
    /// Start screen title should be of the election event.
    #[strum(serialize = "election-event")]
    #[serde(rename = "election-event")]
    ElectionEvent,
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
/// Policy for requiring security confirmation before voting.
pub enum ESecurityConfirmationPolicy {
    /// No security confirmation required (default).
    #[strum(serialize = "none")]
    #[serde(rename = "none")]
    #[default]
    NONE,
    /// Security confirmation is mandatory.
    #[strum(serialize = "mandatory")]
    #[serde(rename = "mandatory")]
    MANDATORY,
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
/// Configuration for the audit button in the voting portal.
pub enum AuditButtonCfg {
    /// Show audit button (default).
    #[strum(serialize = "show")]
    #[serde(rename = "show")]
    #[default]
    SHOW,
    /// Do not show audit button.
    #[strum(serialize = "not-show")]
    #[serde(rename = "not-show")]
    NOT_SHOW,
    /// Show audit button in help section.
    #[strum(serialize = "show-in-help")]
    #[serde(rename = "show-in-help")]
    SHOW_IN_HELP,
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
/// Policy for showing or hiding the cast vote logs tab.
pub enum ShowCastVoteLogs {
    /// Show logs tab.
    #[strum(serialize = "show-logs-tab")]
    #[serde(rename = "show-logs-tab")]
    ShowLogsTab,
    /// Hide logs tab (default).
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
/// Policy for the order in which elections are displayed on the ballot.
pub enum ElectionsOrder {
    /// Elections order is randomized.
    #[strum(serialize = "random")]
    #[serde(rename = "random")]
    Random,
    /// Elections order is custom-defined.
    #[strum(serialize = "custom")]
    #[serde(rename = "custom")]
    Custom,
    /// Elections order is alphabetical (default).
    #[strum(serialize = "alphabetical")]
    #[serde(rename = "alphabetical")]
    #[default]
    Alphabetical,
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
/// Election data structure.
pub struct Election {
    /// Unique election identifier.
    pub id: String,
    /// Election event identifier.
    pub election_event_id: String,
    /// Tenant identifier.
    pub tenant_id: String,
    /// Election name.
    pub name: Option<String>,
    /// Internationalized election name.
    pub name_i18n: Option<I18nContent>,
    /// Election description.
    pub description: Option<String>,
    /// Internationalized election description.
    pub description_i18n: Option<I18nContent>,
    /// Election alias.
    pub alias: Option<String>,
    /// Internationalized election alias.
    pub alias_i18n: Option<I18nContent>,
    /// Image document ID.
    pub image_document_id: Option<String>,
    /// List of contests in the election.
    pub contests: Vec<Contest>,
    /// Presentation configuration for the election.
    pub presentation: Option<ElectionPresentation>,
    /// Annotations for the election.
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
/// Policy for handling invalid votes.
pub enum InvalidVotePolicy {
    /// Invalid votes are allowed (default).
    #[strum(serialize = "allowed")]
    #[serde(rename = "allowed")]
    #[default]
    ALLOWED,
    /// Warn on invalid votes.
    #[strum(serialize = "warn")]
    #[serde(rename = "warn")]
    WARN,
    /// Warn on both implicit and explicit invalid votes.
    #[strum(serialize = "warn-invalid-implicit-and-explicit")]
    #[serde(rename = "warn-invalid-implicit-and-explicit")]
    WARN_INVALID_IMPLICIT_AND_EXPLICIT,
    /// Invalid votes are not allowed.
    #[strum(serialize = "not-allowed")]
    #[serde(rename = "not-allowed")]
    NOT_ALLOWED,
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
)]
/// Policy for candidate selection behavior
pub enum CandidatesSelectionPolicy {
    /// if you select one, the previously selected one gets unselected
    #[strum(serialize = "radio")]
    #[serde(rename = "radio")]
    RADIO,
    /// default behaviour
    #[strum(serialize = "cumulative")]
    #[serde(rename = "cumulative")]
    CUMULATIVE,
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
/// Policy for the icon used for candidate selection.
pub enum CandidatesIconCheckboxPolicy {
    /// Checkbox icon by default.
    #[strum(serialize = "square-checkbox")]
    #[serde(rename = "square-checkbox")]
    #[default]
    SQUARE_CHECKBOX,
    /// Radio button icon
    #[strum(serialize = "round-checkbox")]
    #[serde(rename = "round-checkbox")]
    ROUND_CHECKBOX,
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
/// Policy for whether executing key ceremonies at the election event level
/// or election level.
pub enum KeysCeremonyPolicy {
    /// Key ceremonies execute in election event level (default).
    #[strum(serialize = "ELECTION_EVENT")]
    #[serde(rename = "ELECTION_EVENT")]
    #[default]
    ELECTION_EVENT,
    /// Key ceremonies execute in election level.
    #[strum(serialize = "ELECTION")]
    #[serde(rename = "ELECTION")]
    ELECTION,
}

/// Election event materials configuration.
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
/// Materials configuration for an election event.
pub struct ElectionEventMaterials {
    /// True if the election event materials are activated.
    pub activated: Option<bool>,
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
/// Language configuration for an election event.
pub struct ElectionEventLanguageConf {
    /// List of enabled language codes.
    pub enabled_language_codes: Option<Vec<String>>,
    /// Default language code.
    pub default_language_code: Option<String>,
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
/// Presentation configuration for an election event.
pub struct ElectionEventPresentation {
    /// Internationalized content for the event.
    pub i18n: Option<I18nContent<I18nContent<Option<String>>>>,
    /// Materials configuration for the event.
    pub activated: Option<ElectionEventMaterials>,
    /// Language configuration for the event.
    pub language_conf: Option<ElectionEventLanguageConf>,
    /// Logo URL for the event.
    pub logo_url: Option<String>,
    /// Redirect URL after finishing voting.
    pub redirect_finish_url: Option<String>,
    /// Custom CSS for the event.
    pub css: Option<String>,
    /// True if the election list should be skipped.
    pub skip_election_list: Option<bool>,
    /// True if the user profile should be shown (default true).
    pub show_user_profile: Option<bool>,
    /// Show cast vote logs configuration.
    pub show_cast_vote_logs: Option<ShowCastVoteLogs>,
    /// Order in which elections are displayed.
    pub elections_order: Option<ElectionsOrder>,
    /// Countdown policy for the voting portal.
    pub voting_portal_countdown_policy: Option<VotingPortalCountdownPolicy>,
    /// Custom URLs for the event.
    pub custom_urls: Option<CustomUrls>,
    /// Key ceremony policy.
    pub keys_ceremony_policy: Option<KeysCeremonyPolicy>,
    /// Contest encryption policy.
    pub contest_encryption_policy: Option<ContestEncryptionPolicy>,
    /// Decoded ballot inclusion policy.
    pub decoded_ballot_inclusion_policy: Option<DecodedBallotsInclusionPolicy>,
    /// Locked down policy.
    pub locked_down: Option<LockedDown>,
    /// Publish policy.
    pub publish_policy: Option<Publish>,
    /// Enrollment policy.
    pub enrollment: Option<Enrollment>,
    /// OTP policy.
    pub otp: Option<Otp>,
    /// Voter signing policy.
    pub voter_signing_policy: Option<VoterSigningPolicy>,
    /// Policy for voter digital certificate.
    pub voter_digital_cert_policy: Option<VoterDigitalCertPolicy>,
    /// Policy for weighted voting.
    pub weighted_voting_policy: Option<WeightedVotingPolicy>,
    /// Ceremonies policy.
    /// (Whether the ceremonies should be automated)
    pub ceremonies_policy: Option<CeremoniesPolicy>,
    /// Policy for delegated voting.
    pub delegated_voting_policy: Option<DelegatedVotingPolicy>,
}

impl ElectionEvent {
    /// Parses the stored JSON presentation value and returns the typed presentation.
    ///
    /// # Errors
    /// Returns an error if deserializing the event `presentation` JSON string fails.
    pub fn get_presentation(
        &self,
    ) -> Result<Option<ElectionEventPresentation>, Error<serde_json::Error>>
    {
        self.presentation.clone().map(deserialize_value).transpose()
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
    /// No grace period(default).
    #[strum(serialize = "no-grace-period")]
    #[serde(rename = "no-grace-period")]
    NO_GRACE_PERIOD,
    /// Grace period without alert.
    #[strum(serialize = "grace-period-without-alert")]
    #[serde(rename = "grace-period-without-alert")]
    GRACE_PERIOD_WITHOUT_ALERT,
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
/// Voting period dates.
pub struct VotingPeriodDates {
    /// Start date of the voting period.
    pub start_date: Option<String>,
    /// End date of the voting period.
    pub end_date: Option<String>,
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
/// Policy for whether Initialize Report is required to start voting.
pub enum EInitializeReportPolicy {
    /// Initialize Report is required.
    #[strum(serialize = "required")]
    #[serde(rename = "required")]
    REQUIRED,
    /// Initialize Report is not required (default).
    #[strum(serialize = "not-required")]
    #[serde(rename = "not-required")]
    #[default]
    NOT_REQUIRED,
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
/// Configuration for the voting portal countdown Policy.
pub struct VotingPortalCountdownPolicy {
    /// Countdown policy
    pub policy: Option<ECountdownPolicy>,
    /// Countdown anticipation seconds.
    ///  i.e how many seconds before the countdown should start and for how long.
    pub countdown_anticipation_secs: Option<u64>,
    /// Countdown alert anticipation seconds.
    pub countdown_alert_anticipation_secs: Option<u64>,
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
/// Policy for the voting portal countdown.
pub enum ECountdownPolicy {
    /// No countdown
    NO_COUNTDOWN,
    /// Show countdown without alert.
    COUNTDOWN,
    /// Show countdown with alert.
    COUNTDOWN_WITH_ALERT,
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
    Copy,
    Clone,
    EnumString,
    Display,
    Default,
)]

/// Policy for undervotes.
pub enum EUnderVotePolicy {
    /// Undervotes are allowed (default).
    #[strum(serialize = "allowed")]
    #[serde(rename = "allowed")]
    #[default]
    ALLOWED,
    /// Warn on undervotes.
    #[strum(serialize = "warn")]
    #[serde(rename = "warn")]
    WARN,
    /// Warn on undervotes only in review screen.
    #[strum(serialize = "warn-only-in-review")]
    #[serde(rename = "warn-only-in-review")]
    WARN_ONLY_IN_REVIEW,
    /// Warn on undervotes and show alert.
    #[strum(serialize = "warn-and-alert")]
    #[serde(rename = "warn-and-alert")]
    WARN_AND_ALERT,
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
    Copy,
    Clone,
    EnumString,
    Display,
    Default,
)]
/// Policy for blank votes.
pub enum EBlankVotePolicy {
    /// Blank votes are allowed (default).
    #[strum(serialize = "allowed")]
    #[serde(rename = "allowed")]
    #[default]
    ALLOWED,
    /// Warn on blank votes.
    #[strum(serialize = "warn")]
    #[serde(rename = "warn")]
    WARN,
    /// Warn on blank votes only in review screen.
    #[strum(serialize = "warn-only-in-review")]
    #[serde(rename = "warn-only-in-review")]
    WARN_ONLY_IN_REVIEW,
    /// Blank votes are not allowed.
    #[strum(serialize = "not-allowed")]
    #[serde(rename = "not-allowed")]
    NOT_ALLOWED,
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
    Copy,
    Clone,
    EnumString,
    Display,
    Default,
)]
/// Policy for overvotes.
pub enum EOverVotePolicy {
    /// Overvotes are allowed (default).
    #[strum(serialize = "allowed")]
    #[serde(rename = "allowed")]
    ALLOWED,
    /// Overvotes are allowed with a message.
    #[strum(serialize = "allowed-with-msg")]
    #[serde(rename = "allowed-with-msg")]
    ALLOWED_WITH_MSG,
    /// Overvotes are allowed with a message and alert.
    #[strum(serialize = "allowed-with-msg-and-alert")]
    #[serde(rename = "allowed-with-msg-and-alert")]
    #[default]
    ALLOWED_WITH_MSG_AND_ALERT,
    /// Overvotes are not allowed and show a message with alert.
    #[strum(serialize = "not-allowed-with-msg-and-alert")]
    #[serde(rename = "not-allowed-with-msg-and-alert")]
    NOT_ALLOWED_WITH_MSG_AND_ALERT,
    /// Overvotes are not allowed and show a message.
    #[strum(serialize = "not-allowed-with-msg-and-disable")]
    #[serde(rename = "not-allowed-with-msg-and-disable")]
    NOT_ALLOWED_WITH_MSG_AND_DISABLE,
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
    Copy,
    Clone,
    EnumString,
    Display,
    Default,
)]
/// Policy for duplicated ranks in preferential voting.
pub enum EDuplicatedRankPolicy {
    /// Duplicated ranks are allowed (default) but shows a warning and dialog.
    #[strum(serialize = "allowed-warn-and-dialog")]
    #[serde(rename = "allowed-warn-and-dialog")]
    #[default]
    ALLOWED_WARN_AND_DIALOG,
    /// Duplicated ranks are not allowed and shows a warning and dialog.
    #[strum(serialize = "not-allowed-warn-and-dialog")]
    #[serde(rename = "not-allowed-warn-and-dialog")]
    NOT_ALLOWED_WARN_AND_DIALOG,
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
    Copy,
    Clone,
    EnumString,
    Display,
    Default,
)]
/// Policy for preference gaps in preferential voting.
pub enum EPreferenceGapsPolicy {
    /// Preference gaps are allowed (default) but shows a warning and dialog.
    #[strum(serialize = "allowed-warn-and-dialog")]
    #[serde(rename = "allowed-warn-and-dialog")]
    #[default]
    ALLOWED_WARN_AND_DIALOG,
    /// Preference gaps are not allowed and shows a warning and dialog.
    #[strum(serialize = "not-allowed-warn-and-dialog")]
    #[serde(rename = "not-allowed-warn-and-dialog")]
    NOT_ALLOWED_WARN_AND_DIALOG,
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
/// Presentation settings for an election event.
pub struct ElectionPresentation {
    /// Internationalized text for the election.
    pub i18n: Option<I18nContent<I18nContent<Option<String>>>>,
    /// Voting period date configuration.
    pub dates: Option<VotingPeriodDates>,
    /// Language-specific configuration.
    pub language_conf: Option<ElectionEventLanguageConf>,
    /// Order in which contests are shown.
    pub contests_order: Option<ContestsOrder>,
    /// Audit button configuration.
    pub audit_button_cfg: Option<AuditButtonCfg>,
    /// UI sort order.
    pub sort_order: Option<i64>,
    /// Whether to show a cast-vote confirm screen.
    pub cast_vote_confirm: Option<bool>,
    /// Gold-level policy for cast vote confirmation.
    pub cast_vote_gold_level: Option<CastVoteGoldLevelPolicy>,
    /// Start screen title policy.
    pub start_screen_title_policy: Option<StartScreenTitlePolicy>,
    /// Whether grace period is enabled.
    pub is_grace_priod: Option<bool>,
    /// Grace period policy.
    pub grace_period_policy: Option<EGracePeriodPolicy>,
    /// Grace period duration in seconds.
    pub grace_period_secs: Option<u64>,
    /// Initialize report policy.
    pub init_report: Option<InitReport>,
    /// Manual start voting period policy.
    pub manual_start_voting_period: Option<ManualStartVotingPeriod>,
    /// Voting period end policy.
    pub voting_period_end: Option<VotingPeriodEnd>,
    /// Tally policy.
    pub tally: Option<Tally>,
    /// Policy for whether Initialize Report is required to start voting.
    pub initialization_report_policy: Option<EInitializeReportPolicy>,
    /// Security confirmation policy.
    pub security_confirmation_policy: Option<ESecurityConfirmationPolicy>,
    /// Consolidated report policy.
    pub consolidated_report_policy: Option<ConsolidatedReportPolicy>,
}

impl core::Election {
    /// Returns the election's presentation settings, if configured.
    #[must_use]
    pub fn get_presentation(&self) -> Option<ElectionPresentation> {
        let election_presentation: Option<ElectionPresentation> = self
            .presentation
            .clone()
            .and_then(|value| deserialize_value(value).ok());

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
/// Presentation settings for an area.
pub struct AreaPresentation {
    /// Whether early voting is allowed for this area.
    pub allow_early_voting: Option<EarlyVotingPolicy>,
}

impl AreaPresentation {
    /// Returns true if early voting is enabled for this area.
    #[must_use]
    pub fn is_early_voting(&self) -> bool {
        self.allow_early_voting == Some(EarlyVotingPolicy::AllowEarlyVoting)
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
pub struct SubtypePresentation {
    pub name: Option<String>,
    pub name_i18n: Option<I18nContent<Option<String>>>,
    pub sort_order: Option<i64>,
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
pub struct TypePresentation {
    pub name: Option<String>,
    pub name_i18n: Option<I18nContent<Option<String>>>,
    pub sort_order: Option<i64>,
    pub subtypes_presentation:
        Option<HashMap<String, Option<SubtypePresentation>>>,
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
/// Presentation settings for a contest.
#[allow(missing_docs)]
pub struct ContestPresentation {
    pub i18n: Option<I18nContent<I18nContent<Option<String>>>>,
    pub allow_writeins: Option<bool>,
    pub base32_writeins: Option<bool>,
    pub invalid_vote_policy: Option<InvalidVotePolicy>, /* allowed|warn|warn-invalid-implicit-and-explicit */
    pub under_vote_policy: Option<EUnderVotePolicy>,
    pub blank_vote_policy: Option<EBlankVotePolicy>,
    pub over_vote_policy: Option<EOverVotePolicy>,
    pub duplicated_rank_policy: Option<EDuplicatedRankPolicy>,
    pub preference_gaps_policy: Option<EPreferenceGapsPolicy>,
    pub pagination_policy: Option<String>,
    pub cumulative_number_of_checkboxes: Option<u64>,
    pub shuffle_categories: Option<bool>,
    pub shuffle_category_list: Option<Vec<String>>,
    pub show_points: Option<bool>,
    pub enable_checkable_lists: Option<String>, /* disabled|allow-selecting-candidates-and-lists|allow-selecting-candidates|allow-selecting-lists */
    pub candidates_order: Option<CandidatesOrder>,
    pub candidates_selection_policy: Option<CandidatesSelectionPolicy>,
    pub candidates_icon_checkbox_policy: Option<CandidatesIconCheckboxPolicy>,
    pub max_selections_per_type: Option<u64>,
    pub types_presentation: Option<HashMap<String, Option<TypePresentation>>>,
    pub sort_order: Option<i64>,
    pub columns: Option<u64>,
}

impl ContestPresentation {
    #[must_use]
    /// Creates a new `ContestPresentation` instance with default values for all fields.
    pub fn new() -> ContestPresentation {
        ContestPresentation {
            i18n: None,
            allow_writeins: Some(true),
            base32_writeins: Some(true),
            invalid_vote_policy: Some(InvalidVotePolicy::ALLOWED),
            blank_vote_policy: Some(EBlankVotePolicy::ALLOWED),
            over_vote_policy: Some(EOverVotePolicy::ALLOWED),
            pagination_policy: Some(String::new()),
            cumulative_number_of_checkboxes: None,
            shuffle_categories: Some(false),
            shuffle_category_list: None,
            show_points: Some(false),
            enable_checkable_lists: None,
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

/// Contest data structure.
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
#[allow(missing_docs)]
pub struct Contest {
    pub id: String,
    /// Tenant identifier.
    pub tenant_id: String,
    pub election_event_id: String,
    pub election_id: String,
    pub name: Option<String>,
    pub name_i18n: Option<I18nContent>,
    pub description: Option<String>,
    pub description_i18n: Option<I18nContent>,
    pub alias: Option<String>,
    pub alias_i18n: Option<I18nContent>,
    pub max_votes: i64,
    pub min_votes: i64,
    pub winning_candidates_num: i64,
    pub voting_type: Option<String>,
    pub counting_algorithm: Option<CountingAlgType>, /* plurality-at-large|borda-nauru|borda|borda-mas-madrid|desborda3|desborda2|desborda|cumulative */
    pub is_encrypted: bool,
    pub candidates: Vec<Candidate>,
    pub presentation: Option<ContestPresentation>,
    pub created_at: Option<String>,
    pub annotations: Option<Annotations>,
    pub tie_breaking_policy: Option<TieBreakingPolicy>,
}

impl Contest {
    #[must_use]
    /// Return true if the contest presentation is configured to allow write-ins.
    pub fn allow_writeins(&self) -> bool {
        self.presentation
            .as_ref()
            .and_then(|presentation| presentation.allow_writeins)
            .unwrap_or(false)
    }

    #[must_use]
    /// Get the counting algorithm for the contest.
    pub fn get_counting_algorithm(&self) -> CountingAlgType {
        self.counting_algorithm.unwrap_or_default()
    }

    #[must_use]
    /// Return true if the contest presentation is configured to allow base32 write-ins,
    /// defaulting to true if the presentation or the specific configuration value is not set.
    pub fn base32_writeins(&self) -> bool {
        self.presentation
            .as_ref()
            .and_then(|presentation| presentation.base32_writeins)
            .unwrap_or(true)
    }

    #[must_use]
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

    #[must_use]
    /// Get the cumulative number of checkboxes for the contest from the presentation.
    pub fn cumulative_number_of_checkboxes(&self) -> u64 {
        self.presentation
            .as_ref()
            .and_then(|presentation| {
                presentation.cumulative_number_of_checkboxes
            })
            .unwrap_or(1)
    }

    #[must_use]
    /// Return true if the contest presentation is configured to show points, false otherwise.
    pub fn show_points(&self) -> bool {
        self.presentation
            .as_ref()
            .and_then(|presentation| presentation.show_points)
            .unwrap_or(false)
    }

    #[must_use]
    /// Get the all candidate ids that are explicitly marked as invalid.
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
    #[must_use]
    pub fn get_tie_breaking_policy(&self) -> TieBreakingPolicy {
        self.tie_breaking_policy.clone().unwrap_or_default()
    }

    /// Get per-round tie resolutions from contest annotations.
    #[must_use]
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

    /// Insert tie resolutions into the contest's annotations.
    ///
    /// # Errors
    /// Returns an error if serialization of the tie resolutions fails.
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
/// configuration for whether enrollment is enabled or disabled.
pub enum Enrollment {
    #[default]
    #[strum(serialize = "enabled")]
    #[serde(rename = "enabled")]
    /// Enrollment is enabled.
    ENABLED,
    #[strum(serialize = "disabled")]
    #[serde(rename = "disabled")]
    /// Enrollment is disabled.
    DISABLED,
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
/// Configuration for whether OTP is enabled or disabled.
pub enum Otp {
    #[default]
    #[strum(serialize = "enabled")]
    #[serde(rename = "enabled")]
    /// OTP is enabled.
    ENABLED,
    #[strum(serialize = "disabled")]
    #[serde(rename = "disabled")]
    /// OTP is disabled.
    DISABLED,
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
/// Configuration for whether decoded ballots are included or not.
pub enum DecodedBallotsInclusionPolicy {
    #[strum(serialize = "included")]
    #[serde(rename = "included")]
    /// Decoded ballots are included.
    INCLUDED,
    #[default]
    #[strum(serialize = "not-included")]
    #[serde(rename = "not-included")]
    /// Decoded ballots are not included.
    NOT_INCLUDED,
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
/// Configuration for contest encryption policy.
pub enum ContestEncryptionPolicy {
    #[strum(serialize = "multiple-contests")]
    #[serde(rename = "multiple-contests")]
    /// Contests are encrypted together in a single encryption process.
    MULTIPLE_CONTESTS,
    #[default]
    #[strum(serialize = "single-contest")]
    #[serde(rename = "single-contest")]
    /// Each contest is encrypted separately.
    SINGLE_CONTEST,
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
/// Configuration for voter signing policy.
pub enum VoterSigningPolicy {
    #[default]
    #[strum(serialize = "no-signature")]
    #[serde(rename = "no-signature")]
    /// Votes are not signed with the voter's signature.
    NO_SIGNATURE,
    #[strum(serialize = "with-signature")]
    #[serde(rename = "with-signature")]
    /// Votes are signed with the voter's signature.
    WITH_SIGNATURE,
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
/// Configuration for voter digital certificate policy.
#[allow(missing_docs)]
pub enum VoterDigitalCertPolicy {
    #[default]
    #[strum(serialize = "disabled")]
    #[serde(rename = "disabled")]
    DISABLED,
    #[strum(serialize = "enabled")]
    #[serde(rename = "enabled")]
    ENABLED,
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
/// Configuration for whether the election event is locked down or not.
pub enum LockedDown {
    #[strum(serialize = "locked-down")]
    #[serde(rename = "locked-down")]
    /// The election event is locked down, meaning that no further changes to the election configuration are allowed.
    LOCKED_DOWN,
    #[default]
    #[strum(serialize = "not-locked-down")]
    #[serde(rename = "not-locked-down")]
    /// The election event is not locked down.
    NOT_LOCKED_DOWN,
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
/// Configuration for whether able to publish.
pub enum Publish {
    #[default]
    #[strum(serialize = "always")]
    #[serde(rename = "always")]
    /// The election event is always enabled for publishing.
    ALWAYS,
    #[strum(serialize = "after-lockdown")]
    #[serde(rename = "after-lockdown")]
    ///Publishing is enabled only after the election event is locked down.
    AFTER_LOCKDOWN,
}

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
#[serde(default)]
/// Status of the voting for the election event
pub struct ElectionEventStatus {
    /// True if the election event is published, false otherwise.
    pub is_published: Option<bool>,
    /// Voting status.
    pub voting_status: VotingStatus,
    /// Kiosk voting status.
    pub kiosk_voting_status: VotingStatus,
    /// Early voting status.
    pub early_voting_status: VotingStatus,
    /// Voting period dates for the online channel.
    pub voting_period_dates: PeriodDates,
    /// Kiosk voting period dates.
    pub kiosk_voting_period_dates: PeriodDates,
    /// Early voting period dates.
    pub early_voting_period_dates: PeriodDates,
}

impl Default for ElectionEventStatus {
    fn default() -> Self {
        ElectionEventStatus {
            is_published: Some(false),
            voting_status: VotingStatus::NOT_STARTED,
            kiosk_voting_status: VotingStatus::NOT_STARTED,
            early_voting_status: VotingStatus::NOT_STARTED,
            voting_period_dates: PeriodDates::default(),
            kiosk_voting_period_dates: PeriodDates::default(),
            early_voting_period_dates: PeriodDates::default(),
        }
    }
}

impl ElectionEventStatus {
    #[must_use]
    /// Returns the voting status for the specified channel.
    pub const fn status_by_channel(
        &self,
        channel: VotingStatusChannel,
    ) -> VotingStatus {
        match channel {
            VotingStatusChannel::ONLINE => self.voting_status,
            VotingStatusChannel::KIOSK => self.kiosk_voting_status,
            VotingStatusChannel::EARLY_VOTING => self.early_voting_status,
        }
    }

    /// Close `EARLY_VOTING` channel's status automatically if the new online
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

    /// Sets the voting status for the given channel and updates the
    /// corresponding period dates.
    pub fn set_status_by_channel(
        &mut self,
        channel: VotingStatusChannel,
        new_status: VotingStatus,
    ) {
        let period_dates = match channel {
            VotingStatusChannel::ONLINE => {
                self.voting_status = new_status;
                &mut self.voting_period_dates
            }
            VotingStatusChannel::KIOSK => {
                self.kiosk_voting_status = new_status;
                &mut self.kiosk_voting_period_dates
            }
            VotingStatusChannel::EARLY_VOTING => {
                self.early_voting_status = new_status;
                &mut self.early_voting_period_dates
            }
        };
        period_dates.update_period_dates(new_status);
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
/// Voting status.
pub enum VotingStatus {
    #[default]
    /// Voting has not started yet.
    NOT_STARTED,
    /// Voting is currently open.
    OPEN,
    /// Voting is paused.
    PAUSED,
    /// Voting is closed.
    CLOSED,
}

impl VotingStatus {
    #[must_use]
    /// Returns true if the voting status is `NOT_STARTED`.
    pub const fn is_not_started(&self) -> bool {
        match self {
            VotingStatus::NOT_STARTED => true,
            VotingStatus::OPEN
            | VotingStatus::PAUSED
            | VotingStatus::CLOSED => false,
        }
    }

    #[must_use]
    /// Returns true if the voting status is any but `NOT_STARTED`.
    pub const fn is_started(&self) -> bool {
        !self.is_not_started()
    }

    #[must_use]
    /// Returns true if the voting status is `OPEN` or `PAUSED`.
    pub const fn is_open(&self) -> bool {
        match self {
            VotingStatus::OPEN => true,
            VotingStatus::NOT_STARTED
            | VotingStatus::PAUSED
            | VotingStatus::CLOSED => false,
        }
    }

    #[must_use]
    /// Returns true if the voting status is `PAUSED`.
    pub const fn is_paused(&self) -> bool {
        match self {
            VotingStatus::PAUSED => true,
            VotingStatus::NOT_STARTED
            | VotingStatus::OPEN
            | VotingStatus::CLOSED => false,
        }
    }

    #[must_use]
    /// Returns true if the voting status is `CLOSED`.
    pub const fn is_closed(&self) -> bool {
        match self {
            VotingStatus::CLOSED => true,
            VotingStatus::NOT_STARTED
            | VotingStatus::OPEN
            | VotingStatus::PAUSED => false,
        }
    }

    #[must_use]
    /// Returns true if the voting status is `NOT_STARTED` or `CLOSED`.
    pub const fn is_closed_or_never_started(&self) -> bool {
        match self {
            VotingStatus::NOT_STARTED | VotingStatus::CLOSED => true,
            VotingStatus::OPEN | VotingStatus::PAUSED => false,
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
/// Policy for allowing tally before voting period ends.
pub enum AllowTallyStatus {
    #[default]
    #[strum(serialize = "allowed")]
    #[serde(rename = "allowed")]
    /// Tally is allowed before voting period ends.
    ALLOWED,
    #[strum(serialize = "disallowed")]
    #[serde(rename = "disallowed")]
    /// Tally is not allowed before voting period ends.
    DISALLOWED,
    #[strum(serialize = "requires-voting-period-end")]
    #[serde(rename = "requires-voting-period-end")]
    /// Tally is only allowed when voting period ends.
    REQUIRES_VOTING_PERIOD_END,
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
    Copy,
    EnumString,
    JsonSchema,
    IntoStaticStr,
)]
/// Voting channels.
pub enum VotingStatusChannel {
    /// Online voting channel.
    ONLINE,
    /// Kiosk voting channel.
    KIOSK,
    /// Early voting.
    EARLY_VOTING,
}

impl VotingStatusChannel {
    #[must_use]
    /// Returns the channel status from `VotingChannels`.
    pub const fn channel_from(
        &self,
        channels: &core::VotingChannels,
    ) -> Option<bool> {
        match self {
            VotingStatusChannel::ONLINE => channels.online,
            VotingStatusChannel::KIOSK => channels.kiosk,
            VotingStatusChannel::EARLY_VOTING => channels.early_voting,
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
/// Statistics related to an election event.
pub struct ElectionEventStatistics {
    /// Number of emails sent.
    pub num_emails_sent: Option<i64>,
    /// Number of SMS sent.
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
/// Statistics related to an election, such as the number of emails and SMS sent.
pub struct ElectionStatistics {
    /// Number of emails sent.
    pub num_emails_sent: Option<i64>,
    /// Number of SMS sent.
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
/// Policy for initialization report.
pub enum InitReport {
    #[default]
    #[strum(serialize = "allowed")]
    #[serde(rename = "allowed")]
    /// Initialization report is allowed to be generated.
    ALLOWED,
    #[strum(serialize = "disallowed")]
    #[serde(rename = "disallowed")]
    /// Initialization report is not allowed to be generated.
    DISALLOWED,
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
/// Policy for manually starting the voting period.
pub enum ManualStartVotingPeriod {
    #[default]
    #[strum(serialize = "allowed")]
    #[serde(rename = "allowed")]
    /// Manually starting the voting period is allowed.
    ALLOWED,
    #[strum(serialize = "only-when-initialization-report-has-been-performed")]
    #[serde(rename = "only-when-initialization-report-has-been-performed")]
    /// Manually starting the voting period is only allowed when the initialization report has been performed.
    ONLY_WHEN_INITIALIZATION_REPORT_HAS_BEEN_PERFORMED,
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
pub enum VotingPeriodEnd {
    #[default]
    #[strum(serialize = "allowed")]
    #[serde(rename = "allowed")]
    ALLOWED,
    #[strum(serialize = "disallowed")]
    #[serde(rename = "disallowed")]
    DISALLOWED,
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
/// Policy for allowing tally before voting period ends.
pub enum Tally {
    #[default]
    #[strum(serialize = "always-allow")]
    #[serde(rename = "always-allow")]
    /// Tally is always allowed.
    ALWAYS_ALLOW,
    #[strum(serialize = "allow-when-voting-period-ends")]
    #[serde(rename = "allow-when-voting-period-ends")]
    /// Tally is allowed when voting period ends.
    ONLY_WHEN_VOTING_PERIOD_ENDS,
}

#[derive(
    Serialize, Deserialize, PartialEq, Eq, JsonSchema, Debug, Clone, Default,
)]
/// Struct to hold the first and last timestamps for each voting
/// status change during a voting period.
pub struct PeriodDates {
    /// The first time the voting period was started.
    pub first_started_at: Option<DateTime<Utc>>,
    /// The last time the voting period was started.
    pub last_started_at: Option<DateTime<Utc>>,
    /// The first time the voting period was paused.
    pub first_paused_at: Option<DateTime<Utc>>,
    /// The last time the voting period was paused.
    pub last_paused_at: Option<DateTime<Utc>>,
    /// The first time the voting period was stopped.
    pub first_stopped_at: Option<DateTime<Utc>>,
    /// The last time the voting period was stopped.
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
/// Struct to hold the stringified first and last timestamps for each
///  voting status change during a voting period.
pub struct StringifiedPeriodDates {
    pub first_started_at: Option<String>,
    pub last_started_at: Option<String>,
    pub first_paused_at: Option<String>,
    pub last_paused_at: Option<String>,
    pub first_stopped_at: Option<String>,
    pub last_stopped_at: Option<String>,
    pub scheduled_event_dates: Option<HashMap<String, ScheduledEventDates>>,
}

#[derive(
    Serialize, Deserialize, PartialEq, Eq, JsonSchema, Debug, Clone, Default,
)]
pub struct ReportDates {
    pub start_date: String,
    pub end_date: String,
    pub election_date: String,
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
/// Struct to hold the scheduled and stopped timestamps for scheduled events.
pub struct ScheduledEventDates {
    /// The scheduled time for the event.
    pub scheduled_at: Option<String>,
    /// The time when the scheduled event was stopped.
    pub stopped_at: Option<String>,
}

impl PeriodDates {
    /// Updates the period dates based on the new voting status.
    fn update_period_dates(&mut self, new_status: VotingStatus) {
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
            *first = *last;
        }
    }

    #[must_use]
    /// Converts period dates to string fields.
    pub fn to_string_fields(&self) -> StringifiedPeriodDates {
        StringifiedPeriodDates {
            first_started_at: format_date_opt(&self.first_started_at),
            last_started_at: format_date_opt(&self.last_started_at),
            first_paused_at: format_date_opt(&self.first_paused_at),
            last_paused_at: format_date_opt(&self.last_paused_at),
            first_stopped_at: format_date_opt(&self.first_stopped_at),
            last_stopped_at: format_date_opt(&self.last_stopped_at),
            scheduled_event_dates: Option::default(),
        }
    }
}

/// Helper method to format the date or return `default`.
#[must_use]
pub fn format_date(date: &Option<DateTime<Utc>>, default: &str) -> String {
    date.map_or(default.to_string(), |d| d.to_rfc3339())
}

/// Helper method to format the date or return `None`.
#[must_use]
pub fn format_date_opt(date: &Option<DateTime<Utc>>) -> Option<String> {
    date.map(|d| d.to_rfc3339())
}

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
#[serde(default)]
/// Struct to hold the election status related to voting and tally.
pub struct ElectionStatus {
    /// True if the election is published, false otherwise.
    pub is_published: Option<bool>,
    /// Voting status for the online channel.
    pub voting_status: VotingStatus,
    /// Policy for initialization report.
    pub init_report: InitReport,
    /// Voting status for the kiosk channel.
    pub kiosk_voting_status: VotingStatus,
    /// Voting status for the early voting channel.
    pub early_voting_status: VotingStatus,
    /// Voting period dates for the online channel.
    pub voting_period_dates: PeriodDates,
    /// Kiosk voting period dates.
    pub kiosk_voting_period_dates: PeriodDates,
    /// Early voting period dates.
    pub early_voting_period_dates: PeriodDates,
    /// Policy for allowing tally before voting period ends.
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
            voting_period_dates: PeriodDates::default(),
            kiosk_voting_period_dates: PeriodDates::default(),
            early_voting_period_dates: PeriodDates::default(),
            allow_tally: AllowTallyStatus::default(),
        }
    }
}

impl ElectionStatus {
    #[must_use]
    /// Returns the voting status of the given channel.
    pub const fn status_by_channel(
        &self,
        channel: VotingStatusChannel,
    ) -> VotingStatus {
        match channel {
            VotingStatusChannel::ONLINE => self.voting_status,
            VotingStatusChannel::KIOSK => self.kiosk_voting_status,
            VotingStatusChannel::EARLY_VOTING => self.early_voting_status,
        }
    }

    #[must_use]
    /// Returns the period dates of the given channel.
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

    /// Close `EARLY_VOTING` channel's status automatically if the new online
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

    /// Sets the voting status for the given channel and updates the
    /// corresponding period dates.
    pub fn set_status_by_channel(
        &mut self,
        channel: VotingStatusChannel,
        new_status: VotingStatus,
    ) {
        let period_dates = match channel {
            VotingStatusChannel::ONLINE => {
                self.voting_status = new_status;
                &mut self.voting_period_dates
            }
            VotingStatusChannel::KIOSK => {
                self.kiosk_voting_status = new_status;
                &mut self.kiosk_voting_period_dates
            }
            VotingStatusChannel::EARLY_VOTING => {
                self.early_voting_status = new_status;
                &mut self.early_voting_period_dates
            }
        };
        period_dates.update_period_dates(new_status);
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
/// Struct representing the ballot style, which includes information
/// about the contests, areas, and presentation settings for a specific ballot.
pub struct BallotStyle {
    /// Unique identifier for the ballot style.
    pub id: String,
    /// Tenant identifier.
    pub tenant_id: String,
    /// Election event identifier.
    pub election_event_id: String,
    /// Election identifier.
    pub election_id: String,
    /// Number of allowed revotes for this ballot style, if any.
    pub num_allowed_revotes: Option<i64>,
    /// Description of the election.
    pub description: Option<String>,
    /// Public key.
    pub public_key: Option<PublicKeyConfig>,
    /// Unique identifier for the area associated with this ballot style.
    pub area_id: String,
    /// Presentation settings for the area associated with this ballot style.
    pub area_presentation: Option<AreaPresentation>,
    /// List of contests included in this ballot style.
    pub contests: Vec<Contest>,
    /// Presentation settings for the election event.
    pub election_event_presentation: Option<ElectionEventPresentation>,
    /// Presentation settings for the election.
    pub election_presentation: Option<ElectionPresentation>,
    /// Dates related to the election, such as voting period dates.
    pub election_dates: Option<StringifiedPeriodDates>,
    /// Annotations for the election event.
    pub election_event_annotations: Option<HashMap<String, String>>,
    /// Annotations for the election.
    pub election_annotations: Option<HashMap<String, String>>,
    /// Annotations for the election.
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
/// Struct to hold custom URLs for the election event.
pub struct CustomUrls {
    /// Custom login URL for the election event.
    pub login: Option<String>,
    /// Custom enrollment URL for the election event.
    pub enrollment: Option<String>,
    /// Custom SAML URL for the election event.
    pub saml: Option<String>,
}

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
/// Struct to represent the weight of an area, which can be used in
/// weighted voting systems.
pub struct Weight(Option<u64>);

impl Default for Weight {
    fn default() -> Self {
        Self(Some(1)) // default weight is 1
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
/// Struct to hold annotations for an area.
pub struct AreaAnnotations {
    /// Weight of the area, which can be used in weighted voting systems.
    pub weight: Option<Weight>,
    /// Tally operation for the area, which can specify how the results for
    /// the area should be handled during the tallying process.
    pub tally_operation: Option<TallyOperation>,
}

impl AreaAnnotations {
    /// Get the weight of the area, returning the default weight if it is not specified.
    #[must_use]
    pub fn get_weight(&self) -> Weight {
        self.weight.unwrap_or_default()
    }
}

impl Area {
    /// Get the annotations for the area, deserializing them from the raw annotations if they are present.
    /// If the annotations are not present, return `None`. If deserialization fails, return an error.
    ///
    /// # Errors
    /// Returns an error if deserialization fails.
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
/// Policy to determine whether weighted voting is enabled.
pub enum WeightedVotingPolicy {
    /// Weighted voting is disabled.
    #[default]
    #[serde(rename = "disabled-weighted-voting")]
    DISABLED_WEIGHTED_VOTING,
    /// Weighted voting is allowed for areas.
    #[serde(rename = "areas-weighted-voting")]
    AREAS_WEIGHTED_VOTING,
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
/// Policy to determine if and when delegated voting is allowed.
pub enum DelegatedVotingPolicy {
    /// Delegated voting is not allowed.
    #[default]
    #[serde(rename = "disabled")]
    DISABLED,
    /// Delegated voting is allowed.
    #[serde(rename = "enabled")]
    ENABLED,
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
/// Policy to determine if the consolidated report should be generated.
pub enum ConsolidatedReportPolicy {
    /// The consolidated report will not be generated.
    #[default]
    #[strum(serialize = "do-not-generate")]
    #[serde(rename = "do-not-generate")]
    DO_NOT_GENERATE,
    /// The consolidated report will be generated.
    #[strum(serialize = "generate")]
    #[serde(rename = "generate")]
    GENERATE,
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
pub enum TieBreakingPolicy {
    #[default]
    #[strum(serialize = "random")]
    #[serde(rename = "random")]
    RANDOM,
    #[strum(serialize = "external-procedure")]
    #[serde(rename = "external-procedure")]
    EXTERNAL_PROCEDURE,
}
