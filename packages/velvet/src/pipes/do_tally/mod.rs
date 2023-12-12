// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

pub mod invalid_vote;

mod counting_algorithm;
mod error;
mod tally;

mod do_tally;
pub use do_tally::*;
