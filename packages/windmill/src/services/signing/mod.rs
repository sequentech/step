// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Signing quorum for protected actions.

pub mod actions;
pub mod approve;
pub mod certificates;
pub mod configuration;
pub mod context;
pub mod crl;
pub mod custody_proofs;
#[cfg(feature = "comelec-demo-direct-hsm")]
pub mod direct_actions;
#[cfg(feature = "comelec-demo-direct-hsm")]
pub mod direct_initialize;
#[cfg(feature = "comelec-demo-direct-hsm")]
pub mod direct_protocol;
pub mod directory;
pub mod executors;
pub mod guard;
pub mod issuers;
#[cfg(all(
    feature = "felix3-local-initialize",
    not(feature = "operational-signing-demo-fixture")
))]
pub mod local_initialize_attachment;
pub mod log;
pub mod operational_approval;
#[cfg(feature = "operational-signing-candidate")]
pub mod operational_ca;
pub mod operational_cms;
pub mod operational_device;
#[cfg(feature = "operational-signing-candidate")]
pub mod operational_initialize;
pub mod operational_policy;
mod operational_quorum;
pub mod pades;
pub mod page_texts;
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
