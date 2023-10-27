// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

pub mod error;
pub mod pipe_inputs;
pub mod pipe_name;

// Pipes
pub mod decode_ballots;
pub mod do_tally;
pub mod mark_winners;

mod pipes;
pub use pipes::*;
