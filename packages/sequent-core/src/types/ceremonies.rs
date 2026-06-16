// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
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
    Copy,
    EnumString,
)]
pub enum KeysCeremonyExecutionStatus {
    AWAITING_TRUSTEE_KEYS, /* waiting for every selected trustee's key to be
                            * registered; config message not yet added to
                            * the board */
    IN_PROGRESS, /* config message has been added to the board and trustees
                  * are working */
    SUCCESS,   // successful completion
    CANCELLED, // cancelation
}

/// One error type for every illegal move. Carries enough context for a
/// descriptive Harvest response (e.g. mapped to a generic
/// `INVALID_CEREMONY_TRANSITION` error body).
#[derive(Debug, PartialEq, Eq)]
pub struct InvalidTransition {
    pub from: KeysCeremonyExecutionStatus,
    pub to: KeysCeremonyExecutionStatus,
}

impl std::fmt::Display for InvalidTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid ceremony transition {:?} -> {:?}", self.from, self.to)
    }
}
impl std::error::Error for InvalidTransition {}

impl KeysCeremonyExecutionStatus {
    /// Validate a requested transition. Returns the target status on success
    /// so callers can write it straight to the DB:
    ///
    /// ```ignore
    /// let next = current.try_transition(IN_PROGRESS)?;
    /// update_execution_status(ceremony_id, next).await?;
    /// ```
    pub fn try_transition(
        self,
        to: KeysCeremonyExecutionStatus,
    ) -> std::result::Result<KeysCeremonyExecutionStatus, InvalidTransition> {
        use KeysCeremonyExecutionStatus::*;

        let ok = matches!(
            (self, to),
            // forward progress: AWAITING_TRUSTEE_KEYS jumps straight to
            // IN_PROGRESS because the beat task that gates on key
            // availability also posts the Configuration message in the
            // same step.
            (AWAITING_TRUSTEE_KEYS, IN_PROGRESS)
                | (IN_PROGRESS, SUCCESS)
                // cancellation. SUCCESS -> CANCELLED is allowed at the enum
                // level; the caller (cancel endpoint) is responsible for
                // additionally checking that no election in the event has
                // started its voting period before invoking it.
                | (AWAITING_TRUSTEE_KEYS, CANCELLED)
                | (IN_PROGRESS, CANCELLED)
                | (SUCCESS, CANCELLED)
        );

        if ok { Ok(to) } else { Err(InvalidTransition { from: self, to }) }
    }

    pub fn is_terminal(self) -> bool {
        matches!(self, KeysCeremonyExecutionStatus::CANCELLED)
    }
}

#[cfg(test)]
mod keys_ceremony_execution_status_tests {
    use super::KeysCeremonyExecutionStatus::*;

    #[test]
    fn happy_path() {
        assert_eq!(AWAITING_TRUSTEE_KEYS.try_transition(IN_PROGRESS), Ok(IN_PROGRESS));
        assert_eq!(IN_PROGRESS.try_transition(SUCCESS), Ok(SUCCESS));
    }

    #[test]
    fn cancellation_arms() {
        assert!(AWAITING_TRUSTEE_KEYS.try_transition(CANCELLED).is_ok());
        assert!(IN_PROGRESS.try_transition(CANCELLED).is_ok());
        assert!(SUCCESS.try_transition(CANCELLED).is_ok()); // caller must additionally
                                                             // verify voting has not started
    }

    #[test]
    fn cancelled_is_terminal() {
        assert!(CANCELLED.try_transition(IN_PROGRESS).is_err());
        assert!(CANCELLED.try_transition(SUCCESS).is_err());
        assert!(CANCELLED.try_transition(CANCELLED).is_err());
    }

    #[test]
    fn success_cannot_progress_forward() {
        assert!(SUCCESS.try_transition(IN_PROGRESS).is_err());
        assert!(SUCCESS.try_transition(AWAITING_TRUSTEE_KEYS).is_err());
    }

    #[test]
    fn round_trips_through_serde_as_the_db_would() {
        let s = serde_json::to_string(&AWAITING_TRUSTEE_KEYS).unwrap();
        let back: super::KeysCeremonyExecutionStatus = serde_json::from_str(&s).unwrap();
        assert_eq!(back.try_transition(IN_PROGRESS), Ok(IN_PROGRESS));
    }
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
    /// Snapshot of the public key actually used to build the on-board
    /// `Configuration` for this ceremony, written once by `create_keys_impl`
    /// at the AWAITING_TRUSTEE_KEYS -> IN_PROGRESS transition. Frozen from
    /// that point on — never re-read from or overwritten by the live
    /// `sequent_backend.trustee.public_key` column, which other ceremonies
    /// may later overwrite. `#[serde(default)]` so ceremonies created before
    /// this field existed still deserialize.
    #[serde(default)]
    pub public_key: Option<String>,
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
pub enum CeremoniesPolicy {
    #[default]
    #[strum(serialize = "manual-ceremonies")]
    #[serde(rename = "manual-ceremonies")]
    MANUAL_CEREMONIES,
    #[strum(serialize = "automated-ceremonies")]
    #[serde(rename = "automated-ceremonies")]
    AUTOMATED_CEREMONIES,
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
    Copy,
)]
pub enum TrusteeModePolicy {
    #[default]
    #[strum(serialize = "browser-based")]
    #[serde(rename = "browser-based")]
    BROWSER_BASED,
    #[strum(serialize = "server-based")]
    #[serde(rename = "server-based")]
    SERVER_BASED,
}

#[derive(
    Debug,
    Display,
    EnumString,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    BorshSerialize,
    BorshDeserialize,
)]
pub enum TallyOperation {
    #[strum(serialize = "process-ballots-all")]
    ProcessBallotsAll, /* Process ballots to calculate Candidate Results
                        * and participation
                        * statistics */
    #[strum(serialize = "aggregate-results")]
    AggregateResults, /* Aggregate results that have been processed in
                       * every area */
    #[strum(serialize = "skip-candidate-results")]
    SkipCandidateResults, /* Needs the ballots to calculate participation
                           * statistics but without the Candidate Results */
}

#[derive(Debug, Display)]
pub enum ScopeOperation {
    Area(TallyOperation),
    Contest(TallyOperation),
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
    #[serde(rename = "plurality-at-large")]
    #[default]
    PluralityAtLarge,
    #[strum(serialize = "instant-runoff")]
    #[serde(rename = "instant-runoff")]
    InstantRunoff,
    #[strum(serialize = "borda-nauru")]
    #[serde(rename = "borda-nauru")]
    BordaNauru,
    #[strum(serialize = "borda")]
    #[serde(rename = "borda")]
    Borda,
    #[strum(serialize = "borda-mas-madrid")]
    #[serde(rename = "borda-mas-madrid")]
    BordaMasMadrid,
    #[strum(serialize = "pairwise-beta")]
    #[serde(rename = "pairwise-beta")]
    PairwiseBeta,
    #[strum(serialize = "desborda3")]
    #[serde(rename = "desborda3")]
    Desborda3,
    #[strum(serialize = "desborda2")]
    #[serde(rename = "desborda2")]
    Desborda2,
    #[strum(serialize = "desborda")]
    #[serde(rename = "desborda")]
    Desborda,
    #[strum(serialize = "cumulative")]
    #[serde(rename = "cumulative")]
    Cumulative,
}

impl CountingAlgType {
    /// Returns true if the counting algorithm is preferential (ranked-choice).
    pub fn is_preferential(&self) -> bool {
        matches!(
            self,
            CountingAlgType::InstantRunoff
                | CountingAlgType::Borda
                | CountingAlgType::BordaNauru
                | CountingAlgType::BordaMasMadrid
                | CountingAlgType::PairwiseBeta
                | CountingAlgType::Desborda
                | CountingAlgType::Desborda2
                | CountingAlgType::Desborda3
        )
    }

    pub fn get_default_tally_operation_for_contest(&self) -> TallyOperation {
        if self.is_preferential() {
            TallyOperation::ProcessBallotsAll
        } else {
            TallyOperation::AggregateResults
        }
    }

    pub fn get_default_tally_operation_for_area(&self) -> TallyOperation {
        if self.is_preferential() {
            TallyOperation::SkipCandidateResults
        } else {
            TallyOperation::ProcessBallotsAll
        }
    }
}

/// Whether a trustee session is currently reachable.
#[derive(
    Display,
    Serialize,
    Deserialize,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    EnumString,
    Default,
)]
pub enum TrusteeSessionStatus {
    #[strum(serialize = "ACTIVE")]
    #[serde(rename = "ACTIVE")]
    ACTIVE,
    #[default]
    #[strum(serialize = "NOT_ACTIVE")]
    #[serde(rename = "NOT_ACTIVE")]
    NOT_ACTIVE,
}

/// Body sent by a trustee to B4's `POST /sessions/heartbeat`.
#[derive(Debug, Serialize, Deserialize)]
pub struct HeartbeatRequest {
    pub board_name: String,
    pub sender_pk: String,
    pub trustee_name: String,
    pub trustee_mode: TrusteeModePolicy,
}

/// A single trustee session as returned by `GET /sessions`.
#[derive(Debug, Serialize, Deserialize)]
pub struct TrusteeSessionResponse {
    pub board_name: String,
    pub sender_pk: String,
    pub trustee_name: String,
    pub trustee_mode: TrusteeModePolicy,
    pub status: TrusteeSessionStatus,
}

/// Response body for `GET /sessions`.
#[derive(Debug, Serialize, Deserialize)]
pub struct SessionsListResponse {
    pub sessions: Vec<TrusteeSessionResponse>,
}
