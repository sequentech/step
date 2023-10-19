mod error;

use crate::cli::CliRun;

use self::error::{Error, Result};

use super::{pipe_inputs::PipeInputs, Pipe};

pub struct DoTally {
    pub pipe_input: PipeInputs,
}

impl Pipe for DoTally {
    type Error = Error;

    fn new(cli: &CliRun) -> Result<Self, Error> {
        Ok(Self {
            pipe_input: PipeInputs::new(cli)?,
        })
    }

    fn exec(&self) -> Result<(), Self::Error> {
        dbg!("do tally pipe exec");
        Ok(())
    }
}
