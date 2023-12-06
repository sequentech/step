// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
#![allow(non_camel_case_types)]

use serde::{Deserialize, Serialize};
use strum_macros::Display;
use strum_macros::EnumString;

#[derive(Display, Serialize, Deserialize, Debug, PartialEq, Eq, Clone, EnumString)]
pub enum ExecutionStatus {
    NOT_STARTED,
    IN_PROCESS,
    SUCCESS,
    CANCELLED,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Log {
    pub created_date: String,
    pub log_text: String,
}

#[derive(Display, Serialize, Deserialize, Debug, PartialEq, Eq, Clone, EnumString)]
pub enum TrusteeStatus {
    WAITING,
    KEY_GENERATED,
    KEY_RETRIEVED,
    KEY_CHECKED,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Trustee {
    pub name: String,
    pub status: TrusteeStatus,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CeremonyStatus {
    pub stop_date: Option<String>,
    pub public_key: Option<String>,
    pub logs: Vec<Log>,
    pub trustees: Vec<Trustee>,
}


#[derive(Display, Serialize, Deserialize, Debug, PartialEq, Eq, Clone, EnumString)]
pub enum TallyExecutionStatus {
    NOT_STARTED,
    IN_PROCESS,
    SUCCESS,
    CANCELLED,
}


#[derive(Display, Serialize, Deserialize, Debug, PartialEq, Eq, Clone, EnumString)]
pub enum TallyTrusteeStatus {
    WAITING,
    KEY_RESTORED,
    KEY_CHECKED,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TallyTrustee {
    pub name: String,
    pub status: TallyTrusteeStatus,
}

#[derive(Display, Serialize, Deserialize, Debug, PartialEq, Eq, Clone, EnumString)]
pub enum TallyElectionStatus {
    WAITING,
    MIXING,
    DECRYPTING,
    COUNTING,
    SUCCESS,
    ERROR,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TallyElection {
    election_id: String,
    status: TallyElectionStatus,
    progress: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TallyCeremonyStatus {
    pub stop_date: Option<String>,
    pub public_key: Option<String>,
    pub logs: Vec<Log>,
    pub trustees: Vec<TallyTrustee>,
    pub elections_status: Vec<TallyElection>,
}
