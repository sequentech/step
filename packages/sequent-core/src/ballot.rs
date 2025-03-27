// SPDX-FileCopyrightText: 2022-2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
#![allow(non_snake_case)]
#![allow(dead_code)]
use crate::ballot_codec::PlaintextCodec;
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
use strand::zkp::Schnorr;
use strand::{backend::ristretto::RistrettoCtx, context::Ctx};
use strum_macros::{Display, EnumString, IntoStaticStr};

pub const TYPES_VERSION: u32 = 1;

pub type I18nContent<T = Option<String>> = HashMap<String, T>;

pub type Annotations = HashMap<String, String>;

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
    pub is_explicit_blank: Option<bool>,
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
    pub annotations: Option<Annotations>,
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
pub enum CandidatesOrder {
    #[strum(serialize = "random")]
    #[serde(rename = "random")]
    Random,
    #[strum(serialize = "custom")]
    #[serde(rename = "custom")]
    Custom,
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
pub enum ContestsOrder {
    #[strum(serialize = "random")]
    #[serde(rename = "random")]
    Random,
    #[strum(serialize = "custom")]
    #[serde(rename = "custom")]
    Custom,
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
pub enum CastVoteGoldLevelPolicy {
    #[strum(serialize = "gold-level")]
    #[serde(rename = "gold-level")]
    GoldLevel,
    #[strum(serialize = "no-gold-level")]
    #[serde(rename = "no-gold-level")]
    #[default]
    NoGoldLevel,
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
pub enum AuditButtonCfg {
    #[strum(serialize = "show")]
    #[serde(rename = "show")]
    #[default]
    SHOW,
    #[strum(serialize = "not-show")]
    #[serde(rename = "not-show")]
    NOT_SHOW,
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
pub enum ElectionsOrder {
    #[strum(serialize = "random")]
    #[serde(rename = "random")]
    Random,
    #[strum(serialize = "custom")]
    #[serde(rename = "custom")]
    Custom,
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
pub struct Election {
    pub id: String,
    pub election_event_id: String,
    pub tenant_id: String,
    pub name: Option<String>,
    pub name_i18n: Option<I18nContent>,
    pub description: Option<String>,
    pub description_i18n: Option<I18nContent>,
    pub alias: Option<String>,
    pub alias_i18n: Option<I18nContent>,
    pub image_document_id: Option<String>,
    pub contests: Vec<Contest>,
    pub presentation: Option<ElectionPresentation>,
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
pub enum InvalidVotePolicy {
    #[strum(serialize = "allowed")]
    #[serde(rename = "allowed")]
    #[default]
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
    #[strum(serialize = "square-checkbox")]
    #[serde(rename = "square-checkbox")]
    #[default]
    SQUARE_CHECKBOX, // Checkbox icon by default
    #[strum(serialize = "round-checkbox")]
    #[serde(rename = "round-checkbox")]
    ROUND_CHECKBOX, // RadioButton icon
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
pub enum KeysCeremonyPolicy {
    #[strum(serialize = "ELECTION_EVENT")]
    #[serde(rename = "ELECTION_EVENT")]
    #[default]
    ELECTION_EVENT,
    #[strum(serialize = "ELECTION")]
    #[serde(rename = "ELECTION")]
    ELECTION,
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
    pub skip_election_list: Option<bool>,
    pub show_user_profile: Option<bool>, // default is true
    pub elections_order: Option<ElectionsOrder>,
    pub voting_portal_countdown_policy: Option<VotingPortalCountdownPolicy>,
    pub custom_urls: Option<CustomUrls>,
    pub keys_ceremony_policy: Option<KeysCeremonyPolicy>,
    pub contest_encryption_policy: Option<ContestEncryptionPolicy>,
    pub locked_down: Option<LockedDown>,
    pub publish_policy: Option<Publish>,
    pub enrollment: Option<Enrollment>,
    pub otp: Option<Otp>,
    pub voter_signing_policy: Option<VoterSigningPolicy>,
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
pub enum EGracePeriodPolicy {
    #[strum(serialize = "no-grace-period")]
    #[serde(rename = "no-grace-period")]
    NO_GRACE_PERIOD,
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
pub struct VotingPeriodDates {
    pub start_date: Option<String>,
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
)]
pub enum EInitializeReportPolicy {
    #[strum(serialize = "required")]
    #[serde(rename = "required")]
    REQUIRED,
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
pub struct VotingPortalCountdownPolicy {
    pub policy: Option<ECountdownPolicy>,
    pub countdown_anticipation_secs: Option<u64>,
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
pub enum ECountdownPolicy {
    NO_COUNTDOWN,
    COUNTDOWN,
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
pub enum EUnderVotePolicy {
    #[strum(serialize = "allowed")]
    #[serde(rename = "allowed")]
    #[default]
    ALLOWED,
    #[strum(serialize = "warn")]
    #[serde(rename = "warn")]
    WARN,
    #[strum(serialize = "warn-only-in-review")]
    #[serde(rename = "warn-only-in-review")]
    WARN_ONLY_IN_REVIEW,
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
pub enum EBlankVotePolicy {
    #[strum(serialize = "allowed")]
    #[serde(rename = "allowed")]
    #[default]
    ALLOWED,
    #[strum(serialize = "warn")]
    #[serde(rename = "warn")]
    WARN,
    #[strum(serialize = "warn-only-in-review")]
    #[serde(rename = "warn-only-in-review")]
    WARN_ONLY_IN_REVIEW,
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
pub enum EOverVotePolicy {
    #[strum(serialize = "allowed")]
    #[serde(rename = "allowed")]
    ALLOWED,
    #[strum(serialize = "allowed-with-msg")]
    #[serde(rename = "allowed-with-msg")]
    ALLOWED_WITH_MSG,
    #[strum(serialize = "allowed-with-msg-and-alert")]
    #[serde(rename = "allowed-with-msg-and-alert")]
    #[default]
    ALLOWED_WITH_MSG_AND_ALERT,
    #[strum(serialize = "not-allowed-with-msg-and-alert")]
    #[serde(rename = "not-allowed-with-msg-and-alert")]
    NOT_ALLOWED_WITH_MSG_AND_ALERT,
    #[strum(serialize = "not-allowed-with-msg-and-disable")]
    #[serde(rename = "not-allowed-with-msg-and-disable")]
    NOT_ALLOWED_WITH_MSG_AND_DISABLE,
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
    pub is_grace_priod: Option<bool>,
    pub grace_period_policy: Option<EGracePeriodPolicy>,
    pub grace_period_secs: Option<u64>,
    pub init_report: Option<InitReport>,
    pub manual_start_voting_period: Option<ManualStartVotingPeriod>,
    pub voting_period_end: Option<VotingPeriodEnd>,
    pub tally: Option<Tally>,
    pub initialization_report_policy: Option<EInitializeReportPolicy>,
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
            cast_vote_gold_level: Some(CastVoteGoldLevelPolicy::default()),
            is_grace_priod: None,
            grace_period_policy: None,
            grace_period_secs: None,
            initialization_report_policy: None,
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
    pub annotations: Option<Annotations>,
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
pub struct ElectionEventStatus {
    pub is_published: Option<bool>,
    pub voting_status: VotingStatus,
    pub kiosk_voting_status: VotingStatus,
    pub voting_period_dates: PeriodDates,
    pub kiosk_voting_period_dates: PeriodDates,
}

impl Default for ElectionEventStatus {
    fn default() -> Self {
        ElectionEventStatus {
            is_published: Some(false),
            voting_status: VotingStatus::NOT_STARTED,
            kiosk_voting_status: VotingStatus::NOT_STARTED,
            voting_period_dates: Default::default(),
            kiosk_voting_period_dates: Default::default(),
        }
    }
}

impl ElectionEventStatus {
    pub fn status_by_channel(
        &self,
        channel: &VotingStatusChannel,
    ) -> VotingStatus {
        match channel {
            &VotingStatusChannel::ONLINE => self.voting_status.clone(),
            &VotingStatusChannel::KIOSK => self.kiosk_voting_status.clone(),
        }
    }

    pub fn set_status_by_channel(
        &mut self,
        channel: &VotingStatusChannel,
        new_status: VotingStatus,
    ) {
        let mut period_dates = match channel {
            &VotingStatusChannel::ONLINE => {
                self.voting_status = new_status.clone();
                &mut self.voting_period_dates
            }
            &VotingStatusChannel::KIOSK => {
                self.kiosk_voting_status = new_status.clone();
                &mut self.kiosk_voting_period_dates
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
    EnumString,
    JsonSchema,
    IntoStaticStr,
)]
pub enum VotingStatusChannel {
    ONLINE,
    KIOSK,
}

impl VotingStatusChannel {
    pub fn channel_from(
        &self,
        channels: &core::VotingChannels,
    ) -> Option<bool> {
        match self {
            &VotingStatusChannel::ONLINE => channels.online.clone(),
            &VotingStatusChannel::KIOSK => channels.kiosk.clone(),
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

// Helper method to format the date or return "-"
pub fn format_date(date: &Option<DateTime<Utc>>, default: &str) -> String {
    date.map_or(default.to_string(), |d| d.to_rfc3339())
}

pub fn format_date_opt(date: &Option<DateTime<Utc>>) -> Option<String> {
    date.map(|d| d.to_rfc3339())
}

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
pub struct ElectionStatus {
    pub is_published: Option<bool>,
    pub voting_status: VotingStatus,
    pub init_report: InitReport,
    pub kiosk_voting_status: VotingStatus,
    pub voting_period_dates: PeriodDates,
    pub kiosk_voting_period_dates: PeriodDates,
    pub allow_tally: AllowTallyStatus,
}

impl Default for ElectionStatus {
    fn default() -> Self {
        ElectionStatus {
            is_published: Some(false),
            voting_status: VotingStatus::NOT_STARTED,
            init_report: InitReport::ALLOWED,
            kiosk_voting_status: VotingStatus::NOT_STARTED,
            voting_period_dates: Default::default(),
            kiosk_voting_period_dates: Default::default(),
            allow_tally: Default::default(),
        }
    }
}

impl ElectionStatus {
    pub fn status_by_channel(
        &self,
        channel: &VotingStatusChannel,
    ) -> VotingStatus {
        match channel {
            &VotingStatusChannel::ONLINE => self.voting_status.clone(),
            &VotingStatusChannel::KIOSK => self.kiosk_voting_status.clone(),
        }
    }

    pub fn dates_by_channel(
        &self,
        channel: &VotingStatusChannel,
    ) -> PeriodDates {
        match channel {
            &VotingStatusChannel::ONLINE => self.voting_period_dates.clone(),
            &VotingStatusChannel::KIOSK => {
                self.kiosk_voting_period_dates.clone()
            }
        }
    }

    pub fn set_status_by_channel(
        &mut self,
        channel: &VotingStatusChannel,
        new_status: VotingStatus,
    ) {
        let period_dates = match channel {
            &VotingStatusChannel::ONLINE => {
                self.voting_status = new_status.clone();
                &mut self.voting_period_dates
            }
            &VotingStatusChannel::KIOSK => {
                self.kiosk_voting_status = new_status.clone();
                &mut self.kiosk_voting_period_dates
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
    pub contests: Vec<Contest>,
    pub election_event_presentation: Option<ElectionEventPresentation>,
    pub election_presentation: Option<ElectionPresentation>,
    pub election_dates: Option<StringifiedPeriodDates>,
    pub election_event_annotations: Option<HashMap<String, String>>,
    pub election_annotations: Option<HashMap<String, String>>,
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
