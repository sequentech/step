// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

pub use super::error::{Error, Result};
use crate::pipes::do_tally::ContestResult;

pub trait CountingAlgorithm {
    fn tally(&self) -> Result<ContestResult>;
}
