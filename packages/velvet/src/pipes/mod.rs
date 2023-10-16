pub mod decode_ballots;
pub mod error;
pub mod pipe_name;

use std::fs;

use crate::cli::CliRun;

use self::error::{Error, Result};

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

    fn read_input_dir_config(&self) -> Result<()> {
        let entries = fs::read_dir(format!(
            "{}/default/configs",
            &self.cli().input_dir.to_str().ok_or(Error::Toto)?
        ))?;

        // entries.map(|e| e.path()).collect::<Result<Vec<_>>>();
        for entry in entries {
            dbg!(entry?.path());
        }

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
