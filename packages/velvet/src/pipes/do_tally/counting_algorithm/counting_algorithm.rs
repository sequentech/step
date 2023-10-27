// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

pub use super::error::{Error, Result};
use crate::pipes::do_tally::{invalid_vote::InvalidVote, ContestResult};
use serde::Serialize;
use std::collections::HashMap;

pub trait CountingAlgorithm {
    fn tally(&self) -> Result<ContestResult>;
}
