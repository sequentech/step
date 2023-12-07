// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
#![allow(non_snake_case)]
#![allow(dead_code)]
use crate::types::hasura_types::Uuid;
use borsh::{BorshDeserialize, BorshSerialize};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
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
}

impl ContestPresentation {
    pub fn new() -> ContestPresentation {
        ContestPresentation {
            allow_writeins: true,
            base32_writeins: true,
            invalid_vote_policy: "allowed".into(),
            cumulative_number_of_checkboxes: None,
            shuffle_categories: true,
            shuffle_all_options: true,
            shuffle_category_list: None,
            show_points: false,
            enable_checkable_lists: None,
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
pub struct Contest {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub election_event_id: Uuid,
    pub election_id: Uuid,
    pub name: Option<String>,
    pub description: Option<String>,
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
    pub stopped: Option<bool>,
    pub voting_status: VotingStatus,
}

impl ElectionEventStatus {
    pub fn is_config_created(&self) -> bool {
        self.config_created.unwrap_or(false)
    }

    pub fn is_stopped(&self) -> bool {
        self.stopped.unwrap_or(false)
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
pub struct KeyCeremonyLog {
    pub created_date: String,
    pub log_text: String, 
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
)]
pub enum KeyCeremonyStatus {
    NOT_STARTED,
    IN_PROCESS,
    SUCCESS,
    CANCELLED,
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
)]
pub enum KeyCeremonyTrusteeStatus {
    WAITING,
    KEY_GENERATED,
    KEY_RETRIEVED,
    KEY_CHECKED,
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
pub struct KeyCeremonyTrustee {
    pub name: String,
    pub status: KeyCeremonyTrusteeStatus
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
pub struct KeyCeremony {
    pub start_date: String,
    pub stop_date: String,
    pub status: KeyCeremonyStatus,
    pub is_latest: bool,
    pub threshold: u8,
    pub public_key: String,
    pub logs: Vec<KeyCeremonyLog>,
    pub trustees: Vec<KeyCeremonyTrustee>,
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
    pub keys_ceremony: Vec<KeyCeremony>,
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

#[cfg(test)]
mod tests {
    use crate::ballot::HashableBallot;
    use crate::serialization::base64::Base64Deserialize;
    use strand::backend::ristretto::RistrettoCtx;

    #[test]
    fn test_deserialize_hashable_ballot() {
        let ballot_str = r#"AQAAAAoAAAAxMC8xMS8yMDIzAQAAACQAAAA2OWYyZjk4Ny00NjBjLTQ4YWMtYWM3YS00ZDQ0ZDk5YjM3ZTaG/s2pIrKndrRcGlg8ht0D7TYHk2Kf6mfx3gRRwmHWFITf7bwtu3UrZVhmAVpwGWBDG6GNuy0APnkMN6eiP4MVCGJ31h+Nzi3fjcnKZVgA99RDRtxMI9GW+hC19abXeBvM08E7VOMQ8E661Ot2Av0ubGgyLKpTmu/YGk7YB3lODFdUaawRUAa74pImRt7aRBGjjOmGqCSczRjfq0hLcv0LJAAAAGYyZTAzYTBjLWI3YzItNGMwNy1iMTk5LWViOTBiOWMzYWZiNyQAAAA5MDUwNWM4YS0yM2E5LTRjZGYtYTI2Yi00ZTE5ZjZhMDk3ZDUkAAAAMzNmMTg1MDItYTY3Yy00ODUzLTgzMzMtYTU4NjMwNjYzNTU5JAAAAGYyZjEwNjVlLWI3ODQtNDZkMS1iODFhLWM3MWJmZWI5YWQ1NQHkAAAAVGhpcyBpcyB0aGUgZGVzY3JpcHRpb24gb2YgdGhlIGVsZWN0aW9uLiBZb3UgY2FuIGFkZCBzaW1wbGUgaHRtbCBsaWtlIDxzdHJvbmc+Ym9sZDwvc3Ryb25nPiBvciA8YSBocmVmPSJodHRwczovL3NlcXVlbnRlY2guaW8iIHJlbD0ibm9mb2xsb3ciPmxpbmtzIHRvIHdlYnNpdGVzPC9hPi4KCjxiciAvPjxiciAvPllvdSBuZWVkIHRvIHVzZSB0d28gYnIgZWxlbWVudCBmb3IgbmV3IHBhcmFncmFwaHMuASsAAAB3R3ZZU0h3YVFmL0NKU3lFMDdBQXNYYUFZZm95M1VwWndoOTR0V2tvVVVzACQAAAAyZjMxMmEzNi1mMzljLTQ2ZTQtOTY3MC0xZDFjZTQ2MjU3NDUBAQEAAAAkAAAANjlmMmY5ODctNDYwYy00OGFjLWFjN2EtNGQ0NGQ5OWIzN2U2JAAAADkwNTA1YzhhLTIzYTktNGNkZi1hMjZiLTRlMTlmNmEwOTdkNSQAAAAzM2YxODUwMi1hNjdjLTQ4NTMtODMzMy1hNTg2MzA2NjM1NTkkAAAAZjJmMTA2NWUtYjc4NC00NmQxLWI4MWEtYzcxYmZlYjlhZDU1ASQAAABXaG8ncyB0aGUgYmVzdCBwcmVzaWRlbnQgb2YgdGhlIFVTQT8BEgAAAENob29zZSBhIHByZXNpZGVudAEAAAAAAAAAAQAAAAAAAAABEwAAAGZpcnN0LXBhc3QtdGhlLXBvc3QBEgAAAHBsdXJhbGl0eS1hdC1sYXJnZQEDAAAAJAAAAGEyNDMwM2RlLTU3OTgtNDdjZC05YjNlLTRmMzkxZDFiYWU3YiQAAAA5MDUwNWM4YS0yM2E5LTRjZGYtYTI2Yi00ZTE5ZjZhMDk3ZDUkAAAAMzNmMTg1MDItYTY3Yy00ODUzLTgzMzMtYTU4NjMwNjYzNTU5JAAAAGYyZjEwNjVlLWI3ODQtNDZkMS1iODFhLWM3MWJmZWI5YWQ1NSQAAAA2OWYyZjk4Ny00NjBjLTQ4YWMtYWM3YS00ZDQ0ZDk5YjM3ZTYBCQAAAEpvZSBCaWRlbgEVAAAAVGhlIGN1cnJlbnQgcHJlc2lkZW50AAAkAAAAZDkyNDkzNDUtMTFiZS00NjUyLWFkMDQtMjk4ZDcwOTMxNjEwJAAAADkwNTA1YzhhLTIzYTktNGNkZi1hMjZiLTRlMTlmNmEwOTdkNSQAAAAzM2YxODUwMi1hNjdjLTQ4NTMtODMzMy1hNTg2MzA2NjM1NTkkAAAAZjJmMTA2NWUtYjc4NC00NmQxLWI4MWEtYzcxYmZlYjlhZDU1JAAAADY5ZjJmOTg3LTQ2MGMtNDhhYy1hYzdhLTRkNDRkOTliMzdlNgEMAAAARG9uYWxkIFRydW1wARUAAABBIHJpZ2h0LXdpbmcgcG9wdWxpc3QAACQAAAAxODIyMDg5ZC1hZTE3LTRhMDMtODkzNS0yNTE2NGIzZjIxNDIkAAAAOTA1MDVjOGEtMjNhOS00Y2RmLWEyNmItNGUxOWY2YTA5N2Q1JAAAADMzZjE4NTAyLWE2N2MtNDg1My04MzMzLWE1ODYzMDY2MzU1OSQAAABmMmYxMDY1ZS1iNzg0LTQ2ZDEtYjgxYS1jNzFiZmViOWFkNTUkAAAANjlmMmY5ODctNDYwYy00OGFjLWFjN2EtNGQ0NGQ5OWIzN2U2AQwAAABCYXJyYWsgT2JhbWEBKgAAAEZpcnN0IEJsYWNrIHByZXNpZGVudCBhbmQgdmVyeSBjaGFyaXNtYXRpYwAAAA"#;

        let hashable_ballot: HashableBallot<RistrettoCtx> =
            Base64Deserialize::deserialize(ballot_str.to_string()).unwrap();
    }
}
