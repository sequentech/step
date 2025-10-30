// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
#![allow(non_camel_case_types)]

use borsh::{BorshDeserialize, BorshSerialize};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::default::Default;
use strum_macros::{Display, EnumString};

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
pub enum KeysCeremonyExecutionStatus {
    USER_CONFIGURATION, // user can configure the ceremony at this step
    #[default]
    STARTED, /* process starts but the config message hasn't
                         * been added to the board */
    IN_PROGRESS, /* config message has been added to the board and trustees
                  * are working */
    SUCCESS,   // successful completion
    CANCELLED, // cancelation
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
pub struct KeysCeremonyStatus {
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
pub enum TallyType {
    #[default]
    #[strum(serialize = "ELECTORAL_RESULTS")]
    ELECTORAL_RESULTS,
    #[strum(serialize = "INITIALIZATION_REPORT")]
    INITIALIZATION_REPORT,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq)]
pub struct TallySessionDocuments {
    pub sqlite: Option<String>,
    pub xlsx: Option<String>,
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
pub enum CeremoniesPolicy {
    #[default]
    #[strum(serialize = "manual-ceremonies")]
    MANUAL_CEREMONIES,
    #[strum(serialize = "automated-ceremonies")]
    AUTOMATED_CEREMONIES,
}

#[derive(
    Debug,
    Display,
    EnumString,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Default,
    Serialize,
    Deserialize,
    BorshSerialize,
    BorshDeserialize,
)]
pub enum TallyOperation {
    #[strum(serialize = "process-ballots")]
    #[default]
    ProcessBallots,
    #[strum(serialize = "aggregate-results")]
    AggregateResults,
    #[strum(serialize = "participation-summary")]
    ParticipationSummary,
}

#[derive(
    Eq,
    PartialEq,
    Debug,
    EnumString,
    Display,
    Default,
    Serialize,
    Deserialize,
    BorshSerialize,
    BorshDeserialize,
    JsonSchema,
    Clone,
    Copy,
)]
pub enum CountingAlgType {
    #[strum(serialize = "plurality-at-large")]
    #[default]
    PluralityAtLarge,
    #[strum(serialize = "instant-runoff")]
    InstantRunoff,
    #[strum(serialize = "borda-nauru")]
    BordaNauru,
    #[strum(serialize = "borda")]
    Borda,
    #[strum(serialize = "borda-mas-madrid")]
    BordaMasMadrid,
    #[strum(serialize = "pairwise-beta")]
    PairwiseBeta,
    #[strum(serialize = "desborda3")]
    Desborda3,
    #[strum(serialize = "desborda2")]
    Desborda2,
    #[strum(serialize = "desborda")]
    Desborda,
    #[strum(serialize = "cumulative")]
    Cumulative,
}

impl CountingAlgType {
    /// Returns true if the counting algorithm is preferential (ranked-choice).
    pub fn is_preferential(&self) -> bool {
        match self {
            CountingAlgType::InstantRunoff
            | CountingAlgType::Borda
            | CountingAlgType::BordaNauru
            | CountingAlgType::BordaMasMadrid
            | CountingAlgType::PairwiseBeta
            | CountingAlgType::Desborda
            | CountingAlgType::Desborda2
            | CountingAlgType::Desborda3 => true,
            _ => false,
        }
    }
}
