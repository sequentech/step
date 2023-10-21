// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
#![allow(non_snake_case)]
#![allow(dead_code)]
use crate::hasura_types::Uuid;
use borsh::{BorshDeserialize, BorshSerialize};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use strand::context::Ctx;
use strand::elgamal::Ciphertext;
use strand::zkp::Schnorr;
use strum_macros::Display;
use strum_macros::EnumString;

pub const TYPES_VERSION: u32 = 1;

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
pub struct CandidatePresentation {
    pub is_explicit_invalid: bool,
    pub is_write_in: bool,
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
    pub description: Option<String>,
    pub candidate_type: Option<String>,
    pub presentation: Option<CandidatePresentation>,
}

impl Candidate {
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
                is_write_in: false,
            });
        presentation.is_write_in = is_write_in;
        self.presentation = Some(presentation);
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
pub struct ContestPresentation {
    allow_writeins: bool,
    base32_writeins: bool,
    invalid_vote_policy: String,
    cumulative_number_of_checkboxes: Option<u64>,
    show_points: bool,
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
    pub description: Option<String>,
    pub max_votes: i64,
    pub min_votes: i64,
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
                vec![
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
    pub stopped: Option<bool>,
}

impl ElectionEventStatus {
    pub fn is_config_created(&self) -> bool {
        self.config_created.unwrap_or(false)
    }

    pub fn is_stopped(&self) -> bool {
        self.stopped.unwrap_or(false)
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
//#[serde(crate = "rocket::serde")]
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
    pub status: Option<ElectionStatus>,
    pub contests: Vec<Contest>,
}
