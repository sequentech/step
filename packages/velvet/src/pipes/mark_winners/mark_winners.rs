// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::error::Result;
use crate::pipes::{pipe_inputs::PipeInputs, Pipe};

pub struct MarkWinners {
    pub pipe_inputs: PipeInputs,
}

impl MarkWinners {
    pub fn new(pipe_inputs: PipeInputs) -> Self {
        Self { pipe_inputs }
    }
}

impl Pipe for MarkWinners {
    fn exec(&self) -> Result<()> {
        todo!()
    }
}
