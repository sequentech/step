// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

pub mod bases;
pub mod bigint;
pub mod character_map;
pub mod checker;
pub mod multi_ballot;
pub mod plaintext_contest;
pub mod raw_ballot;
pub mod vec;

pub use bases::*;
pub use bigint::*;
pub use character_map::*;
pub use checker::*;
pub use plaintext_contest::*;
pub use raw_ballot::*;
pub use vec::*;

pub trait BallotCodec: BasesCodec + PlaintextCodec + RawBallotCodec {}
