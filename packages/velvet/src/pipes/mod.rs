pub mod decode_ballots;
pub mod error;
pub mod pipe_name;

use crate::cli::CliRun;

use self::error::Result;

use self::{
    decode_ballots::{ballot_codec::BallotCodec, DecodeBallots},
    pipe_name::PipeName,
};

trait Pipe {
    fn new(cli: &CliRun) -> Self;

    fn cli(&self) -> &CliRun;

    // pipe execution
    fn exec(&self) -> Result<()> {
        dbg!(&self.cli().config);
        dbg!(&self.cli().input_dir);
        dbg!(&self.cli().output_dir);

        // TODO: file handle to log execution process into
        // dbg!(&self.output_log_file);

        Ok(())
    }

    // load input
    // fn input(&self) -> Result<()>;

    // produce output
    // fn output(&self) -> Result<()>;
}

// TODO: rework this better
// TODO: pointeur sur fonction
pub fn match_run(cli: &CliRun, pipe: PipeName) -> Result<()> {
    match pipe {
        PipeName::DecodeBallots => {
            let pipe = DecodeBallots::new(cli);
            pipe.exec()?;
        }
        _ => {}
    };

    Ok(())
}
