// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString, EnumVariantNames};

#[derive(
    Display, Debug, PartialEq, Eq, Clone, EnumString, EnumVariantNames, Serialize, Deserialize,
)]
pub enum ApplicationStatus {
    PENDING,
    ACCEPTED,
    REJECTED,
}

#[derive(
    Display, Debug, PartialEq, Eq, Clone, EnumString, EnumVariantNames, Serialize, Deserialize,
)]
pub enum ApplicationType {
    AUTOMATIC,
    MANUAL,
}

#[allow(non_camel_case_types)]
#[derive(
    Display,
    Default,
    Debug,
    PartialEq,
    Eq,
    Clone,
    EnumString,
    EnumVariantNames,
    Serialize,
    Deserialize,
)]
pub enum ApplicationRejectReason {
    #[strum(to_string = "insufficient-information")]
    INSUFFICIENT_INFORMATION,
    #[strum(to_string = "no-matching-voter")]
    NO_VOTER,
    #[strum(to_string = "voter-already-approved")]
    ALREADY_APPROVED,
    #[default]
    #[strum(to_string = "other")]
    OTHER, //mandatory comment
}

/// What an enrollment whose identity matches no voter of the census becomes.
#[allow(non_camel_case_types)]
#[derive(
    Display,
    Default,
    Debug,
    PartialEq,
    Eq,
    Clone,
    EnumString,
    EnumVariantNames,
    Serialize,
    Deserialize,
)]
pub enum NoMatchingVoterPolicy {
    /// Rejected automatically.
    #[default]
    REJECT,
    /// Pending, for an election manager to review in the election event's
    /// Approvals.
    PENDING_APPROVAL,
}

#[allow(non_camel_case_types)]
#[derive(
    Display, Debug, PartialEq, Eq, Clone, EnumString, EnumVariantNames, Serialize, Deserialize,
)]
pub enum ApplicationsError {
    #[strum(serialize = "Approved_Voter")]
    #[strum(to_string = "Approved_Voter")]
    APPROVED_VOTER,
}
