// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::ballot_images::mcballot_images::MCBallotImages;
use super::ballot_images::BallotImages;
use super::decode_ballots::decode_mcballots::DecodeMCBallots;
use super::decode_ballots::DecodeBallots;
use super::error::Result;
use super::generate_reports::GenerateReports;
use super::mark_winners::MarkWinners;
use super::pipe_inputs::PipeInputs;
use super::pipe_name::PipeName;
use crate::cli::state::Stage;
use crate::cli::CliRun;
use crate::pipes::do_tally::DoTally;
use crate::pipes::generate_db::GenerateDatabase;
use tracing::instrument;

pub trait Pipe {
    fn exec(&self) -> Result<()>;
}

pub struct PipeManager;

impl PipeManager {
    #[instrument(err, skip_all, name = "PipeManager::get_pipe")]
    pub fn get_pipe(cli: CliRun, stage: Stage) -> Result<Option<Box<dyn Pipe>>> {
        let pipe_inputs = PipeInputs::new(cli, stage)?;

        if let Some(current_pipe) = pipe_inputs.stage.current_pipe {
            Ok(match current_pipe {
                PipeName::DecodeBallots => Some(Box::new(DecodeBallots::new(pipe_inputs))),
                PipeName::BallotImages => Some(Box::new(BallotImages::new(pipe_inputs))),
                PipeName::DecodeMCBallots => Some(Box::new(DecodeMCBallots::new(pipe_inputs))),
                PipeName::MCBallotReceipts => Some(Box::new(MCBallotImages::new(pipe_inputs))),
                PipeName::MCBallotImages => Some(Box::new(MCBallotImages::new(pipe_inputs))),
                PipeName::DoTally => Some(Box::new(DoTally::new(pipe_inputs))),
                PipeName::MarkWinners => Some(Box::new(MarkWinners::new(pipe_inputs))),
                PipeName::GenerateReports => Some(Box::new(GenerateReports::new(pipe_inputs))),
                PipeName::GenerateDatabase => Some(Box::new(GenerateDatabase::new(pipe_inputs))),
            })
        } else {
            Ok(None)
        }
    }
}
