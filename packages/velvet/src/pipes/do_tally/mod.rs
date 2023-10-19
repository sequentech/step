mod error;

use self::error::{Error, Result};

use super::{pipe_inputs::PipeInputs, Pipe};

pub struct DoTally<'a> {
    pub pipe_input: &'a PipeInputs,
}

impl<'a> Pipe<'a> for DoTally<'a> {
    type Error = Error;

    fn new(pipe_input: &'a PipeInputs) -> Self {
        Self { pipe_input }
    }

    fn exec(&self) -> Result<(), Self::Error> {
        dbg!("do tally pipe exec");
        Ok(())
    }
}
