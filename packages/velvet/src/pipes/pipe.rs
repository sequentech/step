
use super::error::Error;
use super::pipe_inputs::PipeInputs;
use super::{decode_ballots::DecodeBallots, pipe_name::PipeName};
use crate::cli::state::Stage;
use crate::cli::CliRun;
use crate::pipes::do_tally::DoTally;
use std::error::Error as StdError;

pub trait Pipe {
    fn exec(&self) -> Result<(), Box<dyn StdError>>;
}

pub struct PipeManager;

impl PipeManager {
    pub fn new(cli: CliRun, stage: Stage) -> Result<Option<Box<dyn Pipe>>, Error> {
        let pipe_inputs = PipeInputs::new(cli, stage)?;

        Ok(match pipe_inputs.stage.current_pipe {
            PipeName::DecodeBallots => Some(Box::new(DecodeBallots::new(pipe_inputs))),
            PipeName::DoTally => Some(Box::new(DoTally::new(pipe_inputs))),
            _ => None,
        })
    }
}
