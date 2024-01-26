// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
#![allow(non_snake_case)]
#![allow(dead_code)]
use crate::types::hasura_types::Uuid;
use borsh::{BorshDeserialize, BorshSerialize};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, default::Default};
use strand::context::Ctx;
use strand::elgamal::Ciphertext;
use strand::zkp::Schnorr;
use strum_macros::{Display, EnumString};

pub const TYPES_VERSION: u32 = 1;

type I18nContent = HashMap<String, String>;

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
    pub contest_id: Uuid,
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

#[derive(BorshSerialize, BorshDeserialize, PartialEq, Eq, Debug, Clone)]
pub struct AuditableBallot<C: Ctx> {
    pub version: u32,
    pub issue_date: String,
    pub config: BallotStyle,
    pub contests: Vec<AuditableBallotContest<C>>,
    pub ballot_hash: String,
}

#[derive(BorshSerialize, BorshDeserialize, PartialEq, Eq, Debug, Clone)]
pub struct HashableBallotContest<C: Ctx> {
    pub contest_id: Uuid,
    pub ciphertext: Ciphertext<C>,
    pub proof: Schnorr<C>,
}

#[derive(BorshSerialize, BorshDeserialize, PartialEq, Eq, Debug, Clone)]
pub struct HashableBallot<C: Ctx> {
    pub version: u32,
    pub issue_date: String,
    pub contests: Vec<HashableBallotContest<C>>,
    pub config: BallotStyle,
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

impl<C: Ctx> From<&AuditableBallot<C>> for HashableBallot<C> {
    fn from(value: &AuditableBallot<C>) -> HashableBallot<C> {
        assert!(TYPES_VERSION == value.version);
        HashableBallot {
            version: TYPES_VERSION,
            issue_date: value.issue_date.clone(),
            contests: value
                .contests
                .iter()
                .map(|contest| HashableBallotContest::<C>::from(contest))
                .collect(),
            config: value.config.clone(),
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
)]
pub struct CandidatePresentation {
    pub is_explicit_invalid: bool,
    pub is_category_list: bool,
    pub invalid_vote_position: Option<String>, // top|bottom
    pub is_write_in: bool,
    pub sort_order: Option<i64>,
    pub urls: Option<Vec<CandidateUrl>>,
}

impl CandidatePresentation {
    pub fn new() -> CandidatePresentation {
        CandidatePresentation {
            is_explicit_invalid: false,
            is_category_list: false,
            invalid_vote_position: None,
            is_write_in: false,
            sort_order: None,
            urls: None,
        }
    }
}

impl Default for CandidatePresentation {
    fn default() -> Self {
        CandidatePresentation::new()
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
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub election_event_id: Uuid,
    pub election_id: Uuid,
    pub contest_id: Uuid,
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
            .unwrap_or(false)
    }

    pub fn is_explicit_invalid(&self) -> bool {
        self.presentation
            .as_ref()
            .map(|presentation| presentation.is_explicit_invalid)
            .unwrap_or(false)
    }

    pub fn is_write_in(&self) -> bool {
        self.presentation
            .as_ref()
            .map(|presentation| presentation.is_write_in)
            .unwrap_or(false)
    }

    pub fn set_is_write_in(&mut self, is_write_in: bool) {
        let mut presentation =
            self.presentation.clone().unwrap_or(CandidatePresentation {
                is_explicit_invalid: false,
                is_category_list: false,
                is_write_in: false,
                sort_order: Some(0),
                urls: None,
                invalid_vote_position: None,
            });
        presentation.is_write_in = is_write_in;
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
    pub allow_writeins: bool,
    pub base32_writeins: bool,
    pub invalid_vote_policy: String, /* allowed|warn|warn-invalid-implicit-and-explicit */
    pub cumulative_number_of_checkboxes: Option<u64>,
    pub shuffle_categories: bool,
    pub shuffle_all_options: bool,
    pub shuffle_category_list: Option<Vec<String>>,
    pub show_points: bool,
    pub enable_checkable_lists: Option<String>, /* disabled|allow-selecting-candidates-and-lists|allow-selecting-candidates|allow-selecting-lists */
    pub candidates_order: Option<CandidatesOrder>,
}

impl ContestPresentation {
    pub fn new() -> ContestPresentation {
        ContestPresentation {
            allow_writeins: true,
            base32_writeins: true,
            invalid_vote_policy: "allowed".into(),
            cumulative_number_of_checkboxes: None,
            shuffle_categories: false,
            shuffle_all_options: false,
            shuffle_category_list: None,
            show_points: false,
            enable_checkable_lists: None,
            candidates_order: None,
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
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub election_event_id: Uuid,
    pub election_id: Uuid,
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
}

impl Contest {
    pub fn allow_writeins(&self) -> bool {
        self.presentation
            .as_ref()
            .map(|presentation| presentation.allow_writeins)
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
            .unwrap_or(true)
    }

    pub fn allow_explicit_invalid(&self) -> bool {
        self.presentation
            .as_ref()
            .map(|presentation| {
                [
                    "allowed".to_string(),
                    "warn".to_string(),
                    "warn-invalid-implicit-and-explicit".to_string(),
                ]
                .contains(&presentation.invalid_vote_policy)
            })
            .unwrap_or(false)
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
            .unwrap_or(false)
    }

    pub fn get_invalid_candidate_ids(&self) -> Vec<Uuid> {
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
    Default,
)]
pub struct ElectionEventStatistics {
    pub num_emails_sent: i64,
    pub num_sms_sent: i64,
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
    Default,
)]
pub struct ElectionStatistics {
    pub num_emails_sent: i64,
    pub num_sms_sent: i64,
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
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub election_event_id: Uuid,
    pub election_id: Uuid,
    pub description: Option<String>,
    pub public_key: Option<PublicKeyConfig>,
    pub area_id: Uuid,
    pub contests: Vec<Contest>,
}
