// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Signing quorum for protected actions.

pub mod approve;
pub mod certificates;
pub mod context;
pub mod crl;
pub mod directory;
pub mod executors;
pub mod guard;
pub mod issuers;
pub mod log;
pub mod pades;
pub mod pdf;
pub mod permissions;
pub mod requests;
pub mod rules;
pub mod signers;
pub mod staff_certificates;

pub use context::{
    action_title, allowed_by, allowed_by_permission, log_scope, Allowance, InvalidReason,
    PostReach, SigningCaller, SigningError, SigningResult,
};
