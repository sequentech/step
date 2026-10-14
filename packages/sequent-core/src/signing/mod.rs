// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Signing of protected actions with staff certificates: the action catalog,
//! the per-event rules and checks, the canonical payload that every signer
//! signs and the short signing code that signers compare aloud.

pub mod canonical;
pub mod code;
pub mod types;

pub use canonical::*;
pub use code::*;
pub use types::*;
