// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The platform's side of the bulletin boards of crypto core
//! (`packages/wbraid`: braid, the `cryptography` crate and the b4 board
//! service).
//!
//! braid gives the protocol: a committee of trustees runs a distributed key
//! generation on a board.
//! This crate implements what the *platform* decides around it as the board's
//! protocol manager; how boards are named, how keys are written down in the
//! database and in a trustee's keys file, which trustees make up a committee,
//! and what a ceremony's state becomes when the board says a trustee has
//! published its shares, or a trustee reports a halt, etc.
//!
//! Everything but `board` is pure: no network, no database.
//!
//! # Uses
//!
//! `windmill` drives the ceremonies with it, harvest answers the trustees with
//! its request and response types, and the trustee program shares the same
//! board names, key encodings and keys file, so nothing about a board is
//! decided twice in two places.

mod board;
mod ceremony;
mod committee;
mod configuration;
mod encoding;
mod ids;
mod secrets;
mod trustee_api;
mod view;

#[cfg(test)]
mod tests;

use cryptography::context::{Context, RistrettoCtx};
use cryptography::utils::signatures::SignatureScheme;

/// The cryptographic context the platform runs the protocol in.
pub type Ctx = RistrettoCtx;
pub(crate) type Scheme = <Ctx as Context>::SignatureScheme;
pub(crate) type Rng = <Ctx as Context>::Rng;
pub(crate) type VerifyingKey = <Scheme as SignatureScheme<Rng>>::Verifier;
pub(crate) type Element = <Ctx as Context>::Element;
pub(crate) type Scalar = <Ctx as Context>::Scalar;
pub(crate) type Signer = <Scheme as SignatureScheme<Rng>>::Signer;
/// The platform's own signing identity on a board: braid calls the participant
/// that authors a `Configuration` the protocol manager, and the platform is
/// that participant.
pub type BoardManager = wbraid::protocol_manager::ProtocolManager<Ctx>;

pub use board::BoardHandle;
pub use ceremony::{
    initial_trustees, transition, CeremonyState, PlatformEvent, Timestamps,
};
pub use committee::{Committee, RawTrusteeRecord};
pub use configuration::{DkgBoard, SignedConfiguration};
pub use encoding::{encode_manager_key, generate_manager, HashHex};
pub use ids::BoardName;
pub use secrets::{TrusteePublicKeys, TrusteeSecrets};
pub use sequent_core::types::protocol_board::ProtocolBoardKind;
pub use trustee_api::{
    TrusteeBoard, TrusteeBoardsResponse, TrusteeReport, TrusteeReportKind,
};
pub use view::{DkgStatus, DkgView};
