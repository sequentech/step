// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
#![allow(non_snake_case)]
#![allow(dead_code)]
use crate::error::BallotError;
use crate::serialization::base64::{Base64Deserialize, Base64Serialize};
use borsh::{BorshDeserialize, BorshSerialize};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::Serializer;
use std::{collections::HashMap, default::Default};
use strand::elgamal::Ciphertext;
use strand::zkp::Schnorr;
use strand::{backend::ristretto::RistrettoCtx, context::Ctx};
use strum_macros::{Display, EnumString};

pub const TYPES_VERSION: u32 = 1;

pub type I18nContent<T = Option<String>> = HashMap<String, T>;

#[derive(BorshSerialize, BorshDeserialize, PartialEq, Eq, Debug, Clone)]
pub struct ReplicationChoice<C: Ctx> {
    pub ciphertext: Ciphertext<C>,
    pub plaintext: C::P,
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
pub struct PublicKeyConfig {
    pub public_key: String,
    pub is_demo: bool,
}

#[derive(BorshSerialize, BorshDeserialize, PartialEq, Eq, Debug, Clone)]
pub struct AuditableBallotContest<C: Ctx> {
    pub contest_id: String,
    pub choice: ReplicationChoice<C>,
    pub proof: Schnorr<C>,
}

#[derive(BorshSerialize, BorshDeserialize, PartialEq, Eq, Debug, Clone)]
pub struct RawAuditableBallot<C: Ctx> {
    pub election_url: String,
    pub issue_date: String,
    pub contests: Vec<AuditableBallotContest<C>>,
    pub ballot_hash: String,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct AuditableBallot {
    pub version: u32,
    pub issue_date: String,
    pub config: BallotStyle,
    pub contests: Vec<String>, // Vec<AuditableBallotContest<C>>,
    pub ballot_hash: String,
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

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct HashableBallot {
    pub version: u32,
    pub issue_date: String,
    pub contests: Vec<String>, // Vec<HashableBallotContest<C>>,
    pub config: BallotStyle,
}

#[derive(BorshSerialize, BorshDeserialize, PartialEq, Eq, Debug, Clone)]
pub struct RawHashableBallot<C: Ctx> {
    pub version: u32,
    pub issue_date: String,
    pub contests: Vec<HashableBallotContest<C>>,
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

impl<C: Ctx> From<&AuditableBallotContest<C>> for HashableBallotContest<C> {
    fn from(value: &AuditableBallotContest<C>) -> HashableBallotContest<C> {
        HashableBallotContest {
            contest_id: value.contest_id.clone(),
            ciphertext: value.choice.ciphertext.clone(),
            proof: value.proof.clone(),
        }
    }
}

impl TryFrom<&AuditableBallot> for HashableBallot {
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

        Ok(HashableBallot {
            version: TYPES_VERSION,
            issue_date: value.issue_date.clone(),
            contests: HashableBallot::serialize_contests::<RistrettoCtx>(
                &hashable_ballot_contest,
            )?,
            config: value.config.clone(),
        })
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
)]
pub struct CandidateUrl {
    pub url: String,
    pub kind: Option<String>,
    pub title: Option<String>,
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
pub struct CandidatePresentation {
    pub i18n: Option<I18nContent<I18nContent<Option<String>>>>,
    pub is_explicit_invalid: Option<bool>,
    pub is_disabled: Option<bool>,
    pub is_category_list: Option<bool>,
    pub invalid_vote_position: Option<String>, // top|bottom
    pub is_write_in: Option<bool>,
    pub sort_order: Option<i64>,
    pub urls: Option<Vec<CandidateUrl>>,
    pub subtype: Option<String>,
}

impl CandidatePresentation {
    pub fn new() -> CandidatePresentation {
        CandidatePresentation {
            i18n: None,
            is_explicit_invalid: Some(false),
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
)]
pub struct Candidate {
    pub id: String,
    pub tenant_id: String,
    pub election_event_id: String,
    pub election_id: String,
    pub contest_id: String,
    pub name: Option<String>,
    pub name_i18n: Option<I18nContent>,
    pub description: Option<String>,
    pub description_i18n: Option<I18nContent>,
    pub alias: Option<String>,
    pub alias_i18n: Option<I18nContent>,
    pub candidate_type: Option<String>,
    pub presentation: Option<CandidatePresentation>,
}

impl Candidate {
    pub fn is_category_list(&self) -> bool {
        self.presentation
            .as_ref()
            .map(|presentation| presentation.is_category_list)
            .flatten()
            .unwrap_or(false)
    }

    pub fn is_explicit_invalid(&self) -> bool {
        self.presentation
            .as_ref()
            .map(|presentation| presentation.is_explicit_invalid)
            .flatten()
            .unwrap_or(false)
    }

    pub fn is_disabled(&self) -> bool {
        self.presentation
            .as_ref()
            .map(|presentation| presentation.is_disabled)
            .flatten()
            .unwrap_or(false)
    }

    pub fn is_write_in(&self) -> bool {
        self.presentation
            .as_ref()
            .map(|presentation| presentation.is_write_in)
            .flatten()
            .unwrap_or(false)
    }

    pub fn set_is_write_in(&mut self, is_write_in: bool) {
        let mut presentation =
            self.presentation.clone().unwrap_or(CandidatePresentation {
                i18n: None,
                is_explicit_invalid: Some(false),
                is_disabled: Some(false),
                is_category_list: Some(false),
                is_write_in: Some(false),
                sort_order: Some(0),
                urls: None,
                invalid_vote_position: None,
                subtype: None,
            });
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
)]
pub enum CandidatesOrder {
    #[strum(serialize = "random")]
    #[serde(rename = "random")]
    Random,
    #[strum(serialize = "custom")]
    #[serde(rename = "custom")]
    Custom,
    #[strum(serialize = "alphabetical")]
    #[serde(rename = "alphabetical")]
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
)]
pub enum InvalidVotePolicy {
    #[strum(serialize = "allowed")]
    #[serde(rename = "allowed")]
    ALLOWED,
    #[strum(serialize = "warn")]
    #[serde(rename = "warn")]
    WARN,
    #[strum(serialize = "warn-invalid-implicit-and-explicit")]
    #[serde(rename = "warn-invalid-implicit-and-explicit")]
    WARN_INVALID_IMPLICIT_AND_EXPLICIT,
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
pub enum CandidatesSelectionPolicy {
    #[strum(serialize = "radio")]
    #[serde(rename = "radio")]
    RADIO, // if you select one, the previously selected one gets unselected
    #[strum(serialize = "cumulative")]
    #[serde(rename = "cumulative")]
    CUMULATIVE, // default behaviour
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
pub struct ElectionEventMaterials {
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
pub struct ElectionEventLanguageConf {
    pub enabled_language_codes: Option<Vec<String>>,
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
pub struct ElectionEventPresentation {
    pub i18n: Option<I18nContent<I18nContent<Option<String>>>>,
    pub materials: Option<ElectionEventMaterials>,
    pub language_conf: Option<ElectionEventLanguageConf>,
    pub logo_url: Option<String>,
    pub redirect_finish_url: Option<String>,
    pub css: Option<String>,
    pub hide_audit: Option<bool>,
    pub skip_election_list: Option<bool>,
    pub show_user_profile: Option<bool>, // default is true
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
pub struct ElectionDates {
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub scheduled_closing: Option<bool>,
    pub scheduled_opening: Option<bool>,
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
pub struct ElectionPresentation {
    pub i18n: Option<I18nContent<I18nContent<Option<String>>>>,
    pub dates: Option<ElectionDates>,
    pub language_conf: Option<ElectionEventLanguageConf>,
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
    pub cumulative_number_of_checkboxes: Option<u64>,
    pub shuffle_categories: Option<bool>,
    pub shuffle_category_list: Option<Vec<String>>,
    pub show_points: Option<bool>,
    pub enable_checkable_lists: Option<String>, /* disabled|allow-selecting-candidates-and-lists|allow-selecting-candidates|allow-selecting-lists */
    pub candidates_order: Option<CandidatesOrder>,
    pub candidates_selection_policy: Option<CandidatesSelectionPolicy>,
    pub max_selections_per_type: Option<u64>,
    pub types_presentation: Option<HashMap<String, Option<TypePresentation>>>,
    pub under_vote_alert: Option<bool>,
}

impl ContestPresentation {
    pub fn new() -> ContestPresentation {
        ContestPresentation {
            i18n: None,
            allow_writeins: Some(true),
            base32_writeins: Some(true),
            invalid_vote_policy: Some(InvalidVotePolicy::ALLOWED),
            cumulative_number_of_checkboxes: None,
            shuffle_categories: Some(false),
            shuffle_category_list: None,
            show_points: Some(false),
            enable_checkable_lists: None,
            candidates_order: None,
            candidates_selection_policy: None,
            max_selections_per_type: None,
            types_presentation: None,
            under_vote_alert: Some(false),
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
    pub counting_algorithm: Option<String>, /* plurality-at-large|borda-nauru|borda|borda-mas-madrid|desborda3|desborda2|desborda|cumulative */
    pub is_encrypted: bool,
    pub candidates: Vec<Candidate>,
    pub presentation: Option<ContestPresentation>,
    pub created_at: Option<String>,
}

impl Contest {
    pub fn allow_writeins(&self) -> bool {
        self.presentation
            .as_ref()
            .map(|presentation| presentation.allow_writeins)
            .flatten()
            .unwrap_or(false)
    }

    pub fn get_counting_algorithm(&self) -> String {
        self.counting_algorithm
            .clone()
            .unwrap_or("plurality-at-large".into())
    }

    pub fn base32_writeins(&self) -> bool {
        self.presentation
            .as_ref()
            .map(|presentation| presentation.base32_writeins)
            .flatten()
            .unwrap_or(true)
    }

    pub fn allow_explicit_invalid(&self) -> bool {
        let invalid_vote_policy = self
            .presentation
            .clone()
            .unwrap_or(ContestPresentation::new())
            .invalid_vote_policy
            .unwrap_or(InvalidVotePolicy::ALLOWED);

        [InvalidVotePolicy::ALLOWED, InvalidVotePolicy::WARN]
            .contains(&invalid_vote_policy)
    }

    pub fn cumulative_number_of_checkboxes(&self) -> u64 {
        self.presentation
            .as_ref()
            .map(|presentation| {
                presentation.cumulative_number_of_checkboxes.unwrap_or(1)
            })
            .unwrap_or(1)
    }

    pub fn show_points(&self) -> bool {
        self.presentation
            .as_ref()
            .map(|presentation| presentation.show_points)
            .flatten()
            .unwrap_or(false)
    }

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
pub struct ElectionEventStatus {
    pub config_created: Option<bool>,
    pub keys_ceremony_finished: Option<bool>,
    pub tally_ceremony_finished: Option<bool>,
    pub is_published: Option<bool>,
    pub voting_status: VotingStatus,
}

impl Default for ElectionEventStatus {
    fn default() -> Self {
        ElectionEventStatus {
            config_created: Some(false),
            keys_ceremony_finished: Some(false),
            tally_ceremony_finished: Some(false),
            is_published: Some(false),
            voting_status: VotingStatus::NOT_STARTED,
        }
    }
}

impl ElectionEventStatus {
    pub fn is_config_created(&self) -> bool {
        self.config_created.unwrap_or(false)
    }
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
    JsonSchema,
)]
pub enum VotingStatus {
    NOT_STARTED,
    OPEN,
    PAUSED,
    CLOSED,
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
pub struct ElectionStatus {
    pub voting_status: VotingStatus,
}

impl Default for ElectionStatus {
    fn default() -> Self {
        ElectionStatus {
            voting_status: VotingStatus::NOT_STARTED,
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
pub struct BallotStyle {
    pub id: String,
    pub tenant_id: String,
    pub election_event_id: String,
    pub election_id: String,
    pub num_allowed_revotes: Option<i64>,
    pub description: Option<String>,
    pub public_key: Option<PublicKeyConfig>,
    pub area_id: String,
    pub contests: Vec<Contest>,
    pub election_event_presentation: Option<ElectionEventPresentation>,
    pub election_presentation: Option<ElectionPresentation>,
}
