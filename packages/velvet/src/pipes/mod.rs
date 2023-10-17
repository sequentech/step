pub mod PipeInputs;
pub mod decode_ballots;
pub mod error;
pub mod pipe_name;

use self::error::{Error, Result};
use self::{decode_ballots::DecodeBallots, pipe_name::PipeName};
use crate::cli::CliRun;

trait Pipe {
    type Error;

    fn new(cli: &CliRun) -> Result<Self, Self::Error>
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
pub fn match_run(cli: &CliRun, pipe: PipeName) -> Result<(), Error> {
    match pipe {
        PipeName::DecodeBallots => {
            let pipe = DecodeBallots::new(cli).map_err(|e| Error::FromPipe(e.to_string()))?;
            pipe.exec().map_err(|e| Error::FromPipe(e.to_string()))?;
        }
        _ => {}
    };

    Ok(())
}
