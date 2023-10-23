pub mod error;
pub mod pipe_inputs;
pub mod pipe_name;

// Pipes
pub mod decode_ballots;
pub mod do_tally;

use self::error::Error;
use self::pipe_inputs::PipeInputs;
use self::{decode_ballots::DecodeBallots, pipe_name::PipeName};
use crate::cli::state::Stage;
use crate::cli::CliRun;
use crate::pipes::do_tally::DoTally;
use std::error::Error as StdError;

pub trait Pipe {
    fn exec(&self) -> Result<(), Box<dyn StdError>>;
}

pub struct PipeManager;

impl PipeManager {
    pub fn new(cli: CliRun, stage: &Stage) -> Result<Option<Box<dyn Pipe>>, Error> {
        let pipe_inputs = PipeInputs::new(cli)?;

        Ok(match stage.current_pipe {
            PipeName::DecodeBallots => Some(Box::new(DecodeBallots::new(pipe_inputs))),
            PipeName::DoTally => Some(Box::new(DoTally::new(pipe_inputs))),
            _ => None,
        })
    }
}
