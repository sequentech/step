pub mod error;
pub mod pipe_inputs;
pub mod pipe_name;

// Pipes
pub mod decode_ballots;
pub mod do_tally;

use self::error::{Error, Result};
use self::pipe_inputs::PipeInputs;
use self::{decode_ballots::DecodeBallots, pipe_name::PipeName};
use crate::cli::CliRun;
use crate::pipes::do_tally::DoTally;

trait Pipe<'a> {
    type Error;

    fn new(pipe_inputs: &'a PipeInputs) -> Result<Self, Self::Error>
    where
        Self: Sized;

    // pipe execution
    fn exec(&self) -> Result<(), Self::Error>;

    // load input
    // fn input(&self) -> Result<()>;

    // produce output
    // fn output(&self) -> Result<()>;
}

// TODO: rework this better
// TODO: pointeur sur fonction
// TODO: Error needs to be generic? DecodeBallotsError, etc...
pub fn match_run(cli: &CliRun, pipe: PipeName) -> Result<(), Error> {
    let pipe_inputs = PipeInputs::new(cli)?;

    match pipe {
        PipeName::DecodeBallots => {
            let pipe =
                DecodeBallots::new(&pipe_inputs).map_err(|e| Error::FromPipe(e.to_string()))?;
            pipe.exec().map_err(|e| Error::FromPipe(e.to_string()))?;
        }
        PipeName::DoTally => {
            let pipe = DoTally::new(&pipe_inputs).map_err(|e| Error::FromPipe(e.to_string()))?;
            pipe.exec().map_err(|e| Error::FromPipe(e.to_string()))?;
        }
        _ => {}
    };

    Ok(())
}
