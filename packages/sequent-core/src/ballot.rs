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
        self.urls
            .iter()
            .any(|url| url.title == "invalidVoteFlag" && url.url == "true")
    }
    pub fn is_write_in(&self) -> bool {
        self.urls
            .iter()
            .any(|url| url.title == "isWriteIn" && url.url == "true")
    }
    pub fn set_is_write_in(&mut self, is_write_in: bool) {
        if is_write_in == self.is_write_in() {
            return;
        }
        if is_write_in {
            self.urls.push(Url {
                title: "isWriteIn".to_string(),
                url: "true".to_string(),
            })
        } else {
            self.urls = self
                .urls
                .clone()
                .into_iter()
                .filter(|url| {
                    !(url.title == "invalidVoteFlag" && url.url == "true")
                })
                .collect();
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
pub struct ContestPresentation {
    allow_writeins: bool,
    base32_writeins: bool,
    invalid_vote_policy: String,
    cumulative_number_of_checkboxes: Option<i64>,
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
    pub counting_algorithm: Option<String>,
    pub is_encrypted: bool,
    pub answer_total_votes_percentage: String,
    pub candidates: Vec<Candidate>,
    pub presentation: Option<ContestPresentation>,
}

impl Question {
    pub fn allow_writeins(&self) -> bool {
        self.extra_options
            .as_ref()
            .map(|options| options.allow_writeins.unwrap_or(false))
            .unwrap_or(false)
    }

    pub fn base32_writeins(&self) -> bool {
        self.extra_options
            .as_ref()
            .map(|options| options.base32_writeins.unwrap_or(true))
            .unwrap_or(true)
    }

    pub fn allow_explicit_invalid(&self) -> bool {
        self.extra_options
            .as_ref()
            .map(|options| {
                vec![
                    "allowed".to_string(),
                    "warn".to_string(),
                    "warn-invalid-implicit-and-explicit".to_string(),
                ]
                .contains(
                    &options
                        .invalid_vote_policy
                        .clone()
                        .unwrap_or_else(|| "not-allowed".to_string()),
                )
            })
            .unwrap_or(false)
    }

    pub fn cumulative_number_of_checkboxes(&self) -> u64 {
        self.extra_options
            .as_ref()
            .map(|options| options.cumulative_number_of_checkboxes.unwrap_or(1))
            .unwrap_or(1)
    }

    pub fn show_points(&self) -> bool {
        self.extra_options
            .as_ref()
            .map(|options| options.show_points.unwrap_or(false))
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
pub struct BallotStyle {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub election_event_id: Uuid,
    pub election_id: Uuid,
    pub description: Option<String>,
    pub area_id: Uuid,
    pub status: Option<String>,
    pub contests: Vec<Contest>,
}
