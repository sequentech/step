// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

mod error;
pub mod instant_runoff;
pub mod plurality_at_large;
pub mod utils;

mod counting_algorithm;
pub use counting_algorithm::*;
