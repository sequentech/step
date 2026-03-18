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
/// Content for a signed ballot, including the public key and signature.
pub struct SignedContent {
    /// Public key used for signing.
    pub public_key: String,
    /// Signature value.
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
            .map(|presentation| presentation.is_category_list)
            .flatten()
            .unwrap_or(false)
    }

    #[must_use]
    /// Checks if the candidate is explicitly marked as invalid based on its presentation configuration.
    pub fn is_explicit_invalid(&self) -> bool {
        self.presentation
            .as_ref()
            .map(|presentation| presentation.is_explicit_invalid)
            .flatten()
            .unwrap_or(false)
    }

    #[must_use]
    /// Checks if the candidate is explicitly marked as blank based on its presentation configuration.
    pub fn is_explicit_blank(&self) -> bool {
        self.presentation
            .as_ref()
            .map(|presentation| presentation.is_explicit_blank)
            .flatten()
            .unwrap_or(false)
    }

    #[must_use]
    /// Checks if the candidate is disabled based on its presentation configuration.
    pub fn is_disabled(&self) -> bool {
        self.presentation
            .as_ref()
            .map(|presentation| presentation.is_disabled)
            .flatten()
            .unwrap_or(false)
    }

    #[must_use]
    /// Checks if the candidate is a write-in based on its presentation configuration.
    pub fn is_write_in(&self) -> bool {
        self.presentation
            .as_ref()
            .map(|presentation| presentation.is_write_in)
            .flatten()
            .unwrap_or(false)
    }

    /// Sets the write-in status for the candidate.
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
    /// Checkbox icon by default
    #[strum(serialize = "square-checkbox")]
    #[serde(rename = "square-checkbox")]
    #[default]
    SQUARE_CHECKBOX,
    /// RadioButton icon
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
    /// Policy for weighted voting.
    pub weighted_voting_policy: Option<WeightedVotingPolicy>,
    /// Ceremonies policy.
    /// (Whether the ceremonies should be automated)
    pub ceremonies_policy: Option<CeremoniesPolicy>,
    /// Policy for delegated voting.
    pub delegated_voting_policy: Option<DelegatedVotingPolicy>,
}

impl ElectionEvent {
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
pub struct ElectionPresentation {
    pub i18n: Option<I18nContent<I18nContent<Option<String>>>>,
    pub dates: Option<VotingPeriodDates>,
    pub language_conf: Option<ElectionEventLanguageConf>,
    pub contests_order: Option<ContestsOrder>,
    pub audit_button_cfg: Option<AuditButtonCfg>,
    pub sort_order: Option<i64>,
    pub cast_vote_confirm: Option<bool>,
    pub cast_vote_gold_level: Option<CastVoteGoldLevelPolicy>,
    pub start_screen_title_policy: Option<StartScreenTitlePolicy>,
    pub is_grace_priod: Option<bool>,
    pub grace_period_policy: Option<EGracePeriodPolicy>,
    pub grace_period_secs: Option<u64>,
    pub init_report: Option<InitReport>,
    pub manual_start_voting_period: Option<ManualStartVotingPeriod>,
    pub voting_period_end: Option<VotingPeriodEnd>,
    pub tally: Option<Tally>,
    pub initialization_report_policy: Option<EInitializeReportPolicy>,
    pub security_confirmation_policy: Option<ESecurityConfirmationPolicy>,
    pub consolidated_report_policy: Option<ConsolidatedReportPolicy>,
}

impl core::Election {
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
pub struct AreaPresentation {
    pub allow_early_voting: Option<EarlyVotingPolicy>,
}

impl AreaPresentation {
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
pub struct Contest {
    pub id: String,
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
}

impl Contest {
    #[must_use]
    pub fn allow_writeins(&self) -> bool {
        self.presentation
            .as_ref()
            .map(|presentation| presentation.allow_writeins)
            .flatten()
            .unwrap_or(false)
    }

    #[must_use]
    pub fn get_counting_algorithm(&self) -> CountingAlgType {
        self.counting_algorithm.unwrap_or_default()
    }

    #[must_use]
    pub fn base32_writeins(&self) -> bool {
        self.presentation
            .as_ref()
            .map(|presentation| presentation.base32_writeins)
            .flatten()
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
    pub fn cumulative_number_of_checkboxes(&self) -> u64 {
        self.presentation
            .as_ref()
            .map(|presentation| {
                presentation.cumulative_number_of_checkboxes.unwrap_or(1)
            })
            .unwrap_or(1)
    }

    #[must_use]
    pub fn show_points(&self) -> bool {
        self.presentation
            .as_ref()
            .map(|presentation| presentation.show_points)
            .flatten()
            .unwrap_or(false)
    }

    #[must_use]
    pub fn get_invalid_candidate_ids(&self) -> Vec<String> {
        self.candidates
            .iter()
            .filter(|candidate| candidate.is_explicit_invalid())
            .collect::<Vec<&Candidate>>()
            .iter()
            .map(|candidate| candidate.id.clone())
            .collect()
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
pub enum Enrollment {
    #[default]
    #[strum(serialize = "enabled")]
    #[serde(rename = "enabled")]
    ENABLED,
    #[strum(serialize = "disabled")]
    #[serde(rename = "disabled")]
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
pub enum Otp {
    #[default]
    #[strum(serialize = "enabled")]
    #[serde(rename = "enabled")]
    ENABLED,
    #[strum(serialize = "disabled")]
    #[serde(rename = "disabled")]
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
pub enum DecodedBallotsInclusionPolicy {
    #[strum(serialize = "included")]
    #[serde(rename = "included")]
    INCLUDED,
    #[default]
    #[strum(serialize = "not-included")]
    #[serde(rename = "not-included")]
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
pub enum ContestEncryptionPolicy {
    #[strum(serialize = "multiple-contests")]
    #[serde(rename = "multiple-contests")]
    MULTIPLE_CONTESTS,
    #[default]
    #[strum(serialize = "single-contest")]
    #[serde(rename = "single-contest")]
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
pub enum VoterSigningPolicy {
    #[default]
    #[strum(serialize = "no-signature")]
    #[serde(rename = "no-signature")]
    NO_SIGNATURE,
    #[strum(serialize = "with-signature")]
    #[serde(rename = "with-signature")]
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
pub enum LockedDown {
    #[strum(serialize = "locked-down")]
    #[serde(rename = "locked-down")]
    LOCKED_DOWN,
    #[default]
    #[strum(serialize = "not-locked-down")]
    #[serde(rename = "not-locked-down")]
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
pub enum Publish {
    #[default]
    #[strum(serialize = "always")]
    #[serde(rename = "always")]
    ALWAYS,
    #[strum(serialize = "after-lockdown")]
    #[serde(rename = "after-lockdown")]
    AFTER_LOCKDOWN,
}

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
#[serde(default)]
pub struct ElectionEventStatus {
    pub is_published: Option<bool>,
    pub voting_status: VotingStatus,
    pub kiosk_voting_status: VotingStatus,
    pub early_voting_status: VotingStatus,
    pub voting_period_dates: PeriodDates,
    pub kiosk_voting_period_dates: PeriodDates,
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

    /// Sets the voting status for the given channel and updates the
    /// corresponding period dates.
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
pub enum VotingStatus {
    #[default]
    NOT_STARTED,
    OPEN,
    PAUSED,
    CLOSED,
}

impl VotingStatus {
    #[must_use]
    /// Returns true if the voting status is NOT_STARTED.
    pub const fn is_not_started(&self) -> bool {
        match self {
            VotingStatus::NOT_STARTED => true,
            VotingStatus::OPEN
            | VotingStatus::PAUSED
            | VotingStatus::CLOSED => false,
        }
    }

    #[must_use]
    /// Returns true if the voting status is any but NOT_STARTED.
    pub const fn is_started(&self) -> bool {
        !self.is_not_started()
    }

    #[must_use]
    /// Returns true if the voting status is OPEN or PAUSED.
    pub const fn is_open(&self) -> bool {
        match self {
            VotingStatus::OPEN => true,
            VotingStatus::NOT_STARTED
            | VotingStatus::PAUSED
            | VotingStatus::CLOSED => false,
        }
    }

    /// Returns true if the voting status is PAUSED.
    pub const fn is_paused(&self) -> bool {
        match self {
            VotingStatus::PAUSED => true,
            VotingStatus::NOT_STARTED
            | VotingStatus::OPEN
            | VotingStatus::CLOSED => false,
        }
    }

    #[must_use]
    /// Returns true if the voting status is CLOSED.
    pub const fn is_closed(&self) -> bool {
        match self {
            VotingStatus::CLOSED => true,
            VotingStatus::NOT_STARTED
            | VotingStatus::OPEN
            | VotingStatus::PAUSED => false,
        }
    }

    #[must_use]
    /// Returns true if the voting status is NOT_STARTED or CLOSED.
    pub const fn is_closed_or_never_started(&self) -> bool {
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
pub enum AllowTallyStatus {
    #[default]
    #[strum(serialize = "allowed")]
    #[serde(rename = "allowed")]
    ALLOWED,
    #[strum(serialize = "disallowed")]
    #[serde(rename = "disallowed")]
    DISALLOWED,
    #[strum(serialize = "requires-voting-period-end")]
    #[serde(rename = "requires-voting-period-end")]
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
pub enum VotingStatusChannel {
    ONLINE,
    KIOSK,
    EARLY_VOTING,
}

impl VotingStatusChannel {
    /// Returns the channel status from VotingChannels.
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
pub struct ElectionEventStatistics {
    pub num_emails_sent: Option<i64>,
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
pub struct ElectionStatistics {
    pub num_emails_sent: Option<i64>,
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
pub enum InitReport {
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
pub enum ManualStartVotingPeriod {
    #[default]
    #[strum(serialize = "allowed")]
    #[serde(rename = "allowed")]
    ALLOWED,
    #[strum(serialize = "only-when-initialization-report-has-been-performed")]
    #[serde(rename = "only-when-initialization-report-has-been-performed")]
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
pub enum Tally {
    #[default]
    #[strum(serialize = "always-allow")]
    #[serde(rename = "always-allow")]
    ALWAYS_ALLOW,
    #[strum(serialize = "allow-when-voting-period-ends")]
    #[serde(rename = "allow-when-voting-period-ends")]
    ONLY_WHEN_VOTING_PERIOD_ENDS,
}

#[derive(
    Serialize, Deserialize, PartialEq, Eq, JsonSchema, Debug, Clone, Default,
)]
pub struct PeriodDates {
    pub first_started_at: Option<DateTime<Utc>>,
    pub last_started_at: Option<DateTime<Utc>>,
    pub first_paused_at: Option<DateTime<Utc>>,
    pub last_paused_at: Option<DateTime<Utc>>,
    pub first_stopped_at: Option<DateTime<Utc>>,
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
pub struct ScheduledEventDates {
    pub scheduled_at: Option<String>,
    pub stopped_at: Option<String>,
}

impl PeriodDates {
    /// Updates the period dates based on the new voting status.
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
            scheduled_event_dates: Default::default(),
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
pub struct ElectionStatus {
    pub is_published: Option<bool>,
    pub voting_status: VotingStatus,
    pub init_report: InitReport,
    pub kiosk_voting_status: VotingStatus,
    pub early_voting_status: VotingStatus,
    pub voting_period_dates: PeriodDates,
    pub kiosk_voting_period_dates: PeriodDates,
    pub early_voting_period_dates: PeriodDates,
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
    #[must_use]
    /// Returns the voting status of the given channel.
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

    /// Sets the voting status for the given channel and updates the
    /// corresponding period dates.
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
    pub id: String,
    pub tenant_id: String,
    pub election_event_id: String,
    pub election_id: String,
    pub num_allowed_revotes: Option<i64>,
    pub description: Option<String>,
    pub public_key: Option<PublicKeyConfig>,
    pub area_id: String,
    pub area_presentation: Option<AreaPresentation>,
    pub contests: Vec<Contest>,
    pub election_event_presentation: Option<ElectionEventPresentation>,
    pub election_presentation: Option<ElectionPresentation>,
    pub election_dates: Option<StringifiedPeriodDates>,
    pub election_event_annotations: Option<HashMap<String, String>>,
    pub election_annotations: Option<HashMap<String, String>>,
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
pub struct CustomUrls {
    pub login: Option<String>,
    pub enrollment: Option<String>,
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
pub struct AreaAnnotations {
    pub weight: Option<Weight>,
    pub tally_operation: Option<TallyOperation>,
}

impl AreaAnnotations {
    pub fn get_weight(&self) -> Weight {
        self.weight.unwrap_or_default()
    }
}

impl Area {
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
/// Policy to determine if and when weighted voting is allowed (by areas).
pub enum WeightedVotingPolicy {
    /// Weighted voting is not allowed.
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
/// Policy to determine if and when the consolidated report should be generated.
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
