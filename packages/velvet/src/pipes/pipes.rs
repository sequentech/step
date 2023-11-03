// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::decode_ballots::DecodeBallots;
use super::error::Error;
use super::generate_reports::GenerateReports;
use super::mark_winners::MarkWinners;
use super::pipe_inputs::PipeInputs;
use super::pipe_name::PipeName;
use crate::cli::state::Stage;
use crate::cli::CliRun;
use crate::pipes::do_tally::DoTally;
use std::error::Error as StdError;

pub trait Pipe {
    fn exec(&self) -> Result<(), Box<dyn StdError>>;
}

pub struct PipeManager;

impl PipeManager {
    pub fn get_pipe(cli: CliRun, stage: Stage) -> Result<Option<Box<dyn Pipe>>, Error> {
        let pipe_inputs = PipeInputs::new(cli, stage)?;

        Ok(match pipe_inputs.stage.current_pipe {
            PipeName::DecodeBallots => Some(Box::new(DecodeBallots::new(pipe_inputs))),
            PipeName::DoTally => Some(Box::new(DoTally::new(pipe_inputs))),
            PipeName::MarkWinners => Some(Box::new(MarkWinners::new(pipe_inputs))),
            PipeName::GenerateReports => Some(Box::new(GenerateReports::new(pipe_inputs))),
        })
    }
}
