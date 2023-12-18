// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
#![allow(non_camel_case_types)]

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::default::Default;
use strum_macros::{Display, EnumString};

#[derive(
    Display, Serialize, Deserialize, Debug, PartialEq, Eq, Clone, EnumString,
)]
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

#[derive(
    Display, Serialize, Deserialize, Debug, PartialEq, Eq, Clone, EnumString,
)]
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

#[derive(
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
pub enum TallyExecutionStatus {
    #[default]
    STARTED,
    CONNECTED,
    IN_PROGRESS,
    SUCCESS,
    CANCELLED,
}

#[derive(
    Display,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Eq,
    Clone,
    EnumString,
    Default,
)]
pub enum TallyTrusteeStatus {
    #[default]
    WAITING,
    KEY_RESTORED,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct TallyTrustee {
    pub name: String,
    pub status: TallyTrusteeStatus,
}

#[derive(
    Display,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Eq,
    Clone,
    EnumString,
    Default,
)]
pub enum TallyElectionStatus {
    #[default]
    WAITING,
    MIXING,
    DECRYPTING,
    SUCCESS,
    ERROR,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct TallyElection {
    pub election_id: String,
    pub status: TallyElectionStatus,
    pub progress: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct TallyCeremonyStatus {
    pub stop_date: Option<String>,
    pub logs: Vec<Log>,
    pub trustees: Vec<TallyTrustee>,
    pub elections_status: Vec<TallyElection>,
}
