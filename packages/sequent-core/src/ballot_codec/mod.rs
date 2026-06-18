// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Mixed-radix encoding and decoding of ballot choices for encryption.

/// Radix base calculation for each ballot position.
pub mod bases;
/// Big-integer representation of encoded ballot contests.
pub mod bigint;
/// Character encoding for write-in candidate text.
pub mod character_map;
/// Validation helpers for encoded ballots.
pub mod checker;
/// Multi-contest ballot encoding.
pub mod multi_ballot;
/// Plaintext contest encode/decode.
pub mod plaintext_contest;
/// Raw mixed-radix digit vectors before bigint packing.
pub mod raw_ballot;
/// Vector utilities for ballot encoding.
pub mod vec;

pub use bases::*;
pub use bigint::*;
pub use character_map::*;
pub use checker::*;
pub use plaintext_contest::*;
pub use raw_ballot::*;
pub use vec::*;

/// Full ballot codec combining bases, plaintext, and raw-ballot operations.
pub trait BallotCodec: BasesCodec + PlaintextCodec + RawBallotCodec {}
