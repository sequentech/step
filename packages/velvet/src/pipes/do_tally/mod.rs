mod error;

use super::{pipe_inputs::PipeInputs, Pipe};
use std::error::Error as StdError;

pub struct DoTally {
    pub pipe_inputs: PipeInputs,
}

impl DoTally {
    pub fn new(pipe_inputs: PipeInputs) -> Self {
        Self { pipe_inputs }
    }
}

impl Pipe for DoTally {
    fn exec(&self) -> Result<(), Box<dyn StdError>> {
        dbg!("do tally pipe exec");
        Ok(())
    }
}
