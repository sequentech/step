// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Voter enrollment application enums (status, type, rejection reasons, and error tags).
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString, EnumVariantNames};

/// Lifecycle state of a voter's enrollment application.
#[derive(
    Display, Debug, PartialEq, Eq, Clone, EnumString, EnumVariantNames, Serialize, Deserialize,
)]
pub enum ApplicationStatus {
    /// Awaiting review.
    PENDING,
    /// Application was approved.
    ACCEPTED,
    /// Application was denied.
    REJECTED,
}

/// Whether the application is resolved by automated rules or requires manual review.
#[derive(
    Display, Debug, PartialEq, Eq, Clone, EnumString, EnumVariantNames, Serialize, Deserialize,
)]
pub enum ApplicationType {
    /// Matched or rejected by system rules.
    AUTOMATIC,
    /// Left open for an administrator to review.
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
/// Machine-readable reason stored when an application is rejected.
pub enum ApplicationRejectReason {
    /// Submitted data did not satisfy minimum requirements for matching.
    #[strum(to_string = "insufficient-information")]
    INSUFFICIENT_INFORMATION,
    /// No voter record matched the supplied identity or credentials.
    #[strum(to_string = "no-matching-voter")]
    NO_VOTER,
    /// A voter with the same enrollment is already approved for this election.
    #[strum(to_string = "voter-already-approved")]
    ALREADY_APPROVED,
    /// Rejection reason not covered by the specific variants above.
    #[default]
    #[strum(to_string = "other")]
    OTHER, //mandatory comment
}

#[allow(non_camel_case_types)]
#[derive(
    Display, Debug, PartialEq, Eq, Clone, EnumString, EnumVariantNames, Serialize, Deserialize,
)]
/// Tags application-related failures surfaced to clients.
pub enum ApplicationsError {
    /// The target voter already has an approved application, so the action is invalid.
    #[strum(serialize = "Approved_Voter")]
    #[strum(to_string = "Approved_Voter")]
    APPROVED_VOTER,
}
